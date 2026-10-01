// Measures what the ROM and bootloader leave on each pad before user code runs:
// pull-up / pull-down / input-enable / drive strength / function select, plus the
// JTAG eFuse that decides whether GPIO39 keeps a pull-up.
#include <Arduino.h>
#include <hal/usb_serial_jtag_ll.h>
#include <soc/io_mux_reg.h>
#include <esp_efuse.h>
#include <esp_efuse_table.h>

static bool inFlight = false;

static void usbWrite(const char* d, size_t len) {
  size_t pos = 0;
  while (pos < len) {
    if (inFlight) {
      uint32_t w = 0;
      while (!USB_SERIAL_JTAG.int_raw.serial_in_empty_int_raw && w < 200) { delay(1); w++; }
      if (!USB_SERIAL_JTAG.int_raw.serial_in_empty_int_raw) return;
      inFlight = false;
    }
    const size_t want = len - pos < 64 ? len - pos : 64;
    const uint32_t n = usb_serial_jtag_ll_write_txfifo((const uint8_t*)(d + pos), want);
    if (n == 0) return;
    pos += n;
    usb_serial_jtag_ll_clr_intsts_mask(USB_SERIAL_JTAG_INTR_SERIAL_IN_EMPTY);
    usb_serial_jtag_ll_txfifo_flush();
    inFlight = true;
  }
}

static char buf[160];
static String report;

static void line(const char* fmt, ...) {
  va_list ap; va_start(ap, fmt);
  const int n = vsnprintf(buf, sizeof buf, fmt, ap);
  va_end(ap);
  report += buf;
  usbWrite(buf, n);
}

static const int pins[] = {1,2,3,4,5,6,7,8,9,43,44,39,40,41,42};
static const size_t kCount = sizeof(pins) / sizeof(pins[0]);

void setup() {
  esp_log_level_set("*", ESP_LOG_NONE);
  // Snapshot every pad of interest BEFORE anything configures them.
  uint32_t reg[kCount];
  int level[kCount];
  for (size_t i = 0; i < kCount; i++) {
    reg[i] = REG_READ(GPIO_PIN_MUX_REG[pins[i]]);
    level[i] = digitalRead(pins[i]);
  }
  // The datasheet's EFUSE_DIS_PAD_JTAG is HARD_DIS_JTAG in ESP-IDF's table.
  const bool hardDisJtag = esp_efuse_read_field_bit(ESP_EFUSE_HARD_DIS_JTAG);
  const bool disUsbJtag = esp_efuse_read_field_bit(ESP_EFUSE_DIS_USB_JTAG);
  const bool strapJtagSel = esp_efuse_read_field_bit(ESP_EFUSE_STRAP_JTAG_SEL);

  delay(1500);
  line("# pad state as the bootloader left it\n");
  line("# EFUSE HARD_DIS_JTAG(=DIS_PAD_JTAG)=%d DIS_USB_JTAG=%d STRAP_JTAG_SEL=%d\n",
       hardDisJtag, disUsbJtag, strapJtagSel);
  line("gpio  reg         WPU WPD IE DRV FUNC level\n");
  for (size_t i = 0; i < kCount; i++) {
    line("%-5d 0x%08lx  %d   %d   %d  %lu   %lu    %d\n", pins[i], (unsigned long)reg[i],
         (reg[i] & FUN_PU) ? 1 : 0, (reg[i] & FUN_PD) ? 1 : 0, (reg[i] & FUN_IE) ? 1 : 0,
         (unsigned long)((reg[i] >> FUN_DRV_S) & FUN_DRV),
         (unsigned long)((reg[i] >> MCU_SEL_S) & MCU_SEL), level[i]);
  }
  line("# done\n");
}

void loop() {
  if (usb_serial_jtag_ll_rxfifo_data_available()) {
    uint8_t d[64];
    while (usb_serial_jtag_ll_rxfifo_data_available()) usb_serial_jtag_ll_read_rxfifo(d, 64);
    usbWrite(report.c_str(), report.length());
  }
  delay(50);
}
