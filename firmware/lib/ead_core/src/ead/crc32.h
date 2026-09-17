#pragma once

#include <cstddef>
#include <cstdint>

namespace ead {

// IEEE 802.3 CRC-32 (reflected polynomial 0xEDB88320, init and final XOR
// 0xFFFFFFFF); matches zlib.crc32.
uint32_t crc32(const uint8_t* data, size_t len);

}  // namespace ead
