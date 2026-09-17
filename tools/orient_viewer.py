#!/usr/bin/env python3
"""EAD-V1 orientation tester — live 3D view of Foot + Shank MPU6050.

What it does (layman): draws two small boards on screen that tilt/turn
as you move your real sensors, so you can check you mounted them the
right way per the placement image (X=toes, Y=big-toe side, Z=up).

Usage:
  python3 tools/orient_viewer.py                 # live from /dev/ttyACM0
  python3 tools/orient_viewer.py --port /dev/ttyACM0 --baud 115200
  python3 tools/orient_viewer.py --simulate      # try UI without hardware
  python3 tools/orient_viewer.py --text-only     # no 3D window, prints values

Firmware must print lines like:
  CSV,fax,fay,faz,fgx,fgy,fgz,sax,say,saz,sgx,sgy,sgz,fi,si
(units: g and dps, anatomical frame assumption)

If a guided test fails (e.g. lifting toes moves the wrong axis),
edit REMAP below (flip signs / swap) and re-run — then tell the agent
so the fix goes into firmware config_v1.h.
"""
import argparse
import math
import sys
import time

# ---- Chip->anatomical remap. Identity = raw chip axes treated as
# ---- X=fwd, Y=medial, Z=up (matches image if breakout is mounted
# ---- exactly as drawn). Change only if guided tests say so. ----
REMAP = {
    "foot": {"swap_xy": False, "sx": 1, "sy": 1, "sz": 1},
    "shank": {"swap_xy": False, "sx": 1, "sy": 1, "sz": 1},
}


def apply_remap(x, y, z, cfg):
    if cfg["swap_xy"]:
        x, y = y, x
    return x * cfg["sx"], y * cfg["sy"], z * cfg["sz"]


def accel_angles(ax, ay, az):
    """Pitch (about Y) and roll (about X) from gravity, degrees."""
    pitch = math.degrees(math.atan2(-ax, math.hypot(ay, az)))
    roll = math.degrees(math.atan2(ay, az))
    return pitch, roll


def parse_csv(line):
    p = line.strip().split(",")
    if len(p) != 15 or p[0] != "CSV":
        return None
    try:
        v = list(map(float, p[1:]))
    except ValueError:
        return None
    return {
        "fax": v[0], "fay": v[1], "faz": v[2],
        "fgx": v[3], "fgy": v[4], "fgz": v[5],
        "sax": v[6], "say": v[7], "saz": v[8],
        "sgx": v[9], "sgy": v[10], "sgz": v[11],
    }


def gravity_check(ax, ay, az):
    mag = math.sqrt(ax * ax + ay * ay + az * az)
    ok = abs(mag - 1.0) <= 0.25
    z_ok = az > 0.7
    return mag, ok and z_ok


def text_loop(args):
    if args.simulate:
        print("SIMULATE mode: fake slow pitch oscillation.")
        t0 = time.time()
        while True:
            t = time.time() - t0
            fax, fay, faz = 0.3 * math.sin(t), 0.0, 0.95
            print(f"foot a=({fax:+.2f},{fay:+.2f},{faz:+.2f})g "
                  f"pitch={accel_angles(fax, fay, faz)[0]:+.1f}deg")
            time.sleep(0.2)
    else:
        import serial
        s = serial.Serial(args.port, args.baud, timeout=1)
        print(f"Listening on {args.port} @ {args.baud} ...")
        while True:
            line = s.readline().decode(errors="ignore")
            d = parse_csv(line)
            if not d:
                continue
            fx, fy, fz = apply_remap(d["fax"], d["fay"], d["faz"], REMAP["foot"])
            sx, sy, sz = apply_remap(d["sax"], d["say"], d["saz"], REMAP["shank"])
            fp, fr = accel_angles(fx, fy, fz)
            sp, sr = accel_angles(sx, sy, sz)
            fmag, fok = gravity_check(fx, fy, fz)
            smag, sok = gravity_check(sx, sy, sz)
            print(f"FOOT a=({fx:+.2f},{fy:+.2f},{fz:+.2f}) p={fp:+06.1f} r={fr:+06.1f} "
                  f"|a|={fmag:.2f} {'STILL-OK' if fok else 'MOVE/BAD'} | "
                  f"SHANK a=({sx:+.2f},{sy:+.2f},{sz:+.2f}) p={sp:+06.1f} r={sr:+06.1f} "
                  f"|a|={smag:.2f} {'STILL-OK' if sok else 'MOVE/BAD'}")


def gui_loop(args):
    import numpy as np
    import matplotlib.pyplot as plt
    from matplotlib.animation import FuncAnimation
    from collections import deque

    use_sim = args.simulate
    ser = None
    if not use_sim:
        import serial
        ser = serial.Serial(args.port, args.baud, timeout=0.2)
        print(f"Listening on {args.port} @ {args.baud} ...")

    state = {"f": (0, 0, 0, 0, 0, 0), "s": (0, 0, 0, 0, 0, 0)}
    hist = deque(maxlen=200)  # foot pitch history for motion sparkline

    fig = plt.figure(figsize=(11, 6))
    fig.suptitle("EAD-V1 orientation test  |  X(red)=toes  Y(green)=big-toe side  Z(blue)=up")
    ax1 = fig.add_subplot(131, projection="3d")
    ax2 = fig.add_subplot(132, projection="3d")
    ax3 = fig.add_subplot(133)
    for ax, nm in ((ax1, "FOOT 0x68"), (ax2, "SHANK 0x69")):
        ax.set_title(nm)
        ax.set_xlim(-1.2, 1.2); ax.set_ylim(-1.2, 1.2); ax.set_zlim(-1.2, 1.2)
        ax.set_xlabel("X"); ax.set_ylabel("Y")
    ax3.set_title("foot pitch history (lift toes = moves)")
    ax3.set_ylim(-60, 60); ax3.set_xlabel("samples"); ax3.set_ylabel("deg")
    line_hist, = ax3.plot([], [])

    txt = fig.text(0.01, 0.01, "", fontsize=9, family="monospace")

    def rot_matrix(pitch_deg, roll_deg, yaw_deg):
        p, r, y = map(math.radians, (pitch_deg, roll_deg, yaw_deg))
        Rx = np.array([[1, 0, 0], [0, math.cos(r), -math.sin(r)],
                       [0, math.sin(r), math.cos(r)]])
        Ry = np.array([[math.cos(p), 0, math.sin(p)], [0, 1, 0],
                       [-math.sin(p), 0, math.cos(p)]])
        Rz = np.array([[math.cos(y), -math.sin(y), 0],
                       [math.sin(y), math.cos(y), 0], [0, 0, 1]])
        return Rz @ Ry @ Rx

    def draw_board(ax, pitch, roll, yaw):
        ax.cla()
        R = rot_matrix(pitch, roll, yaw)
        origin = np.zeros(3)
        for vec, col, lb in ((np.array([1, 0, 0]), "r", "X"),
                             (np.array([0, 1, 0]), "g", "Y"),
                             (np.array([0, 0, 1]), "b", "Z")):
            v = R @ vec
            ax.quiver(0, 0, 0, v[0], v[1], v[2], color=col, length=1.0,
                      arrow_length_ratio=0.15)
            ax.text(v[0] * 1.15, v[1] * 1.15, v[2] * 1.15, lb,
                    color=col, fontsize=12, weight="bold")
        # board plate (small square in XY plane, rotated)
        c = np.array([[-0.35, -0.3, 0], [0.35, -0.3, 0], [0.35, 0.3, 0],
                      [-0.35, 0.3, 0], [-0.35, -0.3, 0]]).T
        plate = R @ c
        ax.plot(plate[0], plate[1], plate[2], color="k", linewidth=2)
        ax.set_xlim(-1.2, 1.2); ax.set_ylim(-1.2, 1.2); ax.set_zlim(-1.2, 1.2)

    yaw_f = yaw_s = 0.0
    last_t = time.time()
    sim_t0 = time.time()

    def update(_):
        nonlocal yaw_f, yaw_s, last_t
        now = time.time()
        dt = min(now - last_t, 0.2)
        last_t = now
        if use_sim:
            t = now - sim_t0
            state["f"] = (0.35 * math.sin(t * 0.8), 0, 0.94,
                          0, 25 * math.cos(t * 0.8), 0)
            state["s"] = (0.05 * math.sin(t * 0.8), -0.05, 0.99, 0, 3, 0)
        else:
            for _ in range(20):
                line = ser.readline().decode(errors="ignore")
                d = parse_csv(line)
                if d:
                    state["f"] = (d["fax"], d["fay"], d["faz"],
                                  d["fgx"], d["fgy"], d["fgz"])
                    state["s"] = (d["sax"], d["say"], d["saz"],
                                  d["sgx"], d["sgy"], d["sgz"])
        fax, fay, faz, fgx, fgy, fgz = state["f"]
        sax, say, saz, sgx, sgy, sgz = state["s"]
        fx, fy, fz = apply_remap(fax, fay, faz, REMAP["foot"])
        sx, sy, sz = apply_remap(sax, say, saz, REMAP["shank"])
        # gyro remap (same signs)
        _, fgy2, fgz2 = apply_remap(fgx, fgy, fgz, REMAP["foot"])
        _, sgy2, sgz2 = apply_remap(sgx, sgy, sgz, REMAP["shank"])
        fp, fr = accel_angles(fx, fy, fz)
        sp, sr = accel_angles(sx, sy, sz)
        yaw_f += fgz2 * dt
        yaw_s += sgz2 * dt
        draw_board(ax1, fp, fr, yaw_f)
        ax1.set_title(f"FOOT p={fp:+.0f} r={fr:+.0f}")
        draw_board(ax2, sp, sr, yaw_s)
        ax2.set_title(f"SHANK p={sp:+.0f} r={sr:+.0f}")
        hist.append(fp)
        line_hist.set_data(range(len(hist)), list(hist))
        ax3.set_xlim(0, max(50, len(hist)))
        fmag, fok = gravity_check(fx, fy, fz)
        smag, sok = gravity_check(sx, sy, sz)
        txt.set_text(
            f"FOOT a=({fx:+.2f},{fy:+.2f},{fz:+.2f}) |a|={fmag:.2f} "
            f"{'STILL-OK' if fok else 'MOVE/BAD'}\n"
            f"SHANK a=({sx:+.2f},{sy:+.2f},{sz:+.2f}) |a|={smag:.2f} "
            f"{'STILL-OK' if sok else 'MOVE/BAD'}\n"
            "Tests: 1)still->STILL-OK+Z~+1 2)toes-up->pitch moves 3)tilt in->roll moves "
            "4)push fwd->X jumps 5)twist->yaw drifts (ok)")
        return []

    FuncAnimation(fig, update, interval=100, blit=False)
    plt.show()

    if ser:
        ser.close()


def main():
    ap = argparse.ArgumentParser(description="EAD-V1 MPU6050 orientation tester")
    ap.add_argument("--port", default="/dev/ttyACM0")
    ap.add_argument("--baud", type=int, default=115200)
    ap.add_argument("--simulate", action="store_true")
    ap.add_argument("--text-only", action="store_true")
    args = ap.parse_args()
    try:
        if args.text_only:
            text_loop(args)
        else:
            gui_loop(args)
    except KeyboardInterrupt:
        print("Stopped.")


if __name__ == "__main__":
    main()
