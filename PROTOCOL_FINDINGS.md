# Protocol findings

Verified Raptor identity: USB 2bc5:06a8, firmware observed 1.5.7. Three UVC
control interfaces 0/2/4 and an IMU HID interface 6. Video node minor numbers
change after reboot; identify by USB parent, interface and QUERYCAP, never
adjacent numbers. Image and metadata sequences pair exactly on this kernel.

This firmware repeats SCR values across frames, while PTS advances. The
installed development kernel preserves the first PTS-bearing metadata header
per frame. Linux metadata records comprise u64 host timestamp, u16 SOF,
followed by the variable-length UVC header. Per-frame PTS is little-endian
u32 at UVC header offset 2 when header flags bit 2 is set. PTS tick frequency
and inter-camera time relation require further measurement.

Color buffers start with JPEG SOI and appear zero-padded after JPEG EOI.
Initial strict-end validation was wrong; regression test and hardware retest
passed, including full JPEG decoding of 30 saved color frames. IR image bytes are not depth samples and must never be
exported as a purported genuine point cloud.

Prior local research (outside production source) established read-only XU
communication and incomplete calibration data. Rust XU controls and full
calibration decoding are not implemented. Do not infer flash layout validity
for other firmware or write calibration without a validated procedure.

## Incomplete-frame investigation

Three recorded rejected left-IR frames were short by 429660, 61380 and 245520
bytes: 35, 5 and 20 payloads of 12276 data bytes (12288 USB payload minus a
12-byte UVC header). Multiples of five suggest the five-URB kernel queue as
a hypothesis, not an established cause. A low-load release-build V4L2 trial
still rejected 13 / 3013 received frames at 50 fps. A 30-fps trial during
compilation rejected 80 / 3080; conditions differ and cannot isolate rate.

The nusb experiment did not acquire frames. A UVC control request stalled,
kernel rebinding failed, and USB enumeration subsequently failed with -71.
Exact control-stage failure and causality remain unestablished. Hardware mode
is disabled by default pending recovery and an independently reviewed changed
hypothesis. Existing successful V4L2 acquisition remains the default backend.

## Simultaneous native camera observations

`scancraft-cli stereo-probe 100` runs both IR UVC streams concurrently, restores
formats/rates, and records only metadata/counters (no image payloads). A run
with host-overlap association observed 72 exact sensor-PTS matches between
100 accepted left and 100 accepted right buffers. Both observed tick/host
microsecond ratios were near one in short runs. This is not an independent
clock-frequency or exposure-timing validation. Host-nearest association can
choose the adjacent frame; it is not a synchronization algorithm.

Detailed rejection samples show buffers with no metadata and image lengths
such as 12271 bytes, alongside full-size startup buffers containing multiple
metadata headers. Many returned-buffer sequence increments therefore do not
map one-to-one to physical exposures. Legacy rejection percentages are
buffer-level proxies; physical corruption and lost-exposure rates require
further isolation. No error flags are ignored to improve reported metrics.

## Recorded dual-stream optical uncertainty

Concurrent native V4L2 raw recording succeeds with complete CRC/footer replay.
The 100-frame-per-camera trial has 72 identical sensor-PTS intersections; first
left/right saved timestamps differ by 8 host microseconds. This is evidence of
shared timestamp values, not proof of synchronized exposure. Image inspection
at indices 0 and 50 shows low intensities and pronounced left horizontal
banding. Signal ranges differ between cameras. Cause is not established: do
not infer successful projector/laser setup or correct optical decoding from
frame byte length, error flags and timestamp acceptance. Controlled reference
illumination/exposure/format tests are required after the current transport soak.

Cadence comparisons now distinguish sequence gaps from sensor PTS skips. In
the initial dual soak, 10646 long left PTS intervals have sequence step one;
right has none. Isolated 50 FPS and dual 30 FPS run close to requested rates.
A later 50 FPS repeat recovers higher rate after many startup rejects. USB
IR endpoints 0x81/0x82 advertise equal 1024-byte packets and max burst 15.
These observations support a transport/startup investigation but identify no
fixed cause. Public Orbbec UVC mappings do not prove Raptor exposure units.

## Native bounded factory reads

`factory-header REGION` and `factory-read REGION NEW_DIRECTORY` independently
read existing flash through the observed UVC extension-unit protocol. Only
regions 200000, 202000 and 20a000 are accepted; each full read is fixed at
8 KiB, in 32-byte chunks with opcode 25. SET_CUR transports the read-command
envelope; no flash-write, calibration or firmware-update command is exposed.
Header signature/length and each reply envelope/length are checked. Output
publication refuses an existing target.

A native 200000 read succeeded and its header matches the previous vendor-
reference read. Raw data remains private. The observed body signature at
offset 48 is 0x4644610e; words at offsets 52/56 are 4/1680. Four candidate
records starting at 60 end at 6780, exactly the top length word; remaining
bytes are erased FF. Labels are LAS_L_SCAN and CROSS_LAS_L_SCAN, each twice.
This is structural evidence only: the checksum, optical field layout, units,
distortion model and camera assignments remain unknown. No RAPCAL record is
created from these hypotheses. See evidence/factory-read-summary.json.

## Verified factory CRC and original laser record index

The stored word at offset 4 is standard CRC-32/ISO-HDLC over bytes from zero
to the declared length at offset 8, with bytes 4..8 replaced by zero. Native
CRC verification matches both actual 200000 and 202000 reads. Static inspection
of the vendor read path independently shows zeroing that word, CRC calculation
and comparison before records are copied. Source hash and reference locations
are in evidence/factory-checksum-research.json; proprietary disassembly remains
private and is not part of production code.

The Rust parser accepts only top signature 0x46447311, body signature
0x4644610e, 1680-byte records, nonzero bounded count, and exact
60 + count * stride = declared length. Other schemas are unsupported. It
checks CRC before exposing labels/offsets. Input is limited to 8192 bytes;
CLI requires a regular file. Padding beyond declared length is explicitly
unauthenticated. A valid container checksum establishes byte integrity, not
optical correctness or physical accuracy. Factory data is never converted to
RAPCAL without independently verified optical semantics.

## Distinct named-camera payload at region 20a000

The native read returned a valid 6646-byte declared envelope with top signature
0x46447312 and body signature 0x4644610f. CRC-32 with offset 4 zeroed verifies.
This payload differs from the 1680-byte laser records: it contains IR_L, IR_R,
COLOR and Brown tokens. The header words at 52/56 are 1/6586, consistent with
one body ending at 60+6586=6646, but the body field schema is not verified.
The optical fields, units and camera-to-camera transform direction remain
unknown. No RAPCAL conversion or activation is performed.

`factory-verify FILE` authenticates the two observed signature pairs and
declared byte range, without interpreting the payload or flash padding.
`factory-inspect FILE` continues to reject the second payload schema. Valid
CRC alone must never authorize interpreting an unknown schema as laser data.
Static review of calib_params_parser_ob_pro.dll shows a file-loading API with
multiple format branches; its applicability to this payload is not established.

## Named lens block extraction with a file-parser reference

For only the observed 3.0.13 payload, named cameras start at payload offset
50 with stride 2178. Dimensions are little-endian u32 at 64 (height) and
68 (width), Brown label at 72, and 12 consecutive little-endian f32 values
at 104 (four intrinsic, eight distortion values). Offsets are relative to
byte 60 of the complete container plus the per-camera stride. The camera
count byte is payload offset 48. No generic pointers or object memory are
read by the independent decoder.

The installed vendor file parser, invoked offline in isolation, returns
exactly the same 36 float bit patterns, dimensions and labels. This confirms
field extraction and camera labeling, not optical coefficient conventions
or pose semantics. The oracle is optional research tooling, never a native
production dependency. See docs/FACTORY_CALIBRATION.md for scope and evidence.

## Raw pose extraction and named file export

For the supported named-camera schema, six pose floats are at payload offset
2203 + camera_index*2178. The reference object's corresponding values are
at 0x864 + camera_index*0x840. Native extraction matches all 18 float bit
patterns exactly. A private file-only YAML export labels the first three
Rotate.RX/RY/RZ and the last three Translate.TX/TY/TZ, attached to IR_L,
IR_R and COLOR. These names do not specify Euler order versus another angle
representation, degrees/radians, transform direction or translation units.
No SO(3) matrix or calibrated stereo model is inferred from plausibility.

The same YAML comparison confirms lens array Fx/Fy/Cx/Cy and Brown slots
0/1/2/3/6/7 as K1/K2/K3/K4/P1/P2. Slots 4/5 are not named by this export.
The mathematical distortion formula and its direction still need independent
verification. The temporary reference helper optionally calls the DLL's local
file exporter in a device-inaccessible sandbox; it never calls a scanner
write API. Private outputs remain excluded from redistribution.

## Isolated numerical pose-prefix evidence

The hashed mx6600_depth_engine.dll routine at RVA 0x127f0 reads named-pose
fields at object offsets 0x864..0x878. Static review identifies a half-angle
quaternion calculation followed by a matrix conversion. A private numerical
probe supplies synthetic input and intercepts execution at RVA 0x12b7b before
the second camera or the remaining engine executes. It captures the rotation
and homogeneous matrix; all six cases exit through that interception.

After converting the observed column-major storage to row-major, the formula
is Rz(RZ)*Rx(RX)*Ry(RY), radians, with raw translation unchanged. Native Rust
agrees within 1e-12 on identity, separate axes, asymmetric mixed angles and
nonzero translation. All five other Euler multiplication orders fail the
mixed-angle comparison. Evidence and exact reference hash are in
evidence/factory-pose-math-summary.json. The helper source contains only
independent instrumentation, not vendor instructions.

This verifies one routine's numerical convention. The relative camera
composition, camera/world direction, translation units, actual active mode
and physical accuracy remain unverified. Merely finding a JSON reader or
an imported Rodrigues function does not establish its runtime reachability
or its use for these factory fields. No such inference is made.

## Ordered relative pose numerical-prefix evidence

The depth-engine routine builds matrices for the first two named-pose object
slots, then forms T(second)*inverse(T(first)). The extended offline synthetic
probe intercepts at RVA 0x12edf, capturing stack offsets 0x310 (first), 0x390
(second), 0x610 (inverse first) and 0x1c0 (relative), all column-major f64.
Six cases include identity/translated baseline, each independently rotated
camera, both rotations, and two noncommuting rotation/translation pairs.
The reference inverse matches an independent numerical inverse; the observed
product agrees within 1e-11 and reversed multiplication fails mixed cases.

Native Rust uses the rigid inverse R-transpose and computes relative rotation
R2*R1-transpose and translation t2-relative_rotation*t1, keeping raw units.
It matches the captured cases and rejects nonfinite fields. The physical
meaning of source poses and mapping to active scanner images must still be
verified; a numerical composition alone does not establish calibrated camera
extrinsics or millimeters. See evidence/factory-relative-pose-summary.json.

## Brown coefficient order and direction

The installed create_distortion_lut.dll exported Brown generator accepts eight
coefficients in order k1,k2,k3,k4,k5,k6,p1,p2. Synthetic independent numerator,
denominator and tangential tests confirm the forward map with radial factor
(1+k1*r2+k2*r4+k3*r6)/(1+k4*r2+k5*r4+k6*r6), plus standard p1/p2 tangential
terms. The two output arrays are separate f64 normalized displacements
(output minus input), not packed float pairs. Comparisons cover in-image
points only; expanded LUT border behavior is not implemented or qualified.

4608 samples agree within 1e-7 normalized component error; Rust regression
uses 210 public synthetic samples with tolerance 2e-7. The actual named
factory coefficients have k4/k5/k6 zero for all three cameras, so Brown5
coefficient reduction is lossless. This does not establish an inverse-safe
optical domain, active-mode binding or physical accuracy. Nonzero denominator
coefficients are explicitly refused by the Brown5 converter.

An equal-PTS genuine stereo pair yielded no geometry-independent mutual
SIFT matches in the offline reference check. No epipolar score or inferred
physical camera direction is reported from an empty correspondence set.
Exposure/illumination setup must be resolved before that evidence is available.

## Native standard IR gain writes and restoration

Both IR gain controls report raw range 1000..15000 in steps of 100, with
original value 1000. Both raw exposure values already equal their reported
maximum 3000; actual duration units remain unverified. The right exposure
control is inactive under reported auto mode, so it is not changed. Native
V4L2 S_CTRL for gain with G_CTRL readback succeeds on both IR interfaces.
Each temporary trial restores both original values with readback before
publication. Session locking excludes cooperating native/Wine sessions.

30 FPS captures at four gain settings show intensity increases, but the
scene remains unsuitable for common robust feature matches. At 8000/15000
there are only 2/3 mutual SIFT matches; neither raw-pose hypothesis has an
approximate pixel Sampson error below two for those matches. This tiny set
does not validate or falsify the factory camera model. High left-image
saturation at 15000 argues against simply maximizing gain. Saved images
remain private. No vendor, laser/projector, exposure, calibration or firmware
write command was used. See evidence/ir-gain-trial-summary.json.

## Illumination-related property reads and fixed flood boolean

Stock HostProtocol builds opcode-1 GET_PROPERTY with two payload words
containing the property ID, and expects exactly 20 returned data bytes in
current/maximum/minimum/default/step order. SDK metadata identifies laser
3, flood 6, line enable 151, line mode switch 192 and left/right line light
types 197/198. Native read-only snapshot details and mapping references are
in evidence/illumination-property-mapping.json and illumination-status-native.json.

The current device reports valid boolean ranges for generic laser, flood,
line enable and several additional laser switches; it returns status 0x2
for several other properties. IDs 7/99/119 return status zero but contradictory
integer ranges (including step zero). Those are unusable for setting, not
valid power/brightness readings. The implementation flags this explicitly.

Only fixed flood property 6 is writable through the new scoped trial. The
opcode-2 packet contains four payload words, fixed ID 6 and a boolean value;
response payload must be empty and a fresh GET must confirm the setting.
The original consistent boolean is restored and read back before successful
trial publication. A genuine 60-frame-per-camera trial observed 0->1->0 and
restored every sampled property/control value. Image brightness increases,
but zero mutual image features remain under the reference check. This does
not prove physical emitter state or establish usable scanning geometry.

## Fixed line-laser boolean operational evidence

The stock-mapped `OB_PROP_LASER_LINE_BOOL` ID 151 returns a consistent 0..1,
step-1 range on this Raptor. A narrowly scoped opcode-2 boolean setter uses the
existing XU envelope and u32 property/value payload, with opcode-1 GET readbacks.
No generic setter, numeric mode/current or flash operation is exposed. Trial IDs
4000/4001/4002 cover original/enable/readback; 4010 verifies after capture;
4020/4021 restore/read back original. The native trial observed 0→1→0 and genuine
multiline patterns in both captured IR views, absent after restoration. Mode 192
and all other sampled controls match baseline after the trial. Wavelength,
manufacturer scan-mode name and calibration binding are not established by
monochrome images. See `docs/NATIVE_LINE_TRIALS.md` and aggregate evidence.

## Sustained dual-IR truncation evidence

The 2026-10-26 native 30 FPS soak accepts 18,100 buffers per camera over 606.44 s,
with 92 explicit rejects and 92 accepted-sequence gaps on each. All retained first
32 rejects per camera have video/metadata error flags and 22 metadata bytes;
video payloads are short. Thirty of 32 deficits per camera are multiples of
61,380 bytes (five 12,276-byte data chunks). The first rejection is near sequence
34, and samples span ~235 s, so this is not solely startup stabilization.
Thirty-one unique raw rejected PTS values are shared across cameras with host
differences of 3–80 us. A common truncation mechanism is a hypothesis; there is
no verified controller/kernel/firmware cause or exposure correspondence.

Post-soak warm startup also produces a left-stream burst of small error buffers:
111 left rejects versus one right reject before/around ten accepted buffers in
0.922 s. This reinforces that returned-buffer counts cannot be relabeled as
physical exposure losses. Detailed capped-sample and temporal audits are in
evidence/stereo-ir-soak-20261026/. Existing kernel/module settings are preserved;
no relaxed frame acceptance, firmware or calibration writes were used.

## Metadata error flags versus retained PTS structure

Read-only review of the unchanged local Linux UVC baseline source shows that an
uncompressed size mismatch sets video error, and next_buffers can copy video
error onto the metadata buffer. Its built module matches the installed module
file; source fingerprint matches the prior audit. Therefore the first failure
label "metadata buffer error flag" is not proof of malformed metadata or a device
UVC ERR bit. New independent native rejection fields distinguish these cases.
A subsequent run observes five short/error-flagged video buffers per camera with
valid retained PTS structure and first UVC flags 0x8c/0x8d (no 0x40 ERR bit).
No claim is made about unretained USB headers or the exact truncation cause.
Evidence: rejection-kernel-source-review.json and
stereo-rejection-indicators-20261026/. No kernel/firmware/calibration writes.
