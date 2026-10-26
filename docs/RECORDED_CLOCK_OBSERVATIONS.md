# Historical stereo clock observations

Confirmed replay evidence, not synchronization or physical accuracy qualification.
The preserved private `native-stereo-recording-01` contains 100 frames per camera.
No new acquisition, scanner command, firmware change or physical calibration was
performed. Both files' SHA256 values were identical before/after each replay.
Image payloads, absolute timestamps and device data remain private.

The native command fully validates both files and produces this aggregate evidence:

```
scancraft-cli recording-timing LEFT_RECORDING RIGHT_RECORDING 1000
```

| Diagnostic | Observed |
| --- | ---: |
| Unique nearest-host candidates within 1000 us | 6 |
| Host-nearest candidates outside limit | 94 |
| Unique equal raw PTS pairs across whole recordings | 72 |
| Equal-PTS pairs also within 1000 us host difference | 3 |
| Minimum absolute host difference among equal-PTS pairs | 8 us |
| Maximum absolute host difference among equal-PTS pairs | 20026 us |

The 1000 us limit is caller-selected for investigation, not an approved exposure
tolerance. `raw_pts_comparison` does not unwrap clocks or infer common frequency.
Unique raw-value matching explicitly withholds values repeated in either recording.
Source: workflow `recording_timing/raw_pts.rs::compare`; aggregate native result:
`docs/reviews/evidence/2026-10-26-genuine-clock-comparison.json`.

An independent exploratory Python parser checked both recording header/frame CRCs,
lengths, UVC PTS fields, frame counts and EOF, then compared equal raw PTS directly.
It agrees with native Rust on the above intersection count, within-limit count and
host-difference extrema. It additionally observes a median absolute equal-PTS host
difference of 8508.5 us, median left host interval 26136 us versus right 20002 us,
and median wrapped raw PTS interval 20005 on both sides. These PTS intervals are
raw ticks, not independently verified microseconds. Exploratory parser and raw
outputs remain local in /tmp/scancraft-investigate-recorded-clocks.py and
/tmp/scancraft-genuine-clock-investigation.json; native reproduction above is the
supported implementation and does not require that Python parser.

All 200 recorded buffer flag values are 0x12001. The local Linux UAPI header's
masks identify declared MONOTONIC timestamp type (0x2000) and SOE source (0x10000).
The [Linux buffer documentation](https://docs.kernel.org/6.12/userspace-api/media/v4l/buffer.html)
defines these as monotonic-clock timestamps and start-of-exposure source. These
are driver-reported classifications, not independent verification of the scanner's
actual optical exposure timing or timestamp reconstruction. Device acquisition
copies the returned buffer timestamp/flags without correcting either; see
`scancraft-device/src/lib.rs` Packet construction after DQBUF.

Engineering inference: nearest-host association can select a different candidate
than equal-raw-PTS lookup in this dataset. Host differences alone should not be
used to reject or certify physical pairing. Neither metric identifies the physical
cause. Possible clock reconstruction, transport scheduling, metadata/video
association, frame loss or sensor behavior require controlled evidence; none is
established by this observation. Do not estimate/apply a global offset to make
these pairs appear synchronized, or feed them automatically into qualified geometry.

Next investigation requires verified mode/calibration binding, optically useful
matched content and independent exposure evidence on controlled acquisitions when
hardware is available. Calibration remains deferred. A valid recording and matching
integer PTS are insufficient to pass genuine reconstruction or accuracy gates.
