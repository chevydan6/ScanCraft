# Read-only mirror diagnostics

Important clarification from subsequent SDK-source inspection: these are raw
firmware queries using SDK enum numbers, **not verified SDK mirror-state
reads**. SDK properties can be implemented in host software. The command now
explicitly reports `sdk_accessor_equivalence_verified: false`. The earlier
color result means a consistent raw boolean response, not a verified SDK or
physical color-image orientation. Historical observations below are unchanged.

`scancraft-cli mirror-status` queries only SDK property IDs 18 (IR mirror),
112 (right IR mirror), and 81 (color mirror) with opcode 1 GET_PROPERTY.
It holds the session lock and existing XU-channel lock. There is no mirror
setter or arbitrary-property command. Unsupported properties and malformed
responses are retained as individual errors, so one failure does not hide the
remaining observations.

An enabled/disabled interpretation requires a consistent boolean range:
minimum 0, maximum 1, step 1, valid current and default. Otherwise
`reported_enabled` is null, with raw words and an error where available.
A reported flag does not establish the physical image axes, which sensor
maps to each calibration slot, or which convention the stored calibration uses.

## Native observation

`evidence/native-mirror-status.json` records the connected Raptor's response:

| Property | Result | Interpretation |
| --- | --- | --- |
| 18, IR | current 1, maximum/minimum/default/step all 0 | inconsistent range; unknown |
| 112, right IR | device response status 0x2 | failed query; unknown |
| 81, color | current 0, range 0..1, default 0, step 1 | reported disabled |

The IR current word alone is insufficient to confirm mirroring. It could reflect
different property response semantics or unsupported behavior; this remains
unresolved. No mirror/calibration/firmware/emitter state was written.

## Installed host reference

Read-only static inspection of the installed `lib_orbbec_scan.dll` found error
strings associated with IR/right-IR/color mirror operations. Cross-references
call a common helper at RVA 0x74d890 with second arguments 18, 112 and 81.
One conditional path supplies a zero third-argument low byte; another propagates
the same caller-supplied byte to all three operations. This supports the ID
association, not execution on the current Raptor or physical-axis semantics.
No vendor function was executed by this investigation.

DLL SHA256, cross-reference addresses and call-site RVAs are recorded in
`evidence/mirror-control-static-summary.json`. Raw DLL bytes and disassembly
remain private. For reproduction, use `objdump -h` to map .rdata string offsets
to virtual addresses, locate RIP-relative LEA references to the three error
strings, and inspect each site's preceding calls using
`objdump -d -M intel --start-address=ADDRESS --stop-address=ADDRESS DLL`.
Inspect both reference groups; do not assume the initialization branch runs
for every device or infer runtime behavior from an error string alone.

Two Rust tests verify fixed GET envelopes/property IDs and reject malformed
boolean ranges. This diagnostic narrows the next investigation to IR property
semantics and runtime camera conventions; it does not resolve or activate the
[reflection hypothesis](CAMERA_REFLECTION_HYPOTHESIS.md).

## SDK accessor semantics

The local reference checkout of OrbbecSDK_v2 at commit
`85e94c4ee4032817df2e33228dbd7f51517d3d4e` shows that
`VendorPropertyAccessor::getPropertyValue` and `getPropertyRange` use the same
GET_PROPERTY request/response structure. The former extracts the current word;
the latter extracts all five words. This establishes no alternate wire format
that would resolve our inconsistent IR range.

More significantly, Gemini2 registers mirror properties with a frame-transform
accessor, which dispatches them to host frame processors. The IR accessor
inverts the mirror boolean on both set and get; its source explains that raw
hardware frames are mirrored. Right IR also depends on a specific depth work
mode. Consequently direct numeric firmware queries cannot substitute for those
SDK accessor results. This is evidence about Gemini2 source, not proof of which
accessor the customized Raptor SDK uses. We neither relax the native range
decoder nor infer an SDK mirror state from the raw current word.

Source paths, hashes, revision and tracked-diff checks are in
`evidence/sdk-mirror-accessor-source-audit.json`. No SDK code was executed or
copied into the native implementation. The next reference investigation must
identify the Raptor's actual accessor/frame-processing path.

## Installed SDK host getter

The installed `OrbbecSDK-stock.dll` also contains a host-side mirror accessor.
`tools/check_sdk_mirror_dispatch.py PRIVATE_DLL` refuses any hash other than
the inspected reference, checks the getter's control-flow landmarks with
objdump, and decodes its compressed property switch. It outputs only aggregate
facts/addresses; no proprietary code or disassembly is distributed or executed.
`evidence/sdk-mirror-dispatch-audit.json` records the reference hash and results.

In the base `FrameProcessor` getter at RVA 0x344920, properties 14 and 18 map
through the compressed switch to RVA 0x344a3d; property 112 branches directly
to the same block. That block looks up the host `FrameMirror` processor and
conditionally inverts its returned boolean based on a flag at this-minus-0x5f.
The flag's value and initialization for this Raptor have not been established.
Property 81 goes to the unsupported case of this **base getter**; specialized
color accessors may handle it, so this is not a claim that the SDK cannot mirror
color. The inspected `FrameMirror::processFunc` region is RVA 0x32e4c0..0x32feec;
its presence does not prove that the Raptor's active pipeline executes it.

This strengthens the distinction between raw firmware property words and SDK
host state. It does not resolve camera assignment, output orientation or
calibration binding. A synthetic unrelated file is rejected by the hash guard
before disassembly (`evidence/sdk-mirror-dispatch-hash-guard.json`).

## Inversion flag initialization paths

The extended audit finds a leaf flag setter at RVA 0x348760, storing the
supplied boolean at object-base offset 0xa1. The getter's lookup uses a
this-minus-0x100 adjustment; its flag access at this-minus-0x5f is consistent
with that object-base offset. All six discovered **direct relative call** sites
supply true:

| Referenced IR creation class | Function RVA | Setter call RVA |
| --- | --- | --- |
| Astra2Device | 0x1b9e60 | 0x1ba2b7 |
| Gemini2Device | 0x1defb0 | 0x1df75c |
| Gemini2LDevice | 0x1f2830 | 0x1f2fd0 |
| AvatarDevice | 0x2591e0 | 0x259f1e |
| FerretUvcDevice | 0x265d90 | 0x266ace |
| PioneerDevice | 0x281fb0 | 0x282efe |

The script enumerates direct calls in the hashed text section, checks the
true argument, and verifies each enclosing function's class-qualified
`createIrSensor` name reference. It also checks that the Pioneer primary
vtable contains the inspected IR creation function, its RTTI identifies
PioneerDevice, and its constructor initializes that vtable. Aggregate results
are in `evidence/sdk-mirror-inversion-flag-audit.json`.

This is stronger than identifying a mirror error string: it locates how several
installed device implementations enable inversion. However, enumeration does
not exclude indirect calls, inlined or aliased writes, later overrides, failed
construction or alternate processing chains. Pioneer-named exported APIs do
not by themselves bind the connected PID 0x06a8 to this constructor. That
dispatch and actual frame processing remain unverified. No DLL was loaded or
executed, no device was accessed, and no calibration or mirror state changed.

## Raptor PID membership paths

Further read-only inspection links the Raptor PID to conditional Pioneer
construction in both installed SDK enumerators. The extended hash-guarded tool
checks the two list initializers, reads their four u16 IDs from the initialized
stack constants, checks the begin/end range and common vector-construction
call, follows the membership branch and checks its Pioneer constructor call.

| Transport | List initializer RVA | List global RVA | Membership test RVA | Match branch RVA | Pioneer constructor call RVA |
| --- | --- | --- | --- | --- | --- |
| USB | 0x59480 | 0x186ddb8 | 0x37c4c6 | 0x37c6f6 | 0x37c793 |
| Network | 0x5a620 | 0x186e098 | 0x382953 | 0x382ca3 | 0x382d33 |

Both lists contain `0x06a8`, `0x0c00`, `0x0c03`, and `0x0c11`. Membership is a
u16 comparison against device-info offset 0x28; the exported PID getter at
RVA 0x3954f0 reads that same field. The common constructor is RVA 0x27fc10,
already linked to the PioneerDevice RTTI/vtable and IR-creation path above.
The enclosing factory functions reference `UsbDeviceEnumerator::createDevice`
and `NetDeviceEnumerator::createDevice` respectively. The separately observed
WiFi-handle combo path is not treated as direct USB dispatch evidence.

Evidence: `evidence/sdk-scancraft-pid-dispatch-audit.json`. This closes the previous
missing **static conditional membership** link; it does not establish that the
current session executed either branch. The audit does not prove earlier PID
lists are disjoint, that runtime support gates permit construction, that all
later processing succeeds, or that the reference DLL is the executing instance.
The inherited `scancraft_pid_to_pioneer_constructor_binding_verified: false` field
continues to mean runtime binding is unverified; the new per-path conditional
membership flags describe the narrower static result. Physical image axes,
factory camera assignment, active mirror state and geometry remain unqualified.

## Pioneer mirror configuration entries

The next static link is captured in
`evidence/sdk-pioneer-mirror-configuration-audit.json`. Both inspected branches
of Pioneer IR creation build a `FrameMirror` configuration entry with flag 1
(writes at RVAs 0x282a41 and 0x282cdb). The common frame-processor constructor
reads an entry's flag at offset 0x20 and forwards it through virtual slot 0x10.
The inspected mirror vtable connects that slot to a setter storing the byte
at object offset 0xb9. Its process-function slot points to the previously
identified mirror implementation at RVA 0x32e4c0.

This establishes static configuration intent to enable a host mirror block.
It does not prove the final processing state. In particular, the mirror
getter at RVA 0x32cac0 conditionally inverts the enabled byte using an additional
internal flag at offset 0xd8; the higher-level processor getter has the separate
inversion flag described above. Neither the internal flag nor later overrides,
actual branch execution, data-format handling or delivered frame orientation
has been observed. No native default, calibration mapping or scanner setting
is changed on the strength of this evidence.

## Mirror internal flag construction

The inspected mirror creator at RVA 0x33f460 calls the base transform
constructor at RVA 0x32c980, then installs the mirror vtable at RVA 0x6b4368.
The base constructor explicitly initializes the internal flag at offset 0xd8
to zero. Thus the earlier internal-flag uncertainty is narrowed to its runtime
value and subsequent writes, rather than an unknown initial value for this
construction path. The audit checks both the creator and base-constructor
landmarks: `evidence/sdk-mirror-internal-initialization-audit.json`.
The flag scan is not a complete alias/write analysis; it does not prove the
flag remains zero or that the current Raptor session ran this creator. No
hardware state or active calibration is changed.

## Format-9 byte-copy reference experiment

Read-only inspection of the same hashed installed reference identifies the
exported `ob_frame_format` accessor at RVA0x39a100 using virtual slot0x10.
The width/height accessors at0x39a700/0x39a2e0 use slots0x90/0xa0. The mirror
process path at0x32fca7 dispatches numeric format9 to the call at0x32fd0b,
which invokes the byte-copy leaf at0x3297b0 with width/height as its last two
arguments. The public SDK header names9 Y8, but the installed enum ABI has
not been dynamically queried. Numeric dispatch and instruction inspection
are static evidence, not a live device-format observation.

`tools/probe_sdk_byte_mirror.py PRIVATE_STOCK_SDK` makes the numeric experiment
reproducible on Linux x86-64 with GCC and bubblewrap. It refuses any whole-DLL
hash or leaf hash except the inspected reference. It extracts only the93-byte
self-contained leaf into a private temporary directory; no DLL is loaded.
The independently written C harness uses the Microsoft x64 calling convention
and runs with unshared namespaces/network, private `/dev`, read-only libraries
and work directory, no scanner nodes and a ten-second process timeout. Code
memory transitions writable to executable, never both simultaneously. Test
input is read-only for the first call, and buffer mappings have guard pages.
No proprietary instructions or binaries are included in the repository.

For six synthetic shapes (1x1,1x7,2x3,7x3,8x5,1920x1200), the actual leaf
matched `out[y*w+x] = in[(y+1)*w-1-x]` at every one of2,304,075 pixels. A
second application restored the original synthetic values exactly. This is
**executed vendor-leaf evidence**, distinct from previous static-only audits;
it does not execute the SDK wrapper, device constructor or frame pipeline.
Aggregate hashes/results: `docs/reviews/evidence/2026-10-27-sdk-byte-mirror-reference.json`.
The helper uses no device data and no factory calibration. Private extracted
bytes and compiled helper are removed on exit. Rejection tests require no DLL:
`python3 -m unittest discover -s tools -p test_sdk_byte_mirror_probe.py`.

This establishes the byte leaf's horizontal reflection behavior for the tested
sizes and values. It does not establish active enable state, wrapper/format
conversion behavior, camera assignment, calibration convention, synchronized
exposures or physical accuracy. The existing native Rust reflection remains
explicitly selected; no production default or calibration mapping changes.
The matching index formula is not a direct cross-runtime comparison of every
native Rust pixel. Further work must observe the actual runtime chain or obtain
usable independent correspondences before selecting an optical mapping.
