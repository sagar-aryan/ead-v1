#!/usr/bin/env python3
"""Desktop dashboard for the EAS ERM driver board: six sliders, one per motor.

Talks to firmware/erm_channel_test over USB serial at 115200, one line per command
("<channel> <percent>"). tkinter (standard library) plus pyserial, no web stack.

    python3 erm_dashboard.py                 pick the port in the window
    python3 erm_dashboard.py /dev/ttyACM0    connect to that port at startup
    python3 erm_dashboard.py --offline       run the UI with no board attached
    python3 erm_dashboard.py --selftest      check the protocol, no GUI, no hardware
"""
import sys
import time
import tkinter as tk
from tkinter import ttk

BAUD = 115200
CHANNELS = 6
SEND_MS = 40      # slider updates are coalesced into one send per this interval
PING_S = 1.0      # keepalive: the firmware kills all channels after 3 s of silence
RAILS = {1: "A", 2: "A", 3: "A", 4: "B", 5: "B", 6: "B"}


def command(channel, percent):
    """The exact line the firmware expects. Percent is clamped, never trusted from the widget."""
    return f"{int(channel)} {max(0, min(100, int(percent)))}\n"


class Link:
    """Serial port, or a stand-in that just records lines when running offline."""

    def __init__(self, offline=False):
        self.port = None
        self.offline = offline
        self.sent = []

    @staticmethod
    def ports():
        try:
            from serial.tools import list_ports
        except ImportError:
            return []
        return [p.device for p in list_ports.comports()]

    def open(self, device):
        import serial
        self.close()
        # DTR/RTS must stay low: on the ESP32-S3's native USB they are the reset/boot lines,
        # and asserting them on open drops the chip into the ROM bootloader.
        port = serial.Serial()
        port.port, port.baudrate, port.timeout = device, BAUD, 0.1
        port.dtr = port.rts = False
        port.open()
        self.port = port

    def close(self):
        if self.port:
            try:
                self.write("0\n")             # bare "0" is the firmware's all-off; "0 0" is channel 0
                self.port.close()
            except Exception:
                pass
        self.port = None

    def drain(self):
        """Read and discard what the board sent. An undrained port is what wedges the firmware."""
        if not self.port:
            return ""
        try:
            waiting = self.port.in_waiting
            return self.port.read(waiting).decode(errors="replace") if waiting else ""
        except Exception:
            return ""

    def write(self, line):
        self.sent.append(line)
        if self.port:
            self.port.write(line.encode())
        elif not self.offline:
            # Never fail silently: a motor keeps running at its last duty until a new value
            # actually reaches the board, so a slider that cannot be sent must say so.
            raise ConnectionError("not connected")


class Dashboard:
    def __init__(self, root, link, device=None):
        self.link, self.pending, self.sliders, self.labels = link, {}, {}, {}
        self.last_ping = 0.0
        root.title("EAS ERM driver")
        root.columnconfigure(0, weight=1)

        bar = ttk.Frame(root, padding=8)
        bar.grid(row=0, column=0, sticky="ew")
        self.port = ttk.Combobox(bar, width=22, values=Link.ports())
        self.port.grid(row=0, column=0, padx=(0, 6))
        if device:
            self.port.set(device)
        elif self.port["values"]:
            self.port.current(0)
        ttk.Button(bar, text="Refresh", command=self.refresh).grid(row=0, column=1, padx=3)
        self.connect_btn = ttk.Button(bar, text="Connect", command=self.toggle)
        self.connect_btn.grid(row=0, column=2, padx=3)
        self.status = ttk.Label(bar, text="not connected")
        self.status.grid(row=0, column=3, padx=8)

        body = ttk.Frame(root, padding=(8, 0))
        body.grid(row=1, column=0, sticky="nsew")
        body.columnconfigure(2, weight=1)
        for ch in range(1, CHANNELS + 1):
            ttk.Label(body, text=f"Motor {ch}").grid(row=ch, column=0, sticky="w", pady=4)
            ttk.Label(body, text=f"rail {RAILS[ch]}", foreground="#777").grid(row=ch, column=1, padx=6)
            s = ttk.Scale(body, from_=0, to=100, orient="horizontal",
                          command=lambda v, c=ch: self.moved(c, v))
            s.grid(row=ch, column=2, sticky="ew", padx=6)
            self.sliders[ch] = s
            self.labels[ch] = ttk.Label(body, text="0 %", width=5, anchor="e")
            self.labels[ch].grid(row=ch, column=3)
            ttk.Button(body, text="Off", width=4,
                       command=lambda c=ch: self.set(c, 0)).grid(row=ch, column=4, padx=(6, 0))

        foot = ttk.Frame(root, padding=8)
        foot.grid(row=2, column=0, sticky="ew")
        ttk.Button(foot, text="ALL OFF", command=self.all_off).grid(row=0, column=0)
        ttk.Button(foot, text="ALL 100 %", command=self.all_full).grid(row=0, column=1, padx=6)
        ttk.Label(foot, text="An ERM usually needs ~40 % before it starts turning.",
                  foreground="#777").grid(row=0, column=2, padx=10)

        self.root = root
        root.protocol("WM_DELETE_WINDOW", self.quit)
        root.after(SEND_MS, self.flush)
        if device:
            self.toggle()

    # --- actions -------------------------------------------------------------
    def refresh(self):
        self.port["values"] = Link.ports()

    def toggle(self):
        if self.link.port:
            self.link.close()
            self.connect_btn.config(text="Connect")
            self.disconnected("not connected")
            return
        try:
            self.link.open(self.port.get())
        except Exception as e:                      # a missing port must not kill the window
            self.disconnected(f"failed: {e}")
            return
        self.connect_btn.config(text="Disconnect")
        self.status.config(text=f"connected {self.port.get()}", foreground="")

    def moved(self, channel, value):
        percent = int(float(value))
        self.labels[channel].config(text=f"{percent} %")
        self.pending[channel] = percent          # coalesced; the slider fires on every pixel

    def set(self, channel, percent):
        self.sliders[channel].set(percent)       # the slider callback queues the send

    def all_off(self):
        for ch in range(1, CHANNELS + 1):
            self.set(ch, 0)

    def all_full(self):
        """Every motor to 100 % in one click: the soak test for a stuck channel."""
        for ch in range(1, CHANNELS + 1):
            self.set(ch, 100)

    def flush(self):
        heard = self.link.drain()
        if "failsafe" in heard:
            self.status.config(text="board failsafe: channels cut, link went quiet",
                               foreground="#b00")
        try:
            for channel, percent in sorted(self.pending.items()):
                self.link.write(command(channel, percent))
                self.last_ping = time.monotonic()
            if self.link.port and time.monotonic() - self.last_ping > PING_S:
                self.link.write("k\n")          # silence for 3 s makes the board cut every channel
                self.last_ping = time.monotonic()
        except Exception as e:
            # The port can vanish mid-drag: the XIAO's USB re-enumerates on any reset, and
            # comes back as a different /dev node. Drop the link, keep the window alive.
            self.link.port = None
            self.connect_btn.config(text="Connect")
            self.disconnected("NOT CONNECTED - the motor holds its last duty until a "
                              "command gets through. Refresh, then Connect.")
        self.pending.clear()
        self.root.after(SEND_MS, self.flush)

    def disconnected(self, message):
        self.status.config(text=message, foreground="#b00")

    def quit(self):
        self.all_off()
        self.flush()
        self.link.close()
        self.root.destroy()


def selftest():
    assert command(3, 55) == "3 55\n"
    assert command(1, 250) == "1 100\n" and command(1, -5) == "1 0\n", "duty must be clamped"
    assert command("2", 7.9) == "2 7\n"

    link = Link(offline=True)                      # offline: records instead of writing
    root = tk.Tk()
    root.withdraw()
    dash = Dashboard(root, link)
    dash.set(4, 80)
    dash.set(2, 30)
    dash.flush()
    assert link.sent == ["2 30\n", "4 80\n"], link.sent
    dash.set(4, 80)                                # repeat still sends: the board may have reset
    dash.all_off()
    dash.flush()
    assert link.sent[-6:] == [f"{c} 0\n" for c in range(1, 7)], link.sent[-6:]
    link.port = object()                           # pretend a port is open, so close() writes
    try:
        link.close()
    except Exception:
        pass
    assert link.sent[-1] == "0\n", link.sent[-1]    # all-off, not the rejected "0 0"

    assert link.drain() == ""                      # no port: drain is a no-op, never raises
    dash.link.port = object()                      # pretend connected, so the keepalive runs
    dash.last_ping = 0.0
    dash.flush()
    assert link.sent[-1] == "k\n", link.sent[-1]    # idle link must still be fed
    dash.all_full()
    dash.flush()
    assert link.sent[-6:] == [f"{c} 100\n" for c in range(1, 7)], link.sent[-6:]
    dash.all_off()
    dash.flush()
    link.port = None

    live = Link()                                  # not offline: writing with no port must raise
    try:
        live.write("1 50\n")
        raise AssertionError("a write with no port must not look like success")
    except ConnectionError:
        pass

    def boom(_):                                   # a port that disappears mid-drag
        raise OSError(5, "Input/output error")
    link.port, link.write = object(), boom
    dash.set(1, 50)
    dash.flush()                                   # must not raise: the window has to survive
    assert link.port is None and "NOT CONNECTED" in dash.status.cget("text")
    root.destroy()
    print("selftest ok")


if __name__ == "__main__":
    args = [a for a in sys.argv[1:]]
    if "--selftest" in args:
        selftest()
    else:
        device = next((a for a in args if not a.startswith("-")), None)
        root = tk.Tk()
        Dashboard(root, Link(offline="--offline" in args),
                  None if "--offline" in args else device)
        root.mainloop()
