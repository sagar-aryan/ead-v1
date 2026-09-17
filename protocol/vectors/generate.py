#!/usr/bin/env python3
"""Generate EAD-V1 protocol golden vectors (schema 1).

These bytes are built independently of the firmware: Python `struct`, `zlib`
and `hashlib`, a separate COBS implementation, and the contract JSON for the
configuration section. Firmware native tests and the dashboard tests compare
their encoders/decoders against these files, so a bug has to be made twice,
in two languages, to go unnoticed.

Layouts: docs/protocol.md.  Run:  python3 protocol/vectors/generate.py
"""
import hashlib
import json
import pathlib
import struct
import zlib

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent
CONFIG_JSON = ROOT / "ead_agent_docs_v2" / "CONFIG_V1.json"

PROTOCOL_VERSION = 1
SCHEMA = 3

# Message types (doc 08 §3).
HELLO, CONFIG_GET, STATUS, ERROR = 0x01, 0x02, 0x0C, 0x0E
RAW_SAMPLE_BATCH, BACKFILL_REQUEST, BACKFILL_DATA = 0x08, 0x10, 0x11
SESSION_START, SESSION_STOP = 0x04, 0x05
EVENT_BATCH, STEP_BATCH = 0x09, 0x0A

# Calibration states and reject bits (docs/protocol.md §5.3, §6.5).
CALIB_READY = 2
GAIT_FOOT_FLAT = 4
CALIB_MOVED = 1 << 1

# RAW frame status bits (docs/protocol.md).
RAW_FOOT_READ_FAIL, RAW_SHANK_READ_FAIL, RAW_SHANK_REPEATED = 1 << 0, 1 << 1, 1 << 2
RAW_FOOT_ACCEL_SAT, RAW_FOOT_GYRO_SAT = 1 << 3, 1 << 4
RAW_SHANK_ACCEL_SAT, RAW_SHANK_GYRO_SAT, RAW_FOOT_REPEATED = 1 << 5, 1 << 6, 1 << 7

# STATUS fault bits (docs/protocol.md).
FAULT_FOOT_FROZEN, FAULT_SHANK_FROZEN, FAULT_ACQUISITION_STALLED = 1 << 7, 1 << 8, 1 << 9
STATE_READY = 4
LINK_USB_ACTIVE = 1 << 0
CAP_PSRAM_RING = 1 << 2

# Datasheet sensitivities (MPU-6000/6050 and MPU-6500 product specifications).
ACCEL_LSB_PER_G = {2: 16384.0, 4: 8192.0, 8: 4096.0, 16: 2048.0}
GYRO_LSB_PER_DPS = {250: 131.0, 500: 65.5, 1000: 32.8, 2000: 16.4}

# As-built mount maps (docs/hardware.md); not part of the contract JSON.
FOOT_MOUNT = [[1, 0, 0], [0, 1, 0], [0, 0, 1]]
SHANK_MOUNT = [[0, 0, -1], [0, 1, 0], [1, 0, 0]]
HAPTICS_FITTED = 0  # DEC-006


def header(msg_type, payload_len, seq, time_us, flags=0):
    return struct.pack("<HBBIIQ", PROTOCOL_VERSION, msg_type, flags, payload_len, seq, time_us)


def message(msg_type, payload, seq, time_us):
    return header(msg_type, len(payload), seq, time_us) + payload


def str8(text):
    raw = text.encode("utf-8")
    assert len(raw) <= 255
    return struct.pack("<B", len(raw)) + raw


def cobs_encode(data):
    """Cheshire & Baker reference algorithm (emits a trailing 0x01 block after
    an exact 254-byte run; any conforming decoder accepts it)."""
    out = bytearray([0])
    code_index, code = 0, 1
    for byte in data:
        if byte == 0:
            out[code_index] = code
            code_index, code = len(out), 1
            out.append(0)
            continue
        out.append(byte)
        code += 1
        if code == 0xFF:
            out[code_index] = code
            code_index, code = len(out), 1
            out.append(0)
    out[code_index] = code
    return bytes(out)


def usb_frame(msg):
    return b"\x00" + cobs_encode(msg + struct.pack("<I", zlib.crc32(msg))) + b"\x00"


def raw_frame(ts, index, foot, shank, q_foot, q_shank, status):
    return struct.pack("<QI6h6h4h4hH", ts, index, *foot, *shank, *q_foot, *q_shank, status)


IDENTITY_Q15 = (32767, 0, 0, 0)


def raw_batch_payload(frames):
    return struct.pack("<BB", len(frames), 54) + b"".join(frames)


def config_section(cfg):
    m, p = cfg["mpu6050"], cfg["pins"]
    cal, gait, zupt = cfg["calibration"], cfg["gait"], cfg["zupt"]
    err, hap, net = cfg["error"], cfg["haptics"], cfg["network"]
    sto, ref = cfg["storage"], cfg["reference"]
    out = bytearray()
    out += struct.pack(
        "<BBIHBHBBBff",
        int(m["foot_address"], 16), int(m["shank_address"], 16), m["i2c_hz"], m["sample_hz"],
        m["accelerometer_range_g"], m["gyroscope_range_dps"], m["dlpf_hz"], m["dlpf_cfg"],
        m["sample_rate_divider"], ACCEL_LSB_PER_G[m["accelerometer_range_g"]],
        GYRO_LSB_PER_DPS[m["gyroscope_range_dps"]])
    for mount in (FOOT_MOUNT, SHANK_MOUNT):
        out += struct.pack("<9b", *[v for row in mount for v in row])
    motors = p["motor_pwm_gpio"]
    out += struct.pack("<10B", p["i2c_sda_gpio"], p["i2c_scl_gpio"], p["foot_imu_int_gpio"],
                       p["shank_imu_int_gpio"], *[motors[f"M{i}"] for i in range(1, 7)])
    out += struct.pack("<Bff", cal["static_seconds"], cal["mahony_kp"], cal["mahony_ki"])
    out += struct.pack("<ffffHHH", gait["gait_lowpass_hz"], gait["event_path_lowpass_hz"],
                       gait["min_cycle_s"], gait["max_cycle_s"], gait["event_contact_guard_ms"],
                       gait["toeoff_guard_ms"], gait["ic_candidate_window_ms"])
    out += struct.pack("<ffHH", zupt["accel_tolerance_g"], zupt["gyro_threshold_dps"],
                       zupt["min_duration_ms"], zupt["entry_hysteresis_ms"])
    w = err["weights"]
    out += struct.pack(
        "<fffff7f", err["class_activation_threshold"], err["haptic_on_threshold"],
        err["haptic_off_threshold"], err["confidence_haptic_threshold"], err["robust_z_clip"],
        w["swing_dorsiflexion"], w["initial_contact_plantarflexion"], w["inversion_eversion"],
        w["timing"], w["stance_swing_ratio"], w["cycle_distance"], w["shank_dynamics"])
    deg = hap["motor_positions_deg"]
    out += struct.pack("<BHBBBfBBf6H", HAPTICS_FITTED, hap["pwm_hz"], hap["resolution_bits"],
                       hap["min_duty"], hap["max_duty"], hap["intensity_exponent"],
                       hap["max_continuous_on_s"], hap["rolling_window_s"],
                       hap["rolling_duty_limit"], *[deg[f"M{i}"] for i in range(1, 7)])
    out += struct.pack("<4BHB", *[int(o) for o in net["ip"].split(".")], net["port"],
                       net["sample_batch_frames"])
    out += struct.pack("<HHf", sto["block_payload_bytes"], sto["flush_interval_ms"],
                       sto["free_space_floor"])
    out += struct.pack("<HH", ref["minimum_valid_cycles"], ref["reference_check_cycles"])
    return bytes(out)


def write(name, description, data):
    lines = [f"# {line}" for line in description.strip().splitlines()]
    lines.append("# Generated by protocol/vectors/generate.py; do not edit.")
    for i in range(0, len(data), 16):
        lines.append(" ".join(f"{b:02x}" for b in data[i:i + 16]))
    (HERE / name).write_text("\n".join(lines) + "\n")


def main():
    cfg = json.loads(CONFIG_JSON.read_text())

    hello_request = message(HELLO, struct.pack("<H", SCHEMA), seq=7, time_us=0)
    write("hello_request.hex", "Host HELLO, schema 3, command sequence 7.", hello_request)

    fw = "0.1.0+test"
    sha = hashlib.sha256(b"ead").digest()
    hello_info_payload = (
        struct.pack("<HBBI", SCHEMA, STATE_READY, 1, 0xA1B2C3D4)
        + bytes([0x44, 0xB1, 0x76, 0xAF, 0xFB, 0x7C])
        + struct.pack("<BBB", 0x70, 0x70, CAP_PSRAM_RING) + sha + struct.pack("<II", 1, 42) + str8(fw))
    write("hello_info.hex",
          "Device HELLO: READY, reset reason 1, boot_id 0xA1B2C3D4, MAC 44:B1:76:AF:FB:7C,\n"
          "WHO 0x70/0x70, capabilities 0x04 (PSRAM ring), config SHA-256 of b'ead',\n"
          "sequence window 1..42, firmware '0.1.0+test'. Header sequence 42, time 123456789.",
          message(HELLO, hello_info_payload, seq=42, time_us=123456789))

    status_payload = struct.pack(
        "<BBHIIIIIIIbBIHHHHBIHBI", STATE_READY, LINK_USB_ACTIVE,
        FAULT_SHANK_FROZEN | FAULT_ACQUISITION_STALLED, 123456, 3, 17, 2, 1, 1, 42, -47, 1, 201000,
        1500, 2600, 3100, 4200, CALIB_READY, 500, 0, GAIT_FOOT_FLAT, 37)
    assert len(status_payload) == 58
    write("status.hex",
          "Device STATUS: READY, USB link active, faults 0x0300 (shank frozen + acquisition\n"
          "stalled), frame 123456, dropped 3, shank repeated 17, I2C errors 2, reinits 1,\n"
          "sequence window 1..42, RSSI -47 dBm, 1 station, heap min 201000,\n"
          "stack free 1500/2600/3100/4200, calibration ready from 500 samples,\n"
          "gait FOOT_FLAT_ZV with 37 cycles completed.\n"
          "Header sequence 42, time 987654321.",
          message(STATUS, status_payload, seq=42, time_us=987654321))

    frames = [
        raw_frame(1_000_000, 100, (8192, -8192, 32767, -32768, 1, -1),
                  (0, 16, -16, 655, -655, 32767), IDENTITY_Q15, IDENTITY_Q15,
                  RAW_FOOT_READ_FAIL | RAW_FOOT_GYRO_SAT | RAW_SHANK_ACCEL_SAT),
        raw_frame(1_010_000, 101, (-1, -2, -3, -4, -5, -6),
                  (1, 2, 3, 4, 5, 6), IDENTITY_Q15, (0, -32767, 12345, -12345), RAW_SHANK_REPEATED),
    ]
    raw_batch = message(RAW_SAMPLE_BATCH, raw_batch_payload(frames), seq=3, time_us=1_000_000)
    write("raw_batch.hex",
          "RAW_SAMPLE_BATCH with 2 frames (frame 100 at 1.000000 s, frame 101 at 1.010000 s),\n"
          "negative and saturated counts, status 0x0031 and 0x0004. Header sequence 3.",
          raw_batch)

    error = message(ERROR, struct.pack("<IBH", 9, 0x04, 3) + str8("SESSION_START not supported"),
                    seq=42, time_us=5)
    write("error.hex",
          "ERROR NotSupported (3) for command sequence 9, type SESSION_START (0x04),\n"
          "detail 'SESSION_START not supported'. Header sequence 42, time 5.", error)

    write("backfill_request.hex", "Host BACKFILL_REQUEST for sequences 100..250, command sequence 11.",
          message(BACKFILL_REQUEST, struct.pack("<II", 100, 250), seq=11, time_us=0))

    events = b"".join(struct.pack("<BBIQ", t, 0, frame, us) for t, frame, us in (
        (1, 1200, 12_000_000), (3, 1206, 12_060_000), (2, 1260, 12_600_000)))
    write("event_batch.hex",
          "EVENT_BATCH: initial contact at frame 1200, foot-flat at 1206, toe-off at 1260.\n"
          "Header sequence 51, time 12000000.",
          message(EVENT_BATCH, struct.pack("<BB", 3, 14) + events, seq=51, time_us=12_000_000))

    cycle = (struct.pack("<IIQ", 1200, 1300, 12_000_000)
             + struct.pack("<13f", 1.02, 0.63, 0.39, 0.6176, 0.3824, 117.65,
                           412.5, 14.2, -6.8, 3.1, 1.41, 1.382, 0.58)
             + struct.pack("<HH", 1, 0))
    assert len(cycle) == 72
    write("step_batch.hex",
          "STEP_BATCH: one valid cycle of 1.02 s, stance 0.63 s, cadence 117.65 steps/min,\n"
          "peak shank rate 412.5 deg/s, distance 1.41 m at 1.382 m/s, ZUPT quality 0.58.\n"
          "Header sequence 52, time 12000000.",
          message(STEP_BATCH, struct.pack("<BB", 1, 72) + cycle, seq=52, time_us=12_000_000))

    write("session_start.hex",
          "Host SESSION_START, kind CALIBRATION (1), 5000 ms window. Command sequence 12.",
          message(SESSION_START, struct.pack("<BBH", 1, 0, 5000), seq=12, time_us=0))

    # A record measured the way the hardware actually reads: the foot board sits
    # tilted on the instep, the shank board close to upright (TEST-027).
    def sensor(bias, up, quat, tilt, magnitude, std):
        return (struct.pack("<3f", *bias) + struct.pack("<3f", *up) + struct.pack("<4f", *quat)
                + struct.pack("<3f", tilt, magnitude, std) + bytes(8))

    calibration_payload = (
        struct.pack("<BBHI", 1, 0, 0, 500)
        + sensor((1.5, -0.5, 0.25), (-0.42, 0.34, 0.84), (0.968, 0.166, 0.205, 0.0),
                 33.0, 1.0, 0.1)
        + sensor((-0.75, 0.5, 0.125), (0.02, -0.01, 0.9997), (0.99999, -0.005, -0.01, 0.0),
                 1.3, 1.0, 0.2))
    assert len(calibration_payload) == 128
    write("calibration_record.hex",
          "Device SESSION_STOP: calibration record, kind CALIBRATION, accepted (reject 0),\n"
          "500 samples. Foot bias (1.5, -0.5, 0.25) deg/s tilted 33 deg; shank bias\n"
          "(-0.75, 0.5, 0.125) deg/s tilted 1.3 deg. Header sequence 43, time 987654321.",
          message(SESSION_STOP, calibration_payload, seq=43, time_us=987654321))

    raw_batch_4 = message(RAW_SAMPLE_BATCH, raw_batch_payload(frames[1:]), seq=4, time_us=1_010_000)
    backfill_payload = struct.pack("<IIIB", 11, 3, 4, 0) + raw_batch + raw_batch_4
    write("backfill_data.hex",
          "BACKFILL_DATA for command 11 carrying sequences 3..4 (the raw_batch.hex message,\n"
          "then a one-frame batch with sequence 4), more = 0. Header sequence 42, time 6.",
          message(BACKFILL_DATA, backfill_payload, seq=42, time_us=6))

    section = config_section(cfg)
    write("config_section.hex",
          "CONFIG_GET section format 1, built from ead_agent_docs_v2/CONFIG_V1.json plus\n"
          "as-built mount maps, datasheet sensitivities and haptics_fitted = 0.", section)
    config_payload = (struct.pack("<H", 1) + hashlib.sha256(section).digest()
                      + struct.pack("<H", len(section)) + section)
    write("config_response.hex",
          "Device CONFIG_GET response payload (no header): format 1, SHA-256 of the section,\n"
          "section length, section.", config_payload)

    write("usb_frame_hello_request.hex", "hello_request.hex framed for USB: 00 | COBS(msg || CRC32-LE) | 00.",
          usb_frame(hello_request))

    long_payload = bytes((i % 255) + 1 for i in range(300))
    long_msg = message(STATUS, long_payload, seq=1, time_us=2)
    write("usb_frame_long.hex",
          "USB frame of a message whose 300-byte payload has no zero bytes, crossing a\n"
          "254-byte COBS block. Header: STATUS, length 300, sequence 1, time 2.", usb_frame(long_msg))
    write("long_message.hex", "The unframed message inside usb_frame_long.hex.", long_msg)


if __name__ == "__main__":
    main()
