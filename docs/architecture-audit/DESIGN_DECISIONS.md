Audited source: `ced7697fa0a4e2a262ae255efffa53ca577f117f`. Confirmed/inference/recommendation labels and audit scope are defined in ARCHITECTURE.md.

# Design decisions and rationale

## Evidence policy

A design visible in code is not evidence of why it was selected. The table
separates recorded intent from inferred tradeoffs. “Unknown” means no selection
rationale/alternative comparison was found in the inspected mission, manifests,
architecture and named subsystem documents; it does not mean none ever existed.
Git authors and generic contributor identities cannot establish which AI model
made an individual decision. The user-provided native-Rust mission is a recorded
requirement, not a measured comparison against C++.

| Decision/problem | Recorded rationale and alternatives | Benefits/limitations and evidence | Assessment |
|---|---|---|---|
| Rust primary implementation | docs/MISSION.txt Rust policy requires native Rust and no mandatory C++ reconstruction runtime for stable release; temporary acceleration is permitted | Typed owned frames/models, shared job contracts and no reconstruction FFI are visible. Superiority to C++ speed/accuracy is unmeasured | Appropriate stated product constraint; not evidence every algorithm should be rewritten |
| Direct Rust geometry rather than established C++ integration | Mission policy is recorded; no per-algorithm build-vs-buy evaluation or complete integration experiment was found | Inference: avoids ABI/exception/allocator/cancellation boundaries, permits explicit finite/unit/provenance checks. Cost: recreates numeric methods, tests, maintenance and missing mature features | Retain working implementations while evaluating missing high-value capabilities with data; no blanket replacement conclusion |
| nalgebra for numerical kernels | Actual dependency/use in calibration, geometry and app; selection versus Eigen/Ceres alternatives is unknown | Uses mature SVD/eigen/matrix routines rather than hand-writing all linear algebra; solver/application correctness remains local responsibility | Existing reuse is substantial; no measured benefit from replacement |
| egui/eframe glow desktop | app/Cargo.toml records selection; mission mentions evaluating Slint/egui, but no recorded comparative evaluation was found | Shared Rust/event UI; CPU drawing/projection plus GPU OpenGL presentation. Full dense GPU point rendering and compute are absent | Serviceable experimental workbench; profiling/usability evidence needed for large-data targets |
| Linux V4L2 default transport | README, ARCHITECTURE, device module and failed USB experiment record existing native acquisition and restoration risk | Reuses kernel streaming/metadata; depends on workstation patch/permissions. nusb backend remains separate/disabled | Preserve known acquisition; qualify portable installation/recovery before transport rewrite |
| Byte-preserving raw recording/replay | Mission repeatability requirements; recording module/header/footer/CRC checks | Enables offline regression and retained context. Copies/disk backpressure, private datasets and unknown physical state remain | Appropriate; require full session metadata and integrity/recovery qualification |
| Declared calibration and false qualification flags | calibration/storage.rs, depth/pipeline.rs, acceptance documents explicitly distinguish validation from physical truth | Prevents invented geometry/binding claims; supplied bad associations/units remain possible | Essential until real mode-specific calibration and ground truth are qualified |
| Bounded asynchronous display | capture session/process/IPC docs and review corrections explain preserving raw recording while dropping optional previews | GUI backpressure/oversize preview handling is bounded; serialization/copies and pipe failures still have costs | Correct separation supported by device-free tests; hardware throughput not proven |
| Process isolation for device ownership | CAPTURE_PROCESS_ISOLATION.md, commits283df43/2ea042b/6cd439e and findings005/018–020 responses document stuck-worker/cleanup concerns | Device/restoration stays in child; parent reports unknown cleanup. Adds IPC/protocol/process/log ownership and surviving-child cases | Better testable boundary, not hard OS/hardware deadline; actual driver-stall qualification pending |
| Local ICP plus voxel centroids | Algorithms/configuration visible in tracking/session/fusion; reason for selecting over TSDF/global registration is unknown | Explicit local quality gates and bounded voxel state; drift, symmetry, lost motion and arbitrary-surface output remain limitations | Useful prototype, incomplete professional scanning/tracking/reconstruction capability |
| Organized-grid mesh generation | mesh::from_grid and workflow/stereo_surface explicitly preserve declared adjacency; reason for excluding Poisson/volumetric options is unknown | Avoids inventing arbitrary-cloud connectivity; does not surface-reconstruct fused unorganized clouds | Keep honest scope; a real scanning mesh pipeline remains a priority |
| Conservative mesh edits | mesh_fill/smooth/simplify and UI document preserving boundary/source movement/normal constraints | Bounded reversible operations; not arbitrary repair, QEM quality or global surface-deviation guarantee | Useful imported-mesh tools, not evidence scanning is qualified |
| JSON/CRC versioned projects and no-overwrite output | formats.rs, workflow output/publication, legacy magic compatibility; mutation/history preservation is recorded | Readability/evolvability/provenance and original preservation. Serialization/history copies can consume memory and 64 MiB project cap limits scale | Appropriate prototype persistence; benchmark large real sessions before format redesign |
| Geometry-owned generic cancellation | work.rs imports establish coupling; historical placement rationale unknown | Reuse is convenient; lower-level image/workflow code depends on broad geometry crate (finding007) | Assess extraction only with a concrete boundary benefit; no automatic architecture rewrite |
| Reference-oracle testing | tools/compare_*, subsystem docs and synthetic fixtures record numeric validation | Catches convention/derivative/degenerate mistakes without making C++ runtime mandatory. Oracle agreement is not hardware truth and can share assumptions | Preserve and expand; independently labeled physical data/ground truth still necessary |
| Isolated build outputs per checkout | DEVELOPMENT_REVIEW.md records observed shared-target stale release artifact incident and recovery | Avoids misleading fresh-build/binary claims; costs additional storage/build time | Apply to all worktrees; actual executable/source binding is part of validation |

## What was actually validated

Local fmt/strict Clippy/tests/release and component/oracle comparisons support the
narrow implemented contracts. Review findings exposed real lifecycle/transport
issues despite tests, so test counts are not architectural validation by
 themselves. The latest recorded current-source isolated gate has652 tests; the
independent reviewer did not independently repeat that whole gate. No decision
has been validated by a complete calibrated Raptor-to-qualified-mesh workflow.
Source: tools/check.sh, PERFORMANCE_AND_VALIDATION.md, ACCEPTANCE_RESULTS.md and
REVIEW_RESPONSES.md (not edited by this audit).

No retrieved record establishes that Rust reconstruction is faster or more
accurate than OpenCV/Open3D/PCL/CCCoreLib/VCGlib. Conversely, their existence does
not establish that replacing functioning code would improve ScanCraft. The
appropriate evaluation is a bounded task, identical data/limits, explicit
licenses and tests for all interfaces, then a reviewed decision record.
