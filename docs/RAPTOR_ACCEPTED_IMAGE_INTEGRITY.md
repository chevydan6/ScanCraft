# Accepted-image duplication and current transport evidence

## Finding and scope

**Confirmed:** genuine Raptor recordings contain accepted, full-length images
with repeated pixel blocks and visible horizontal discontinuities. Successful
V4L2 buffer validation and recording CRC replay therefore do not establish that
the image content is intact. This is a developer observation, not an independently
reviewed finding or a new production rejection rule.

All captures use application revision
`d91fc4f696ee275e77145406ea55126475b515c6`, CLI SHA-256
`8ec0f44f18a090a45c90539ec78af2e8220fb8fdfb351e1236259f5ef3d57706`,
and the installed baseline uvcvideo srcversion `9D1275CDD5E013864E4C564`.
No application, driver, firmware, affinity or permanent setup change was made.
The temporary bounded-copy candidate from the
[sustained combined trial](RAPTOR_SUSTAINED_COMBINED.md) was **not** loaded.
That earlier finite result remains evidence for its own configuration; it does
not qualify the current baseline or prove accepted pixel integrity retrospectively.

The scene contains the owner's measured box on the marked turntable. A flood-mode
image confirms its placement. Rotation was offered, but not confirmed started;
these recordings are not represented as a rotating-object experiment. The stated
79.70 × 59.78 × 39.80 mm dimensions remain held out from fitting and thresholds.
No new point cloud or dimensional-accuracy result is claimed here.

## Unmonitored observations

Each arm saves the stated number of accepted images **per camera**, followed by
complete native recording replay. Line-mode captures request raw gain 15000.

| Arm | FPS | Accepted per camera | Startup rejected L/R | Ongoing rejected L/R |
| --- | ---: | ---: | ---: | ---: |
| Initial line observation | 30 | 10 | 1 / 0 | 5 / 5 |
| Cadence 30-a | 30 | 120 | 98 / 1 | 0 / 0 |
| Cadence 15-a | 15 | 120 | 1 / 0 | 1 / 1 |
| Cadence 15-b | 15 | 120 | 0 / 0 | 1 / 1 |
| Cadence 30-b | 30 | 120 | 0 / 0 | 1 / 1 |
| Flood placement observation | 30 | 20 | 0 / 1 | 0 / 0 |

The cadence order was declared 30/15/15/30. These short shared-host samples do not
establish a general rate advantage; both 15 FPS arms still reject ongoing buffers.
Reducing FPS is not a demonstrated remedy. Counts are V4L2 buffers, not proven
physical exposures. The initial failure burst and startup counts are retained,
not pooled away. No validation was relaxed.

### Repeated blocks in accepted images

The private analyzer independently checks the RAPTOR02 header/frame CRCs, declared
lengths and final frame count, then compares all 187 complete 12,276-byte blocks
starting at image offset zero. This size is the observed 12,288-byte maximum USB
payload minus its 12-byte UVC header; the final shorter block is outside this test.
SHA-256 groups candidates, and full byte equality verifies every duplicate group.

| Recording | Accepted images with repeated full blocks | Duplicate groups |
| --- | ---: | ---: |
| Initial left, 10 images | 4 | 9 |
| Initial right, 10 images | 6 | 9 |
| Cadence 30-b left, 120 images | 1 | 1 |
| Cadence 30-b right, 120 images | 1 | 1 |
| Other three cadence arms, 720 images combined | 0 | 0 |

Thus 12 of these 980 accepted images have the tested repeated blocks. This is
**not** an image-corruption prevalence estimate: partial or differently aligned
duplication is outside this particular comparison. For example, the initial
accepted frame at left index 3 has a large adjacent-row discontinuity but no
repeated full aligned block. Absence of this signature is not an integrity pass.

In the two initial index-5 images, an additional exploratory search finds exact
128-byte sequences repeated elsewhere in the same image, consistently displaced
by ±24,552 bytes. No such search hit was in another accepted image of that short
recording. Native-exported initial left/right index 5 images visibly contain
discontinuous strips. Cadence 30-b left index 92 has identical full blocks 141/143;
right index 74 has blocks 6/8. The native left export visibly repeats a strip
across marker features. These are zero-based image/block indices.

**Engineering inference:** repeated noisy scene content separated by two image
payloads supports duplication or misplacement of data, rather than ordinary dark
pixels. The unmonitored recordings cannot locate the fault in device production,
USB/HCD delivery, kernel assembly or application handling. No causal attribution
to the driver, exposure or projector is established.

## Full USB monitor comparison

Two later monitor sessions cover one 120-frame-per-camera capture and then three
consecutive captures of that size, all at 30 FPS, line mode, gain 15000. A privileged
tcpdump opens only the Raptor's current bus monitor, filters its device address,
then drops to the owner account. Snapshot length is 245,824 bytes and ring size
64 MiB. Explicit 45/60-second deadlines bound monitoring; both sessions stop by
SIGINT after acquisition and restoration, exit 0. No permissions are changed.

| Monitored capture | Startup rejected L/R | Ongoing rejected L/R |
| --- | ---: | ---: |
| First session | 0 / 93 | 1 / 1 |
| Restart series 1 | 0 / 102 | 1 / 1 |
| Restart series 2 | 97 / 1 | 2 / 2 |
| Restart series 3 | 96 / 1 | 0 / 0 |

The monitors retain 92,502 and 278,540 events, respectively, with zero reported
kernel drops. Submission/completion identities balance, with no outstanding or
unmatched identities at the end and no truncated successful bulk payloads.
Successful bulk completion counts are 45,746 and 138,585; 10 and 30 canceled
completions retain status -2. UVC COMMIT responses report 2,304,000-byte images
and 12,288-byte maximum transfers in all eight streams.

For **all 960 accepted images**, independently removing the observed UVC headers
and concatenating same-PTS payloads produces bytes identical to the raw recording.
Each accepted group has one FID and one terminal EOF. The restart analyzer confines
each group to its recorded capture time window, avoiding cross-restart mixing.
No repeated full aligned blocks were found in those 960 images. The two first
session images ranked highest by exploratory adjacent-row difference were viewed
and do not show the earlier pronounced discontinuities. This sampling/ranking
does not visually qualify every image.

The duplication was **not reproduced under these monitors**. Their timing cost,
host activity and non-randomized order prevent attributing its absence to any one
factor. Byte agreement for these later images cannot locate the earlier fault.
usbmon observes software/HCD events, not the physical wire or sensor exposure.

### Truncation is already present in the monitored payload stream

Every ongoing rejected image in these sessions has a same-PTS counterpart in the
other camera. Its native short length equals the sum of monitored image payloads:

| Incident | Bytes per camera | Payload blocks | Largest completion gap L/R |
| --- | ---: | ---: | ---: |
| First session | 1,506,060 | 123 | 10,640 / 10,624 µs |
| Restart 1 | 1,506,060 | 123 | 12,466 / 12,447 µs |
| Restart 2, first | 2,181,240 | 178 | 2,899 / 2,880 µs |
| Restart 2, second | 2,181,240 | 178 | 2,975 / 2,957 µs |

Each group has one EOF. Full images require 2,304,000 bytes. These observations
place the missing data upstream of the recording serializer in these incidents,
but do not distinguish device loss from host/controller effects. Startup also
retains nonstandard payload boundaries; this report does not repeat the separate
[shifted-header reconstruction](RAPTOR_USB_STARTUP_FRAMING.md) or assume that every
startup rejection has been individually explained.

## Restoration, reproduction and limitations

All five collection roots retain predeclared plans, bound executables/helpers,
commands, raw stdout/stderr, before/after state and native replay receipts. All ten
capture commands finish successfully, with errors reported inside their summaries.
All seven compared scanner readbacks, saved camera formats and the original active
WirePlumber state restore. The baseline module remains loaded. No monitor or native
capture remains running at this milestone.

Private roots under the developer artifacts directory:

- `box-current-view-20261104`: original 10-image streams and native exports.
- `current-image-cadence-20261104`: four rate arms and extraction receipts.
- `current-flood-view-20261104`: placement observation.
- `accepted-image-usb-20261104`: first full trace, `compare.py`,
  `analyze_bands.py`, `duplicate_payloads.py`, comparisons and native duplicate exports.
- `accepted-image-restarts-20261104`: three-restart full trace and comparison.
- External `accepted-image-publication-20261104`: evidence verification, manifests,
  publication checks and exact commit binding.

Run the retained comparison scripts on their original private roots to reproduce
the payload results. `verify_evidence.py` in the external publication root verifies
all plan input hashes, native replay/restoration receipts, and all 30 retained
native image exports against their raw pixels and metadata CRCs. Private images,
USB traces, factory data and executables are not included in this documentation PR.
Adjacent-row differences and duplicate searches are exploratory diagnostics, not
validated optical quality gates or silent production filters.

Relevant source boundaries are `Capture::next_frame_until` and the mapped-buffer
copy in `crates/scancraft-device/src/lib.rs` (copy before requeue),
`Reader::read_frame_inner` in `crates/scancraft-recording/src/lib.rs` (stored integrity),
and `uvc_video_decode_bulk` / `uvc_video_decode_data` / `uvc_video_complete` in the
privately bound local kernel source. The application source is unchanged, so the
existing independently verified 935-test/fmt/Clippy/release gate applies to its
revision; it is not a new test run or hardware-integrity qualification.

**Next priority:** audit calibration/reconstruction source images for this signature
and capture a duplication incident with enough transport evidence to locate it.
Keep observed anomalies visible in diagnostics and out of claimed metrology; do
not repair, suppress or relabel them as passing measurements. A future integrity
criterion must account for naturally repeated/constant imagery and prove its false
positive/negative limits. Reliable acquisition, physical scale, complete box
coverage, repeated dimensional validation and native end-to-end scanning remain
unfinished. No new generic editor feature or architecture rewrite is justified.
