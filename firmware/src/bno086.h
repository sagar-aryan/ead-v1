#pragma once
// The two BNO086 sensors on the shared SPI bus (DEC-016, DEC-017). After boot
// only the acquisition task calls into here.

#include <cstddef>
#include <cstdint>

#include "ead/protocol.h"
#include "ead/sh2.h"

namespace bno086 {

enum Sensor : uint8_t { kFoot = 0, kShank = 1 };

/// Puts every sensor line in its idle state: both CS high, WAKE high, RST high,
/// pull-ups on both INT inputs (wiring rules 2, 3 and 6), then starts SPI.
void beginBus();

bool intAsserted(Sensor s);

/// Resets both sensors, checks each line of each one (the flags in
/// ead::SensorCheckFlag), and turns their accelerometer, gyroscope and game
/// rotation vector reports on. Takes up to about two seconds.
void resetAndCheck(ead::SensorCheck* foot, ead::SensorCheck* shank);

struct ReadResult {
  size_t samples;   // input reports written to `out`
  bool resetSeen;   // the sensor reported a reset: its reports are off again
  bool bad;         // the bytes read were not a valid packet
};

/// Reads one packet from a sensor whose INT is asserted. `intUs` is when that
/// INT was asserted; sample times are worked out from it.
ReadResult read(Sensor s, int64_t intUs, ead::sh2::Sample* out, size_t cap);

/// Turns the accelerometer, gyroscope and game rotation vector reports on at
/// `intervalUs` (again, after a sensor reset itself). False if the sensor never
/// became ready to receive.
bool enableReports(Sensor s, uint32_t intervalUs);

/// The interval the sensor's hub chose for a report, from its confirmation; 0
/// before one arrived. The hub rounds a request to a rate its part supports.
uint32_t reportIntervalUs(Sensor s, uint8_t reportId);

}  // namespace bno086
