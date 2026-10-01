# firmware/test — Unity tests, run on the host

`pio test -e native` builds `lib/ead_core` for the host and runs seven suites,
63 tests:

| Suite | Tests | Covers |
|---|---:|---|
| `test_protocol` | 17 | Codec and USB framing against `../../protocol/vectors/` |
| `test_reference` | 11 | Reference builder (median/MAD) and the error engine |
| `test_gait` | 10 | Gait state machine, events, ZUPT, cycle features |
| `test_mahony` | 8 | Mahony filter and quaternion helpers |
| `test_calibration` | 7 | Still-window calibration and its reject rules |
| `test_crc_cobs` | 5 | CRC-32 and COBS |
| `test_msg_ring` | 5 | Durable message ring |

`vectors.h` loads the golden vectors through `EAD_VECTORS_DIR`, set in
`platformio.ini`.

Determinism (doc 13): no wall-clock time, no unseeded randomness, no network;
canned inputs in, exact expected values out.
