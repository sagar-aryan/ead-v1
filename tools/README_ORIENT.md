# Orientation tester — how to check your mounting (5 minutes)

You mounted per the corrected image: both boards X=toes, Y=big-toe side,
Z=up. Foot 0x68 on midfoot top, shank 0x69 front of leg 10–15 cm below knee.

## Run (XIAO plugged in)

```bash
python3 tools/orient_viewer.py                  # 3D live view
python3 tools/orient_viewer.py --text-only      # terminal values only
python3 tools/orient_viewer.py --simulate       # try UI with no hardware
```

Window shows two boards (FOOT + SHANK) with red=X, green=Y, blue=Z arrows
that tilt as you move, plus a pitch-history strip and STILL-OK flags.

## The 5 checks (do them in order, leg still between each)

1. **Sit still, foot flat** — both say `STILL-OK`, Z reads ~+0.9…+1.1, X and Y
   near 0. If Z is ~−1 or |a| ≈ 2 g, mounting or accel-range is wrong —
   stop and tell the agent.
2. **Lift toes up (dorsiflex), heel down** — foot PITCH number must move
   clearly (10–30°), ROLL nearly still. If ROLL moves instead, X/Y swapped.
3. **Roll sole inward (big-toe side down, inversion)** — foot ROLL must move,
   PITCH nearly still.
4. **Push foot forward quickly, then stop** — X value must jump positive
   during the push. If Y jumps instead, board is rotated 90°.
5. **Twist leg left/right (yaw)** — yaw drifts slowly (no magnetometer,
   normal). Pitch/roll must stay sane; cubes must not flip wildly.

PASS = checks 1–4 behave as above on BOTH sensors.
FAIL = note which check + which sensor + what moved instead, and send it
to the agent — do not remount blindly. Firmware already outputs the
anatomical frame, so leave `REMAP` in the viewer at identity. Axis fixes go
into the mount maps in `firmware/include/config_v1.h`. Those must stay
proper rotations (the build rejects anything else; see `docs/problems.md`
PROB-002).

This bring-up tool is retired in milestone M1, when the binary protocol
replaces the text stream; the dashboard mounting check (M3) takes over.

Motors stay OFF during this test (send `m` in serial monitor only for
the separate vibration test).
