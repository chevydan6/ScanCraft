#!/usr/bin/env python3
"""Optional synthetic-only reference probe; never loads a DLL or accesses a scanner.

Requires Linux x86-64, GCC and bubblewrap. An exact-hash private reference is
required; no proprietary bytes are included in this repository or report.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import stat
import subprocess
import tempfile

DLL_SHA256 = "d36c0e8ea1e2123c5bc8288d8676608da255bffd8063e2b051dcf39c3fa79231"
LEAF_SHA256 = "79f60568dba34b46690caffd20fb045e0816e08c681fbc16ce40df41ccb4700d"
LEAF_RVA = 0x3297B0
LEAF_LENGTH = 93
MAX_DLL_BYTES = 32 * 1024 * 1024


def extract_leaf(path):
    """Read a bounded regular file once; reject unknown content before execution."""
    descriptor = os.open(path, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW)
    with os.fdopen(descriptor, "rb") as source:
        metadata = os.fstat(source.fileno())
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > MAX_DLL_BYTES:
            raise ValueError("reference must be a bounded regular file")
        data = source.read(MAX_DLL_BYTES + 1)
    if len(data) > MAX_DLL_BYTES or hashlib.sha256(data).hexdigest() != DLL_SHA256:
        raise ValueError("SDK hash differs from the inspected reference; refusing execution")
    # Exact reference .text mapping: file offset0x400 corresponds to RVA0x1000.
    offset = 0x400 + LEAF_RVA - 0x1000
    leaf = data[offset:offset + LEAF_LENGTH]
    if hashlib.sha256(leaf).hexdigest() != LEAF_SHA256:
        raise ValueError("private leaf hash differs; refusing execution")
    return leaf


def probe(path):
    """Run only the checked leaf on generated pixels in a device-free sandbox."""
    if platform.system() != "Linux" or platform.machine().lower() not in ("x86_64", "amd64"):
        raise ValueError("probe requires Linux x86-64 and the Microsoft x64 calling convention")
    leaf = extract_leaf(path)
    compiler, sandbox = shutil.which("gcc"), shutil.which("bwrap")
    if not compiler or not sandbox:
        raise ValueError("optional research probe requires gcc and bubblewrap")
    helper = Path(__file__).with_name("sdk_byte_mirror_probe.c")
    helper_bytes = helper.read_bytes()
    with tempfile.TemporaryDirectory(prefix="scancraft-sdk-mirror-") as temporary:
        root = Path(temporary)
        # This directory is private and removed on success or failure. Vendor
        # bytes never enter source control, stdout, a review upload or a fixture.
        (root / "private-leaf.bin").write_bytes(leaf)
        (root / "probe.c").write_bytes(helper_bytes)
        subprocess.run([compiler, "-O2", "-Wall", "-Wextra", "-Werror", str(root / "probe.c"),
                        "-o", str(root / "probe")], check=True, timeout=30, capture_output=True)
        command = [sandbox, "--unshare-all", "--new-session", "--die-with-parent", "--cap-drop", "ALL",
                   "--clearenv", "--ro-bind", "/usr", "/usr", "--ro-bind", "/lib", "/lib",
                   "--ro-bind-try", "/lib64", "/lib64", "--dev", "/dev", "--proc", "/proc",
                   "--tmpfs", "/tmp", "--ro-bind", str(root), "/work", "--chdir", "/work",
                   "/work/probe", "/work/private-leaf.bin"]
        completed = subprocess.run(command, check=True, timeout=10, capture_output=True, text=True)
        result = json.loads(completed.stdout)
        expected = {"synthetic_shapes": 6, "pixels_checked_each_direction": 2304075,
                    "row_reversal_exact": True, "involution_exact": True}
        if result != expected:
            raise ValueError("unexpected synthetic probe report")
    return {
        "scope": "synthetic numeric leaf, not a loaded SDK or active Raptor pipeline",
        "dll_sha256": DLL_SHA256, "leaf_sha256": LEAF_SHA256,
        "leaf_rva": hex(LEAF_RVA), "leaf_length": LEAF_LENGTH,
        "helper_sha256": hashlib.sha256(helper_bytes).hexdigest(),
        "sandbox": "bubblewrap: unshared namespaces/network, private dev, read-only work and runtime libraries",
        "input_first_pass_read_only": True, "buffer_guard_pages": True,
        "comparison": "output[y*width+x] == input[(y+1)*width-1-x] for every pixel",
        "result": result,
        "vendor_leaf_executed": True, "dll_loaded": False, "hardware_accessed": False,
        "sdk_output_image_orientation_verified": False,
        "camera_assignment_verified": False, "calibration_binding_verified": False,
        "raw_vendor_code_or_disassembly_distributed": False,
        "native_defaults_changed": False,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("private_stock_sdk", type=Path)
    args = parser.parse_args()
    print(json.dumps(probe(args.private_stock_sdk), indent=2))
