#pragma once
// Wi-Fi link (doc 08): the device runs an access point at 192.168.4.1 and serves
// binary WebSocket messages at ws://192.168.4.1:8080/ws through ESP-IDF
// esp_http_server (DEC-010). One client at a time; a new connection replaces
// the previous one.

void startWifiLink();
