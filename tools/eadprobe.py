#!/usr/bin/env python3
"""EAD-V1 protocol probe: an independent host implementation of the device link.

It shares no code with the firmware or the dashboard (own COBS, CRC via zlib,
own WebSocket client, `struct` decoders), so it can verify both.

  eadprobe.py hello                         identify the device (USB, auto-detected)
  eadprobe.py config                        print CONFIG_GET and verify its SHA-256
  eadprobe.py stats --seconds 1800          stream, backfill gaps, report quality
  eadprobe.py stats --record run.eadlog     also record every received message
  eadprobe.py reopen --cycles 20            reopen the USB port; boot_id must not change
  eadprobe.py --ws ws://192.168.4.1:8080/ws stats --seconds 300

Layouts: docs/protocol.md. Requires pyserial for USB.
"""
import argparse
import base64
import hashlib
import math
import os
import socket
import statistics
import struct
import sys
import time
import zlib

PROTOCOL_VERSION = 1
SCHEMA = 3
HEADER = struct.Struct("<HBBIIQ")

HELLO, CONFIG_GET, RAW_SAMPLE_BATCH, STATUS, ERROR = 0x01, 0x02, 0x08, 0x0C, 0x0E
SESSION_START, SESSION_STOP = 0x04, 0x05
EVENT_BATCH, STEP_BATCH = 0x09, 0x0A
BACKFILL_REQUEST, BACKFILL_DATA = 0x10, 0x11
DURABLE = {0x08, 0x09, 0x0A, 0x0B}

STATES = ["BOOT", "SELF_TEST", "CALIBRATING", "REFERENCE_CAPTURE", "READY", "RUNNING",
          "PAUSED", "FAULT", "RECOVERY"]
FAULTS = ["foot_absent", "shank_absent", "foot_config", "shank_config", "foot_no_data_ready",
          "shank_no_data_ready", "no_psram", "foot_frozen", "shank_frozen", "acquisition_stalled"]
RAW_FLAGS = ["foot_read_fail", "shank_read_fail", "shank_repeated", "foot_accel_saturated",
             "foot_gyro_saturated", "shank_accel_saturated", "shank_gyro_saturated",
             "foot_repeated"]
ERROR_CODES = {1: "BadFrame", 2: "SchemaMismatch", 3: "NotSupported", 4: "InvalidState",
               5: "BadPayload", 6: "BackfillUnavailable"}

USB_VID, USB_PID = 0x303A, 0x1001
RAW_FRAME = struct.Struct("<QI6h6h4h4hH")
STATUS_PAYLOAD = struct.Struct("<BBHIIIIIIIbBIHHHHBIHBI")
ACCEL_LSB_PER_G = 8192.0  # +-4 g (datasheet)


def flag_names(value, names):
    return [name for bit, name in enumerate(names) if value & (1 << bit)] or ["none"]


# ---- framing ---------------------------------------------------------------

def message(msg_type, payload=b"", seq=0):
    return HEADER.pack(PROTOCOL_VERSION, msg_type, 0, len(payload), seq, 0) + payload


def parse(msg):
    if len(msg) < HEADER.size:
        raise ValueError("short message")
    version, msg_type, flags, length, seq, time_us = HEADER.unpack_from(msg)
    if version != PROTOCOL_VERSION or length != len(msg) - HEADER.size:
        raise ValueError("bad header")
    return msg_type, seq, time_us, msg[HEADER.size:]


def cobs_encode(data):
    out, block = bytearray(), bytearray()
    for byte in data:
        if byte:
            block.append(byte)
            if len(block) == 254:
                out += bytes([255]) + block
                block = bytearray()
        else:
            out += bytes([len(block) + 1]) + block
            block = bytearray()
    out += bytes([len(block) + 1]) + block
    return bytes(out)


def cobs_decode(data):
    out, i = bytearray(), 0
    while i < len(data):
        code = data[i]
        if code == 0:
            raise ValueError("bad COBS")
        block = data[i + 1:i + code]
        if len(block) != code - 1 or 0 in block:
            raise ValueError("bad COBS")
        out += block
        i += code
        if code != 255 and i < len(data):
            out.append(0)
    return bytes(out)


# ---- transports ------------------------------------------------------------

class UsbTransport:
    def __init__(self, port):
        import serial  # pyserial
        self.port = port or find_usb_port()
        self.serial = serial.Serial()
        self.serial.port, self.serial.baudrate, self.serial.timeout = self.port, 115200, 0
        # Linux asserts DTR and RTS on open. Clearing them one at a time passes
        # through DTR=0/RTS=1, which the ESP32-S3 USB Serial/JTAG treats as a
        # reset request, so leave both asserted.
        self.serial.dtr = True
        self.serial.rts = True
        self.serial.open()
        self.buffer = bytearray()
        self.rejected = 0
        self.name = f"USB {self.port}"

    def send(self, msg):
        self.serial.write(b"\x00" + cobs_encode(msg + struct.pack("<I", zlib.crc32(msg))) + b"\x00")

    def poll(self, timeout):
        deadline = time.monotonic() + timeout
        messages = []
        while True:
            chunk = self.serial.read(65536)
            if chunk:
                self.buffer += chunk
                *frames, rest = bytes(self.buffer).split(b"\x00")
                self.buffer = bytearray(rest)
                for frame in frames:
                    if not frame:
                        continue
                    try:
                        body = cobs_decode(frame)
                        if len(body) < HEADER.size + 4:
                            raise ValueError("short frame")
                        msg, crc = body[:-4], struct.unpack("<I", body[-4:])[0]
                        if zlib.crc32(msg) != crc:
                            raise ValueError("CRC mismatch")
                        parse(msg)
                        messages.append(msg)
                    except ValueError:
                        self.rejected += 1
            if messages or time.monotonic() >= deadline:
                return messages
            time.sleep(0.002)

    def close(self):
        self.serial.close()


def find_usb_port():
    from serial.tools import list_ports
    for p in list_ports.comports():
        if p.vid == USB_VID and p.pid == USB_PID:
            return p.device
    raise SystemExit("no EAD device (USB 303a:1001) found")


class WsTransport:
    GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"

    def __init__(self, url):
        if not url.startswith("ws://"):
            raise SystemExit("only ws:// URLs are supported")
        hostport, _, path = url[5:].partition("/")
        host, _, port = hostport.partition(":")
        self.sock = socket.create_connection((host, int(port or 80)), timeout=3)
        self.sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        key = base64.b64encode(os.urandom(16)).decode()
        self.sock.sendall((f"GET /{path} HTTP/1.1\r\nHost: {hostport}\r\nUpgrade: websocket\r\n"
                           f"Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\n"
                           "Sec-WebSocket-Version: 13\r\n\r\n").encode())
        response = b""
        while b"\r\n\r\n" not in response:
            chunk = self.sock.recv(1024)
            if not chunk:
                raise ConnectionError("closed during handshake")
            response += chunk
        head, _, rest = response.partition(b"\r\n\r\n")
        lines = head.decode(errors="replace").split("\r\n")
        if " 101 " not in lines[0] + " ":
            raise ConnectionError(f"handshake refused: {lines[0]}")
        headers = {k.strip().lower(): v.strip() for k, _, v in (l.partition(":") for l in lines[1:])}
        expected = base64.b64encode(hashlib.sha1((key + self.GUID).encode()).digest()).decode()
        if headers.get("sec-websocket-accept") != expected:
            raise ConnectionError("bad Sec-WebSocket-Accept")
        self.buffer = bytearray(rest)
        self.sock.setblocking(False)
        self.rejected = 0
        self.name = f"WebSocket {url}"

    def _frame(self, opcode, payload):
        n = len(payload)
        head = bytearray([0x80 | opcode])
        if n < 126:
            head.append(0x80 | n)
        elif n < 65536:
            head += bytes([0x80 | 126]) + struct.pack(">H", n)
        else:
            head += bytes([0x80 | 127]) + struct.pack(">Q", n)
        mask = os.urandom(4)
        return bytes(head) + mask + bytes(b ^ mask[i % 4] for i, b in enumerate(payload))

    def send(self, msg):
        self.sock.setblocking(True)
        self.sock.sendall(self._frame(0x2, msg))
        self.sock.setblocking(False)

    def poll(self, timeout):
        deadline = time.monotonic() + timeout
        messages = []
        while True:
            try:
                chunk = self.sock.recv(65536)
                if not chunk:
                    raise ConnectionError("connection closed")
                self.buffer += chunk
            except BlockingIOError:
                pass
            while True:
                parsed = self._take_frame()
                if parsed is None:
                    break
                opcode, payload = parsed
                if opcode == 0x2:
                    try:
                        parse(payload)
                        messages.append(payload)
                    except ValueError:
                        self.rejected += 1
                elif opcode == 0x9:
                    self.sock.setblocking(True)
                    self.sock.sendall(self._frame(0xA, payload))
                    self.sock.setblocking(False)
                elif opcode == 0x8:
                    raise ConnectionError("server closed the WebSocket")
            if messages or time.monotonic() >= deadline:
                return messages
            time.sleep(0.002)

    def _take_frame(self):
        b = self.buffer
        if len(b) < 2:
            return None
        opcode, masked, n, pos = b[0] & 0x0F, b[1] & 0x80, b[1] & 0x7F, 2
        if n == 126:
            if len(b) < 4:
                return None
            n, pos = struct.unpack(">H", b[2:4])[0], 4
        elif n == 127:
            if len(b) < 10:
                return None
            n, pos = struct.unpack(">Q", b[2:10])[0], 10
        mask = b""
        if masked:
            mask, pos = b[pos:pos + 4], pos + 4
        if len(b) < pos + n:
            return None
        payload = bytes(b[pos:pos + n])
        if masked:
            payload = bytes(x ^ mask[i % 4] for i, x in enumerate(payload))
        del b[:pos + n]
        return opcode, payload

    def close(self):
        self.sock.close()


# ---- decoders --------------------------------------------------------------

def decode_hello(p):
    schema, state, reset, boot_id = struct.unpack_from("<HBBI", p)
    mac = ":".join(f"{x:02X}" for x in p[8:14])
    who_foot, who_shank, caps = p[14], p[15], p[16]
    sha = p[17:49].hex()
    oldest, last = struct.unpack_from("<II", p, 49)
    fw = p[58:58 + p[57]].decode()
    return dict(schema=schema, state=STATES[state], reset_reason=reset, boot_id=f"{boot_id:08x}",
                mac=mac, who_foot=f"0x{who_foot:02X}", who_shank=f"0x{who_shank:02X}",
                haptics_fitted=bool(caps & 1), flash_storage=bool(caps & 2),
                psram_ring=bool(caps & 4), config_sha256=sha, oldest_seq=oldest, last_seq=last,
                firmware=fw)


def decode_status(p):
    names = ["state", "links", "faults", "frame_index", "frames_dropped", "shank_repeated",
             "i2c_errors", "imu_reinits", "oldest_seq", "last_seq", "ap_rssi_dbm", "ap_stations",
             "heap_free_min", "stack_free_acquisition", "stack_free_processing", "stack_free_usb",
             "stack_free_wifi", "calibration_state", "calibration_samples",
             "calibration_reject", "gait_state", "cycles_completed"]
    s = dict(zip(names, STATUS_PAYLOAD.unpack(p)))
    s["state"] = STATES[s["state"]]
    s["faults"] = flag_names(s["faults"], FAULTS)
    s["links"] = [n for bit, n in enumerate(["usb", "wifi"]) if s["links"] & (1 << bit)]
    s["calibration_state"] = CALIB_STATES[s["calibration_state"]]
    s["calibration_reject"] = flag_names(s["calibration_reject"], CALIB_REJECTS)
    s["gait_state"] = GAIT_STATES[s["gait_state"]]
    return s


GAIT_STATES = ["INIT", "SWING", "CONTACT_TRANSITION", "STANCE", "FOOT_FLAT_ZV", "PRE_SWING",
               "FAULT"]
GAIT_EVENTS = {1: "INITIAL_CONTACT", 2: "TOE_OFF", 3: "FOOT_FLAT", 4: "ZUPT_START",
               5: "ZUPT_END"}
EVENT_RECORD = struct.Struct("<BBIQ")
CYCLE_RECORD = struct.Struct("<IIQ13fHH")


def decode_events(p):
    count, size = p[0], p[1]
    out = []
    for i in range(count):
        t, _, frame, us = EVENT_RECORD.unpack_from(p, 2 + i * size)
        out.append({"type": GAIT_EVENTS.get(t, t), "frame": frame, "time_s": us / 1e6})
    return out


def decode_cycles(p):
    count, size = p[0], p[1]
    out = []
    for i in range(count):
        v = CYCLE_RECORD.unpack_from(p, 2 + i * size)
        out.append({
            "start_frame": v[0], "end_frame": v[1], "start_s": v[2] / 1e6,
            "cycle_time_s": round(v[3], 3), "stance_time_s": round(v[4], 3),
            "swing_time_s": round(v[5], 3), "stance_ratio": round(v[6], 3),
            "swing_ratio": round(v[7], 3), "cadence": round(v[8], 1),
            "peak_shank_dps": round(v[9], 1), "peak_dorsiflexion_deg": round(v[10], 1),
            "contact_sagittal_deg": round(v[11], 1), "peak_inversion_deg": round(v[12], 1),
            "distance_m": round(v[13], 3), "speed_mps": round(v[14], 3),
            "zupt_quality": round(v[15], 3), "valid": bool(v[16] & 1),
        })
    return out


CALIB_STATES = ["none", "collecting", "ready", "rejected"]
CALIB_REJECTS = ["too_few_samples", "moved", "not_gravity", "upside_down"]
CALIB_SENSOR = struct.Struct("<13f8x")


def decode_calibration(p):
    """SESSION_STOP from the device: the calibration record (docs/protocol.md §5.10)."""
    kind, _, reject, samples = struct.unpack_from("<BBHI", p)
    out = {"kind": kind, "reject": flag_names(reject, CALIB_REJECTS), "samples": samples}
    for index, name in enumerate(("foot", "shank")):
        v = CALIB_SENSOR.unpack_from(p, 8 + index * 60)
        out[name] = {
            "gyro_bias_dps": [round(x, 3) for x in v[0:3]],
            "up": [round(x, 4) for x in v[3:6]],
            "alignment": [round(x, 5) for x in v[6:10]],
            "tilt_deg": round(v[10], 2),
            "accel_magnitude_g": round(v[11], 4),
            "gyro_std_dps": round(v[12], 3),
        }
    return out


def decode_error(p):
    cmd_seq, cmd_type, code = struct.unpack_from("<IBH", p)
    return f"ERROR {ERROR_CODES.get(code, code)} for command {cmd_seq} (type 0x{cmd_type:02X}): " \
           f"{p[8:8 + p[7]].decode(errors='replace')}"


def decode_config(p):
    fmt = struct.unpack_from("<H", p)[0]
    sha, length = p[2:34], struct.unpack_from("<H", p, 34)[0]
    section = p[36:36 + length]
    return fmt, sha, section


def config_fields(section):
    """Decodes the format-1 section in docs/protocol.md order."""
    fields, pos = {}, 0

    def take(fmt, *names):
        nonlocal pos
        values = struct.unpack_from("<" + fmt, section, pos)
        pos += struct.calcsize("<" + fmt)
        if len(names) == 1 and len(values) > 1:
            fields[names[0]] = list(values)
        else:
            fields.update(zip(names, values))

    take("BBIHBHBBBff", "foot_addr", "shank_addr", "i2c_hz", "sample_hz", "accel_range_g",
         "gyro_range_dps", "dlpf_hz", "dlpf_cfg", "smplrt_div", "accel_lsb_per_g",
         "gyro_lsb_per_dps")
    take("9b", "foot_mount")
    take("9b", "shank_mount")
    take("BBBB6B", "pin_sda", "pin_scl", "pin_foot_int", "pin_shank_int", "m1", "m2", "m3", "m4",
         "m5", "m6")
    take("Bff", "cal_static_s", "mahony_kp", "mahony_ki")
    take("ffffHHH", "gait_lowpass_hz", "event_lowpass_hz", "min_cycle_s", "max_cycle_s",
         "contact_guard_ms", "toeoff_guard_ms", "ic_window_ms")
    take("ffHH", "zupt_accel_tol_g", "zupt_gyro_dps", "zupt_min_ms", "zupt_entry_hyst_ms")
    take("fffff", "class_activation", "haptic_on", "haptic_off", "confidence_haptic", "z_clip")
    take("7f", "weights")
    take("BHBBBfBBf", "haptics_fitted", "pwm_hz", "pwm_bits", "min_duty", "max_duty",
         "intensity_exponent", "max_on_s", "rolling_window_s", "rolling_duty_limit")
    take("6H", "motor_deg")
    take("4BHB", "ip0", "ip1", "ip2", "ip3", "ws_port", "batch_frames")
    take("HHf", "block_bytes", "flush_ms", "free_floor")
    take("HH", "ref_min_cycles", "ref_check_cycles")
    if pos != len(section):
        raise ValueError(f"config section has {len(section) - pos} unexpected trailing bytes")
    return fields


# ---- session ---------------------------------------------------------------

class Session:
    def __init__(self, args):
        self.args = args
        self.transport = None
        self.command_seq = 0
        self.pending = []  # messages received while waiting for a reply

    def poll(self, timeout):
        if self.pending:
            messages, self.pending = self.pending, []
            return messages
        return self.transport.poll(timeout)

    def open(self):
        self.pending = []
        self.transport = WsTransport(self.args.ws) if self.args.ws else UsbTransport(self.args.usb)

    def send(self, msg_type, payload=b""):
        self.command_seq += 1
        self.transport.send(message(msg_type, payload, self.command_seq))
        return self.command_seq

    def request(self, msg_type, payload, reply_type, timeout=2.0):
        """Sends a command and returns the first reply payload of `reply_type`."""
        self.send(msg_type, payload)
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            messages = self.transport.poll(0.05)
            for i, msg in enumerate(messages):
                t, _, _, p = parse(msg)
                if t == reply_type:
                    self.pending.extend(messages[i + 1:])
                    return p
                if t == ERROR:
                    print(decode_error(p), file=sys.stderr)
                else:
                    self.pending.append(msg)
        raise TimeoutError(f"no reply of type 0x{reply_type:02X}")

    def hello(self):
        return decode_hello(self.request(HELLO, struct.pack("<H", SCHEMA), HELLO))


def cmd_hello(args):
    s = Session(args)
    s.open()
    for k, v in s.hello().items():
        print(f"{k:16} {v}")
    s.transport.close()


def cmd_calibrate(args):
    """Runs a still window on the device and prints the record it produces."""
    s = Session(args)
    s.open()
    s.hello()
    duration_ms = int(args.seconds * 1000)
    print(f"hold still for {args.seconds:.0f} s ...")
    s.send(SESSION_START, struct.pack("<BBH", 1, 0, duration_ms))
    # The device stops streaming to a host that has gone quiet for 3 s, so the
    # keepalive has to continue while the window runs (docs/protocol.md §5.3).
    deadline = time.monotonic() + args.seconds + 5
    next_keepalive = time.monotonic() + 1.0
    payload = None
    while payload is None and time.monotonic() < deadline:
        if time.monotonic() >= next_keepalive:
            s.send(STATUS, b"")
            next_keepalive += 1.0
        for msg in s.transport.poll(0.05):
            t, _, _, p = parse(msg)
            if t == SESSION_STOP:
                payload = p
            elif t == ERROR:
                print(decode_error(p), file=sys.stderr)
    if payload is None:
        raise TimeoutError("the device sent no calibration record")
    record = decode_calibration(payload)
    print(f"samples {record['samples']}  reject {', '.join(record['reject'])}")
    for name in ("foot", "shank"):
        v = record[name]
        print(f"{name:6} bias {v['gyro_bias_dps']} deg/s  tilt {v['tilt_deg']}  "
              f"|a| {v['accel_magnitude_g']} g  sigma {v['gyro_std_dps']} deg/s")
    s.transport.close()
    return 0 if record["reject"] == ["none"] else 1


def cmd_walk(args):
    """Records a walk and prints the cycles the device detected."""
    s = Session(args)
    s.open()
    s.hello()
    print(f"walking capture: {args.seconds:.0f} s")
    events, cycles = [], []
    end = time.monotonic() + args.seconds
    while time.monotonic() < end:
        s.send(STATUS, b"")
        for msg in s.transport.poll(0.2):
            t, _, _, p = parse(msg)
            if t == EVENT_BATCH:
                events.extend(decode_events(p))
            elif t == STEP_BATCH:
                cycles.extend(decode_cycles(p))
            elif t == ERROR:
                print(decode_error(p), file=sys.stderr)
    contacts = [e for e in events if e["type"] == "INITIAL_CONTACT"]
    print(f"{len(contacts)} initial contacts, {len(cycles)} cycles")
    if cycles:
        print(f"{'cycle':>6} {'time':>6} {'stance':>7} {'cadence':>8} {'dist':>7} "
              f"{'speed':>7} {'zupt':>6} {'dorsi':>7}  valid")
        for i, c in enumerate(cycles, 1):
            print(f"{i:6d} {c['cycle_time_s']:6.2f} {c['stance_ratio']:7.2f} "
                  f"{c['cadence']:8.1f} {c['distance_m']:7.2f} {c['speed_mps']:7.2f} "
                  f"{c['zupt_quality']:6.2f} {c['peak_dorsiflexion_deg']:7.1f}  {c['valid']}")
        valid = [c for c in cycles if c["valid"]]
        if valid:
            total = sum(c["distance_m"] for c in valid)
            print(f"valid cycles {len(valid)}  total distance {total:.2f} m  "
                  f"mean cadence {sum(c['cadence'] for c in valid)/len(valid):.1f} steps/min")
    s.transport.close()


def cmd_config(args):
    s = Session(args)
    s.open()
    info = s.hello()
    fmt, sha, section = decode_config(s.request(CONFIG_GET, b"", CONFIG_GET))
    s.transport.close()
    ok = hashlib.sha256(section).digest() == sha and sha.hex() == info["config_sha256"]
    print(f"format {fmt}, {len(section)} bytes, SHA-256 {sha.hex()} "
          f"({'verified, matches HELLO' if ok else 'MISMATCH'})")
    for k, v in config_fields(section).items():
        print(f"  {k:20} {v}")
    if not ok:
        sys.exit(1)


def cmd_reopen(args):
    boot_ids = []
    for i in range(args.cycles):
        s = Session(args)
        s.open()
        info = s.hello()
        boot_ids.append(info["boot_id"])
        s.transport.close()
        print(f"cycle {i + 1:2}: boot_id {info['boot_id']} state {info['state']} "
              f"last_seq {info['last_seq']}")
        time.sleep(args.pause)
    same = len(set(boot_ids)) == 1
    print(f"{'PASS' if same else 'FAIL'}: {len(set(boot_ids))} distinct boot_id over {args.cycles} opens")
    sys.exit(0 if same else 1)


class Stats:
    def __init__(self):
        self.frames = {}          # frame_index -> (timestamp_us, status, foot, shank)
        self.durable = set()
        self.duplicates = 0
        self.backfilled = 0
        self.last_status = None
        self.errors = []
        self.reconnects = 0
        self.repair_requests = 0

    def add_durable(self, msg, from_backfill):
        t, seq, _, p = parse(msg)
        if seq in self.durable:
            self.duplicates += 1
            return
        self.durable.add(seq)
        if from_backfill:
            self.backfilled += 1
        if t != RAW_SAMPLE_BATCH:
            return
        count, size = p[0], p[1]
        for i in range(count):
            f = RAW_FRAME.unpack_from(p, 2 + i * size)
            ts, index, foot, shank, status = f[0], f[1], f[2:8], f[8:14], f[22]
            if index in self.frames:
                self.duplicates += 1
            self.frames[index] = (ts, status, foot, shank)

    def missing_sequences(self):
        if not self.durable:
            return []
        lo, hi = min(self.durable), max(self.durable)
        return [s for s in range(lo, hi + 1) if s not in self.durable]


def split_backfill(p):
    cmd, first, last, more = struct.unpack_from("<IIIB", p)
    body, pos, messages = p[13:], 0, []
    while pos < len(body):
        length = struct.unpack_from("<I", body, pos + 4)[0] + HEADER.size
        messages.append(body[pos:pos + length])
        pos += length
    return first, last, more, messages


def cmd_stats(args):
    stats = Stats()
    record = open(args.record, "wb") if args.record else None
    if record:
        record.write(b"EADLOG1\n")
    s = Session(args)
    start = time.monotonic()
    next_report = start + 10
    last_keepalive = 0.0
    last_seq = None
    boot_id = None

    def connect():
        nonlocal last_seq, boot_id
        s.open()
        info = s.hello()
        if boot_id is not None and info["boot_id"] != boot_id:
            print("device rebooted: sequence numbers restarted", file=sys.stderr)
            last_seq = None
        boot_id = info["boot_id"]
        print(f"connected over {s.transport.name}: firmware {info['firmware']}, "
              f"state {info['state']}, sequence window {info['oldest_seq']}..{info['last_seq']}")
        if last_seq is not None and info["last_seq"] > last_seq:
            s.send(BACKFILL_REQUEST, struct.pack("<II", last_seq + 1, info["last_seq"]))
            print(f"requested backfill {last_seq + 1}..{info['last_seq']}")

    next_repair = time.monotonic() + 2

    def repair_gaps():
        oldest = (stats.last_status or {}).get("oldest_seq", 0)
        missing = [q for q in stats.missing_sequences() if q >= oldest]
        if not missing:
            return
        first = last = missing[0]
        for q in missing[1:]:
            if q != last + 1:
                break
            last = q
        s.send(BACKFILL_REQUEST, struct.pack("<II", first, last))
        stats.repair_requests += 1

    connect()
    try:
        while time.monotonic() - start < args.seconds:
            try:
                messages = s.poll(0.05)
                now = time.monotonic()
                if now - last_keepalive >= 1.0:
                    s.send(STATUS)
                    last_keepalive = now
            except (OSError, ConnectionError) as exc:
                print(f"link lost ({exc}); reconnecting", file=sys.stderr)
                stats.reconnects += 1
                try:
                    s.transport.close()
                except OSError:
                    pass
                while time.monotonic() - start < args.seconds:
                    try:
                        connect()
                        break
                    except (OSError, ConnectionError, TimeoutError):
                        time.sleep(1)
                continue
            for msg in messages:
                if record:
                    record.write(struct.pack("<IQ", len(msg), time.time_ns() // 1000) + msg)
                t, seq, _, p = parse(msg)
                if t in DURABLE:
                    if last_seq is not None and seq > last_seq + 1:
                        s.send(BACKFILL_REQUEST, struct.pack("<II", last_seq + 1, seq - 1))
                    stats.add_durable(msg, from_backfill=False)
                    last_seq = seq if last_seq is None else max(last_seq, seq)
                elif t == BACKFILL_DATA:
                    for inner in split_backfill(p)[3]:
                        stats.add_durable(inner, from_backfill=True)
                elif t == STATUS:
                    stats.last_status = decode_status(p)
                elif t == ERROR:
                    stats.errors.append(decode_error(p))
                    print(stats.errors[-1], file=sys.stderr)
            if time.monotonic() >= next_repair:
                next_repair += 2
                repair_gaps()
            if time.monotonic() >= next_report:
                next_report += 10
                st = stats.last_status or {}
                print(f"{time.monotonic() - start:7.0f} s  frames {len(stats.frames):7d}  "
                      f"missing seq {len(stats.missing_sequences()):4d}  "
                      f"device dropped {st.get('frames_dropped', '?')}  faults {st.get('faults', '?')}")
    except KeyboardInterrupt:
        pass
    finally:
        s.transport.close()
        if record:
            record.close()
    report(stats, time.monotonic() - start, s.transport.rejected)


def report(stats, elapsed, rejected):
    print("\n==== report ====")
    print(f"duration            {elapsed:.1f} s, reconnects {stats.reconnects}, "
          f"rejected frames {rejected}")
    missing = stats.missing_sequences()
    print(f"durable messages    {len(stats.durable)} (backfilled {stats.backfilled}, "
          f"gap-repair requests {stats.repair_requests}), missing {len(missing)}, "
          f"duplicates {stats.duplicates}")
    if not stats.frames:
        print("no frames received")
        return
    indices = sorted(stats.frames)
    expected = indices[-1] - indices[0] + 1
    print(f"frames              {len(indices)} of {expected} expected "
          f"(index {indices[0]}..{indices[-1]}), missing {expected - len(indices)}")

    periods = [stats.frames[b][0] - stats.frames[a][0] for a, b in zip(indices, indices[1:])
               if b == a + 1]
    if periods:
        print(f"frame period        mean {statistics.fmean(periods):.1f} us, "
              f"sd {statistics.pstdev(periods):.1f} us, min {min(periods)} us, max {max(periods)} us")
        span = stats.frames[indices[-1]][0] - stats.frames[indices[0]][0]
        print(f"frame rate          {1e6 * (indices[-1] - indices[0]) / span:.3f} Hz (device clock)")

    flag_counts = [0] * len(RAW_FLAGS)
    foot_mag, shank_mag = [], []
    for ts, status, foot, shank in stats.frames.values():
        for bit in range(len(RAW_FLAGS)):
            flag_counts[bit] += bool(status & (1 << bit))
        if not status & 1:
            foot_mag.append(math.sqrt(sum(v * v for v in foot[:3])) / ACCEL_LSB_PER_G)
        if not status & 2:
            shank_mag.append(math.sqrt(sum(v * v for v in shank[:3])) / ACCEL_LSB_PER_G)
    print("frame flags         " + ", ".join(f"{n} {c}" for n, c in zip(RAW_FLAGS, flag_counts)))
    for name, mags in (("foot", foot_mag), ("shank", shank_mag)):
        if mags:
            print(f"{name:5} |a|          mean {statistics.fmean(mags):.4f} g, "
                  f"sd {statistics.pstdev(mags):.4f} g")
    if stats.last_status:
        print("device status       " + ", ".join(f"{k} {v}" for k, v in stats.last_status.items()))
    if stats.errors:
        print(f"errors              {len(stats.errors)} (first: {stats.errors[0]})")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    link = ap.add_mutually_exclusive_group()
    link.add_argument("--usb", metavar="PORT", nargs="?", const=None, default=None,
                      help="USB serial port (default: auto-detect 303a:1001)")
    link.add_argument("--ws", metavar="URL", help="WebSocket URL, e.g. ws://192.168.4.1:8080/ws")
    sub = ap.add_subparsers(dest="command", required=True)
    sub.add_parser("hello")
    sub.add_parser("config")
    st = sub.add_parser("stats")
    st.add_argument("--seconds", type=float, default=60)
    st.add_argument("--record", metavar="FILE")
    wk = sub.add_parser("walk")
    wk.add_argument("--seconds", type=float, default=30)
    cal = sub.add_parser("calibrate")
    cal.add_argument("--seconds", type=float, default=5.0)
    ro = sub.add_parser("reopen")
    ro.add_argument("--cycles", type=int, default=20)
    ro.add_argument("--pause", type=float, default=0.5)
    args = ap.parse_args()
    {"hello": cmd_hello, "config": cmd_config, "stats": cmd_stats, "reopen": cmd_reopen,
     "calibrate": cmd_calibrate, "walk": cmd_walk}[args.command](args)


if __name__ == "__main__":
    main()
