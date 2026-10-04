#!/usr/bin/env python3
"""Writes a dashboard session as an .eadlog, so tools/replay can run it.

  session2eadlog.py SESSION_ID out.eadlog [--db PATH]

The session's stored configuration goes first (the replay takes its scales,
mount maps and frame rate from it), then its raw frames as RAW_SAMPLE_BATCH
messages: 70-byte frames with the rotation vectors when the session has them
(schema 6, store schema 8), 54-byte ones before. Only the type byte of each
message header is filled in: the replay reads nothing else.
"""
import argparse
import os
import sqlite3
import struct

COLUMNS = ("timestamp_us,frame_index,fax,fay,faz,fgx,fgy,fgz,sax,say,saz,sgx,sgy,sgz,"
           "fqw,fqx,fqy,fqz,sqw,sqx,sqy,sqz,status")
HEADER_SIZE = 20
CONFIG_GET = 0x02
RAW_SAMPLE_BATCH = 0x08
FRAME = struct.Struct("<QI20hH")  # 54 bytes, before schema 6
FRAME6 = struct.Struct("<QI28hH")  # 70 bytes: then rv_foot, rv_shank (docs/protocol.md §5.4)
assert FRAME.size == 54 and FRAME6.size == 70
RV_COLUMNS = "rfw,rfx,rfy,rfz,rsw,rsx,rsy,rsz"


def message(kind, payload):
    head = bytearray(HEADER_SIZE)
    head[2] = kind
    return bytes(head) + payload


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("session")
    ap.add_argument("out")
    ap.add_argument("--db", default=os.path.expanduser("~/.local/share/com.ead.dashboard/ead.sqlite3"))
    args = ap.parse_args()
    db = sqlite3.connect(f"file:{args.db}?mode=ro", uri=True)
    row = db.execute("select config_format, config_sha256, config_section from sessions "
                     "where session_id = ?", (args.session,)).fetchone()
    if row is None:
        raise SystemExit(f"no session {args.session}")
    fmt, sha, section = row
    has_rv_columns = any(c[1] == "rfw" for c in db.execute("pragma table_info(raw_frames)"))
    columns = COLUMNS.replace(",status", f",{RV_COLUMNS},status") if has_rv_columns else COLUMNS
    frames = db.execute(f"select {columns} from raw_frames where session_id = ? "
                        "order by frame_index", (args.session,)).fetchall()
    # A session from schema 6 on has a rotation vector in every frame.
    schema6 = has_rv_columns and bool(frames) and all(r[22] is not None for r in frames)
    if not schema6:
        frames = [r[:22] + r[-1:] for r in frames] if has_rv_columns else frames
    layout = FRAME6 if schema6 else FRAME
    with open(args.out, "wb") as f:
        f.write(b"EADLOG1\n")

        def put(m):
            f.write(struct.pack("<IQ", len(m), 0) + m)

        put(message(CONFIG_GET, struct.pack("<H", fmt) + bytes.fromhex(sha)
                    + struct.pack("<H", len(section)) + section))
        for i in range(0, len(frames), 4):
            chunk = frames[i:i + 4]
            put(message(RAW_SAMPLE_BATCH, struct.pack("<BB", len(chunk), layout.size)
                        + b"".join(layout.pack(*r) for r in chunk)))
    print(f"{args.session}: {len(frames)} frames -> {args.out}")


if __name__ == "__main__":
    main()
