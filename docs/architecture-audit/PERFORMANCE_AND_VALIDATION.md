Audited source: `ced7697fa0a4e2a262ae255efffa53ca577f117f`. Confirmed/inference/recommendation labels and audit scope are defined in ARCHITECTURE.md.

# Performance, correctness and maturity evidence

## What this audit verifies and what it does not rerun

**Confirmed source/records:** manifests/module inventory, declared limits and
qualification flags were inspected at the audited source. Existing developer
records report a fully isolated current-Rust-source gate of652 tests, formatting,
strict all-target Clippy and release, plus actual synthetic parent/helper/engine
and offline corner CLI publication/integrity/overwrite checks. Rust production
source at ced7697 is unchanged from the named cac7f5b gate; later readiness tooling
has ten offline Python tests. Relevant details are in REVIEW_RESPONSES.md,
TARGET_CORNER_REFINEMENT.md, CAPTURE_PROCESS_ISOLATION.md and DEVELOPMENT_REVIEW.md.
This audit does **not** independently rerun652 tests, physical trials or all
historical oracles; counts alone are not correctness or physical qualification.
Independent reviewer statements remain exclusively in its own findings file.

Shared-worktree output contamination was observed: some debug checks/tests passed
while a release executable/library came from an older checkout. Those failed
release attempts were not full passes. A never-shared build and actual executable
checks recovered the normal binaries. Isolated per-checkout outputs and exact
source/binary identification are part of reproducibility, not a numerical speedup.

## Evidence matrix

| Claim area | Evidence actually available | Scope and missing evidence |
|---|---|---|
| Projection/triangulation/rectification | Synthetic OpenCV comparisons and Rust unit tests (tools/compare_calibration_opencv.py, compare_stereo_opencv.py, compare_rectification_opencv.py; respective examples/docs) | Numerical conventions/domain checks; not actual Raptor calibration/scale/exposure verification |
| Supplied-observation lens/stereo fits | compare_planar_opencv.py, compare_brown_calibration_opencv.py, compare_stereo_fit_opencv.py; fit and Jacobian/stationarity tests | Synthetic supplied correspondences, not detected/qualified real targets; lenses fixed in stereo fit |
| Corner localization |12 synthetic cases; largest recorded Rust difference5.27e-8px from NumPy,1.06e-6px from OpenCV and0.0284px from synthetic truth (TARGET_CORNER_REFINEMENT.md) | Specific quantized smooth saddle family, not real board/glare/blur accuracy or uncertainty |
| Local ICP/robust fits | Geometry tests plus independent SciPy robust-plane/cylinder tools and known-transform cases | Local convergence/conditioning, not repeated physical/global registration or tracking recovery success rate |
| Reconstruction/publication reliability | Synthetic sequence/live process integration, integrity/footer/provenance/overwrite and cancellation/error tests | Demonstrates software contracts, not stable real continuous scan or qualified mesh |
| Hardware acquisition/replay | Historical ACCEPTANCE_RESULTS reports real structural frames, replay and timed trials | Private historical records, not independently re-executed here; setup-specific and optical correctness separate |
| GUI responsiveness/close | Device-free process/worker/selection regressions; isolated capture and preview omission behavior | No measured whole-app hard deadline, real stalled ioctl qualification or fresh GUI/GPU/manual workflow test in this audit |
| Memory | Explicit container/output/voxel limits and reported map coordinate bytes; source-visible document/history/frame copies | No complete processRSS, allocation peak,60-minute growth or large-session distribution qualification |
| CPU/concurrency | Scoped paired remap and sequential fallbacks; recorded component benchmark | No full live CPU utilization profile or release-wide performance gate |
| SIMD | No explicit application SIMD kernel identified in geometry/image/depth sources/manifests | Compiler/dependency vectorization may occur; absence of intrinsics is not proof no CPU vector instructions execute |
| GPU | App glow graphics backend; no production reconstruction compute dispatcher found | OpenGL presentation is not GPU triangulation/ICP/fusion; no CPU-vs-GPU reconstruction result |
| Throughput/latency | Component timings below and raw acquisition duration trials | No measured90% valid-frame processing,20FPS live preview,p95<=150ms,30-minute calibrated scan or full pipeline benchmark |

## Recorded timings, with limits

BENCHMARKS.md and linked checked-in benchmark JSON record:

- Historical million-point trim:26.060871ms release, one trial; selection analytic
  million-point case:12.745299ms/250,000 selected and no analytic classification
  errors. These are narrow synthetic operations, not all gesture/million-point
  editing distributions or GUI completion latency.
- Repeated1920x1200 single-image remap:20 measured runs after warmup on Ryzen9
  6900HS, median40.109ms,p9546.174ms,max49.208ms; one map build202.117ms.
  Coordinate storage55,296,000bytes is not RSS.
- Paired synthetic1920x1200 remapping median89.930ms sequential versus43.795ms
  adaptive,20 alternating rounds, exact output/mask comparisons. Two map coordinate
  buffers110,592,000bytes, not process memory. Allocation and spawn/join included;
  map construction/output comparison excluded. CPU governor/OS load were not fixed
  (PAIRED_REMAP_CONCURRENCY.md). This is a measured advantage over the local
  sequential implementation only, not OpenCV/CrealityScan or full scan throughput.

Historical real acquisition includes30,050 valid frames/604.66s for one IR duration
trial and18,100 accepted per camera/606.44s for a dual-IR structural soak.
The latter's92/18,192 buffer rejection proxy per camera is0.505717%, above the
<0.1% provisional target, and post-soak reopening had asymmetry. Rejected kernel
buffers are not a proven physical exposure-loss count. Exact evidence identifiers
and failed/incomplete gate language are in ACCEPTANCE_RESULTS.md. Do not average
away failures or turn structural duration into calibrated reconstruction success.

## Maturity classification

“Implemented/validated” here always names its tested scope; no whole scanning
subsystem is declared professionally qualified from unit tests.

| Capability | Classification | Evidence/boundary |
|---|---|---|
| Bounds/CRC/footer/no-overwrite parsers and publications | Implemented; unit/integration validated within caps | recording, formats, workflow output and corruption/recovery tests |
| Synthetic optical math, local registration/filtering and basic edits | Implemented; synthetic/reference validated | named algorithm modules and compare_* tools |
| Native structural acquisition and original-byte recording | Implemented; actual workstation observations, reliability incomplete | device/acquisition and historical acceptance failures |
| Native helper-based live processing and GUI display | Implemented experimental orchestration; device-free tests | live_process/live_stereo runner; no calibrated continuous hardware qualification |
| Real scanner mode/optics/coordinate binding | Partial/experimental | factory diagnostics/storage declarations do not establish physical truth |
| Actual calibrated Raptor point cloud and full professional scan | Missing qualification; M2 still incomplete | README, acceptance, false report flags, independent finding006 |
| Local loss detection/reacquisition | Implemented prototype; synthetic evidence | tracking/session gates; no global recovery guarantee |
| Descriptor/global registration, loop closure, pose graph | Not found/missing | local registration/tracking/fusion source and inventory |
| Organized-grid meshing, basic selective mesh repair/edit | Implemented with narrow topology/size constraints | mesh/grid/fill/smooth/simplify tests |
| General fused-cloud watertight surface reconstruction | Missing | organized adjacency only; no Poisson/TSDF/ball-pivot pipeline found |
| Broad editing suite | Partial | basic screen/3D selection, outliers and mesh operations exist; automatic table/RANSAC segmentation, general cloud smoothing, comprehensive hidden/isolate workflows and all mission UI modes are not established by this audit |
| Full portable installation/end-to-end reliability | Missing qualification | workstation kernel/permissions dependency; no10/10 calibrated scan-edit-mesh-measure-reopen qualification |

## Reproducible next measurements

Freeze a revision, use a separate target per checkout and retain toolchain/host,
configuration, dataset hashes, run order, warmups, repetitions and failure cases.
Run tools/check.sh plus independent numerical scripts; verify actual executable
behavior and persisted artifacts. For hardware work, obtain authorized calibrated
objects/mode/binding, characterize uncertainty, capture both raw streams/settings,
measure drops separately from rejects and optical exposure identity, then evaluate
complete reconstruction/tracking/export/reopen repeatedly. Calibration is still
deferred; this audit does not perform it or close any hardware acceptance gate.
