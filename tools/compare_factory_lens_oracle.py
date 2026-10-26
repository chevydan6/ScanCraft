#!/usr/bin/env python3
"""Compare native decoded fields to a private offline vendor-parser dump.
Only emits counts/labels and file hashes, never calibration values or pointers.
The oracle ABI/offsets apply only to the locally investigated bundled DLL.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct


def compare(native_path, oracle_path):
    native_bytes = Path(native_path).read_bytes()
    if len(native_bytes) > 65536:
        raise ValueError("native JSON exceeds diagnostic limit")
    native = json.loads(native_bytes)
    oracle = Path(oracle_path).read_bytes()
    if len(oracle) != 131076 or struct.unpack_from("<i", oracle)[0] != 0:
        raise ValueError("oracle failed or dump size differs")
    data = oracle[4:]
    lenses = native["lenses"]
    if len(lenses) != 3:
        raise ValueError("requires exactly three named lenses")
    for i, (lens, expected) in enumerate(zip(lenses, ["IR_L", "IR_R", "COLOR"])):
        base = i * 0x840
        if lens["label"] != expected or data[base + 0x87C:base + 0x888].split(b"\0")[0].decode() != expected:
            raise ValueError(f"camera label mismatch at {i}")
        height, width = struct.unpack_from("<II", data, base + 0x4C)
        if (width, height) != (lens["width"], lens["height"]):
            raise ValueError(f"dimensions mismatch at {i}")
        values = lens["intrinsic_values"] + lens["brown_values"]
        if len(values) != 12 or struct.pack("<12f", *values) != data[base + 0x54:base + 0x84]:
            raise ValueError(f"lens float bits mismatch at {i}")
        pose = lens["rotation_fields"] + lens["translation_fields"]
        if len(pose) != 6 or struct.pack("<6f", *pose) != data[base + 0x864:base + 0x87C]:
            raise ValueError(f"pose float bits mismatch at {i}")
    return {
        "schema_version": 1,
        "oracle_status": 0,
        "native_json_sha256": hashlib.sha256(native_bytes).hexdigest(),
        "private_oracle_dump_sha256": hashlib.sha256(oracle).hexdigest(),
        "labels_equal": True,
        "camera_count": 3,
        "dimension_integers_equal": 6,
        "float32_values_bit_equal": 36,
        "pose_float32_values_bit_equal": 18,
        "angle_convention_verified": False,
        "translation_units_verified": False,
        "stereo_transform_verified": False,
        "coefficient_conventions_verified": False,
        "physical_accuracy_verified": False,
    }


def compare_named_export(native_path, text_path):
    """Read only the simple scalar subset emitted by this reference exporter."""
    native = json.loads(Path(native_path).read_text())
    text = Path(text_path).read_text()
    if len(text) > 65536:
        raise ValueError("reference text exceeds diagnostic limit")
    groups = {name: {} for name in ["IR_L", "IR_R", "COLOR"]}
    current = None
    for line in text.splitlines():
        if line.rstrip(":") in groups and line.endswith(":"):
            current = line[:-1]
        elif current and ": " in line:
            key, value = line.strip().split(": ", 1)
            if key in groups[current]:
                raise ValueError("duplicate reference scalar")
            groups[current][key] = value
    for lens in native["lenses"]:
        fields = groups[lens["label"]]
        for name, expected in [("ImgWidth", lens["width"]), ("ImgHeight", lens["height"])]:
            if int(fields[name]) != expected:
                raise ValueError("named dimension mismatch")
        values = dict(zip(["Fx", "Fy", "Cx", "Cy"], lens["intrinsic_values"]))
        values.update(zip(["RX", "RY", "RZ"], lens["rotation_fields"]))
        values.update(zip(["TX", "TY", "TZ"], lens["translation_fields"]))
        values.update({name: lens["brown_values"][i] for name, i in
                       [("K1", 0), ("K2", 1), ("K3", 2), ("K4", 3), ("P1", 6), ("P2", 7)]})
        for name, expected in values.items():
            if struct.pack("<f", float(fields[name])) != struct.pack("<f", expected):
                raise ValueError(f"named float mismatch: {lens['label']}.{name}")
    return {"named_float32_fields_equal": 48, "named_dimension_fields_equal": 6,
            "private_text_sha256": hashlib.sha256(text.encode()).hexdigest(),
            "remaining_brown_field_names_verified": False}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("native_json")
    parser.add_argument("private_oracle_dump")
    parser.add_argument("--named-export")
    args = parser.parse_args()
    result = compare(args.native_json, args.private_oracle_dump)
    if args.named_export:
        result.update(compare_named_export(args.native_json, args.named_export))
    print(json.dumps(result, indent=2))
