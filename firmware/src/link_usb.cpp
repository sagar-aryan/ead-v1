#include "link_usb.h"

#include <algorithm>

#include <esp_timer.h>
#include <freertos/FreeRTOS.h>
#include <freertos/task.h>
#include <hal/usb_serial_jtag_ll.h>

#include "device.h"
#include "ead/protocol.h"
#include "link.h"

// Arduino's HWCDC Serial loses and mangles data on ESP32-S3 under sustained
// writes (arduino-esp32 issues #9378, #11959; measured here in PROB-006). This
// link never starts Serial and drives the USB Serial/JTAG IN endpoint itself,
// one packet at a time: fill up to 64 bytes, flush, then write nothing until the
// peripheral reports the endpoint empty (the host collected the packet).
// Appending to a flushed but uncollected packet duplicated bytes on the wire.

namespace {

Link s_link(ead::kLinkUsbActive);
ead::UsbFrameDecoder s_decoder;
uint8_t s_message[ead::kMaxMessageSize];
uint8_t s_frame[ead::kMaxUsbFrameSize];

constexpr uint32_t kPacketSize = 64;               // USB full-speed bulk endpoint
constexpr TickType_t kInFlightDelay = 1;           // ms; bounds throughput to ~64 kB/s
constexpr TickType_t kIdleDelay = pdMS_TO_TICKS(5);
// A host that has sent nothing for this long is gone (port closed or cable out).
// Its uncollected packet is abandoned; the fragment fails CRC at the next host.
constexpr int64_t kAbandonAfterUs = 3000000;

bool inEndpointEmpty() { return USB_SERIAL_JTAG.int_raw.serial_in_empty_int_raw; }

void usbTask(void*) {
  size_t frameLen = 0;  // framed message being written
  size_t framePos = 0;  // bytes of it already placed in packets
  bool inFlight = false;
  uint8_t rx[kPacketSize];

  for (;;) {
    const int64_t now = esp_timer_get_time();

    while (usb_serial_jtag_ll_rxfifo_data_available()) {
      const uint32_t n = usb_serial_jtag_ll_read_rxfifo(rx, sizeof rx);
      for (uint32_t i = 0; i < n; i++) {
        if (s_decoder.feed(rx[i])) s_link.onMessage(s_decoder.message(), s_decoder.length(), now);
      }
    }

    if (inFlight) {
      if (inEndpointEmpty()) {
        inFlight = false;
      } else if (s_link.silentFor(now, kAbandonAfterUs)) {
        inFlight = false;
        frameLen = 0;
        framePos = 0;
      }
    }

    if (!inFlight) {
      uint32_t packetBytes = 0;
      while (packetBytes < kPacketSize) {
        if (framePos == frameLen) {
          const size_t len = s_link.peek(s_message, sizeof s_message, now);
          if (len == 0) break;
          frameLen = ead::encodeUsbFrame(s_message, len, s_frame, sizeof s_frame);
          framePos = 0;
        }
        const uint32_t want = uint32_t(std::min<size_t>(frameLen - framePos, kPacketSize - packetBytes));
        const uint32_t accepted = usb_serial_jtag_ll_write_txfifo(s_frame + framePos, want);
        framePos += accepted;
        packetBytes += accepted;
        // Committed once fully packetised; a packet lost with a vanished host is
        // recovered by the host's backfill (durable data) or retry (replies).
        if (framePos == frameLen) s_link.commit(now);
        if (accepted < want) break;
      }
      if (packetBytes > 0) {
        usb_serial_jtag_ll_clr_intsts_mask(USB_SERIAL_JTAG_INTR_SERIAL_IN_EMPTY);
        usb_serial_jtag_ll_txfifo_flush();
        inFlight = true;
      }
    }

    device::setLinkActive(s_link.activeFlag(), s_link.streaming(now));
    vTaskDelay(inFlight || framePos < frameLen ? kInFlightDelay : kIdleDelay);
  }
}

}  // namespace

void startUsbLink() {
  TaskHandle_t handle = nullptr;
  xTaskCreatePinnedToCore(usbTask, "usb_link", 6144, nullptr, 5, &handle, 0);
  device::registerTask(device::TaskRole::Usb, handle);
}
