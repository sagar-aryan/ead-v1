#include "link_wifi.h"

#include <atomic>
#include <cstdio>

#include <WiFi.h>
#include <esp_http_server.h>
#include <esp_timer.h>
#include <esp_wifi.h>
#include <lwip/sockets.h>

#include "config_v1.h"
#include "device.h"
#include "ead/protocol.h"
#include "ead_secrets.h"
#include "link.h"

namespace {

// Fixed so the laptop reconnects to a known channel; not part of the contract.
constexpr int kApChannel = 6;
constexpr int kApMaxStations = 2;
constexpr uint16_t kCtrlPort = 32769;
// A client that has sent nothing (not even keepalives) for this long is closed.
constexpr int64_t kSilentCloseUs = 10000000;
constexpr int kMaxMessagesPerPass = 32;

httpd_handle_t s_server = nullptr;
std::atomic<int> s_clientFd{-1};
std::atomic<bool> s_pumpQueued{false};

// Link state, rx and tx buffers are only touched from the HTTP server task:
// the WebSocket handler, the close callback and the queued pump all run there.
Link s_link(ead::kLinkWifiActive);
uint8_t s_rx[512];
uint8_t s_tx[ead::kMaxMessageSize];

bool socketWritable(int fd) {
  fd_set writable;
  FD_ZERO(&writable);
  FD_SET(fd, &writable);
  timeval now = {0, 0};
  return select(fd + 1, nullptr, &writable, nullptr, &now) > 0;
}

void pump(void*) {
  s_pumpQueued = false;
  const int fd = s_clientFd.load();
  if (fd < 0 || httpd_ws_get_fd_info(s_server, fd) != HTTPD_WS_CLIENT_WEBSOCKET) return;

  const int64_t now = esp_timer_get_time();
  if (s_link.silentFor(now, kSilentCloseUs)) {
    httpd_sess_trigger_close(s_server, fd);
    return;
  }
  // lwIP reports writable only with more than 2880 bytes free, and every
  // message is at most ead::kMaxMessageSize, so a send after select never blocks.
  for (int i = 0; i < kMaxMessagesPerPass; i++) {
    const size_t len = s_link.peek(s_tx, sizeof s_tx, now);
    if (len == 0 || !socketWritable(fd)) break;
    httpd_ws_frame_t frame = {};
    frame.final = true;
    frame.type = HTTPD_WS_TYPE_BINARY;
    frame.payload = s_tx;
    frame.len = len;
    if (httpd_ws_send_frame_async(s_server, fd, &frame) != ESP_OK) {
      httpd_sess_trigger_close(s_server, fd);
      break;
    }
    s_link.commit(now);
  }
  device::setLinkActive(s_link.activeFlag(), s_link.streaming(now));
  device::setWifiStackFree(uint16_t(uxTaskGetStackHighWaterMark(nullptr)));
}

esp_err_t onWebSocket(httpd_req_t* req) {
  const int fd = httpd_req_to_sockfd(req);
  if (req->method == HTTP_GET) {
    // Handshake completed; this client replaces any previous one.
    const int previous = s_clientFd.exchange(fd);
    if (previous >= 0 && previous != fd) httpd_sess_trigger_close(s_server, previous);
    int noDelay = 1;
    setsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &noDelay, sizeof noDelay);
    s_link.onConnect(esp_timer_get_time());
    return ESP_OK;
  }

  httpd_ws_frame_t frame = {};
  esp_err_t err = httpd_ws_recv_frame(req, &frame, 0);
  if (err != ESP_OK) return err;
  if (frame.len > sizeof s_rx) return ESP_FAIL;  // host commands are small; drop the client
  frame.payload = s_rx;
  err = httpd_ws_recv_frame(req, &frame, frame.len);
  if (err != ESP_OK) return err;
  if (frame.type == HTTPD_WS_TYPE_BINARY && fd == s_clientFd.load()) {
    s_link.onMessage(s_rx, frame.len, esp_timer_get_time());
  }
  return ESP_OK;
}

void onClose(httpd_handle_t, int fd) {
  int expected = fd;
  if (s_clientFd.compare_exchange_strong(expected, -1)) {
    s_link.onDisconnect();
    device::setLinkActive(s_link.activeFlag(), false);
  }
  close(fd);
}

void pumpScheduler(void*) {
  for (;;) {
    if (s_clientFd.load() >= 0 && !s_pumpQueued.exchange(true)) {
      if (httpd_queue_work(s_server, pump, nullptr) != ESP_OK) s_pumpQueued = false;
    }
    vTaskDelay(pdMS_TO_TICKS(10));
  }
}

}  // namespace

void startWifiLink() {
  const uint8_t* mac = device::mac();
  char ssid[16];
  std::snprintf(ssid, sizeof ssid, "EAD-V1-%02X%02X", mac[4], mac[5]);

  WiFi.persistent(false);
  WiFi.mode(WIFI_AP);
  WiFi.softAPConfig(IPAddress(EAD_NET_AP_IP_0, EAD_NET_AP_IP_1, EAD_NET_AP_IP_2, EAD_NET_AP_IP_3),
                    IPAddress(EAD_NET_AP_IP_0, EAD_NET_AP_IP_1, EAD_NET_AP_IP_2, EAD_NET_AP_IP_3),
                    IPAddress(255, 255, 255, 0));
  WiFi.softAP(ssid, EAD_AP_PASSPHRASE, kApChannel, 0, kApMaxStations);
  esp_wifi_set_ps(WIFI_PS_NONE);

  httpd_config_t config = HTTPD_DEFAULT_CONFIG();
  config.server_port = EAD_NET_PORT;
  config.ctrl_port = kCtrlPort;
  config.core_id = 0;
  config.task_priority = 5;
  config.stack_size = 6144;
  config.max_open_sockets = 3;
  config.lru_purge_enable = true;
  config.send_wait_timeout = 1;
  config.close_fn = onClose;
  if (httpd_start(&s_server, &config) != ESP_OK) return;

  httpd_uri_t ws = {};
  ws.uri = EAD_NET_WS_PATH;
  ws.method = HTTP_GET;
  ws.handler = onWebSocket;
  ws.is_websocket = true;
  httpd_register_uri_handler(s_server, &ws);

  xTaskCreatePinnedToCore(pumpScheduler, "wifi_pump", 2048, nullptr, 4, nullptr, 0);
}
