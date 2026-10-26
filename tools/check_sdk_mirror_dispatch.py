#!/usr/bin/env python3
"""Read-only dispatch audit for one hashed private installed SDK; no DLL execution.

Raw vendor code/disassembly is neither saved nor printed. Static conclusions
do not prove the Raptor's active processor or the inversion flag's value.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess

EXPECTED = "d36c0e8ea1e2123c5bc8288d8676608da255bffd8063e2b051dcf39c3fa79231"


def audit(path):
    data = Path(path).read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if digest != EXPECTED:
        raise ValueError("SDK hash differs from the inspected reference; refusing fixed offsets")

    def text_offset(rva):
        if not 0x1000 <= rva < 0x65F048:
            raise ValueError("address outside inspected text section")
        return 0x400 + rva - 0x1000

    def disassemble(start, stop):
        return subprocess.check_output([
            "objdump", "-d", "-M", "intel", f"--start-address={0x180000000 + start}",
            f"--stop-address={0x180000000 + stop}", str(path)], text=True, timeout=30)

    # Inspected getter: IDs <=112, special direct branch for 112,
    # then a compressed switch indexed by property-14 for IDs 14..82.
    getter = disassemble(0x344920, 0x344AA1)
    for pattern in [r"cmp\s+edx,0x70", r"je\s+0x180344a3d",
                    r"add\s+edx,0xfffffff2", r"0x3450b0", r"0x3450a0",
                    r"cmp\s+BYTE PTR \[rdi-0x5f\],bl", r"sete\s+bl"]:
        if not re.search(pattern, getter):
            raise ValueError("expected getter control flow absent")
    targets = {}
    for property_id in [14, 18, 81]:
        index = data[text_offset(0x3450B0 + property_id - 14)]
        destination = struct.unpack_from("<I", data, text_offset(0x3450A0 + index * 4))[0]
        targets[str(property_id)] = hex(destination)
    if targets != {"14": "0x344a3d", "18": "0x344a3d", "81": "0x345063"}:
        raise ValueError("unexpected compressed switch mapping")
    name_offset = 0x65E600 + 0x694360 - 0x660000
    if data[name_offset:name_offset + 12] != b"FrameMirror\0":
        raise ValueError("expected host processor block name absent")
    if not re.search(r"# 0x180694360", getter):
        raise ValueError("getter no longer references inspected mirror block name")
    setter = disassemble(0x348760, 0x348767)
    if not re.search(r"mov\s+BYTE PTR \[rcx\+0xa1\],dl", setter) or not re.search(r"\bret\b", setter):
        raise ValueError("expected host flag setter absent")
    expected_callers = [
        (0x1BA2B7, 0x1B9E60, 0x1BA7BF, "Astra2Device"),
        (0x1DF75C, 0x1DEFB0, 0x1DFE60, "Gemini2Device"),
        (0x1F2FD0, 0x1F2830, 0x1F3610, "Gemini2LDevice"),
        (0x259F1E, 0x2591E0, 0x25A364, "AvatarDevice"),
        (0x266ACE, 0x265D90, 0x266F14, "FerretUvcDevice"),
        (0x282EFE, 0x281FB0, 0x283285, "PioneerDevice"),
    ]
    # Search direct relative calls only. This does not exclude indirect calls,
    # inlined stores or writes through aliases of the object.
    direct = []
    text = data[0x400:0x400 + 0x65E048]
    position = 0
    while (position := text.find(b"\xe8", position)) != -1:
        if position + 5 <= len(text):
            target = 0x1000 + position + 5 + struct.unpack_from("<i", text, position + 1)[0]
            if target == 0x348760:
                direct.append(0x1000 + position)
        position += 1
    if direct != [entry[0] for entry in expected_callers]:
        raise ValueError("direct setter call sites differ from inspected list")
    callers = []
    for call, start, stop, class_name in expected_callers:
        assembly = disassemble(start, stop)
        before_call = disassemble(call - 5, call + 5)
        if not re.search(r"mov\s+dl,0x1", before_call) or not re.search(r"call\s+0x180348760", before_call):
            raise ValueError("call site no longer supplies true")
        names = []
        for address in re.findall(r"# 0x([0-9a-f]+)", assembly):
            rva = int(address, 16) - 0x180000000
            if 0x660000 <= rva < 0x848FE8:
                offset = 0x65E600 + rva - 0x660000
                names.append(data[offset:offset + 180].split(b"\0", 1)[0])
        expected_name = f"libobsensor::{class_name}::createIrSensor".encode()
        if expected_name not in names:
            raise ValueError("sensor creation function name reference absent")
        callers.append({"class_name_reference": class_name, "function_rva": hex(start),
                        "function_end_rva": hex(stop), "setter_call_rva": hex(call),
                        "supplied_boolean": True})
    def rdata_offset(rva):
        if not 0x660000 <= rva < 0x848FE8:
            raise ValueError("address outside inspected rdata section")
        return 0x65E600 + rva - 0x660000

    if struct.unpack_from("<Q", data, rdata_offset(0x6A5150))[0] != 0x180281FB0:
        raise ValueError("Pioneer vtable no longer references inspected IR creation function")
    locator_va = struct.unpack_from("<Q", data, rdata_offset(0x6A4F30))[0]
    if locator_va != 0x18073E078:
        raise ValueError("unexpected Pioneer RTTI locator")
    signature, offset, _, type_rva, _, _ = struct.unpack_from("<6I", data, rdata_offset(0x73E078))
    if (signature, offset, type_rva) != (1, 0, 0x876AC0):
        raise ValueError("unexpected Pioneer RTTI fields")
    descriptor_offset = 0x847600 + type_rva - 0x849000
    if data[descriptor_offset + 16:descriptor_offset + 80].split(b"\0", 1)[0] != b".?AVPioneerDevice@libobsensor@@":
        raise ValueError("Pioneer RTTI name differs")
    constructor = disassemble(0x27FCA9, 0x27FCB3)
    if not re.search(r"# 0x1806a4f38", constructor) or not re.search(r"mov\s+QWORD PTR \[rsi\],rax", constructor):
        raise ValueError("Pioneer constructor vtable initialization differs")
    dispatch_paths = []
    for transport, initializer, immediates, vector, membership, branch, call in [
        ("USB", 0x59480, [0x594A7, 0x594B6], 0x186DDB8, 0x37C4C6, 0x37C6F6, 0x37C793),
        ("network", 0x5A620, [0x5A647, 0x5A656], 0x186E098, 0x382953, 0x382CA3, 0x382D33),
    ]:
        words = b"".join(data[text_offset(rva):text_offset(rva) + 4] for rva in immediates)
        pids = list(struct.unpack("<4H", words))
        if pids != [0x6A8, 0xC00, 0xC03, 0xC11]:
            raise ValueError("unexpected Pioneer PID list initializer")
        init_assembly = disassemble(initializer, initializer + 0x60)
        for pattern in [rf"# 0x{0x180000000 + vector:x}", r"lea\s+r8,\[rsp\+0x30\]",
                        r"lea\s+rdx,\[rsp\+0x28\]", r"call\s+0x1800789e0"]:
            if not re.search(pattern, init_assembly):
                raise ValueError("PID initializer vector/range shape differs")
        membership_assembly = disassemble(membership, membership + 0x35)
        for pattern in [rf"# 0x{0x180000000 + vector:x}",
                        rf"# 0x{0x180000000 + vector + 8:x}",
                        r"WORD PTR \[r(?:8|ax)\+0x28\]",
                        rf"je\s+0x{0x180000000 + branch:x}", r"add\s+r(?:ax|cx),0x2"]:
            if not re.search(pattern, membership_assembly):
                raise ValueError("PID membership branch differs")
        branch_assembly = disassemble(branch, call + 5)
        if not re.search(r"call\s+0x18027fc10", branch_assembly):
            raise ValueError("membership branch lacks Pioneer constructor call")
        dispatch_paths.append({"transport": transport, "initializer_rva": hex(initializer),
                               "pid_list_decimal": pids, "list_global_rva": hex(vector),
                               "membership_test_rva": hex(membership),
                               "match_branch_rva": hex(branch), "constructor_call_rva": hex(call),
                               "conditional_pid_membership_to_constructor_path_verified": True})
    pid_api = disassemble(0x3954F0, 0x39552B)
    if not re.search(r"mov\s+eax,DWORD PTR \[rax\+0x28\]", pid_api):
        raise ValueError("PID API field offset differs")
    mirror_entries = []
    for start, flag_address, stop in [(0x282A28, 0x282A41, 0x282A48),
                                      (0x282CC2, 0x282CDB, 0x282CE2)]:
        assembly = disassemble(start, stop)
        if not re.search(r"# 0x180694360", assembly) or not re.search(
                r"mov\s+BYTE PTR \[rbp\+0x(?:388|2c0)\],0x1", assembly):
            raise ValueError("Pioneer mirror configuration name/entry flag differs")
        mirror_entries.append({"name_reference_region_rva": hex(start),
                               "entry_flag_write_rva": hex(flag_address), "entry_flag": 1})
    processor = disassemble(0x340CF6, 0x340D03)
    if not re.search(r"movzx\s+edx,BYTE PTR \[rsi\+0x20\]", processor) or not re.search(
            r"call\s+QWORD PTR \[rax\+0x10\]", processor):
        raise ValueError("processor no longer forwards configuration entry flag")
    for slot, target in [(0x6B4378, 0x180312D20), (0x6B4380, 0x18032CAC0),
                         (0x6B43C0, 0x18032E4C0)]:
        if struct.unpack_from("<Q", data, rdata_offset(slot))[0] != target:
            raise ValueError("inspected FrameMirror vtable slot differs")
    enable_setter = disassemble(0x312D20, 0x312D27)
    if not re.search(r"xchg\s+BYTE PTR \[rcx\+0xb9\],dl", enable_setter):
        raise ValueError("mirror enable storage differs")
    enable_getter = disassemble(0x32CAC0, 0x32CADF)
    if not re.search(r"BYTE PTR \[rcx\+0xd8\]", enable_getter) or not re.search(r"sete\s+al", enable_getter):
        raise ValueError("mirror getter internal inversion branch differs")
    mirror_creator = disassemble(0x33F4AF, 0x33F4D4)
    for pattern in [r"xor\s+edx,edx", r"mov\s+r8d,0xe0", r"call\s+0x180491c4e",
                    r"call\s+0x18032c980", r"# 0x1806b4368"]:
        if not re.search(pattern, mirror_creator):
            raise ValueError("mirror creator zero-initialization/base construction differs")
    transform_constructor = disassemble(0x32C998, 0x32C9AE)
    if not re.search(r"xor\s+eax,eax", transform_constructor) or not re.search(
            r"mov\s+BYTE PTR \[rbx\+0xd8\],al", transform_constructor):
        raise ValueError("base transform internal inversion flag initialization differs")
    return {
        "scope": "static base FrameProcessor getter, not active device dispatch",
        "dll_sha256": digest, "hash_verified": True,
        "getter_rva": "0x344920", "compressed_switch_targets_rva": targets,
        "direct_case_112_target_rva": "0x344a3d",
        "shared_mirror_block_name": "FrameMirror", "shared_mirror_block_name_rva": "0x694360",
        "getter_conditionally_inverts_host_block_boolean": True,
        "inversion_flag_setter_rva": "0x348760",
        "setter_flag_offset_from_object_base": "0xa1",
        "direct_setter_callers": callers,
        "indirect_or_aliased_flag_writes_excluded": False,
        "pioneer_ir_creation_vtable_slot_rva": "0x6a5150",
        "pioneer_primary_vtable_rva": "0x6a4f38",
        "pioneer_rtti_type_name_verified": True,
        "pioneer_constructor_vtable_initialization_rva": "0x27fca9",
        "scancraft_pid_to_pioneer_constructor_binding_verified": False,
        "static_pid_dispatch_paths": dispatch_paths,
        "pid_getter_field_offset": "0x28",
        "runtime_dispatch_or_gate_state_verified": False,
        "earlier_pid_membership_lists_exclusivity_verified": False,
        "pioneer_mirror_configuration_entries": mirror_entries,
        "processor_forwards_entry_flag_to_block_virtual_setter_rva": "0x340cf9",
        "mirror_enable_setter_rva": "0x312d20",
        "mirror_enable_storage_offset": "0xb9",
        "mirror_getter_rva": "0x32cac0",
        "mirror_getter_additional_inversion_flag_offset": "0xd8",
        "mirror_internal_flag_or_later_overrides_verified": False,
        "mirror_creator_rva": "0x33f460",
        "mirror_creator_calls_base_transform_constructor_rva": "0x32c980",
        "mirror_internal_inversion_flag_initial_value": 0,
        "mirror_internal_inversion_flag_initialization_verified": True,
        "mirror_internal_inversion_flag_runtime_value_verified": False,
        "conditional_flag_address_relative_to_this": "-0x5f",
        "base_getter_case_81_is_unsupported": True,
        "specialized_color_accessor_behavior_verified": False,
        "scancraft_uses_this_getter_verified": False, "runtime_inversion_flag_verified": False,
        "sdk_output_image_orientation_verified": False, "vendor_code_executed": False,
        "raw_vendor_code_or_disassembly_distributed": False,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("private_stock_sdk", type=Path)
    args = parser.parse_args()
    print(json.dumps(audit(args.private_stock_sdk), indent=2))
