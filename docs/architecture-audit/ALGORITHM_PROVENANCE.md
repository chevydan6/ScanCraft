# Algorithm source and provenance inventory

Audited source: `ced7697fa0a4e2a262ae255efffa53ca577f117f`. Scope/labels: ARCHITECTURE.md. This inventory records
what source/records establish; it does not prove originality from absence of a
citation or assign Creality's internal algorithms to ScanCraft.

## Classification and common license/version fields

- **L**: actual library implementation reused through Cargo; exact versions,
  reported upstream revisions and licenses are in DEPENDENCIES.md and
  `docs/publication/DEPENDENCIES.json` (nalgebra SVD/eigen, serde, codecs, CRC, UI).
- **D**: source/documentation explicitly declares an independent mathematical
  implementation with a named reference. This is documented provenance, not an
  independent forensic proof of every line's authorship.
- **B**: format/math interpreted from observed device/vendor behavior and tested
  against private installed-software oracles. No vendor binary, firmware or
  private calibration dump is included by these reports. A behavioral oracle is
  not an open-source upstream and cannot be auto-vendored or redistributed.
- **R**: repository implementation whose source was inspected. Unless a reference
  is named below, historical upstream origin/translation/adaptation rationale is
  **unknown**. Equivalent mathematics in another library is not ancestry.

All local implementations below are inspected at the exact audited Git revision
above and are **declared MIT** by the workspace/LICENSE; the kernel experiment
is separately GPL-2.0-only. No LGPL/GPL C++ geometry implementation is linked by
the manifests. Missing local attribution is not a legal clearance. Retain existing
third-party copyright/license/font notices. For a proposed imported implementation,
record its precise file/revision/license separately before redistribution.
No specific upstream source revision is assigned to R/D/B methods unless named;
publication/docs versions and actual dependency revisions are different things.

## A01. Raw frame and UVC metadata validation

- Purpose/mathematics: Profile/length/error/PTS validation; Linux UVCH copied stride from flags; bounded SCR interpretation.
- Exact implementation files: [crates/scancraft-core/src/lib.rs](../../crates/scancraft-core/src/lib.rs), [crates/scancraft-device/src/scr_observations.rs](../../crates/scancraft-device/src/scr_observations.rs). Entry points: `validate_frame, uvc_pts, observe`.
- Provenance: R/B: repository protocol implementation and observed Linux/device behavior; no upstream code revision attribution asserted.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: core/device tests, recording tests; independent UVCH regression in REVIEW_RESPONSES SCAN-REV-017. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Structural bytes/PTS do not verify illumination, simultaneous exposures or optical correctness.
- Possible optimization (**recommendation**, not measured advantage): Reduce copies only after validating buffer ownership and replay equality.

## A02. Recording codec, indexing and recovery

- Purpose/mathematics: Bounded little-endian records, CRC32, checked footer/count and sequential validation; indexed navigation/recovery remain separate workflows.
- Exact implementation files: [crates/scancraft-recording/src/lib.rs](../../crates/scancraft-recording/src/lib.rs). Entry points: `Writer::push/finish, Reader::read_frame; recovery/playback modules`.
- Provenance: R + L: project container logic; crc32fast is library reuse (locked inventory).
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: recording unit tests; workflow/tests/recording_recovery.rs and recording_reconstruction.rs. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: CRC32 is not authentication; absent footer is incomplete, not silently successful.
- Possible optimization (**recommendation**, not measured advantage): Benchmark chunked I/O and indexes against retained byte-exact records.

## A03. Pinhole/Brown5 projection and inversion

- Purpose/mathematics: Radial/tangential Brown5 projection, analytic Jacobian and iterative inverse/domain checks.
- Exact implementation files: [crates/scancraft-calibration/src/lib.rs](../../crates/scancraft-calibration/src/lib.rs). Entry points: `CameraModel::{validate,project,ray}, PreparedCamera`.
- Provenance: D: optical model documented in CALIBRATION.md/OpenCV references; independent implementation declared.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: compare_calibration_opencv.py, brown_reference example and module unit tests. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Supported lens radius/model must come from validated calibration; factory rational coefficients cannot always be reduced.
- Possible optimization (**recommendation**, not measured advantage): Reuse prepared model checks/maps; measure SIMD only with unchanged domain behavior.

## A04. Stereo triangulation

- Purpose/mathematics: Checked closest-ray geometry with pose, depth, ray separation/angle and reprojection gates.
- Exact implementation files: [crates/scancraft-calibration/src/stereo.rs](../../crates/scancraft-calibration/src/stereo.rs). Entry points: `PreparedStereo::triangulate`.
- Provenance: R/D: explicit optical geometry, repository code; no copied OpenCV triangulation implementation identified.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: compare_stereo_opencv.py and stereo.rs unit tests. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Small baseline angles/incorrect camera association fail or give misleading results if input declarations are wrong.
- Possible optimization (**recommendation**, not measured advantage): Batch prepared rays and profile, preserving degeneracy checks.

## A05. Stereo rectification and epipolar checks

- Purpose/mathematics: Common rectified frame, source/rectified transforms, essential/fundamental matrix geometry and Sampson residual.
- Exact implementation files: [crates/scancraft-calibration/src/rectification.rs](../../crates/scancraft-calibration/src/rectification.rs), [crates/scancraft-calibration/src/epipolar.rs](../../crates/scancraft-calibration/src/epipolar.rs). Entry points: `StereoRectification::{build,source_pixels,rectify_pixels}; EpipolarMatrices::sampson_distance_pixels`.
- Provenance: D/R: documented independent numerical geometry; no upstream port revision recorded.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: compare_rectification_opencv.py; rectification_reference and module tests. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Epipolar consistency does not identify device axes, units, mirror state or exposure synchronization.
- Possible optimization (**recommendation**, not measured advantage): Reuse fixed transforms/maps rather than rebuilding per frame.

## A06. Planar calibration initializer

- Purpose/mathematics: Normalized DLT homographies and Zhang absolute-conic constraints, SVD/rank checks and pose recovery.
- Exact implementation files: [crates/scancraft-calibration/src/planar.rs](../../crates/scancraft-calibration/src/planar.rs). Entry points: `initialize_planar, homography, recover_pose`.
- Provenance: D: explicitly cites Zhang MSR-TR-98-71 sections 2.3/3.1 and appendices A/C; independent code, not a project translation.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: compare_planar_opencv.py and planar.rs tests; PLANAR_CALIBRATION_INITIALIZER.md. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Needs supplied correct metric associations and diverse views; no full-board detector or physical fit qualification.
- Possible optimization (**recommendation**, not measured advantage): Retain normalization/rank tests; compare conditioning before replacing solver.

## A07. Joint lens/target refinement

- Purpose/mathematics: Damped nonlinear least squares with block Schur elimination over Brown5/intrinsics and per-view poses.
- Exact implementation files: [crates/scancraft-calibration/src/refinement.rs](../../crates/scancraft-calibration/src/refinement.rs), [crates/scancraft-calibration/src/normal_blocks.rs](../../crates/scancraft-calibration/src/normal_blocks.rs), [crates/scancraft-calibration/src/projection_differential.rs](../../crates/scancraft-calibration/src/projection_differential.rs). Entry points: `refine_planar, solve_step and analytic projection/pose differentials`.
- Provenance: D: source explicitly cites Zhang section 3.2, Brown/OpenCV projection and standard Schur method; no upstream code copied per recorded declaration.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: compare_brown_calibration_opencv.py, differential/unit tests; BROWN_CALIBRATION_REFINEMENT.md. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Stationarity and reprojection residual are not physical uncertainty, coverage or valid device binding.
- Possible optimization (**recommendation**, not measured advantage): Profile block assembly/factorization; numerical Jacobian comparisons protect analytic changes.

## A08. Fixed-intrinsic stereo fit

- Purpose/mathematics: Joint relative-rig and target-pose least squares with fixed supplied lenses, analytic blocks and final gates.
- Exact implementation files: [crates/scancraft-calibration/src/stereo_fit.rs](../../crates/scancraft-calibration/src/stereo_fit.rs). Entry points: `fit_stereo_extrinsics`.
- Provenance: D/R: independent implementation recorded in STEREO_EXTRINSIC_CALIBRATION.md; no Ceres/OpenCV source port assigned.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: compare_stereo_fit_opencv.py and stereo_fit.rs tests. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Does not jointly refine lenses; declared corresponding target observations are not attested simultaneous exposures.
- Possible optimization (**recommendation**, not measured advantage): Use held-out geometry and conditioning diagnostics before solver optimizations.

## A09. Factory format/optics diagnostics

- Purpose/mathematics: Checked binary fields/checksum, observed rational Brown evaluation, transforms and bounded optical-field interpretation.
- Exact implementation files: [crates/scancraft-calibration/src/factory.rs](../../crates/scancraft-calibration/src/factory.rs), [crates/scancraft-calibration/src/factory_named.rs](../../crates/scancraft-calibration/src/factory_named.rs), [crates/scancraft-calibration/src/factory_brown.rs](../../crates/scancraft-calibration/src/factory_brown.rs), [crates/scancraft-calibration/src/factory_pose.rs](../../crates/scancraft-calibration/src/factory_pose.rs), [crates/scancraft-calibration/src/factory_laser.rs](../../crates/scancraft-calibration/src/factory_laser.rs). Entry points: `verify/inspect; forward/brown5_if_supported; pose/raw field helpers (entry index below)`.
- Provenance: B: behavior-derived device/vendor-format investigations documented in FACTORY_CALIBRATION.md and module docs. No public vendor source repository/revision exists in the audit evidence.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: Synthetic fixtures and compare_factory_* tools; installed-DLL oracle harnesses are test-only and not redistributed DLLs. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Observed field names/math are not proven physical camera/mode/domain binding; diagnostic math is not automatically installed calibration.
- Possible optimization (**recommendation**, not measured advantage): Fuzz bounded decoders; retain independent oracles without uploading proprietary binaries or private dumps.

## A10. Image undistortion/remapping

- Purpose/mathematics: Inverse map preparation, bilinear samples, validity masks and scoped two-image application with sequential fallback.
- Exact implementation files: [crates/scancraft-image/src/lib.rs](../../crates/scancraft-image/src/lib.rs), [crates/scancraft-image/src/stereo.rs](../../crates/scancraft-image/src/stereo.rs). Entry points: `UndistortMap::{build,apply}; StereoImageMaps::{apply,apply_sequential}`.
- Provenance: R/D: project implementation over camera math; math reference comparisons to OpenCV, not an OpenCV runtime.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: compare_undistort_opencv.py; undistort_benchmark/paired_remap_benchmark; PAIRED_REMAP_CONCURRENCY.md. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Large maps consume substantial memory; current component speedup is not full scan throughput.
- Possible optimization (**recommendation**, not measured advantage): Reuse maps/buffers; measure precision and memory before compressed-coordinate/SIMD work.

## A11. Raw laser stripe observations

- Purpose/mathematics: Threshold runs per row, weighted centroids/variance and width/peak/saturation gates; sliding-window morphology via monotone queues.
- Exact implementation files: [crates/scancraft-image/src/laser.rs](../../crates/scancraft-image/src/laser.rs). Entry points: `detect_rows and top-hat/opening support`.
- Provenance: R: repository implementation; no documented upstream code project/revision. Similarity to standard morphology is not source attribution.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: laser.rs tests and CLI detect-stripes diagnostics. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: No stripe continuity/identity, projector-line correspondence or physical wavelength/mode inference.
- Possible optimization (**recommendation**, not measured advantage): Profile row buffers; preserve multiple-run and clipping rejection semantics.

## A12. Laser ray/sheet geometry

- Purpose/mathematics: Quadratic ray-sheet roots with bounded domain and ambiguity handling.
- Exact implementation files: [crates/scancraft-calibration/src/laser_surface.rs](../../crates/scancraft-calibration/src/laser_surface.rs). Entry points: `QuadraticSheet::intersect`.
- Provenance: B/R: independent mathematical evaluation with synthetic observed-reference fixtures; not a qualified vendor laser pipeline.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: laser_surface_reference.py and synthetic laser-surface fixtures/module tests. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Multiple intersections need independent correspondence; physical sheet coefficients/binding remain unqualified.
- Possible optimization (**recommendation**, not measured advantage): Prepared coefficients/batched rays only after correspondence and numeric-domain validation.

## A13. Seeded subpixel target corners

- Purpose/mathematics: Gaussian gradient-orthogonality 2x2 least squares, recentering, contrast/rank/border/shift/convergence checks.
- Exact implementation files: [crates/scancraft-image/src/corners.rs](../../crates/scancraft-image/src/corners.rs), [crates/scancraft-workflow/src/target_corners.rs](../../crates/scancraft-workflow/src/target_corners.rs). Entry points: `refine_corner; target_corners::run`.
- Provenance: D: explicitly follows mathematical description of OpenCV cornerSubPix 4.1.2, no implementation copied.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: corner_reference example; compare_target_corners_opencv.py, image/workflow tests; TARGET_CORNER_REFINEMENT.md. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Not full target detection/indexing; synthetic pixel agreement does not establish actual calibration.
- Possible optimization (**recommendation**, not measured advantage): Batch seeds/reuse scratch only after measuring; preserve association and immutable source bytes.

## A14. Sparse stereo seed selection and integer matching

- Purpose/mathematics: Grid-spaced valid textured patches; signed-disparity ZNCC, uniqueness and reverse-consistency checks with work budgets.
- Exact implementation files: [crates/scancraft-depth/src/matching.rs](../../crates/scancraft-depth/src/matching.rs). Entry points: `select_seeds, match_rectified`.
- Provenance: R: repository implementation of standard patch matching; no OpenCV StereoBM/SGM port source/revision recorded.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: matching.rs tests and sparse stereo reference tools/documentation. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Sparse and local; repetitive/blank patterns reject; not dense SGM or feature-descriptor global matching.
- Possible optimization (**recommendation**, not measured advantage): Profile patch normalization/search; SIMD/integral reuse must retain masks and score gates.

## A15. Subpixel disparity and mapped reconstruction

- Purpose/mathematics: Bounded local quadratic correlation refinement, original-camera observation mapping and checked triangulated Cloud construction.
- Exact implementation files: [crates/scancraft-depth/src/matching/subpixel.rs](../../crates/scancraft-depth/src/matching/subpixel.rs), [crates/scancraft-depth/src/pipeline.rs](../../crates/scancraft-depth/src/pipeline.rs), [crates/scancraft-depth/src/lib.rs](../../crates/scancraft-depth/src/lib.rs). Entry points: `StereoProcessor::{build,process,process_automatic}, reconstruct; subpixel module helpers`.
- Provenance: R/D: independent ZNCC/quadratic method; reference oracle tools/check_subpixel_reference.py.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: depth module tests, subpixel_reference example, workflow recording_reconstruction integration. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Fractional fit is conditional, not a dense depth guarantee; physical flags remain false and empty matches produce no manufactured cloud.
- Possible optimization (**recommendation**, not measured advantage): Cache calibration/maps and benchmark the complete pair path, not only matching.

## A16. Exact spatial queries

- Purpose/mathematics: Axis-partition tree with deterministic exact nearest-neighbor traversal, distances and tie rules.
- Exact implementation files: [crates/scancraft-geometry/src/spatial.rs](../../crates/scancraft-geometry/src/spatial.rs). Entry points: `SpatialIndex::{build,nearest,k_nearest}`.
- Provenance: R: repository KD-tree family implementation; no nanoflann/PCL/Open3D code ancestry documented.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: spatial tests compare brute force; spatial query benchmark in BENCHMARKS.md. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Worst-case query/build and memory behavior is not qualified for arbitrary huge scans.
- Possible optimization (**recommendation**, not measured advantage): Compare distributions/build reuse/cache layout before replacing with an external index.

## A17. Rigid and plane fitting

- Purpose/mathematics: Named Kabsch centered SVD fit with determinant/degeneracy checks; covariance smallest-eigenvector plane fit.
- Exact implementation files: [crates/scancraft-geometry/src/registration.rs](../../crates/scancraft-geometry/src/registration.rs). Entry points: `rigid_fit, fit_plane`.
- Provenance: R: Kabsch is explicitly named in the source. Exact original paper/code revision not recorded; nalgebra performs linear algebra.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: registration.rs and geometry integration/reference tests. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Known correspondences required; degenerate geometry rejects; low RMS is not global alignment proof.
- Possible optimization (**recommendation**, not measured advantage): Reuse centered accumulations and test metric scaling/degenerate cases.

## A18. Point-to-point ICP

- Purpose/mathematics: Exact indexed nearest-neighbor correspondences alternating with rigid fits and convergence/distance bounds.
- Exact implementation files: [crates/scancraft-geometry/src/registration.rs](../../crates/scancraft-geometry/src/registration.rs). Entry points: `icp, icp_with`.
- Provenance: R: local ICP family, not an identified PCL/Open3D port. No upstream source revision recorded.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: registration.rs tests and tracking/session tests. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Requires suitable initial pose; symmetry and low overlap can produce wrong local convergence.
- Possible optimization (**recommendation**, not measured advantage): Reuse index and sampled working sets; compare complete quality gates on common data.

## A19. Point-to-plane ICP and robust loss

- Purpose/mathematics: Linearized point/normal residual least squares and optional Huber weights with explicit millimeter delta and step/rank gates.
- Exact implementation files: [crates/scancraft-geometry/src/point_plane.rs](../../crates/scancraft-geometry/src/point_plane.rs). Entry points: `step_with_loss, icp_with_loss`.
- Provenance: R/D: documented independent robust least-squares implementation; Huber method similarity is not a Ceres/PCL source attribution.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: check_robust_plane_reference.py; point_plane.rs/tracking tests and robust reference docs. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Needs trustworthy target normals/initialization; robust residual weights do not solve global relocalization.
- Possible optimization (**recommendation**, not measured advantage): Profile correspondence/system assembly; preserve independent rank/overlap/final-quality checks.

## A20. Tracking, recovery and session state

- Purpose/mathematics: Local registration quality gates, accepted-pose history, Lost state and controlled retry from retained reference/pose.
- Exact implementation files: [crates/scancraft-geometry/src/tracking.rs](../../crates/scancraft-geometry/src/tracking.rs), [crates/scancraft-geometry/src/session.rs](../../crates/scancraft-geometry/src/session.rs). Entry points: `Tracker::track_with_normals/reset; ScanSession::process/start/pause/resume/stop`.
- Provenance: R: project state machine/orchestration; no upstream tracking-system port documented.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: tracking/session unit and live runner tests. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: No descriptor-based global relocalizer, loop closure or graph optimization was found; local reacquisition is not guaranteed recovery.
- Possible optimization (**recommendation**, not measured advantage): Create real labeled lost/recovery sequences before tuning acceptance heuristics.

## A21. Voxel fusion and downsampling

- Purpose/mathematics: Bounded integer voxel keys, per-cell centroid/attributes and deterministic outputs; rejected tracking is not fused.
- Exact implementation files: [crates/scancraft-geometry/src/fusion.rs](../../crates/scancraft-geometry/src/fusion.rs), [crates/scancraft-geometry/src/lib.rs](../../crates/scancraft-geometry/src/lib.rs). Entry points: `VoxelFusion::integrate_tracked/preview_cloud; Cloud::voxel_downsample_with`.
- Provenance: R: repository implementation, no Open3D/PCL voxel source port recorded.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: fusion/session/downsample tests; sequence and live synthetic publication integration. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Not TSDF/occupancy/surface fusion; averaging can erase detail; large voxel history/memory is not qualified.
- Possible optimization (**recommendation**, not measured advantage): Streaming accumulators/layout and cap behavior should be profiled before parallelization.

## A22. Cloud cleanup/outliers

- Purpose/mathematics: Exact duplicate keys, radius neighbor counts, k-neighbor mean-distance distribution and explicit statistical threshold.
- Exact implementation files: [crates/scancraft-geometry/src/filtering.rs](../../crates/scancraft-geometry/src/filtering.rs), [crates/scancraft-geometry/src/statistical_filter.rs](../../crates/scancraft-geometry/src/statistical_filter.rs). Entry points: `remove_duplicates_with, radius_filter_with, analyze/filter`.
- Provenance: R: familiar filter families also present in PCL/Open3D; no copying or specific upstream revision established.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: geometry tests plus check_statistical_filter_reference.py. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Density thresholds can remove legitimate thin/isolated detail; no automatic physical truth.
- Possible optimization (**recommendation**, not measured advantage): Compare brute-force equality/index reuse and preserve selection/source histories.

## A23. Screen/spatial trimming and history

- Purpose/mathematics: Box/plane tests, even-odd polygons, capsules, depth-per-pixel visibility with deterministic ties; original-index edits and bounded history.
- Exact implementation files: [crates/scancraft-geometry/src/selection.rs](../../crates/scancraft-geometry/src/selection.rs), [crates/scancraft-geometry/src/lib.rs](../../crates/scancraft-geometry/src/lib.rs), [crates/scancraft-app/src/screen_tools.rs](../../crates/scancraft-app/src/screen_tools.rs). Entry points: `Cloud::select_box_with/trimmed_with; Editor::trim/undo/redo; selection APIs`.
- Provenance: R: original repository editing/UI logic; geometric-method similarity is not VCG/CloudCompare source attribution.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: geometry selection/history and app gesture tests; selection benchmark records. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Point-depth visibility approximates point rendering, not occluding reconstructed surfaces; history copies affect memory.
- Possible optimization (**recommendation**, not measured advantage): Measure million-point latency/history memory before changing representation.

## A24. Organized meshing and surface normals

- Purpose/mathematics: Declared grid adjacency triangulation with edge/validity gates; angle-weighted normals from explicit surface topology and optional viewpoint.
- Exact implementation files: [crates/scancraft-geometry/src/mesh.rs](../../crates/scancraft-geometry/src/mesh.rs), [crates/scancraft-geometry/src/mesh_normals.rs](../../crates/scancraft-geometry/src/mesh_normals.rs), [crates/scancraft-workflow/src/stereo_surface.rs](../../crates/scancraft-workflow/src/stereo_surface.rs). Entry points: `from_grid, mesh_normals::estimate, stereo_surface::build`.
- Provenance: R: repository implementation, not Poisson/PCL/VCG reconstruction code.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: mesh/normal tests and stereo surface workflow tests. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Does not reconstruct arbitrary fused clouds or estimate general kNN point normals; no watertightness guarantee.
- Possible optimization (**recommendation**, not measured advantage): Batch topology reuse and retain rejection/source-index mappings.

## A25. Mesh topology, components and repair primitives

- Purpose/mathematics: Indexed edge/face adjacency, component traversal, winding propagation and explicit nonmanifold vertex-fan splitting.
- Exact implementation files: [crates/scancraft-geometry/src/mesh_components.rs](../../crates/scancraft-geometry/src/mesh_components.rs), [crates/scancraft-geometry/src/mesh_boundaries.rs](../../crates/scancraft-geometry/src/mesh_boundaries.rs), [crates/scancraft-geometry/src/mesh_orientation.rs](../../crates/scancraft-geometry/src/mesh_orientation.rs), [crates/scancraft-geometry/src/mesh_split_fans.rs](../../crates/scancraft-geometry/src/mesh_split_fans.rs), [crates/scancraft-geometry/src/mesh.rs](../../crates/scancraft-geometry/src/mesh.rs). Entry points: `topology; analyze, orient and module public entry points`.
- Provenance: R: repository implementations; no CCCoreLib/VCGlib translation identified.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: geometry mesh topology/component/orientation/fan integration tests. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Selective repairs can reject ambiguous topology; not complete arbitrary self-intersection/nonmanifold repair.
- Possible optimization (**recommendation**, not measured advantage): Reuse adjacency caches with explicit invalidation and benchmark large meshes.

## A26. Selective mesh filling

- Purpose/mathematics: Planar contour validation/projection and ear-clipping triangulation without moving original boundary vertices.
- Exact implementation files: [crates/scancraft-geometry/src/mesh_fill.rs](../../crates/scancraft-geometry/src/mesh_fill.rs). Entry points: `fill`.
- Provenance: R: standard algorithm family, no upstream earcut/VCG source attribution recorded.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: mesh fill/boundary tests. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Only eligible explicit planar contours; not automatic filling of arbitrary holes, and intentional openings require user choice.
- Possible optimization (**recommendation**, not measured advantage): Retain planarity/intersection budgets; benchmark large eligible contours.

## A27. Mesh smoothing and simplification

- Purpose/mathematics: Constrained smoothing; priority-queue shortest-edge collapse with cumulative vertex displacement, crease/normal/topology bounds.
- Exact implementation files: [crates/scancraft-geometry/src/mesh_smooth.rs](../../crates/scancraft-geometry/src/mesh_smooth.rs), [crates/scancraft-geometry/src/mesh_simplify.rs](../../crates/scancraft-geometry/src/mesh_simplify.rs). Entry points: `smooth, simplify`.
- Provenance: R: repository code; simplifier is explicitly shortest-edge, not a documented Garland-Heckbert quadric-error port.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: geometry mesh_smooth/mesh_simplify integration tests. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Bounded operation budgets/mesh sizes; vertex/normal bounds are not an independent global Hausdorff certificate.
- Possible optimization (**recommendation**, not measured advantage): Compare fidelity/time against QEM on common retained meshes before changing method.

## A28. Measurements and coordinate alignment

- Purpose/mathematics: Euclidean distances/angles, least-squares primitives, three-start PCA/nonlinear cylinder fitting, reference frames and plane/mesh intersections.
- Exact implementation files: [crates/scancraft-geometry/src/measurement.rs](../../crates/scancraft-geometry/src/measurement.rs), [crates/scancraft-geometry/src/plane_measurement.rs](../../crates/scancraft-geometry/src/plane_measurement.rs), [crates/scancraft-geometry/src/cylinder.rs](../../crates/scancraft-geometry/src/cylinder.rs), [crates/scancraft-geometry/src/coordinate_frame.rs](../../crates/scancraft-geometry/src/coordinate_frame.rs), [crates/scancraft-geometry/src/mesh_section.rs](../../crates/scancraft-geometry/src/mesh_section.rs). Entry points: `angle_degrees, fit_circle, fit_sphere, cylinder::fit, plane_measurement::measure, Frame::from_points, intersect`.
- Provenance: R/D: cylinder module declares independent geometric fitting; no general CAD or upstream fitting-library source revision established.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: geometry measurement/cylinder/section tests; check_cylinder_reference.py. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: No robust global primitive recognition, metrological uncertainty or parametric CAD model; bad geometry remains bad measurements.
- Possible optimization (**recommendation**, not measured advantage): Retain stationarity/conditioning checks and add real ground-truth repeated trials.

## A29. Formats, provenance and publication

- Purpose/mathematics: Bounded parsers, versioned CRC-backed JSON projects, unit-explicit mesh imports and no-overwrite Linux publication.
- Exact implementation files: [crates/scancraft-geometry/src/formats.rs](../../crates/scancraft-geometry/src/formats.rs), [crates/scancraft-geometry/src/mesh_import.rs](../../crates/scancraft-geometry/src/mesh_import.rs), [crates/scancraft-workflow/src/output.rs](../../crates/scancraft-workflow/src/output.rs), [crates/scancraft-workflow/src/lib.rs](../../crates/scancraft-workflow/src/lib.rs). Entry points: `read_ply, save_project/load_project, mesh_import::decode; artifact, publish`.
- Provenance: R + L: project format logic; serde/image/CRC libraries are declared dependencies.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: format/import tests and workflow bundle/recording integration tests. These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Partial failure evidence must be retained; CRC is not authenticity; not all mission interchange/container capabilities exist.
- Possible optimization (**recommendation**, not measured advantage): Profile repeated serialization/history expansion; protect old readers and exact-source provenance.

## A30. Process IPC and display policy

- Purpose/mathematics: Bounded length-framed messages, prepared JSON packets, nonblocking supervisor batches, EOF/exit validation and truthful unconfirmed cleanup.
- Exact implementation files: [crates/scancraft-app/src/capture_ipc.rs](../../crates/scancraft-app/src/capture_ipc.rs), [crates/scancraft-app/src/capture_process.rs](../../crates/scancraft-app/src/capture_process.rs), [crates/scancraft-app/src/live_process.rs](../../crates/scancraft-app/src/live_process.rs), [crates/scancraft-app/src/discovery_process.rs](../../crates/scancraft-app/src/discovery_process.rs). Entry points: `Decoder; supervise; helper_entry; retain_until_exit`.
- Provenance: R + L: project process protocol; serde/libc/std facilities are reused.
- Version/upstream/license: audited local commit and common fields above; no unrecorded upstream revision is invented.
- Validation: app process tests and tests/live_process.rs; independent findings017–020 responses (different scopes). These are code/reference evidence, not a physical qualification unless expressly recorded as such.
- Limitations: Software grace is not a hardware/global realtime bound; real stalls remain unqualified.
- Possible optimization (**recommendation**, not measured advantage): Measure IPC allocation/copy costs before transport changes; never weaken restoration ownership.

## Source-level entry points and unit-test index

Generated from tracked Rust source at the audited revision. Every source module
with public functions or local test functions is included, so helpers outside the
major-family summaries remain discoverable. Integration tests and reference tools
are additional evidence; a function/test name alone is not a passed experiment.
Private helper functions are resolved by inspecting the linked file; macro-
generated entry points are not expanded by this lexical index.

| Source file | Public function names | Local `#[test]` functions |
|---|---|---|
| [crates/scancraft-acquisition/src/lib.rs](../../crates/scancraft-acquisition/src/lib.rs) | `scan` | see integration tests |
| [crates/scancraft-app/src/archive_ui.rs](../../crates/scancraft-app/src/archive_ui.rs) | `is_busy`, `cancel`, `start_paths`, `start_scan_paths`, `tick`, `ui` | `native_worker_archives_every_frame_and_refuses_existing_output`, `native_worker_exports_saved_project_with_full_recordings` |
| [crates/scancraft-app/src/capture_close.rs](../../crates/scancraft-app/src/capture_close.rs) | `advance`, `is_waiting`, `slow`, `keep_open` | `close_waits_for_termination_then_retries_exactly_once`, `prolonged_or_repeated_close_never_forces_release_or_resets_elapsed`, `keeping_open_cancels_only_close_intent_and_idle_close_is_untouched` |
| [crates/scancraft-app/src/capture_ipc.rs](../../crates/scancraft-app/src/capture_ipc.rs) | `write_frame`, `write_complete`, `prepare_live_progress`, `write_live_complete`, `push`, `next`, `finish` | `fragmented_frame_and_completion_preserve_exact_data`, `one_frame_is_visible_without_a_second_frame_or_completion`, `frame_flush_failure_is_a_transport_failure`, `fragmented_live_updates_and_typed_completion_roundtrip`, `live_json_limits_fail_before_any_partial_packet_is_published`, `corrupt_sizes_truncation_and_trailing_data_never_succeed` |
| [crates/scancraft-app/src/capture_process.rs](../../crates/scancraft-app/src/capture_process.rs) | `spawn`, `child`, `private_log`, `run`, `nonblocking`, `supervise`, `helper_entry` | `pre_cancelled_request_does_not_launch_a_helper`, `invalid_helper_request_is_rejected_before_device_access`, `cancelled_stalled_process_does_not_block_supervisor_until_child_exit`, `timed_out_worker_retains_child_lock_until_cleanup_actually_finishes`, `graceful_stop_waits_for_finalization_beyond_abort_grace`, `process_frame_and_completion_require_clean_exit`, `helper_diagnostic_is_private_and_independent_of_ipc`, `helper_exit_without_completion_is_not_success` |
| [crates/scancraft-app/src/capture_session.rs](../../crates/scancraft-app/src/capture_session.rs) | `run` | `acquisition_or_write_error_still_closes_and_retains_restoration_failure`, `cleanup_failure_never_returns_successful_publication_state` |
| [crates/scancraft-app/src/capture_worker.rs](../../crates/scancraft-app/src/capture_worker.rs) | `spawn`, `poll`, `join` | `repeated_immediate_success_keeps_the_actual_completion`, `late_result_between_empty_receive_and_thread_finish_is_retained`, `result_before_thread_cleanup_does_not_block_poll`, `clean_exit_without_result_reports_disconnection` |
| [crates/scancraft-app/src/cloud_jobs.rs](../../crates/scancraft-app/src/cloud_jobs.rs) | `start`, `poll`, `build_preview` | `cylinder_record_worker_preserves_geometry_selection_and_persists_metadata_history`, `cylinder_fit_is_read_only_and_preserves_existing_redo`, `reusable_frame_definition_save_load_and_cross_cloud_apply_are_separate`, `coordinate_alignment_is_atomic_and_project_history_restores_exact_source`, `point_plane_worker_excludes_probe_and_saves_local_index_without_geometry_edits`, `direct_snapshot_worker_retains_vertex_and_commits_only_valid_records`, `direct_measurements_are_explicit_read_only_and_reject_invalid_selection`, `statistical_preview_changes_only_selection_and_delete_undo_restores_exact_attributes`, `scan_folder_load_relocates_references_and_failure_preserves_document_and_selection`, `recorded_origin_survives_gui_worker_edit_save_load_and_missing_bundle`, `reconstruction_job_publishes_before_replacing_document_and_keeps_saved_coordinates`, `failed_or_cancelled_reconstruction_keeps_original_history_and_selection`, `rejected_reconstruction_saves_diagnostics_without_replacing_current_cloud`, `cli_reconstructed_project_loads_by_content_and_can_be_edited_and_undone`, `startup_import_refuses_unspecified_ply_units_until_conversion_is_chosen`, `renamed_project_preserves_history_and_corrupt_import_preserves_current_document`, `unexpected_worker_termination_recovers_document_and_does_not_stay_busy`, `failed_and_cancelled_proposals_preserve_geometry_and_redo`, `selected_normal_flip_preserves_selection_and_survives_history_save_reopen`, `sphere_measurement_preserves_document_selection_and_redo_history`, `circle_measurement_reports_diameter_without_editing_or_consuming_history`, `saved_measurement_worker_marks_dirty_and_project_reopen_retains_source_evidence`, `measurement_management_worker_is_reversible_and_leaves_geometry_selection_intact`, `edits_and_undo_redo_return_correct_full_resolution_previews`, `cancellation_during_filtering_returns_owned_original_document`, `save_roundtrip_is_atomic_and_never_overwrites_existing_data`, `background_load_restores_editable_history_and_failed_import_keeps_document`, `failed_save_retains_a_preexisting_partial_file`, `preview_limits_rendering_but_keeps_unsampled_bounds_and_indices`, `screen_selection_edits_unsampled_points_and_undo_restores_original_attributes`, `picking_preserves_full_resolution_indices_and_toggles`, `sequence_job_prepares_preview_before_publication_and_preserves_document_on_failure`, `recorded_sequence_job_commits_view_after_publication_and_preserves_failures` |
| [crates/scancraft-app/src/cloud_ui.rs](../../crates/scancraft-app/src/cloud_ui.rs) | `open_cylinder_startup`, `open_saved_frame`, `open_frame_startup`, `open_measurements_startup`, `open_normals_startup`, `export_bundle_startup`, `verify_sources_startup`, `statistical_startup`, `request_open`, `open_startup`, `recorded_sequence_startup`, `sequence_startup`, `reconstruct_startup`, `recording_reconstruction_startup`, `tick`, `save_project`, `is_dirty`, `is_busy`, `cancel`, `ui`, `draw` | `expanded_source_and_measurements_leave_viewport_inside_default_window`, `bundle_export_startup_loads_then_exports_without_replacing_active_origin`, `source_verification_startup_waits_for_load_and_keeps_dirty_state_on_later_checks`, `normal_preview_normalizes_only_display_copies_and_skips_zero_vectors`, `normal_arrows_are_bounded_projected_and_independent_of_large_origin`, `reconstruction_and_import_wait_for_unsaved_document_choice`, `point_picking_returns_frontmost_original_index`, `extreme_finite_coordinates_render_without_overflow`, `completed_live_scan_import_keeps_unsaved_document_until_explicit_decision` |
| [crates/scancraft-app/src/discovery_process.rs](../../crates/scancraft-app/src/discovery_process.rs) | `start`, `cancel`, `poll`, `helper_entry` | `polling_and_cancelling_do_not_wait_for_probe_completion`, `failed_probe_keeps_pending_guard_until_actual_child_exit`, `teardown_may_stop_waiting_without_claiming_child_exit`, `invalid_request_never_probes_devices`, `cancelled_discovery_does_not_spawn` |
| [crates/scancraft-app/src/live_fixture.rs](../../crates/scancraft-app/src/live_fixture.rs) | `run`, `run_limited` | see integration tests |
| [crates/scancraft-app/src/live_process.rs](../../crates/scancraft-app/src/live_process.rs) | `run_native`, `helper_entry` | `pause_resume_and_stop_are_distinct_from_eof_abort`, `graceful_stop_can_finalize_beyond_the_abort_grace`, `abort_stops_waiting_for_a_stalled_helper_without_killing_it`, `invalid_or_oversized_preview_is_omitted_without_cancelling_scan_or_counters`, `valid_preview_reaches_the_parent_with_geometry_and_thumbnails_intact`, `actual_display_pipe_failure_requests_abort_and_cleanup`, `summary_stays_small_and_never_attests_hardware` |
| [crates/scancraft-app/src/live_ui.rs](../../crates/scancraft-app/src/live_ui.rs) | `test_job`, `is_busy`, `request_stop`, `abort`, `synthetic_startup`, `tick`, `ui` | `completion_refreshes_counters_and_final_preview_after_poll_boundary` |
| [crates/scancraft-app/src/main.rs](../../crates/scancraft-app/src/main.rs) | — | `recorded_sequence_startup_requires_all_explicit_files`, `statistical_preview_parameters_are_explicit_and_validated`, `scan_project_export_requires_project_sources_both_recordings_and_output`, `full_recording_archive_requires_both_sources_and_new_directory`, `bundle_export_requires_project_source_and_new_output`, `source_verification_requires_explicit_project_and_directory`, `recorded_reconstruction_requires_explicit_numeric_indices`, `recording_playback_startup_is_explicit_and_requires_one_path`, `opening_files_is_distinct_from_explicit_hardware_smoke_mode`, `capture_completion_rechecks_busy_project_before_root_close_command`, `capture_error_cancels_pending_close_so_diagnostic_remains_visible`, `ordinary_close_requests_stop_and_waits_without_abandoning_worker`, `capture_panic_restores_restart_state_and_preserves_partial_recording`, `ordinary_capture_error_stops_and_remains_visible`, `live_capture_close_waits_then_rechecks_other_work`, `pending_close_abort_is_explicit_and_never_bypasses_cleanup_or_document_checks`, `synthetic_startup_requires_explicit_output_and_does_not_select_camera_smoke`, `completion_frame_failure_cancels_a_simultaneous_root_close` |
| [crates/scancraft-app/src/measurement_view.rs](../../crates/scancraft-app/src/measurement_view.rs) | `draw` | `cylinder_outline_preserves_radius_sample_span_and_local_offsets`, `coincident_labels_are_separated_and_stay_inside_viewport`, `corner_labels_are_clamped_and_placement_is_deterministic`, `crowded_or_invalid_view_omits_labels_instead_of_overlapping`, `circle_ring_preserves_radius_and_plane`, `sphere_has_three_closed_great_circles` |
| [crates/scancraft-app/src/mesh_ui.rs](../../crates/scancraft-app/src/mesh_ui.rs) | `is_busy`, `open_external_import`, `open_smooth_preview`, `open_simplify_preview`, `open_with_section`, `open_with_normals`, `open_with_boundary_analysis`, `open_plane_preview`, `open`, `tick`, `ui` | `external_import_requires_units_preserves_source_and_opens_editable_project`, `smoothing_preview_apply_saved_undo_and_redo_preserve_original`, `simplification_preview_apply_saved_undo_and_redo_preserve_original`, `gui_section_snapshot_save_reopen_and_corrupt_open_preserve_source`, `section_worker_is_read_only_and_history_changes_invalidate_analysis`, `section_startup_loads_cube_and_computes_full_closed_contour`, `fan_split_preview_apply_and_history_are_reversible_with_unchanged_face_positions`, `pinched_vertex_diagnostic_emits_marker_without_changing_geometry`, `orientation_preview_normals_apply_and_undo_preserve_exact_source`, `selected_fill_preview_apply_and_history_restore_exact_source`, `boundary_worker_highlight_and_version_invalidation_preserve_geometry`, `mesh_view_emits_shaded_triangle_shapes_without_changing_geometry`, `failed_background_load_preserves_previous_mesh_and_cancellation_rejects_preview`, `gui_trim_preview_apply_undo_redo_and_failed_apply_preserve_exact_versions`, `saved_version_history_is_bounded_and_source_change_rejects_preview`, `full_gui_history_survives_save_reopen_missing_sources_and_further_trim`, `cleanup_preview_apply_and_undo_preserve_original_geometry_and_history`, `component_analysis_explicit_small_part_preview_apply_and_history_are_reversible` |
| [crates/scancraft-app/src/playback_ui.rs](../../crates/scancraft-app/src/playback_ui.rs) | `is_busy`, `cancel`, `start_path`, `tick`, `ui` | `worker_loads_navigates_cancels_and_clears_stale_image_on_failure` |
| [crates/scancraft-app/src/recovery_ui.rs](../../crates/scancraft-app/src/recovery_ui.rs) | `is_busy`, `cancel`, `start_paths`, `tick`, `ui` | `gui_worker_recovers_replays_and_refuses_overwrite_without_source_changes` |
| [crates/scancraft-app/src/screen_tools.rs](../../crates/scancraft-app/src/screen_tools.rs) | `active`, `cancel`, `toolbar`, `interact` | `rectangle_mouse_gesture_preserves_press_and_release_coordinates`, `polygon_clicks_and_enter_apply_without_duplicate_vertices`, `brush_click_is_a_single_disc`, `freehand_and_brush_drags_include_release_position`, `escape_and_viewport_resize_cancel_gestures`, `short_lasso_is_controlled` |
| [crates/scancraft-calibration/src/epipolar.rs](../../crates/scancraft-calibration/src/epipolar.rs) | `raw_field_matrices`, `from_calibration`, `sampson_distance_pixels` | `matches_six_synthetic_reference_cases_without_transposing`, `projected_correspondences_and_pixel_outliers_obey_constraint`, `rejects_invalid_and_degenerate_inputs` |
| [crates/scancraft-calibration/src/factory.rs](../../crates/scancraft-calibration/src/factory.rs) | `verify`, `inspect` | `second_envelope_is_verified_but_not_misindexed_as_laser`, `authenticates_declared_bytes_and_indexes_without_optical_inference`, `rejects_every_truncation_and_single_byte_corruption`, `valid_crc_does_not_bypass_structural_bounds` |
| [crates/scancraft-calibration/src/factory_brown.rs](../../crates/scancraft-calibration/src/factory_brown.rs) | `forward`, `brown5_if_supported` | `forward_and_supported_brown5_match_isolated_vendor_samples`, `guards_nonfinite_poles_overflow_and_lossy_reduction` |
| [crates/scancraft-calibration/src/factory_laser.rs](../../crates/scancraft-calibration/src/factory_laser.rs) | `compare_epipolar_fields`, `inspect` | `synthetic_reference_copy_layout_preserves_all_decoded_fields`, `rejects_truncation_corruption_and_semantically_invalid_fields` |
| [crates/scancraft-calibration/src/factory_named.rs](../../crates/scancraft-calibration/src/factory_named.rs) | `inspect` | `decodes_named_lens_values_without_creating_stereo_calibration`, `refuses_unknown_versions_models_labels_and_nonfinite_lenses_even_with_crc`, `pose_fields_preserve_file_order_and_signed_zero_without_interpreting_angles`, `rejects_all_truncations_and_ignores_opaque_pointer_bytes` |
| [crates/scancraft-calibration/src/factory_pose.rs](../../crates/scancraft-calibration/src/factory_pose.rs) | `matrix_raw_units`, `relative_matrix_raw_units` | `matches_isolated_numerical_prefix_for_identity_axes_mixed_angles_and_translation`, `ordered_relative_composition_matches_isolated_reference`, `relative_composition_rejects_nonfinite_fields_in_either_pose`, `rejects_nonfinite_inputs_without_assigning_physical_units`, `ordered_relative_transform_satisfies_observed_rectification_relations` |
| [crates/scancraft-calibration/src/laser_surface.rs](../../crates/scancraft-calibration/src/laser_surface.rs) | `intersect` | `agrees_with_seven_synthetic_gpu_reference_cases_in_bounded_domain`, `linear_tangent_two_root_and_empty_cases_preserve_ambiguity`, `stable_small_root_survives_large_root_and_validates_numeric_inputs` |
| [crates/scancraft-calibration/src/lib.rs](../../crates/scancraft-calibration/src/lib.rs) | `project`, `ray`, `prepare`, `validate`, `contains_pixel`, `project`, `project_validated`, `ray`, `ray_validated`, `camera` | `analytic_jacobian_matches_finite_differences`, `distorted_projection_and_ray_roundtrip_with_skew`, `zero_distortion_is_analytic_pinhole`, `nonzero_skew_has_independent_analytic_pixel_value`, `reject_folding_nonfinite_and_out_of_domain`, `cubic_domain_check_finds_interior_minimum` |
| [crates/scancraft-calibration/src/normal_blocks.rs](../../crates/scancraft-calibration/src/normal_blocks.rs) | `scaled_gradient_inf`, `solve_step`, `information_min_eigenvalue` | `schur_matches_independent_full_dense_system_for_both_consumers` |
| [crates/scancraft-calibration/src/orientation.rs](../../crates/scancraft-calibration/src/orientation.rs) | `reflect_pixel_x`, `reflect_camera_x`, `reflect_stereo_basis_x` | `distorted_projection_and_inverse_rays_commute_with_reflection`, `pixel_endpoints_and_invalid_domains`, `stereo_pose_projection_and_triangulation_reflect_together` |
| [crates/scancraft-calibration/src/planar.rs](../../crates/scancraft-calibration/src/planar.rs) | `initialize_planar`, `validate_view`, `homography`, `recover_pose` | `recovers_asymmetric_intrinsics_skew_and_metric_poses`, `supports_minimum_three_views_and_four_points_without_thin_svd_loss`, `noisy_observations_report_residuals_and_strict_limit_rejects`, `rejects_repeated_orientation_collinear_and_duplicate_targets`, `metric_scale_and_target_origin_do_not_change_intrinsics`, `validates_bounds_nonfinite_and_resource_limits` |
| [crates/scancraft-calibration/src/projection_differential.rs](../../crates/scancraft-calibration/src/projection_differential.rs) | `project`, `point_pose_jacobian`, `step_pose` | see integration tests |
| [crates/scancraft-calibration/src/rectification.rs](../../crates/scancraft-calibration/src/rectification.rs) | `build`, `source`, `geometry`, `left_from_rectified`, `right_from_rectified`, `point_in_source_left`, `source_pixels`, `rectify_pixels` | `identity_and_vertical_and_reversed_baselines_preserve_epipolar_geometry`, `rotated_asymmetric_distorted_cameras_reconstruct_in_original_axes`, `bad_grids_nonfinite_poses_and_degenerate_baselines_are_rejected`, `unsupported_observations_and_nonfinite_points_fail` |
| [crates/scancraft-calibration/src/refinement.rs](../../crates/scancraft-calibration/src/refinement.rs) | `refine_planar` | `recovers_brown5_intrinsics_and_poses_from_pinhole_seed`, `metric_scaling_preserves_refined_intrinsics_and_distortion`, `analytic_camera_and_pose_jacobians_match_central_differences`, `noisy_fit_improves_and_final_individual_error_limit_is_enforced`, `cancellation_and_nonconvergence_preserve_input`, `rejects_invalid_options_poses_and_insufficient_degrees_of_freedom`, `final_information_rejects_unobservable_repeated_pinhole_views` |
| [crates/scancraft-calibration/src/stereo.rs](../../crates/scancraft-calibration/src/stereo.rs) | `matrix`, `validate`, `apply`, `validate`, `validate`, `prepare`, `triangulate`, `rig`, `limits` | `exact_grid_reconstruction_uses_real_extrinsic_units`, `rotated_asymmetric_camera_pair`, `reject_parallel_behind_bad_match_and_bad_rotation`, `overflowed_or_nonfinite_rotations_are_rejected`, `depth_and_reprojection_limits_are_enforced_separately`, `rectified_disparity_depth_matches_analytic_formula` |
| [crates/scancraft-calibration/src/stereo_fit.rs](../../crates/scancraft-calibration/src/stereo_fit.rs) | `fit_stereo_extrinsics` | `recovers_asymmetric_distorted_skewed_rig_and_target_poses`, `analytic_stereo_rig_and_target_jacobians_match_finite_differences`, `noisy_joint_fit_improves_and_rejects_bad_final_residual`, `metric_scaling_and_camera_swap_preserve_pose_conventions`, `cancellation_nonconvergence_and_invalid_input_do_not_mutate_cameras`, `rejects_collinear_duplicate_bad_pairing_domain_and_zero_baseline` |
| [crates/scancraft-calibration/src/storage.rs](../../crates/scancraft-calibration/src/storage.rs) | `validate`, `validate_for_capture`, `write`, `read` | `exact_roundtrip_and_every_truncation_or_byte_corruption_rejected`, `synthetic_and_mismatched_device_bindings_cannot_pass_capture_checks`, `unsupported_units_and_invalid_geometry_fail_before_writing` |
| [crates/scancraft-cli/src/extract_frame.rs](../../crates/scancraft-cli/src/extract_frame.rs) | `run` | see integration tests |
| [crates/scancraft-cli/src/factory_dump.rs](../../crates/scancraft-cli/src/factory_dump.rs) | `run` | see integration tests |
| [crates/scancraft-cli/src/image_export.rs](../../crates/scancraft-cli/src/image_export.rs) | `publish_png` | see integration tests |
| [crates/scancraft-cli/src/laser_diagnostics.rs](../../crates/scancraft-cli/src/laser_diagnostics.rs) | `run` | `diagnostic_retains_lineage_and_rejects_late_corruption`, `local_diagnostic_labels_contrast_and_preserves_original_frame_crc` |
| [crates/scancraft-cli/src/live_scan.rs](../../crates/scancraft-cli/src/live_scan.rs) | `run` | see integration tests |
| [crates/scancraft-cli/src/reconstruct_stereo.rs](../../crates/scancraft-cli/src/reconstruct_stereo.rs) | `run` | see integration tests |
| [crates/scancraft-cli/src/recording_scr.rs](../../crates/scancraft-cli/src/recording_scr.rs) | `inspect` | `recording_crc_and_conflict_diagnostics` |
| [crates/scancraft-cli/src/stereo_probe.rs](../../crates/scancraft-cli/src/stereo_probe.rs) | `run`, `record`, `record_gain`, `record_flood`, `record_line` | `linear_association_matches_brute_force_with_wrap_and_overlap`, `accepted_rate_uses_host_span_and_not_device_sequence_gaps`, `unsupported_capture_rates_and_unbounded_counts_fail_validation`, `empty_or_nonoverlapping_associations_are_controlled` |
| [crates/scancraft-cli/src/termination.rs](../../crates/scancraft-cli/src/termination.rs) | `check`, `install` | `isolated_signal_child`, `both_signals_cancel_nested_scope_is_rejected_and_previous_handlers_are_restored` |
| [crates/scancraft-core/src/lib.rs](../../crates/scancraft-core/src/lib.rs) | `uvc_pts`, `validate_frame` | `jpeg_padding_and_corruption`, `malformed_metadata_never_panics`, `extended_header_length_uses_copied_standard_fields`, `genuine_record_layout_pts` |
| [crates/scancraft-depth/src/lib.rs](../../crates/scancraft-depth/src/lib.rs) | `cloud`, `validate_source`, `reconstruct` | `mixed_matches_preserve_indices_quality_colors_and_export_roundtrip`, `source_binding_mismatch_and_synthetic_observation_are_errors`, `cancellation_and_empty_input_never_return_partial_clouds`, `all_invalid_is_an_explicit_empty_result_with_all_reasons` |
| [crates/scancraft-depth/src/matching.rs](../../crates/scancraft-depth/src/matching.rs) | `pixel_match`, `select_seeds`, `match_rectified` | `refinement_extra_patch_reads_are_included_in_work_budget`, `automatic_seeds_are_deterministic_masked_bounded_and_cancellable`, `shifted_affine_intensity_images_reconstruct_analytic_plane`, `repeated_texture_and_low_texture_are_rejected`, `masks_borders_and_insufficient_candidates_preserve_original_seed_indices`, `independently_unrelated_images_fail_score_threshold`, `reverse_match_does_not_accept_an_occluded_forward_winner`, `reverse_ambiguity_and_only_one_candidate_do_not_pass_uniqueness`, `wrong_geometry_size_invalid_config_duplicates_and_cancellation_fail`, `opposite_baseline_and_negative_disparity_are_supported`, `supported_lens_disk_is_checked_for_entire_patch` |
| [crates/scancraft-depth/src/matching/subpixel.rs](../../crates/scancraft-depth/src/matching/subpixel.rs) | `validate`, `offset` | `analytic_quadratic_peak_sign_curvature_and_invalid_shapes` |
| [crates/scancraft-depth/src/pipeline.rs](../../crates/scancraft-depth/src/pipeline.rs) | `build`, `rectification`, `process`, `process_automatic` | `rotated_camera_images_match_and_triangulate_with_original_calibration`, `ambiguous_images_retain_rejections_without_inventing_vertices`, `pipeline_enforces_source_binding_dimensions_and_cancellation` |
| [crates/scancraft-device/src/capture_observations.rs](../../crates/scancraft-device/src/capture_observations.rs) | `observe` | `failed_get_queries_remain_unknown_on_a_non_device_descriptor`, `attributes_are_bounded_utf8_and_empty_values_are_not_identities` |
| [crates/scancraft-device/src/cleanup_errors.rs](../../crates/scancraft-device/src/cleanup_errors.rs) | `record`, `finish` | `retains_every_failed_phase_and_first_kind_across_successes`, `nested_stream_failures_preserve_both_nodes_and_inner_phases`, `empty_and_successful_cleanup_have_no_failure` |
| [crates/scancraft-device/src/control.rs](../../crates/scancraft-device/src/control.rs) | `parse_response`, `base`, `parse`, `parse`, `protocol_version`, `factory_header`, `factory_region` | `factory_reads_are_fixed_opcode_aligned_and_region_bounded`, `factory_header_preserves_opaque_fields_without_inventing_optics`, `rejects_bad_envelopes`, `valid_envelope_and_mismatches`, `descriptor_bounds` |
| [crates/scancraft-device/src/control/flood_trial.rs](../../crates/scancraft-device/src/control/flood_trial.rs) | `enable`, `verify_enabled`, `state`, `restore` | `only_flood_boolean_can_be_encoded_and_inconsistent_ranges_fail` |
| [crates/scancraft-device/src/control/illumination.rs](../../crates/scancraft-device/src/control/illumination.rs) | `parse`, `consistent_integer`, `snapshot` | `only_fixed_get_property_packets_are_constructible`, `inconsistent_success_payload_is_not_a_usable_setting_range`, `property_words_keep_exact_signed_order_and_reject_wrong_lengths` |
| [crates/scancraft-device/src/control/line_trial.rs](../../crates/scancraft-device/src/control/line_trial.rs) | `enable`, `verify_enabled`, `state`, `restore` | `only_line_boolean_can_be_encoded_and_inconsistent_ranges_fail` |
| [crates/scancraft-device/src/control/mirror_status.rs](../../crates/scancraft-device/src/control/mirror_status.rs) | `snapshot` | `fixed_snapshot_has_only_get_envelopes`, `malformed_boolean_ranges_never_become_mirror_claims` |
| [crates/scancraft-device/src/ir_gain.rs](../../crates/scancraft-device/src/ir_gain.rs) | `read`, `apply`, `changes`, `restore` | `ambiguous_second_write_failure_restores_both_controls`, `restore_failure_does_not_skip_other_camera_or_claim_success`, `gain_trial_honors_raw_range_steps_and_unavailable_flags` |
| [crates/scancraft-device/src/lib.rs](../../crates/scancraft-device/src/lib.rs) | `discover`, `open`, `open_recording`, `recording_observations`, `next_frame`, `next_frame_until`, `close`, `acquire` | see integration tests |
| [crates/scancraft-device/src/rejection.rs](../../crates/scancraft-device/src/rejection.rs) | `new`, `retain` | `startup_saturation_cannot_hide_later_rejections_and_both_samples_stay_bounded`, `flag_propagation_and_short_image_do_not_imply_bad_pts_structure`, `device_uvc_error_is_distinct_from_v4l2_flags`, `missing_truncated_and_inconsistent_metadata_are_separate_indicators`, `expected_lengths_respect_format_and_compressed_unknown_size` |
| [crates/scancraft-device/src/scr_observations.rs](../../crates/scancraft-device/src/scr_observations.rs) | `observe` | `known_unknown_and_override_tags_with_optional_pts`, `missing_unknown_and_conflicting_records_are_not_collapsed`, `extended_declared_length_does_not_consume_next_record`, `truncated_headers_and_flagged_errors_are_rejected` |
| [crates/scancraft-device/src/standard_controls.rs](../../crates/scancraft-device/src/standard_controls.rs) | `snapshot`, `read_controls` | `standard_control_ioctl_layouts_match_linux_uapi`, `exposure_units_are_interpreted_only_for_positive_absolute_exposure` |
| [crates/scancraft-device/src/stream_group.rs](../../crates/scancraft-device/src/stream_group.rs) | `capture`, `restore` | `group_restores_all_formats_before_any_rate_then_verifies_every_entry`, `every_entry_is_attempted_and_readback_failures_are_not_hidden`, `invalid_groups_fail_before_discovery_or_device_changes` |
| [crates/scancraft-device/src/stream_state.rs](../../crates/scancraft-device/src/stream_state.rs) | `read`, `from_fd` | see integration tests |
| [crates/scancraft-device/src/wait_policy.rs](../../crates/scancraft-device/src/wait_policy.rs) | `dequeue` | `stop_and_expired_deadline_prevent_another_read`, `remaining_deadline_caps_the_supplied_wait`, `stopping_during_timeout_preserves_real_errors_and_successful_data` |
| [crates/scancraft-geometry/src/coordinate_frame.rs](../../crates/scancraft-geometry/src/coordinate_frame.rs) | `at_origin`, `from_points`, `validate`, `apply` | see integration tests |
| [crates/scancraft-geometry/src/cylinder.rs](../../crates/scancraft-geometry/src/cylinder.rs) | `fit` | `analytic_tangent_jacobian_matches_independent_central_differences`, `nonlinear_refinement_recovers_tilted_axis_center_and_radius` |
| [crates/scancraft-geometry/src/filtering.rs](../../crates/scancraft-geometry/src/filtering.rs) | `remove_duplicates`, `remove_duplicates_with`, `radius_filter`, `radius_filter_with` | `labeled_grid_and_outliers`, `compare_against_brute_force`, `duplicates_and_signed_zero` |
| [crates/scancraft-geometry/src/formats.rs](../../crates/scancraft-geometry/src/formats.rs) | `read_ply`, `save_project`, `load_project` | `ply_exact_roundtrip`, `corrupt_and_oversized_ply_rejected`, `unknown_units_require_choice`, `project_roundtrip_restores_undo`, `project_preserves_difficult_f64_bits_and_undo_history`, `project_corruption_and_all_truncations_rejected`, `invalid_project_rejected` |
| [crates/scancraft-geometry/src/fusion.rs](../../crates/scancraft-geometry/src/fusion.rs) | `new`, `len`, `is_empty`, `last_frame`, `integrate_tracked`, `integrate`, `cloud`, `preview_cloud` | `weighted_centroids_attributes_and_signed_voxel_order_are_analytic`, `normals_rotate_without_translation_and_invalid_frames_are_atomic`, `tracked_sequence_fuses_to_world_and_never_fuses_lost_frames`, `display_sampling_bounds_allocation_and_preserves_complete_fusion` |
| [crates/scancraft-geometry/src/lib.rs](../../crates/scancraft-geometry/src/lib.rs) | `validate`, `validate_with`, `bounds`, `select_box`, `select_box_with`, `trimmed_with`, `voxel_downsample`, `voxel_downsample_with`, `validate_history`, `new`, `copied_with_source_directory`, `recordings`, `copied_with_recordings`, `origin`, `attach_origin`, `sequence`, `attach_sequence`, `live_session`, `attach_live_session`, `measurements`, `add_measurement`, `has_measurement_history`, `can_undo_measurement`, `can_redo_measurement`, `rename_measurement`, `remove_measurement`, `undo_measurement`, `redo_measurement`, `replace`, `trim`, `undo_cloud`, `redo_cloud`, `undo`, `redo`, `distance_mm`, `write_ply` | `exact_trim_undo_100_cycles`, `keep_and_invalid_selection`, `inclusive_box_boundaries`, `deterministic_voxels_negative_coordinates`, `reject_nonfinite_geometry`, `measurement_units`, `ply_preserves_properties`, `bounded_history_and_redo_invalidation` |
| [crates/scancraft-geometry/src/live_references.rs](../../crates/scancraft-geometry/src/live_references.rs) | `validate` | `rejects_incomplete_unsafe_duplicate_and_unfounded_qualified_lineage`, `version8_retains_lineage_through_edits_and_reopen_without_opening_sources` |
| [crates/scancraft-geometry/src/measurement.rs](../../crates/scancraft-geometry/src/measurement.rs) | `angle_degrees`, `fit_circle`, `fit_sphere` | `known_angles_and_arm_exchange`, `independent_scales_subnormals_and_overflowing_differences`, `invalid_coordinates_and_zero_arms_are_rejected` |
| [crates/scancraft-geometry/src/measurement_history.rs](../../crates/scancraft-geometry/src/measurement_history.rs) | `apply`, `validate` | see integration tests |
| [crates/scancraft-geometry/src/measurement_snapshot.rs](../../crates/scancraft-geometry/src/measurement_snapshot.rs) | `cylinder`, `point_plane`, `distance`, `angle`, `sphere`, `circle`, `validate` | see integration tests |
| [crates/scancraft-geometry/src/mesh.rs](../../crates/scancraft-geometry/src/mesh.rs) | `triangle_normal`, `validate`, `topology`, `write_stl`, `write_obj`, `from_grid` | `grid_preserves_holes_rejects_discontinuities_and_has_expected_orientation`, `topology_reports_boundaries_nonmanifold_winding_duplicates_and_isolated_vertices`, `exporters_preserve_f64_coordinates_and_indexing_under_independent_text_parse`, `invalid_geometry_and_cancellation_fail_without_exported_mesh_bytes` |
| [crates/scancraft-geometry/src/mesh_boundaries.rs](../../crates/scancraft-geometry/src/mesh_boundaries.rs) | `analyze` | `large_vertex_branch_stays_ambiguous_and_perimeter_overflow_fails`, `distinguishes_outer_and_inner_contours_without_calling_either_a_hole`, `closed_mesh_has_no_boundary_and_removed_face_has_oriented_contour`, `touching_contours_and_bad_winding_are_ambiguous_not_forced_into_loops` |
| [crates/scancraft-geometry/src/mesh_components.rs](../../crates/scancraft-geometry/src/mesh_components.rs) | `analyze`, `select` | `shared_edges_connect_but_vertex_touch_and_geometric_proximity_do_not`, `keep_remove_exact_faces_compaction_and_undo_do_not_drop_small_unselected_parts`, `invalid_selection_and_cancellation_return_no_partial_result` |
| [crates/scancraft-geometry/src/mesh_edit.rs](../../crates/scancraft-geometry/src/mesh_edit.rs) | `trim_plane`, `validate_history`, `new`, `from_history`, `into_history`, `mesh`, `previous_mesh`, `history_depth`, `replace`, `undo`, `redo`, `cleanup` | `cleanup_preserves_first_winding_coordinates_and_exact_undo`, `cleanup_rejects_empty_operation_invalid_source_and_cancelled_work`, `planar_cut_splits_shared_edge_once_preserves_area_and_leaves_boundary_open`, `oblique_cut_of_closed_tetrahedron_produces_shared_open_boundary_without_caps`, `on_plane_vertices_empty_results_and_invalid_planes_are_explicit`, `undo_redo_restore_exact_mesh_across_one_hundred_cycles_and_invalid_proposals` |
| [crates/scancraft-geometry/src/mesh_fill.rs](../../crates/scancraft-geometry/src/mesh_fill.rs) | `fill` | `selected_inner_contour_fills_without_moving_vertices_and_outer_overlap_fails`, `concave_prism_cap_and_tetrahedron_missing_face_have_consistent_winding`, `invalid_tolerance_nonplanar_contour_crossing_face_and_cancellation_fail`, `segment_interior_predicate_handles_vertex_crossing_and_shared_edges` |
| [crates/scancraft-geometry/src/mesh_import.rs](../../crates/scancraft-geometry/src/mesh_import.rs) | `from_labels`, `decode` | see integration tests |
| [crates/scancraft-geometry/src/mesh_normals.rs](../../crates/scancraft-geometry/src/mesh_normals.rs) | `estimate` | see integration tests |
| [crates/scancraft-geometry/src/mesh_orientation.rs](../../crates/scancraft-geometry/src/mesh_orientation.rs) | `orient` | `repairs_parity_preserving_seed_coordinates_and_is_idempotent`, `explicit_components_match_first_face_connectivity_and_reject_invalid_selections`, `mobius_strip_is_refused_instead_of_partially_oriented` |
| [crates/scancraft-geometry/src/mesh_project.rs](../../crates/scancraft-geometry/src/mesh_project.rs) | `save`, `load` | `exact_roundtrip_restores_current_undo_and_redo_and_f64_bits`, `corruption_all_truncations_trailing_bytes_and_oversize_are_controlled`, `crc_valid_bad_history_schema_units_and_cancellation_are_rejected` |
| [crates/scancraft-geometry/src/mesh_section.rs](../../crates/scancraft-geometry/src/mesh_section.rs) | `intersect` | see integration tests |
| [crates/scancraft-geometry/src/mesh_simplify.rs](../../crates/scancraft-geometry/src/mesh_simplify.rs) | `validate`, `simplify` | see integration tests |
| [crates/scancraft-geometry/src/mesh_smooth.rs](../../crates/scancraft-geometry/src/mesh_smooth.rs) | `smooth` | see integration tests |
| [crates/scancraft-geometry/src/mesh_split_fans.rs](../../crates/scancraft-geometry/src/mesh_split_fans.rs) | `split` | `pinched_shells_split_indices_with_bit_exact_face_positions_and_no_gap`, `only_selected_vertex_is_split_and_invalid_selection_or_cancellation_fails`, `open_bowtie_becomes_two_boundary_fans_without_changing_faces`, `vertex_budget_and_nonmanifold_edges_refuse_without_source_mutation`, `splitting_pinched_component_allows_separate_explicit_ring_fill` |
| [crates/scancraft-geometry/src/mesh_vertex_links.rs](../../crates/scancraft-geometry/src/mesh_vertex_links.rs) | `new`, `join`, `fan_counts`, `corner_roots` | `closed_edge_test_does_not_hide_disconnected_vertex_links`, `fill_refuses_a_valid_boundary_when_source_has_a_separate_pinched_vertex`, `boundary_bowtie_is_flagged_and_split_indices_keep_shells_separate`, `manifold_patch_and_empty_or_isolated_vertices_have_explicit_classification`, `nonmanifold_edge_flags_both_endpoints_and_duplicate_faces_block_candidate` |
| [crates/scancraft-geometry/src/normal_edit.rs](../../crates/scancraft-geometry/src/normal_edit.rs) | `flip_selected` | see integration tests |
| [crates/scancraft-geometry/src/plane_measurement.rs](../../crates/scancraft-geometry/src/plane_measurement.rs) | `measure` | see integration tests |
| [crates/scancraft-geometry/src/point_plane.rs](../../crates/scancraft-geometry/src/point_plane.rs) | `validate`, `step`, `step_with_loss`, `icp`, `icp_with_loss` | see integration tests |
| [crates/scancraft-geometry/src/project_origin.rs](../../crates/scancraft-geometry/src/project_origin.rs) | `validate` | see integration tests |
| [crates/scancraft-geometry/src/recording_references.rs](../../crates/scancraft-geometry/src/recording_references.rs) | `validate` | see integration tests |
| [crates/scancraft-geometry/src/registration.rs](../../crates/scancraft-geometry/src/registration.rs) | `validate`, `identity`, `apply`, `compose`, `rigid_fit`, `fit_plane`, `icp`, `icp_with` | `finite_rotation_entries_cannot_bypass_validation_through_nan_arithmetic`, `kabsch_known_rotation_and_translation`, `rejects_collinear`, `fits_plane_exactly`, `icp_small_translation`, `bad_overlap_rejected` |
| [crates/scancraft-geometry/src/selection.rs](../../crates/scancraft-geometry/src/selection.rs) | `new`, `project`, `project_offset`, `in_viewport`, `select_screen`, `combine_selection` | `offset_projection_matches_world_position_and_preserves_small_offsets`, `million_labeled_points_match_analytic_rectangle`, `polygon_boundaries_concavity_and_even_odd_crossing`, `brush_is_union_of_capsules_with_explicit_tolerance`, `visibility_and_tie_rules_are_deterministic`, `selection_modes_and_invalid_indices`, `invalid_shapes_projection_and_cancellation_are_controlled` |
| [crates/scancraft-geometry/src/sequence_references.rs](../../crates/scancraft-geometry/src/sequence_references.rs) | `validate` | `metadata_rejects_unsafe_incomplete_duplicate_and_qualified_references`, `version7_retains_references_through_history_without_external_io_and_rejects_downgrades` |
| [crates/scancraft-geometry/src/session.rs](../../crates/scancraft-geometry/src/session.rs) | `new`, `new_with_method`, `id`, `state`, `tracking_state`, `history`, `last_accepted`, `last_integrated`, `preview_cloud`, `cloud`, `start`, `pause`, `resume`, `stop`, `process` | `fusion_failure_rolls_back_tracking_reference_history_and_frame_consumption`, `lifecycle_loss_recovery_and_history_limit_are_explicit`, `invalid_sampling_and_cancelled_processing_never_publish_state` |
| [crates/scancraft-geometry/src/spatial.rs](../../crates/scancraft-geometry/src/spatial.rs) | `build`, `nearest`, `k_nearest` | `coincident_neighbors_are_exact_without_visiting_the_whole_tree`, `mixed_coincident_groups_keep_brute_force_order_for_all_query_types`, `k_queries_match_brute_force_with_duplicates_ties_exclusion_and_guards`, `exact_queries_match_independent_brute_force_including_duplicates_and_ties`, `empty_large_coordinates_invalid_inputs_and_cancellation_are_controlled` |
| [crates/scancraft-geometry/src/statistical_filter.rs](../../crates/scancraft-geometry/src/statistical_filter.rs) | `analyze`, `filter` | see integration tests |
| [crates/scancraft-geometry/src/tracking.rs](../../crates/scancraft-geometry/src/tracking.rs) | `requires_normals`, `new`, `method`, `new_with_method`, `state`, `last_accepted`, `reset`, `track`, `track_with_normals` | `trajectory_composes_camera_to_world_and_recovers_without_reanchoring`, `quality_rejections_preserve_anchor_and_never_expose_a_new_pose`, `cancellation_bad_inputs_and_frame_order_leave_state_unchanged` |
| [crates/scancraft-geometry/src/work.rs](../../crates/scancraft-geometry/src/work.rs) | `cancel`, `check`, `update`, `progress` | see integration tests |
| [crates/scancraft-image/src/corners.rs](../../crates/scancraft-image/src/corners.rs) | `refine_corner` | `recovers_shifted_corners_under_polarity_and_subpixel_offsets`, `flat_and_single_edges_are_rejected`, `borders_invalid_options_and_nonfinite_seeds_fail`, `displacement_cancellation_and_nonconvergence_are_explicit` |
| [crates/scancraft-image/src/diagnostics.rs](../../crates/scancraft-image/src/diagnostics.rs) | `statistics`, `contrast_lut` | `quantiles_match_sorted_nearest_rank_and_count_exact_clipping`, `display_lut_is_monotonic_and_leaves_source_unchanged` |
| [crates/scancraft-image/src/laser.rs](../../crates/scancraft-image/src/laser.rs) | `detect_rows`, `detect_rows_with_opening` | `monotone_queue_matches_independent_window_extrema_at_every_boundary`, `local_contrast_preserves_raw_clipping_and_recovers_stripe_over_pedestal`, `local_opening_handles_gradient_and_keeps_cancellation_and_limits`, `analytic_subpixel_centroid_multiple_stripes_and_source_preservation`, `rejects_borders_width_clipping_and_weak_runs_without_merging`, `cancellation_invalid_limits_and_overflow_never_return_partial_results` |
| [crates/scancraft-image/src/lib.rs](../../crates/scancraft-image/src/lib.rs) | `new`, `from_frame`, `size`, `reflect_x_into`, `new`, `size`, `intensity`, `valid`, `sample`, `build`, `size`, `coordinates`, `apply` | `masked_image_constructor_rejects_malformed_data_and_keeps_invalid_samples_absent`, `identity_preserves_every_pixel_including_last_row_and_column`, `identity_with_nonbinary_intrinsics_keeps_edge_pixels`, `bilinear_ramp_shift_and_invalid_border_are_analytic`, `bilinear_singleton_and_invalid_coordinates`, `invalid_models_rotation_and_oversize_fail_before_mapping`, `coverage_mask_and_turned_away_rays_do_not_become_black_observations`, `distorted_skewed_map_agrees_with_inverse_camera_rays`, `source_rotation_direction_is_checked_by_known_ninety_degree_rotation`, `cancellation_and_wrong_source_size_fail_without_partial_output`, `raw_frame_adapter_keeps_core_validation_and_borrows_payload` |
| [crates/scancraft-image/src/stereo.rs](../../crates/scancraft-image/src/stereo.rs) | `build`, `rectification`, `apply`, `apply_sequential`, `coordinate_storage_bytes` | `large_distorted_rotated_pair_matches_sequential_bits_and_masks`, `small_pair_keeps_sequential_contract`, `large_pair_rejects_wrong_source_and_cancel_without_returning_partial_images` |
| [crates/scancraft-recording/src/indexed.rs](../../crates/scancraft-recording/src/indexed.rs) | `build`, `source`, `profile`, `context`, `version`, `len`, `is_empty`, `timings`, `read_frame` | `existing_reader_rejects_crc_valid_missing_or_malformed_pts_before_index_caching`, `out_of_order_navigation_reads_only_selected_record`, `index_rejects_bad_suffix_even_when_first_frame_valid`, `budgets_and_cancellation_are_errors_not_partial_indexes`, `rejects_changed_frame_even_with_recomputed_crc` |
| [crates/scancraft-recording/src/lib.rs](../../crates/scancraft-recording/src/lib.rs) | `add_capture_observations`, `new`, `with_context`, `push`, `finish`, `new`, `read_frame` | `header_corruption_rejected`, `duplicate_frame_rejected`, `roundtrip_100_replays`, `every_truncation_rejected`, `corrupted_payload_rejected`, `oversized_record_rejected`, `invalid_profiles_fail_before_writer_output_and_before_empty_replay`, `supported_profiles_and_pixel_limit_accept_empty_recordings_without_allocating_images` |
| [crates/scancraft-recording/src/recovery.rs](../../crates/scancraft-recording/src/recovery.rs) | `recover_prefix` | see integration tests |
| [crates/scancraft-recording/src/selection.rs](../../crates/scancraft-recording/src/selection.rs) | `read_selected` | `selected_frame_retains_raw_metadata_context_and_zero_based_index`, `selected_early_frame_does_not_hide_later_corruption_or_missing_footer` |
| [crates/scancraft-usb/src/main.rs](../../crates/scancraft-usb/src/main.rs) | — | `complete_frame_and_truncated_frame`, `fid_change_rejects_partial_and_recovers`, `malformed_headers_are_errors` |
| [crates/scancraft-workflow/src/calibration_bundle.rs](../../crates/scancraft-workflow/src/calibration_bundle.rs) | `check`, `ensure_new`, `publish_fit` | see integration tests |
| [crates/scancraft-workflow/src/frame_document.rs](../../crates/scancraft-workflow/src/frame_document.rs) | `new`, `validate`, `save`, `load` | `frame_roundtrip_preserves_exact_parameters_and_no_overwrite`, `crc_header_length_and_trailing_corruptions_fail`, `crc_valid_invalid_units_mapping_rotation_and_labels_are_rejected` |
| [crates/scancraft-workflow/src/lib.rs](../../crates/scancraft-workflow/src/lib.rs) | `create_editor`, `cloud`, `report`, `prepare`, `publish_prepared` | `cancellation_after_preparation_never_publishes_an_output_bundle`, `input_limits_and_special_files_fail_without_blocking`, `artifact_writer_stops_accepting_bytes_after_cancellation`, `png_validation_rejects_color_mismatched_truncated_and_huge_headers`, `atomic_publication_refuses_a_target_that_appears_after_staging` |
| [crates/scancraft-workflow/src/live_stereo.rs](../../crates/scancraft-workflow/src/live_stereo.rs) | `index`, `new`, `push`, `remaining`, `open`, `source`, `config`, `profiles`, `input_bytes`, `process`, `set_paused`, `preview_cloud`, `finish` | see integration tests |
| [crates/scancraft-workflow/src/live_stereo/control.rs](../../crates/scancraft-workflow/src/live_stereo/control.rs) | `new`, `set_paused`, `pause_requested`, `request_stop`, `stop_requested`, `phase`, `recorded_frames`, `processing_drops`, `skipped_previews`, `take_preview`, `begin`, `set_phase`, `preview_enabled`, `wants_preview`, `offer`, `snapshot` | `commands_are_single_session_and_mailbox_is_bounded`, `thumbnail_caps_preserve_selected_raw_samples_and_source` |
| [crates/scancraft-workflow/src/live_stereo/control/display.rs](../../crates/scancraft-workflow/src/live_stereo/control/display.rs) | `validate`, `new_remote_display`, `take_display_update`, `apply_display_update` | `roundtrip_import_preserves_parent_commands_and_replaces_only_display_state`, `invalid_update_cannot_partially_change_counters_or_mailbox`, `local_runner_controls_reject_import_and_transport_replacements_are_counted` |
| [crates/scancraft-workflow/src/live_stereo/runner.rs](../../crates/scancraft-workflow/src/live_stereo/runner.rs) | `require_new_target`, `bundle`, `bundle_controlled` | `saturated_processing_queue_keeps_every_raw_frame_and_reports_drops`, `read_and_close_errors_are_both_preserved`, `complete_injected_pipeline_restores_before_publish_and_validates_every_artifact`, `pair_boundary_observes_pause_and_resume_after_the_initial_state_sample`, `worker_failure_closes_both_sources_restores_and_retains_partial`, `coordinator_unwind_joins_worker_instead_of_detaching`, `restoration_failure_and_late_cancellation_never_publish`, `absent_parent_and_dangling_target_fail_before_any_source_open`, `blank_scan_publishes_raw_diagnostics_without_a_fabricated_model`, `complete_raw_replay_rejects_repeated_pts_even_when_processing_delivery_dropped_it`, `source_read_panic_still_attempts_explicit_close` |
| [crates/scancraft-workflow/src/mesh_document.rs](../../crates/scancraft-workflow/src/mesh_document.rs) | `load`, `decode` | `bounded_loader_requires_schema_units_valid_indices_and_respects_cancellation` |
| [crates/scancraft-workflow/src/mesh_history.rs](../../crates/scancraft-workflow/src/mesh_history.rs) | `step`, `load_input`, `save_snapshot` | see integration tests |
| [crates/scancraft-workflow/src/mesh_trim.rs](../../crates/scancraft-workflow/src/mesh_trim.rs) | `mesh`, `original_mesh`, `report`, `import_mesh`, `run`, `prepare`, `prepare_snapshot`, `prepare_smooth`, `prepare_smooth_snapshot`, `prepare_simplify`, `prepare_simplify_snapshot`, `prepare_split_fans`, `prepare_split_fans_snapshot`, `prepare_orientation`, `prepare_orientation_snapshot`, `prepare_fill`, `prepare_fill_snapshot`, `prepare_cleanup`, `prepare_cleanup_snapshot`, `prepare_components`, `prepare_components_snapshot`, `publish_prepared` | see integration tests |
| [crates/scancraft-workflow/src/meshing.rs](../../crates/scancraft-workflow/src/meshing.rs) | `run` | see integration tests |
| [crates/scancraft-workflow/src/output.rs](../../crates/scancraft-workflow/src/output.rs) | `publish`, `staging` | see integration tests |
| [crates/scancraft-workflow/src/planar_calibration.rs](../../crates/scancraft-workflow/src/planar_calibration.rs) | `run` | `publishes_exact_request_and_unqualified_fit_without_overwriting`, `invalid_schema_oversize_and_cancellation_do_not_publish` |
| [crates/scancraft-workflow/src/project_bundle.rs](../../crates/scancraft-workflow/src/project_bundle.rs) | `export`, `export_nested`, `export_file` | `live_export_preserves_histories_and_relocates_only_the_copied_references`, `live_export_cancellation_and_source_mutation_never_publish`, `export_preserves_edited_state_histories_sources_and_original_document`, `existing_output_and_bad_sources_are_refused_before_staging`, `cancellation_after_copy_retains_staging_without_publication_or_original_changes`, `source_change_and_destination_race_cannot_publish_over_existing_data`, `file_entry_rejects_bad_project_and_precancellation_without_creating_output`, `oversized_document_is_rejected_before_staging_or_source_access` |
| [crates/scancraft-workflow/src/recorded_sequence.rs](../../crates/scancraft-workflow/src/recorded_sequence.rs) | `cloud`, `create_editor`, `staging_directory`, `run`, `prepare`, `publish_prepared` | see integration tests |
| [crates/scancraft-workflow/src/recording_archive.rs](../../crates/scancraft-workflow/src/recording_archive.rs) | `archive` | `complete_files_metadata_and_all_frames_are_preserved_without_source_changes`, `legacy_header_is_not_rewritten_or_retroactively_crc_qualified`, `invalid_suffix_budget_wrong_role_and_empty_recordings_never_publish`, `cancellation_after_full_copy_retains_staging_and_preserves_originals`, `source_mutation_and_destination_race_cannot_clobber_or_publish`, `source_symlinks_and_fifos_are_rejected_without_blocking` |
| [crates/scancraft-workflow/src/recording_playback.rs](../../crates/scancraft-workflow/src/recording_playback.rs) | `open`, `profile`, `timings`, `len`, `is_empty`, `check_source`, `version`, `select_raw`, `read` | see integration tests |
| [crates/scancraft-workflow/src/recording_reconstruction.rs](../../crates/scancraft-workflow/src/recording_reconstruction.rs) | `project_origin`, `write_artifacts`, `lineage`, `open`, `check_sources`, `prepare_pair`, `prepare` | see integration tests |
| [crates/scancraft-workflow/src/recording_recovery.rs](../../crates/scancraft-workflow/src/recording_recovery.rs) | `run` | see integration tests |
| [crates/scancraft-workflow/src/recording_timing.rs](../../crates/scancraft-workflow/src/recording_timing.rs) | `inspect` | `nearest_candidates_keep_ties_reuse_and_limits_explicit`, `endpoints_large_timestamps_and_sample_bound_do_not_wrap_or_invent_pairs`, `binary_search_association_matches_independent_exhaustive_distance_reference`, `full_recording_inspection_preserves_inputs_and_rejects_bad_suffix_and_profiles` |
| [crates/scancraft-workflow/src/recording_timing/raw_pts.rs](../../crates/scancraft-workflow/src/recording_timing/raw_pts.rs) | `compare` | `raw_value_matches_are_independent_of_host_nearest_and_limits`, `wrap_and_repeated_values_are_not_assigned_to_arbitrary_epochs`, `sorted_lookup_matches_exhaustive_raw_value_reference_with_collisions`, `absent_matches_and_extreme_host_values_keep_optional_bounds_and_cancel` |
| [crates/scancraft-workflow/src/scan_project.rs](../../crates/scancraft-workflow/src/scan_project.rs) | `associate`, `references`, `export`, `export_file` | `full_recordings_and_edited_project_survive_relocation_without_originals`, `wrong_recording_lineage_or_retained_artifact_cannot_publish`, `changed_retained_reference_missing_origin_and_truncated_source_are_rejected`, `cancellation_and_publication_race_preserve_existing_output` |
| [crates/scancraft-workflow/src/scan_project_load.rs](../../crates/scancraft-workflow/src/scan_project_load.rs) | `load`, `verify_recordings`, `verify_project_recordings` | `relocated_load_checks_every_artifact_and_updates_only_memory`, `corruption_of_unselected_frames_and_nested_manifests_is_rejected`, `saved_edited_projects_keep_complete_recording_identity_and_explicit_verification`, `crc_valid_nested_project_cannot_disagree_with_container_recording_fingerprints`, `unexpected_paths_symlinks_and_late_changes_or_cancel_are_rejected` |
| [crates/scancraft-workflow/src/section_document.rs](../../crates/scancraft-workflow/src/section_document.rs) | `save`, `load` | `snapshot_roundtrip_retains_exact_source_and_recomputes_graph`, `corruption_missing_source_and_trailing_data_are_rejected`, `crc_valid_inconsistent_totals_and_unknown_metadata_are_rejected`, `original_input_normal_survives_resaving_without_normalization_drift` |
| [crates/scancraft-workflow/src/sequence.rs](../../crates/scancraft-workflow/src/sequence.rs) | `cloud`, `run`, `prepare`, `publish_prepared` | see integration tests |
| [crates/scancraft-workflow/src/source_verification.rs](../../crates/scancraft-workflow/src/source_verification.rs) | `verify`, `verify_sequence`, `verify_live`, `verify_project` | `relocated_directory_matches_without_changing_files_or_reference`, `reports_corruption_size_missing_symlink_and_fifo_without_blocking`, `final_check_detects_mutation_of_an_already_hashed_file`, `cancellation_before_or_after_reads_is_fatal_and_symlink_root_is_rejected`, `project_entry_requires_valid_origin_but_does_not_rewrite_project` |
| [crates/scancraft-workflow/src/statistical.rs](../../crates/scancraft-workflow/src/statistical.rs) | `run` | see integration tests |
| [crates/scancraft-workflow/src/stereo_calibration.rs](../../crates/scancraft-workflow/src/stereo_calibration.rs) | `run` | `retains_fixed_models_exact_request_and_existing_output`, `invalid_request_and_cancellation_do_not_publish` |
| [crates/scancraft-workflow/src/stereo_surface.rs](../../crates/scancraft-workflow/src/stereo_surface.rs) | `build` | see integration tests |
| [crates/scancraft-workflow/src/target_corners.rs](../../crates/scancraft-workflow/src/target_corners.rs) | `run` | `publishes_exact_sources_refined_association_and_checked_artifacts`, `rejection_and_cancellation_never_publish_partial_measurements` |

## Recorded mathematical references and external equivalence

`calibration/planar.rs` directly cites
[Zhang MSR-TR-98-71](https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/tr98-71.pdf).
`refinement.rs` identifies Zhang/Brown/Schur methods. `image/corners.rs` identifies
[OpenCV cornerSubPix documentation 4.1.2](https://docs.opencv.org/4.1.2/dd/d1a/group__imgproc__feature.html).
The compare_* scripts and named docs record other OpenCV/SciPy numerical oracles.
Kabsch is named in registration.rs, but the source does not record a copied
Kabsch project/revision. PCL/Open3D filter/ICP names and VCG mesh functions are
comparators only (OPEN_SOURCE_COMPARISON.md), not invented implementation sources.

A source-history audit can recover introduction/fix commits with
`git log --follow -- <file>` and `git blame <revision> -- <file>`. Commit authorship
alone does not identify which AI model produced a line or prove original source
rights. Existing mission/review notes disclose decisions where recorded; unknown
history remains unknown. No history or application source is rewritten here.
