# 03 — Final XIAO ESP32-S3 GPIO Map

## 1. Pin map
The pin assignment below is fixed for V1. It intentionally avoids ESP32-S3 strapping pins GPIO0/GPIO3/GPIO45/GPIO46 and avoids GPIO19/20 USB-JTAG use. ESP32-S3 peripheral signals can be routed through its GPIO matrix; the selected pins are used as ordinary GPIO/peripheral endpoints. Espressif identifies GPIO0, GPIO3, GPIO45 and GPIO46 as strapping pins, and GPIO19/20 as USB-JTAG defaults. citeturn346183search0turn346183search1

| Function | XIAO pin | ESP32-S3 GPIO | Direction |
|---|---|---:|---|
| I²C SDA | D4 | GPIO5 | I/O |
| I²C SCL | D5 | GPIO6 | Output/open-drain peripheral |
| Foot MPU INT | D8 | GPIO7 | Input |
| Shank MPU INT | D9 | GPIO8 | Input |
| Motor 1 PWM | D0 | GPIO1 | Output |
| Motor 2 PWM | D1 | GPIO2 | Output |
| Motor 3 PWM | D3 | GPIO4 | Output |
| Motor 4 PWM | D10 | GPIO9 | Output |
| Motor 5 PWM | D6 | GPIO43 | Output |
| Motor 6 PWM | D7 | GPIO44 | Output |

### Why GPIO43/44 are used
GPIO43/44 are normally associated with UART0 TX/RX on the ESP32-S3 module, but V1 does not use UART0 as a board-level communication interface. Native USB is used for development/logging. Therefore GPIO43/44 are available for the last two PWM channels without consuming strapping pins or USB-JTAG GPIO19/20. Espressif documents UART0 on GPIO43/44 for UART boot/download. citeturn346183search4

## 2. Reserved / do-not-use pins
- GPIO0: reserved/strapping.
- GPIO3: reserved/strapping.
- GPIO19/20: leave for native USB/USB-JTAG functions; do not use for motors.
- GPIO26–37: leave unused because they are associated with SPI flash/PSRAM on ESP32-S3 module variants and are not recommended for general use. citeturn346183search0
- GPIO45/46: reserved/strapping.
- GPIO43/44: dedicated to Motor 5/6 in V1; no UART0.

## 3. No battery/switch GPIO
No battery ADC is present. No switch-status GPIO is present. Do not add either to firmware configuration.

## 4. Haptic output initialization
At boot, configure all six motor GPIOs as outputs and drive them LOW before enabling PWM timers. Haptic outputs remain OFF until the full firmware enters READY and all sensor health checks pass.
