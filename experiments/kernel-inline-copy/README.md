# Bounded Raptor completion-copy experiment

**Opt-in experimental kernel patch. Offline checks and finite capture/cancellation
trials pass within their scope; acquisition reliability remains unqualified.** It is not a required ScanCraft
dependency, persistent driver installation, or established acquisition fix.

The patch and test harness are **GPL-2.0-only**, separately from the MIT Rust
workspace. `LICENSE.GPL-2.0` preserves the Linux license. The patch modifies
existing Linux UVC code; no proprietary driver code or firmware is included.

## Why test this boundary?

The [host-service trace](../../docs/RAPTOR_HOST_SERVICE_TRACE.md) observed camera
copy jobs waiting 909–1,506 µs, alongside 923/895 µs with zero software-pending
video requests and a paired short frame. The original UVC callback queues copy
operations; its worker copies data, releases buffer references and resubmits the
USB request. This experiment tests whether avoiding that scheduling dependency
for a small, bounded copy changes the observed starvation mechanism.

The same trace contains other truncations overlapping longer intervals without
selected host events. This patch does not purport to fix those intervals, startup
framing, the earlier exceptional byte count, physical synchronization or accuracy.
The [same-candidate hardware comparison](../../docs/RAPTOR_INLINE_COPY_TRIAL.md)
records ongoing rejects per camera of 2 → 9 → 3 with the option off → on → off.
The enabled sample has no camera-copy worker events but still loses frame data.
No reliable acquisition or general performance improvement is established. The
installed baseline was restored after each trial; keep this option experimental.

## Scope and invariants

`scancraft_inline_copy` defaults to false, accepts the existing kernel boolean
parser at module load, and is read-only in sysfs. A setter freeze before USB
registration rejects subsequent setter calls. The option is stable during stream
lifetime.

After normal decoding, completion-context copying is permitted only when:

- The option is enabled and USB VID:PID is `2bc5:06a8`.
- The streaming interface is 1 or 3, the stream is video capture, and the USB
  pipe is bulk input.
- Actual transfer length is positive and no greater than the allocated transfer
  length, which is at most 12,288 bytes.
- Exactly one copy operation exists, and its byte length is no greater than
  actual transfer length.

An eligible callback performs that one memcpy, releases exactly the reference
previously acquired by decoding, and clears the completed operation count. It
then follows the existing `GFP_ATOMIC` submission/error-reporting path. Ineligible
cases follow the original completion logic: zero operations resubmit immediately;
remaining operations use the asynchronous worker. The original worker's source
is unchanged.

The cap bounds **copied bytes and operation count, not elapsed interrupt time**.
Reference release may complete a video buffer. Its existing kernel path already
supports invocation from completion context, but IRQ latency and runtime races
still require direct validation. Cancellation/error dispatch precedes decoding
and is unchanged. Queue depth, payload parsing, error flags, metadata, recording
formats and scanner commands are unchanged; the original five requests remain.

## Reproduction and checks

The patch applies to the workstation's existing Linux 7.2.11 UVC override source,
which includes its existing format/metadata changes. This is a specific baseline,
not a claim that the patch applies unchanged to every Linux version.

| Build | SHA-256 / identity |
| --- | --- |
| Rebuilt baseline and installed module | `579c7633261b27cfd76af16f4b1ad57c1306ce6c7d75ca914e15693e3ec127e5` |
| Experimental candidate | `bcce45bd2752db746373be96e216d866f3393ad74a4a9fccc01462f1e076bbbf` |
| Candidate srcversion | `12034367675945449357638` |
| Vermagic | `7.2.11-6900hs-lto SMP preempt mod_unload` |

The baseline rebuild matches the installed module byte for byte. The candidate
build uses the same existing Clang/LLD toolchain. Applying the public patch to a
clean baseline source copy reproduces its three modified files; every other
source file remains identical. Building that clean copy produces a byte-identical
candidate module. No kernel module or private data is committed.

With a matching source tree and toolchain available, the relevant commands are:

```sh
patch -d /path/to/copied-uvc-source -p1 < raptor-bounded-copy.patch
python3 test-policy.py /path/to/copied-uvc-source
make -C /path/to/matching-kernel M=/path/to/copied-uvc-source LLVM=1 -j4 modules
perl /path/to/matching-kernel/scripts/checkpatch.pl --no-tree --no-signoff - < raptor-bounded-copy.patch
```

The harness extracts the actual candidate completion, worker, inline-copy and
parameter-setter functions and compiles them with `-std=c11 -O2 -Wall -Wextra
-Werror`. Its **54 cases** check opt-out behavior, both IR interfaces, copy sizes
including zero/exact bound, adjacent bounds, unsupported devices/interfaces/pipes,
zero/multiple operations, byte sentinels, reference release after copying,
unchanged error state, submission flags/errors, cancellation/frozen/error paths,
metadata presence, and parameter freeze. The initial freeze location and sysfs
permissions are also checked in source.

These are userspace stubs, not kernel USB, allocator, real boolean-parser,
V4L2-concurrency, suspend/disconnect or interrupt-latency tests. They do not prove
runtime safety. Linux checkpatch reports zero errors/warnings. The unchanged Rust
workspace passes formatting, strict Clippy, 890 tests/one existing ignored and
optimized release build. Private `inline-copy-20261104` binds source/build logs,
tests, baseline comparison and patch reproduction.

## Before any hardware conclusion

Independent review and a temporary, controlled comparison must retain the exact
installed module and normal scanner locks/consumer checks. Compare the **same
candidate binary with the option disabled and enabled**, keeping queue depth and
capture settings fixed. Restore the installed module, formats and service after
each arm. Preserve every startup/ongoing rejection and recorded frame; measure
copy dispatch, IRQ intervals, accepted image integrity, cancellation and cleanup.

Begin with a short capture and include cancellation/error cases before sustained
testing. A lower worker-delay count alone is insufficient: higher IRQ latency,
reference/lifetime defects or unchanged truncation would limit or reject the
approach. Do not make it a permanent driver setting on the basis of compilation
or stub tests. No load command is part of this reproduction section.
