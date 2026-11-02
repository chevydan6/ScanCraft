# Rotating board and dark-target diagnostic evidence

These private hardware trials used the native CLI at the block-opening checkpoint.
The owner operated the turntable and moved/replaced the target. No physical
calibration, firmware or persistent control changes were performed. The normal
line-capture command enabled its existing temporary illumination trial and restored
the previous state. Before/after illumination queries were identical; line-control
readback and stream-setting restoration both reported PASS.

## Acquisition and coverage

The rotating-board run saved 900 accepted frames per camera. Each camera's accepted
host-time span was about 30.104 seconds, approximately 29.863 accepted buffers/s.
All 900 frames had equal-raw-PTS partners within a 1000-microsecond host window.
Recording indexes validate both complete files, including checksums and footers.
Equal PTS/near host timestamps still do not prove simultaneous physical exposure.

Each camera rejected four ongoing V4L2 buffers: 4/(900+4), about **0.442%**.
Startup rejections were separately 80 and 108. Buffer counts are not a complete
physical exposure-loss measurement. This still exceeds the mission's provisional
0.1% corrupted-frame acceptance target; reliability is not qualified.

The timing command deliberately limits its example output to the first 100
candidates. That sample list was not used as full-sequence coverage. A private
read-only helper accessed all indexed timings and selected unique equal-PTS pairs
at indices 0,112,224,337,449,561,674,786,899. Actual equal indices were an observed
result, not an assumed pairing rule. Original timing/CRC references accompany
normalized frame copies and preview images. The first/middle/last views show the
board front, an edge/back view and the front again, consistent with a full turn;
no angular encoder or exact angular coverage measurement was available.

## Matching across changing views

The same prior geometry, sheet hypothesis, detection/support/triangulation limits
were used without fitting a new reference plane or tuning by surface residual.

| Original frame index | Supported first observations | Candidate points | Outcome |
| --- | ---: | ---: | --- |
| 0 | 809 | 0 | No unique supported matches |
| 112 | 970 | 779 | Diagnostics published |
| 224 | — | — | Conservative total-work bound rejected input |
| 337 | 1650 | 953 | Diagnostics published |
| 449 | 2087 | 1275 | Diagnostics published |
| 561 | 2167 | 1449 | Diagnostics published |
| 674 | 1274 | 648 | Diagnostics published |
| 786 | 2953 | 1276 | Diagnostics published |
| 899 | 787 | 0 | No unique supported matches |

All eight published diagnostic bundles passed artifact length/CRC checks. Their
candidate counts are neither tracking success rates nor measurements of correct
board reconstruction: background surfaces and the back face also contribute.
No old stationary reference plane was reused across moving poses.

Front-facing samples 0/899 had 4940/5389 saturation-rejected runs in the second
camera and only one continuity-supported second trace. Limited usable signal is
an observed input problem; saturation is a plausible contributor to the empty
matches, not independently proven as their sole cause. A visible horizontal
discontinuity in one oblique preview remains uninvestigated; valid payload size
and recording checksums alone do not establish exposure-content correctness.

The conservative workload failure is a bounded-processing limit, not corruption.
Blindly raising/removing it would not solve long-session resource control. Future
work should coordinate actual work accounting, bounded streamed diagnostics and
recovery while preserving existing schema behavior and truthful partial results.

## Stationary cardboard-box signal

The owner replaced the board with a box and left it stationary. Color frames
confirm a solid cardboard target, while laser-camera intensity is much lower.
The raw right image's 99th/99.9th percentiles were 26/57 versus 155/255 for the
preceding board view. Only 3704 box-image pixels were at least 50, versus 78365
in that board image. These are whole-image statistics, not matched surface patches.
Fresh captures differ in pixel noise, so the weak view was not a byte-identical
stale frame. Color and stereo captures occurred separately.

Only software contrast thresholds changed on saved data; opening radius12,
width12, saturation255, continuity and geometric limits remained fixed. Parameters
were chosen after observing the signal, so this is exploratory tuning.

| Background / minimum peak | Laser-on queries | Candidates | Laser-off candidates |
| --- | ---: | ---: | ---: |
| 30 / 50 | 183 | 0 | 0 |
| 10 / 20 | 2604 | 1764 | 0 |
| 5 / 12 | — | Query-count bound rejected input | 0 |

At 10/20 there were no per-query errors; all artifact lengths/CRCs passed. The
points span depths about 332–393 in unverified model units. No independent surface
identity or dimensional accuracy was established; these cannot yet be called a
correct box model. The results motivate signal-quality feedback and explicit
parameter control, while keeping clipping and workload failures visible.

All captures, color images, source/model values, preview frames, rejected results,
helper source/build hashes and numerical reports remain private. Public artifacts
contain aggregate findings only. The original camera/sheet/unit/timing hypotheses
remain unqualified. The application still needs validated candidate visualization,
live integration, tracking/recovery and calibrated end-to-end reconstruction.
