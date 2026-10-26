# Raptor IR queue-depth experiment

**Compiled and policy-tested; not loaded, installed or hardware-qualified.**
This is a workstation-specific kernel experiment, not an application dependency
or a production fix. The installed known-good module remains unchanged.

The patch and policy test in this directory are GPL-2.0-only, separately from
the MIT Rust workspace. See LICENSE.GPL-2.0 and the upstream Linux source for
original authorship. No proprietary driver code or firmware is included.

## Hypothesis and evidence

Initial concurrent 50 FPS streams produced a left accepted rate of 36.5 Hz,
large timestamp skips without matching sequence gaps and visible horizontal
banding. Isolated cameras reached about 49.7 Hz. Concurrent 30 FPS streams
reached 29.9 Hz each, with no pronounced banding in a selected left sample.
A later 50 FPS repeat reached 48.5/50.0 Hz after 212 rejected left startup
buffers; its selected later left sample also lacked pronounced banding.
Thus the failure is intermittent and sensitive to startup/reconfiguration/load.
A queue-depth limit is a hypothesis, not an established cause or remedy.

The existing kernel uses five URBs per stream. For the previously observed
12288-byte bulk payload, five queued transfers provide only about 61 KiB of
buffered USB work. A deeper queue could tolerate more host completion latency.
Actual queue effects, firmware behavior and frame-content correctness need
measurement; successful compilation does not establish them.

## Scope and invariants

- `scancraft_urbs` defaults to 5 and accepts only 5..32 at module load.
- An explicit freeze before USB registration rejects later setter calls.
- The override selects only USB VID:PID 2bc5:06a8, streaming interfaces 1 and 3.
- Raptor color interface 5 and every other device/interface retain five URBs.
- The storage array has capacity 32; allocation and all iteration/free paths
  use the same selected count, which remains stable for active stream lifetime.
- The existing Y14 mapping and metadata timestamp changes are retained unchanged.
- No frame-error checks, UVC parsing rules or firmware commands are changed.

The patch applies to `/home/developer/kernel-6900hs-lto/scancraft-uvc`, the existing
Linux 7.2.11 workstation override source. Candidate source/build lives separately
at `/home/developer/kernel-6900hs-lto/scancraft-uvc-queue`. Source and binary fingerprints
are saved in evidence/kernel-queue-candidate.json, including the installed module
fingerprint. A clean-copy patch application reproduces all three candidate files.

## Reproduced build and policy check

The existing rootless container supplies the matching Clang/LLD 22.1.8 toolchain:

```sh
podman start kernel-6900hs-lto-build
podman exec kernel-6900hs-lto-build make -C /work/linux-7.2.11 M=/work/scancraft-uvc-queue LLVM=1 -j4 modules
python3 experiments/kernel-queue/test-policy.py /home/developer/kernel-6900hs-lto/scancraft-uvc-queue
podman stop kernel-6900hs-lto-build
```

The policy harness compiles the actual candidate functions in userspace stubs.
Its 79 cases cover device/interface selection at three depths, parameter bounds,
unchanged values after rejected input, and rejection after the initialization
freeze. It does not execute kernel allocation, streaming, cleanup, real parser
behavior of kstrtouint, or USB transport. The full kernel module also compiles
without warnings and has matching kernel vermagic. Runtime safety/behavior
remains unverified.

## Pending hardware experiment

Do not replace the current installed module merely because this candidate
builds. A future administrator-controlled test must preserve the exact original
module, hold scanner locks, check all camera consumers, and provide rollback.
Prefer a temporary module load and restore the installed module afterward;
do not persist parameters or change firmware. Compare depth 5 and 32 with the
same format/rate/start ordering, raw image content, timestamps, error/rejection
accounting, reconnect/cleanup and non-Raptor camera behavior. Begin with short
runs before repeating duration trials. Never ignore error flags to improve rates.

Noninteractive sudo authentication was unavailable during preparation. No
candidate load or installed-module modification was attempted. The candidate
is not part of any claimed passing hardware gate.
