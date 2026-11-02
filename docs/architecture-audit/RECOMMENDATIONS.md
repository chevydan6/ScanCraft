Audited source: `ced7697fa0a4e2a262ae255efffa53ca577f117f`. Confirmed/inference/recommendation labels and audit scope are defined in ARCHITECTURE.md.

# Prioritized recommendations

These are **recommendations only**. This audit modifies no production code,
architecture, libraries, firmware, calibration, protections or reviewer findings.
Implementation requires the owner's subsequent direction where the prior audit
mission requires approval. Later development authorization does not justify
silently replacing algorithms during this read-only audit.

## P0: preserve truth, owners and review boundaries

Keep experimental/unqualified flags and physical-calibration deferral. Native
bytes, synthetic meshes and low reprojection RMS do not establish a CrealityScan
replacement. Preserve genuine captures privately, source/review separation,
restoration ownership and the owner's final merge authority. A green/skipped App
check is not a current-head review; independent003/002 remain relevant. Use the
read-only readiness diagnostic as evidence, not a permission boundary. Sources:
README, ACCEPTANCE_RESULTS, AGENTS, DEVELOPMENT_REVIEW, review_readiness.py,
REVIEW_RESPONSES and calibration/storage/depth report flags.

## P1: make one real scan mode correct end to end

When hardware/physical calibration is authorized, resolve camera order/mirror,
active mode, optical model/domain, metric units and simultaneous-exposure evidence.
Keep observed factory fields distinct from a qualified CalibrationRecord. Use
traceable targets and repeated held-out geometry, not merely an internal fit.
Then require a native raw-capture -> point cloud -> tracking/fusion -> trustworthy
surface -> measurements -> export/project reopen path. This addresses the actual
M2/release gap before more general editing breadth. Sources: MISSION, roadmap,
FACTORY_CALIBRATION, calibration/storage, image/laser, depth/pipeline, live_stereo,
ACCEPTANCE_RESULTS and independent006.

This is not permission to calibrate now. While deferred, continue adversarial
numeric/reference tests and documented mode/domain investigation without activating
unverified coefficients or changing firmware/owner settings.

## P1: qualify acquisition and recovery, not just software isolation

Measure reconnect/start reliability, continuous dual-stream accepted/rejected/
missing-frame distributions, optical synchronization, real driver stalls and
settings restoration. Preserve the recorded0.505717% rejection-proxy failure and
post-soak asymmetry. Process isolation now provides a testable ownership boundary;
it does not itself close005 or bound all OS/file I/O. Test stalled setup/read/close,
consumer/pipe failure and restart only after actual cleanup, with private durable
logs. Sources: device Capture/stream_group, acquisition::scan, CAPTURE_PROCESS_ISOLATION,
app supervisors, acceptance and review responses.

## P1: design a verifiable long-session lifecycle

The current 1,000-frame per-camera source, association, replay and live-reference
limits prevent the mission's 30-minute/throughput gates under normal configured
frame rates. Evaluate recording/storage capacity, per-pair file and in-memory
history scaling, full-file verification, interruption/recovery, PTS rollover,
project-reference limits and publication/legacy compatibility together. A lone
larger constant would leave other limits and resource guarantees unresolved.
The exact selection, implementation and any data-format changes require explicit
authorization; the audit makes no such change. Source evidence and arithmetic:
ARCHITECTURE and PERFORMANCE_AND_VALIDATION, live_stereo.rs, runner.rs,
live_references.rs and session.rs. No physical scan or disk benchmark was run.

## P2: surface reconstruction and tracking quality before editing expansion

Current voxel centroids and organized-grid triangles are not a general fused-scan
mesh pipeline. Evaluate a documented surface method on qualified retained clouds;
measure holes, manifoldness, deviation and preservation of small details. Add
labeled tracking-loss/recovery cases and investigate global alignment/pose graphs
only with explicit failure data. Use Open3D/PCL/CCCoreLib/VCG references for missing
capabilities before committing to another large implementation. Preserve working
local ICP/filter/edit methods unless a measured or maintenance benefit justifies
replacement (OPEN_SOURCE_COMPARISON, geometry/fusion/tracking/mesh).

## P2: complete numerical/provenance decision records

Each new algorithm should record its publication/project/revision, independent
versus adapted status, license/notice obligations, parameter units, degeneracies,
reference datasets and measured limits. Existing unnamed history remains unknown;
do not retroactively invent an upstream. Add decision records for C++ adapter vs
Rust implementation where missing high-value capabilities are considered. Sources:
ALGORITHM_PROVENANCE, DESIGN_DECISIONS and current module docs/compare tools.

## P3: profile complete jobs and memory before optimization

Measure acquisition->pair->map->match->triangulate->register->fuse->display/publish
latency distributions, CPU, allocations/RSS and long-run growth. Separate compiler
SIMD/dependency acceleration, presentation GPU and actual compute kernels. Large
maps, history/JSON snapshots, raw copies and IPC are candidates, not proven global
bottlenecks. Retain the measured paired remap benefit without extrapolating full
FPS or asserting external libraries are slower. Sources: BENCHMARKS,
PAIRED_REMAP_CONCURRENCY, image/stereo, formats, capture_ipc/live_process.

## P3: external update workflow and portable distribution

Start with read-only crate/advisory/upstream-change proposals, exact source/license
mapping, differential numeric tests and benchmark fixtures; never auto-apply or
merge semantic math ports. Record actual selected dependency/native-driver/runtime
versions, preserve third-party/font/kernel notices and resolve inactive-platform
notice gaps before distributing those binaries. Validate stock/declared Linux
installation and multi-session/raw/project compatibility. Sources: DEPENDENCIES,
README, Cargo.lock, publication audit/notices and workflow project tests.

## Should editing/filtering take priority now?

**Assessment:** no, not broadly. The source already contains extensive independent
editing/filtering work while the mandatory genuine calibrated reconstruction and
end-to-end qualification remain incomplete. Fix critical/high correctness or
data-loss defects in any subsystem immediately; otherwise prioritize the real
scan path and its evidence. Additional editing can improve the workbench but
cannot compensate for wrong scale, uncertain optics, acquisition failures or an
unqualified mesh. This priority follows the recorded mission/acceptance gap,
not a measured proof that an individual editor feature is useless.
