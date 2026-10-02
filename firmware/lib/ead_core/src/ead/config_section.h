#pragma once
// CONFIG_GET section: the fixed V1 configuration compiled into the firmware
// (include/config_v1.h), serialised in the order documented in
// docs/protocol.md. Its SHA-256 is the configuration hash reported in HELLO
// (doc 18: log the configuration hash with every session).

#include <cstddef>
#include <cstdint>

namespace ead {

// Format 2: the BNO086/SPI build (DEC-016, DEC-017). Format 1 was the MPU6500 one.
constexpr uint16_t kConfigFormat = 2;

constexpr uint8_t kSensorKindBno086 = 1;

// Returns the section length, or 0 if `cap` is too small.
size_t encodeConfigSection(uint8_t* out, size_t cap);

}  // namespace ead
