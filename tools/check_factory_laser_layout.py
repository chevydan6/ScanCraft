#!/usr/bin/env python3
"""Check synthetic serializer markers, never factory data or hardware."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

SHA256 = "1eeafa48017e8b18deb3bd01fe49e9652f3a810a2a64a7eb01bb26eccdc3402a"
RANGES = [(0x2C, 0, 52), (0x60, 0x34, 52), (0x94, 0x98, 52),
          (0xC8, 0x68, 48), (0xF8, 0xCC, 48), (0x128, 0xFC, 1280)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dll", type=Path)
    parser.add_argument("markers", type=Path)
    args = parser.parse_args()
    if hashlib.sha256(args.dll.read_bytes()).hexdigest() != SHA256:
        raise SystemExit("reference DLL hash mismatch")
    data = args.markers.read_bytes()
    if len(data) != 1680:
        raise SystemExit("marker output length mismatch")
    for destination, source, length in RANGES:
        expected = struct.pack("<" + "f" * (length // 4),
                               *(1000 + source // 4 + i for i in range(length // 4)))
        if data[destination:destination + length] != expected:
            raise SystemExit(f"marker mismatch at record offset {destination:#x}")
    print(json.dumps({
        "reference_sha256": SHA256,
        "entry_rva": "0x44b4", "stop_rva": "0x4635",
        "synthetic_only": True,
        "factory_data_used_in_reference": False,
        "compared_f32_words": sum(n // 4 for _, _, n in RANGES),
        "all_marker_words_equal": True,
        "copy_ranges": [{"record_offset": d, "internal_offset": s, "bytes": n}
                        for d, s, n in RANGES],
        "physical_camera_assignment_verified": False,
        "translation_units_verified": False,
        "plane_formula_verified": False,
        "image_mode_binding_verified": False,
    }, indent=2))


if __name__ == "__main__":
    main()
