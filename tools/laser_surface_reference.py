#!/usr/bin/env python3
"""Execute a private captured numerical fragment on synthetic rays only.

No captured source is bundled, printed or written. No scanner access occurs.
Uses the local OpenCL GPU; this is an offline numerical reference, not scanning.
"""
import argparse
import ctypes as C
import hashlib
import json
import os
from pathlib import Path
import struct

SOURCE_SHA = "6163b6d8448a01317e7336e761bbc9bce8bc4f65dc665deaa4e733cbba1a1f49"
CASES = [
    ([0., 0., 0., 100.], [.25, .1]),
    ([0., 0., -.4, 80.], [.1, .2]),
    ([.001, .02, -.3, 100.], [.05, .2]),
    ([.001, 0., 0., 100.], [1., 1.]),
    ([1., 0., 0., 1.], [2., 1.]),
    ([1., 0., 0., 1.], [0., 1.]),
    ([0., 0., 0., 100.], [-.25, .1]),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("private_captured_source", type=Path)
    args = parser.parse_args()
    source = args.private_captured_source.read_bytes()
    if hashlib.sha256(source).hexdigest() != SOURCE_SHA:
        raise SystemExit("captured source hash mismatch")
    start = source.index(b"float l_ny2 =")
    end = source.index(b"bool lambda1_in_depth", start)
    fragment = source[start:end]
    # Keep the original numerical expressions private. Only this independently
    # written harness and synthetic results are distributed.
    prefix = b"""__kernel void probe(__global float* rays, __global float* params,
        __global float* result) {
        int i=get_global_id(0), param_idx=7*i;
        float l_nx=rays[2*i], l_ny=rays[2*i+1];
        float fx_l=1.f, fy_l=1.f, cx_l=0.f, cy_l=0.f;
        __global float* d_planes_params=params;
    """
    program_source = prefix + fragment + b"result[2*i]=lambda1; result[2*i+1]=lambda2; }"
    os.environ.setdefault("RUSTICL_ENABLE", "radeonsi")
    cl = C.CDLL("libOpenCL.so.1")
    P, U, I, S, Q = C.c_void_p, C.c_uint, C.c_int, C.c_size_t, C.c_uint64

    def api(name, result, args):
        fn = getattr(cl, name)
        fn.restype, fn.argtypes = result, args
        return fn

    def ok(status):
        if status != 0:
            raise RuntimeError(f"OpenCL status {status}")

    platforms = api("clGetPlatformIDs", I, [U, C.POINTER(P), C.POINTER(U)])
    devices = api("clGetDeviceIDs", I, [P, Q, U, C.POINTER(P), C.POINTER(U)])
    device_info = api("clGetDeviceInfo", I, [P, U, S, P, C.POINTER(S)])
    context_create = api("clCreateContext", P, [P, U, C.POINTER(P), P, P, C.POINTER(I)])
    queue_create = api("clCreateCommandQueue", P, [P, P, Q, C.POINTER(I)])
    program_create = api("clCreateProgramWithSource", P, [P, U, C.POINTER(C.c_char_p), C.POINTER(S), C.POINTER(I)])
    build = api("clBuildProgram", I, [P, U, C.POINTER(P), C.c_char_p, P, P])
    kernel_create = api("clCreateKernel", P, [P, C.c_char_p, C.POINTER(I)])
    buffer_create = api("clCreateBuffer", P, [P, Q, S, P, C.POINTER(I)])
    set_arg = api("clSetKernelArg", I, [P, U, S, P])
    run = api("clEnqueueNDRangeKernel", I, [P, P, U, P, C.POINTER(S), P, U, P, P])
    read = api("clEnqueueReadBuffer", I, [P, P, U, S, S, P, U, P, P])
    release = {kind: api("clRelease" + kind, I, [P]) for kind in ["Context", "CommandQueue", "Program", "Kernel", "MemObject"]}
    resources = []
    error = I()

    def keep(kind, handle):
        ok(error.value)
        if not handle:
            raise RuntimeError(f"null OpenCL {kind}")
        resources.append((kind, handle))
        return handle

    try:
        ps, count = (P * 16)(), U()
        ok(platforms(16, ps, C.byref(count)))
        chosen = None
        for platform in ps[:count.value]:
            ds, number = (P * 16)(), U()
            if devices(platform, 4, 16, ds, C.byref(number)) == 0 and number.value:
                chosen = (P * 1)(ds[0])
                break
        if chosen is None:
            raise RuntimeError("no OpenCL GPU available")
        name = C.create_string_buffer(1024)
        ok(device_info(chosen[0], 0x102B, len(name), name, None))
        context = keep("Context", context_create(None, 1, chosen, None, None, C.byref(error)))
        queue = keep("CommandQueue", queue_create(context, chosen[0], 0, C.byref(error)))
        texts, sizes = (C.c_char_p * 1)(program_source), (S * 1)(len(program_source))
        program = keep("Program", program_create(context, 1, texts, sizes, C.byref(error)))
        options = b"-cl-std=CL1.2 -cl-single-precision-constant"
        ok(build(program, 1, chosen, options, None, None))
        kernel = keep("Kernel", kernel_create(program, b"probe", C.byref(error)))
        rays = (C.c_float * (2 * len(CASES)))(*(v for _, ray in CASES for v in ray))
        coefficients = (C.c_float * (7 * len(CASES)))(*(v for surface, _ in CASES for v in surface + [0., 0., 0.]))
        output = (C.c_float * (2 * len(CASES)))()
        handles = []
        for array in [rays, coefficients]:
            handles.append(keep("MemObject", buffer_create(context, 4 | 32, C.sizeof(array), array, C.byref(error))))
        handles.append(keep("MemObject", buffer_create(context, 2, C.sizeof(output), None, C.byref(error))))
        for i, handle in enumerate(handles):
            ok(set_arg(kernel, i, C.sizeof(P), C.byref(P(handle))))
        ok(run(queue, kernel, 1, None, (S * 1)(len(CASES)), None, 0, None, None))
        ok(read(queue, handles[-1], 1, 0, C.sizeof(output), output, 0, None, None))
        # Preserve exact f32 synthetic inputs and finite roots; nonfinite roots
        # are explicit nulls (miss case), never invalid JSON or fabricated roots.
        f32 = lambda v: struct.unpack("<f", struct.pack("<f", v))[0]
        finite = lambda v: v if abs(v) < float("inf") else None
        results = [{"coefficients": list(map(f32, surface)), "normalized_ray": list(map(f32, ray)),
                    "reference_roots": [finite(output[2*i]), finite(output[2*i+1])]}
                   for i, (surface, ray) in enumerate(CASES)]
        print(json.dumps({"captured_source_sha256": SOURCE_SHA,
                          "fragment_sha256": hashlib.sha256(fragment).hexdigest(),
                          "device": name.value.decode(), "options": options.decode(),
                          "synthetic_only": True, "factory_record_binding_verified": False,
                          "fragment_only_not_full_kernel": True,
                          "reference_adds_quadratic_epsilon": 1e-10,
                          "cases": results}, indent=2))
    finally:
        for kind, handle in reversed(resources):
            release[kind](handle)


if __name__ == "__main__":
    main()
