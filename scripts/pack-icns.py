#!/usr/bin/env python3
"""Pack an .iconset's PNGs into an .icns verbatim. iconutil re-encodes them, which undoes
pngquant and triples the file."""
import struct, sys, pathlib

TYPES = {
    "icon_16x16.png": b"icp4", "icon_16x16@2x.png": b"ic11",
    "icon_32x32.png": b"icp5", "icon_32x32@2x.png": b"ic12",
    "icon_128x128.png": b"ic07", "icon_128x128@2x.png": b"ic13",
    "icon_256x256.png": b"ic08", "icon_256x256@2x.png": b"ic14",
    "icon_512x512.png": b"ic09", "icon_512x512@2x.png": b"ic10",
}
src, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
body = b""
for name, tag in TYPES.items():
    data = (src / name).read_bytes()
    body += tag + struct.pack(">I", len(data) + 8) + data
out.write_bytes(b"icns" + struct.pack(">I", len(body) + 8) + body)
