# Exact block-based laser background removal

`scancraft-image/src/laser.rs::row_extreme` now computes row minima/maxima using
block prefix/suffix extrema. The public detector, thresholds, clipping checks,
centroid/variance arithmetic and output ordering are unchanged. The measured
motivation was about 71 ms for two-image stripe detection after trace matching
had been optimized.

## Method and provenance

For radius `r`, pad the row by `r` neutral values at each end: 255 for minimum,
0 for maximum. Split this padded row into blocks of width `2r+1`. Calculate a
prefix extremum and suffix extremum within each block. A full sliding window
spans at most two blocks; combine its starting suffix and ending prefix. Neutral
padding preserves the original truncated-window behavior at both image edges.
Apply minimum then maximum for opening, followed by the unchanged saturating
subtraction from raw intensity. Raw samples still determine saturation rejection.

This is an independently written Rust implementation of block extrema in the
van Herk/Gil-Werman algorithm family. The method reference is
[van Herk's 1992 publication](https://doi.org/10.1016/0167-8655(92)90069-C);
its publisher abstract describes forward/backward block recurrences. The naming
and method overview also appear in
[Torres-Huitzil (2013), section 2](https://pmc.ncbi.nlm.nih.gov/articles/PMC3833061/#sec2).
These references identify the mathematical method; no pseudocode or upstream
source revision was copied. No third-party implementation, asset or dependency was added. The new
project code remains under the repository's MIT license. The preceding monotone
queue implementation from this repository was retained in a private comparison
probe as the baseline. Historical architecture-audit reports retain their frozen
source-revision descriptions.

Each pass is O(row width + radius), hence O(row width) under the existing window
fit constraint. The implementation uses two reusable byte buffers, each of length
`width + 2r`, replacing the old window queue. This increases scratch storage to
O(width); at width 1920 and radius 12 these two buffers occupy 3888 bytes. The
existing contrast image and two row buffers remain. Image/radius/output limits,
safe indexing and row-boundary cooperative cancellation are preserved. This is
not a new hard cancellation deadline or a general peak-memory qualification.

## Correctness

The existing independent brute-force window test remains, renamed for the new
implementation. Added tests cover every radius 1–128, odd/even widths and partial
blocks, random/constant/alternating pixels and both image edges. Complete detection
results also match independently evaluated erosion/dilation neighborhoods,
including all observations and rejection counters. Existing clipping, gradients,
cancellation and output-limit regressions remain. Full 705-test, formatting,
strict Clippy and release gate passed; no lint was disabled.

All twelve private saved Raptor frame reports (three selections, on/off states,
both cameras) are exactly equal to their prior queue-based JSON reports. This
includes source integrity, detections, rejection counters and continuity. The
automatic recorded matcher retains exactly 473/480/473 candidates and every
complete earlier query result; the older seeded path retains 79/81/74. Input
preservation, artifact CRCs and lengths also pass. These are numerical equivalence
checks on saved data, not physical calibration or dimensional accuracy evidence.

## Measurements

A private optimized Rust probe compared the original queue function and the new
independent block implementation on two saved 1920×1200 images. For each radius,
it ran one warm-up and ten measured paired iterations, alternating execution
order and comparing every contrast byte outside timers. The same binary computes
both methods. Timings include morphology buffers/allocation and contrast creation,
but exclude feature detection and file I/O. Source, command, binary/library hashes
and raw timing rows are retained locally.

| Radius | Queue median | Block median | Median paired speedup |
| --- | ---: | ---: | ---: |
| 1 | 69.21 ms | 19.64 ms | 3.54× |
| 4 | 71.27 ms | 20.20 ms | 3.58× |
| 12 | 70.22 ms | 14.60 ms | 4.81× |
| 32 | 69.53 ms | 12.02 ms | 5.93× |
| 128 | 68.54 ms | 10.55 ms | 6.57× |

On the Ryzen 9 6900HS host, the subsequent public-library phase probe measured
two-image detection at median 18.79 ms over ten post-warm-up iterations. Its
earlier observation was 71.52 ms; those detector runs used separately built probes,
not a paired old/new detector benchmark. Cached matching remained about 8.26 ms.

Three complete CLI runs took 0.13/0.13/0.14 seconds, user CPU 0.06 seconds each,
system CPU 0.05/0.06/0.05 seconds and peak RSS 18280/18184/18440 KiB. Earlier
prepared-matcher runs were 0.19 seconds. These reused-file observations include
full recording validation and diagnostic publication/fsync on a shared machine;
they are not live frame-rate measurements or a controlled end-to-end speedup.
Performance on other images, radii and machines needs separate measurement.

No scanner access, settings, firmware or calibration changed. Source/device/unit
binding, live integration, sustained reliability and physical qualification remain
unfinished; the optimization preserves those existing boundaries.
