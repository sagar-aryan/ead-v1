#!/usr/bin/env python3
"""Bench viewer for one BNO086 (firmware/bench/bno086).

Reads the bench firmware's USB CDC lines. Prints the wiring checks and the
sensor identity, then draws a block that tilts the way the sensor does.

  python3 tools/bno_view.py                # auto-detect the XIAO, open the window
  python3 tools/bno_view.py --checks-only  # no window, just the checks
  python3 tools/bno_view.py --log bno.csv  # also save the raw data lines
"""

import argparse
import math
import sys
import threading

import serial
from serial.tools import list_ports

USB_VID, USB_PID = 0x303A, 0x1001

RESET, RED, GREEN, DIM = "\033[0m", "\033[31m", "\033[32m", "\033[2m"


def find_port():
    for p in list_ports.comports():
        if p.vid == USB_VID and p.pid == USB_PID:
            return p.device
    raise SystemExit("no XIAO (USB 303a:1001) found; pass --port")


def quat_to_rpy(qi, qj, qk, qr):
    """Roll/pitch/yaw in degrees, from the sensor's own (Android) frame."""
    roll = math.atan2(2 * (qr * qi + qj * qk), 1 - 2 * (qi * qi + qj * qj))
    s = max(-1.0, min(1.0, 2 * (qr * qj - qk * qi)))
    pitch = math.asin(s)
    yaw = math.atan2(2 * (qr * qk + qi * qj), 1 - 2 * (qj * qj + qk * qk))
    return tuple(math.degrees(v) for v in (roll, pitch, yaw))


def quat_matrix(qi, qj, qk, qr):
    import numpy as np

    n = math.sqrt(qi * qi + qj * qj + qk * qk + qr * qr) or 1.0
    x, y, z, w = qi / n, qj / n, qk / n, qr / n
    return np.array([
        [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
        [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
        [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
    ])


class Reader(threading.Thread):
    """Keeps only the latest sample; the window redraws far slower than 100 Hz."""

    daemon = True

    def __init__(self, port, log=None):
        super().__init__()
        self.ser = serial.Serial(port, 115200, timeout=1)
        self.log = open(log, "w") if log else None
        self.latest = None
        self.count = 0
        self.stop = False

    def run(self):
        # The checks run once at boot, long before this tool is usually
        # started. Any byte makes the firmware re-send them.
        self.ser.write(b"?")
        while not self.stop:
            try:
                line = self.ser.readline().decode("ascii", "replace").strip()
            except serial.SerialException as exc:
                print(f"{RED}serial closed: {exc}{RESET}")
                return
            if not line:
                continue
            if line.startswith("D,"):
                parts = line.split(",")
                if len(parts) != 13:
                    continue
                try:
                    self.latest = [float(v) for v in parts[1:]]
                except ValueError:
                    continue
                self.count += 1
                if self.log:
                    self.log.write(line + "\n")
            elif line.startswith("CHK,"):
                _, name, verdict, detail = line.split(",", 3)
                colour = {"PASS": GREEN, "FAIL": RED}.get(verdict, DIM)
                print(f"{colour}{verdict:4}{RESET} {name:22} {detail}")
            elif line.startswith("ID,"):
                print(f"{DIM}id  {RESET} {line[3:]}")
            else:
                print(f"{DIM}{line}{RESET}")
            sys.stdout.flush()


def draw(reader):
    import numpy as np
    import matplotlib.pyplot as plt
    from matplotlib.animation import FuncAnimation
    from mpl_toolkits.mplot3d.art3d import Poly3DCollection

    # A flat slab, so which face is up is obvious. Sizes are the sensor
    # board's rough proportions, not its real dimensions.
    sx, sy, sz = 1.0, 0.7, 0.12
    corners = np.array([[x, y, z] for x in (-sx, sx) for y in (-sy, sy) for z in (-sz, sz)]) / 2
    faces = [(0, 1, 3, 2), (4, 5, 7, 6), (0, 1, 5, 4), (2, 3, 7, 6), (0, 2, 6, 4), (1, 3, 7, 5)]
    colours = ["#cfd6dd"] * 6
    colours[1] = "#3d7ea6"  # +Z face, the one that should point up when it lies flat

    fig = plt.figure(figsize=(7, 6))
    fig.canvas.manager.set_window_title("BNO086 bench")
    ax = fig.add_subplot(projection="3d")
    ax.set_box_aspect((1, 1, 1))
    for lim in (ax.set_xlim, ax.set_ylim, ax.set_zlim):
        lim(-1, 1)
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z (up)")
    slab = Poly3DCollection([], facecolors=colours, edgecolors="#31393f", linewidths=0.8)
    ax.add_collection3d(slab)
    axes_lines = [ax.plot([], [], [], c, lw=2)[0] for c in ("r", "g", "b")]
    readout = fig.text(0.02, 0.02, "", family="monospace", fontsize=9, va="bottom")

    last_count = [0]

    def update(_frame):
        sample = reader.latest
        if sample is None:
            readout.set_text("waiting for data...")
            return
        qi, qj, qk, qr, acc, ax_, ay, az, gx, gy, gz = sample[1:]
        rot = quat_matrix(qi, qj, qk, qr)
        pts = corners @ rot.T
        slab.set_verts([[pts[i] for i in f] for f in faces])
        for line, col in zip(axes_lines, rot.T):
            line.set_data([0, col[0]], [0, col[1]])
            line.set_3d_properties([0, col[2]])
        roll, pitch, yaw = quat_to_rpy(qi, qj, qk, qr)
        rate = (reader.count - last_count[0]) * 20  # updates run at 50 ms
        last_count[0] = reader.count
        readout.set_text(
            f"roll  {roll:+7.1f} deg   pitch {pitch:+7.1f} deg   yaw {yaw:+7.1f} deg\n"
            f"accel {ax_:+6.2f} {ay:+6.2f} {az:+6.2f} m/s^2   |a| {math.sqrt(ax_**2+ay**2+az**2):5.2f}\n"
            f"gyro  {gx:+6.2f} {gy:+6.2f} {gz:+6.2f} rad/s\n"
            f"quat accuracy {acc:4.1f} deg   reports {rate:4.0f} /s"
        )

    # Kept so the animation is not garbage-collected.
    ani = FuncAnimation(fig, update, interval=50, cache_frame_data=False)
    fig._ani = ani
    plt.show()


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", help="serial port (default: auto-detect 303a:1001)")
    ap.add_argument("--checks-only", action="store_true", help="no window")
    ap.add_argument("--log", help="write the raw D, lines to this file")
    args = ap.parse_args()

    reader = Reader(args.port or find_port(), args.log)
    reader.start()
    if args.checks_only:
        try:
            reader.join()
        except KeyboardInterrupt:
            pass
        return
    draw(reader)
    reader.stop = True


if __name__ == "__main__":
    main()
