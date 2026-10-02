#!/usr/bin/env python3
"""Writes a dashboard session as an .eadlog, so tools/replay can run it.

  session2eadlog.py SESSION_ID out.eadlog [--db PATH]

The session's stored configuration goes first (the replay takes its scales and
mount maps from it), then its raw frames as RAW_SAMPLE_BATCH messages. Only the
type byte of each message header is filled in: the replay reads nothing else.
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
FRAME = struct.Struct("<QI20hH")  # 54 bytes, docs/protocol.md
assert FRAME.size == 54


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
    frames = db.execute(f"select {COLUMNS} from raw_frames where session_id = ? "
                        "order by frame_index", (args.session,)).fetchall()
    with open(args.out, "wb") as f:
        f.write(b"EADLOG1\n")

        def put(m):
            f.write(struct.pack("<IQ", len(m), 0) + m)

        put(message(CONFIG_GET, struct.pack("<H", fmt) + bytes.fromhex(sha)
                    + struct.pack("<H", len(section)) + section))
        for i in range(0, len(frames), 4):
            chunk = frames[i:i + 4]
            put(message(RAW_SAMPLE_BATCH, struct.pack("<BB", len(chunk), FRAME.size)
                        + b"".join(FRAME.pack(*r) for r in chunk)))
    print(f"{args.session}: {len(frames)} frames -> {args.out}")


if __name__ == "__main__":
    main()
