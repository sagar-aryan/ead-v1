# firmware/test — Unity replay tests (placeholder)

Determinism requirement (doc 13, TEST_AND_VALIDATION_PLAN):
- All codec/algorithm replay tests MUST be deterministic.
- Fixed PRNG seeds; golden vectors checked into the repo.
- No wall-clock time, no unseeded randomness, no network in tests.
- Raw-frame replay: canned ADC-count frames in, exact expected structs out.

`test_codec_placeholder.c` reserves the Unity test file slot.
Real tests land here as `test_codec.cpp`, `test_replay.cpp`, etc.
Run via `pio test` once PlatformIO is installed.
