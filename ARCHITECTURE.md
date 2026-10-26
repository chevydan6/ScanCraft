# Architecture

- `scancraft-core`: sensor profiles, owned raw frames, UVC metadata validation.
- `scancraft-device`: sysfs identity, Linux UAPI bindings, V4L2 mmap streams,
  exact-sequence metadata pairing, bounded startup retries and RAII cleanup.
- `scancraft-recording`: versioned streaming records with integrity checks.
- `scancraft-geometry`: finite millimeter point clouds, exact trimming, bounded edit
  history, voxel centroids, PLY and project formats, rigid registration/plane fit
  and small-cloud reference ICP.
- `scancraft-cli`: repeatable discovery, capture and replay diagnostics.
- `scancraft-app`: Rust egui desktop client. Capture and disk recording run on a
  worker. Preview channel capacity is two frames; full queues drop preview
  images and increment a counter. The disk recording path receives every
  validated frame before preview enqueue. Cancellation is checked per frame;
  frame validation uses an approximately eight-second deadline; kernel setup/control
  I/O is outside that deadline and has no qualified shutdown bound.

All device unsafe code is isolated to V4L2 ABI structs, ioctl, poll, mmap and
munmap; each site documents its lifetime/bounds contract. Kernel buffer
contents are copied only while dequeued, before being requeued. Mapping and
file ownership prevent buffer references from escaping cleanup. Shutdown
stops streams, releases buffers and restores prior format/frame rate.

Recording v2: RAPTOR02 magic, little-endian u32 JSON header length, u32 header CRC and UTF-8
header with profile/context; repeated u32 frame body length, u32 CRC32, binary body (u32 sequence,
u64 host microseconds, u32 flags, u32 metadata length, u32 payload length,
metadata and payload); zero length and u64 frame count footer. Truncated
records or absent footers are errors. Length caps precede allocation. CRC
covers frame header, metadata and raw image bytes. Context has explicit fields for calibration bytes/status, device identity
and control state. Defaults say calibration is not loaded/verified, identity
is absent and controls unknown; acquiring these values is pending. Legacy
v1 recordings remain readable but their session header had no CRC. Host timestamp clock flags are preserved; do not assume wall time.

Future crates separate calibration, image processing, depth, registration,
point clouds, editing, meshes, GPU work and project persistence. Sensor
coordinates, world axes and units will be explicit before reconstruction.

Editable project v2 uses ORPROJ02 magic, u32 body length, u32 CRC32 and a
versioned JSON body holding millimeter geometry and bounded undo/redo history.
Both save and load cap the JSON body at 64 MiB. Save checks the size and hashes
the deterministic serialization before writing, then serializes to a partial
file; a completed file is linked into place only after flush and sync succeed.
This prototype format is not yet optimized for large clouds with long history.

The scancraft-usb crate is a separate, default-disabled experiment. It is not a
backend used by the desktop app. Its attempted hardware test failed to restore
the device; see status and protocol findings before considering more tests.

Geometry GUI jobs own a Document through an Arc/Mutex retained by the job
controller, allowing recovery even if a worker terminates or cannot start.
The UI never locks this document while the worker is running. A capacity-one
completion channel returns status; the full document is recovered after the
worker has joined. Cached rendering data includes original indices, sampled
positions/colors/highlights, and bounds computed from the complete cloud.

WorkControl exposes atomic cancellation and coarse stage/count progress.
Read-only proposals, preview preparation and validation precede the edit
commit point. Save cancellation is honored before hard-link publication; a
cancellation arriving after publication does not undo a completed save.
Ordinary operation errors retain the prior geometry and selection. Unexpected
panic recovery retains ownership but still requires diagnostic review.

Screen selection shares OrthographicProjection between the viewport and
geometry worker. Gesture state snapshots projection and viewport; resize or
Escape cancels the gesture. Rectangle and even-odd polygon/lasso tests include
boundaries; brush strokes are unions of capsules. Through mode tests every
visible-viewport point. Frontmost-per-pixel mode builds a depth map from the
complete cloud and deterministically resolves equal-depth points by original
index. Selection set operations preserve sorted full-resolution indices.

`scancraft-calibration` defines explicit optical-camera coordinates, millimeter
extrinsics, a checked pinhole/Brown-5 model, prepared stereo triangulation and
versioned calibration storage. It is independent of the device, GUI and any
proprietary runtime. Physical capture callers must validate the declared
binding and later establish optical-model and geometric qualification.

The stereo diagnostic opens each V4L2 capture in its own Rust thread, uses a
shared session lock/cancellation flag, and reports bounded metadata samples.
Host-nearest association is linear over monotonically ordered samples and is
limited to the overlapping host-time interval. It is diagnostic, not a source
of reconstruction pairs. Capture now rejects duplicate sensor PTS and stops
on a backwards/reset sensor clock, handling normal wrapping separately.

## Sparse depth stage

`scancraft-depth` reconstructs explicitly supplied calibrated pixel matches into
left-camera millimeter coordinates. Calibration validation and declared source
binding precede processing. Accepted vertices retain source index, color, ray
gap, angle and reprojection residuals; invalid matches retain an indexed reason.
Cancellation returns no partial result, and inputs are capped at one million
matches. Conversion to the editing cloud is explicit; PLY carries positions and
color but not correspondence provenance, so retain the reconstruction report.
This stage does not detect correspondences, verify exposure synchronization,
estimate calibration, or prove physical accuracy.

`scancraft-image` separates image preparation from acquisition and depth. It borrows
validated GREY payloads, precomputes calibrated destination-to-source maps and
returns floating-point images with explicit validity masks. `PreparedCamera`
validates immutable calibration once for repeated projection/ray operations.
See docs/IMAGE_PREPARATION.md for axes, rotation direction and memory bounds.

`scancraft-depth::matching` consumes ideal rectified masked image pairs and bounded
seed lists. It returns correspondence and rejection diagnostics plus the exact
ideal stereo geometry used. Conversion to `PixelMatch` connects directly to
checked sparse triangulation. Pixel coordinates from this stage must not be
interpreted using an original distorted camera record. Automatic stereo
rectification and original-to-rectified provenance propagation are next work.

`scancraft-calibration::rectification` constructs validated source/ideal transforms.
`scancraft-image::stereo::StereoImageMaps` binds paired remaps to that geometry.
`scancraft-depth::pipeline::StereoProcessor` preserves the original calibration
record and runs remap -> match -> source-pixel recovery -> original-camera
triangulation, with explicit index lineage and all-rejected diagnostics. The
serialized report is a diagnostic artifact, not yet a project/recording format.
See docs/STEREO_RECTIFICATION.md for the frame and provenance contract.

The CLI offline reconstruction adapter bounds/validates calibration, PNG and
request inputs, invokes StereoProcessor, and publishes a checksummed artifact
bundle using same-parent staging and Linux no-replace atomic directory rename.
PLY/project contain original-left-frame millimeter coordinates; reports retain
provenance and seed lineage separately. JSON parsing uses float_roundtrip so
project/calibration values survive decimal parsing without avoidable ULP changes.

The CLI dual-capture adapter shares the existing scanner lock, uses one bounded
streaming recording writer per camera thread and joins/cancels workers before
publication. Whole-file replay validates both recordings. A shared output module
centralizes atomic no-replace Linux directory publication. Recording selection
validates the entire stream/footer and retains a single selected frame, rather
than returning early and overlooking corruption later in a source recording.

`scancraft-device::standard_controls` is a read-only fixed-allowlist QUERYCTRL/G_CTRL
backend. It serializes driver metadata and explicit unsupported/error states,
without a generic control setter or vendor transport access. CLI callers hold
the scanner session lock. Hardware snapshots do not retroactively populate the
unknown settings of earlier raw recordings.

Workbench import now reads and replays a small content prefix to dispatch
versioned projects versus ASCII PLY. Worker fallback units are optional: startup
opens use None, while interactive imports carry the UI's explicit choice. Startup
argument parsing occurs before renderer initialization and camera acquisition is
limited to the separate explicit hardware smoke option or user camera actions.

`scancraft-workflow` now owns bounded file input, immutable prepared reconstruction
and cancellable checksummed bundle publication, shared by CLI and app. Shared
Linux staging/rename primitives avoid duplicate unsafe publication code. The
app prepares a new editor/preview, publishes the bundle, then commits document
replacement. No-geometry results retain the current document. Progress stages
separate image mapping, resampling, matching and triangulation; source-provenance
and timing/accuracy flags remain explicit and unqualified.

`scancraft-geometry::session` now owns bounded tracking/fusion state and lifecycle.
Tentative tracker updates publish only after atomic fusion, preventing a failed
integration from changing the pose reference. Shared `scancraft-workflow::sequence`
replays bounded unit-aware PLY sequences through this controller and publishes
pose diagnostics, derived PLY and an editable project in a no-overwrite bundle.
The CLI exposes it as `fuse-sequence`. This is offline cloud processing, not
acquisition/live scanning; original recordings and calibration stay separate.

Cloud-sequence preparation/publication is now split into an immutable prepared
result and atomic output writer. The GUI geometry worker prepares both the
candidate editor and viewport preview before publishing, then commits document
replacement without another cancellation boundary. Its sequence job uses the
same shared implementation as CLI replay and the same unsaved-work guard as
stereo reconstruction/import.

`scancraft-geometry::spatial` provides a borrowed balanced median tree with exact,
deterministically tied nearest queries and cooperative cancellation. Reference
ICP builds it per target; tracking final coverage uses the same query API.
Cloud/iteration caps and explicit quality gates remain in place.

`scancraft-geometry::mesh` adds bounded indexed triangles, rescaled normal validation,
organized-grid meshing, edge-topology diagnostics and native ASCII STL/OBJ export.
`scancraft-workflow::meshing` reads explicit unit/source grids and publishes
no-overwrite result bundles; CLI exposes `mesh-grid`. The point-cloud document
format/view remains separate and does not yet support editable meshes.

`scancraft-workflow::stereo_surface` connects accepted reconstruction points back
to an explicitly configured rectified seed lattice, preserving original-frame
geometry and vertex-to-seed lineage. Optional mesh artifacts publish in the
existing stereo bundle transaction. The GUI reports surface exports while its
current document/view remains point-cloud based.

`scancraft-workflow::mesh_document` validates bounded mesh JSON envelopes.
`scancraft-app::mesh_ui` loads them on an independent cancellable worker, retaining
the old model until validation/render preparation succeeds. The independent
Meshes tab draws a capped orthographic shaded/wireframe preview via egui.
Full loaded topology and sampled drawing are distinct; point-cloud ownership
and unsaved-work guards remain unchanged.

`scancraft-geometry::mesh_edit` implements immutable uncapped plane clipping with
shared source-edge intersections and a bounded exact snapshot editor.
`scancraft-workflow::mesh_trim` exposes it through original-preserving no-overwrite
bundles and CLI `trim-mesh`. The GUI mesh pane can inspect those outputs;
interactive editing and editable mesh history persistence remain separate work.

Mesh plane edits now use immutable shared PreparedTrim objects, built alongside
candidate viewport state on a worker. The UI keeps its current saved Model until
publication succeeds, then records Arc-owned saved versions for exact undo/redo.
Preview/render state is separate from the current model. Saved-version history
is bounded; arbitrary source-file changes are detected before trimming. This
extends the mesh pane without changing point-cloud ownership or close guards.

`scancraft-geometry::mesh_project` persists MeshEditor current/undo/redo using bounded
MSHPRJ01 length/CRC framing and strict full-history validation. Trim bundles now
include a backend project and can continue from one. Shared `mesh_history` workflow
reopens exact undo/redo and publishes a new bundle; GUI saved-version navigation
remains separate until explicit project hydration/snapshot integration is added.

Mesh history workflow now supplies validated history parts and snapshot bundle
publication to the GUI. The mesh worker hydrates all cached version previews
before committing a project load, enforces retained viewport-data bounds, and
saves current/undo/redo from immutable Arc models. Reopened project versions are
self-contained snapshot sources for further trimming, avoiding external file
references. JSON sources retain consistency checks against their loaded geometry.

Mesh edit preparation now supports explicit duplicate/unused cleanup as well as
plane cuts, retaining the same immutable proposal/durable publication contract.
Core cleanup preserves surviving coordinate/order and first-face winding; GUI
controller uses shared generic apply/history paths and operation-specific counts.
CLI exposes `cleanup-mesh` for mesh JSON or checksummed history input.

`scancraft-geometry::mesh_components` uses union/find over shared indexed edges to
produce stable first-face component IDs and summaries. Explicit whole-component
keep/remove enters the shared mesh edit preparation/publication pipeline.
The GUI performs analysis on its worker, virtualizes summary rows and invalidates
analysis on current-model changes; previews/applied versions retain exact history.

Native stereo trial orchestration now distinguishes None/Flood/Line with a typed
choice. Fixed property-151 scoped line control is separate from flood and gain,
with original/readback/restore behavior and reported-only capture context. Real
optical on/off evidence is kept separate from runtime switch-readback claims.

Native acquisition rejection diagnostics retain separate first-overall and
post-first-valid samples, each capped at 32 metadata-only records per camera.
Exact phase counters continue after sample caps; legacy total rejects are
unchanged. Independent V4L2 flags, expected image length and PTS parse indicators
avoid treating a first-failure string as a root cause. See
docs/REJECTION_DIAGNOSTICS.md.

Stereo CLI acquisition uses a scoped SIGINT/SIGTERM guard whose signal callback
only sets an atomic. Workers check at capture boundaries and unwind normally;
prior handlers are restored after the enclosing operation/control leases finish.
GET-only stream-state snapshots provide non-atomic format/rate diagnostics without
starting capture. See docs/STEREO_CANCELLATION.md for the exact tested scope.

Recording recovery is a streaming scancraft-recording prefix operation, wrapped by
scancraft-workflow source-stamp checks, verified artifact replay and atomic bundle
publication. The GUI runs it independently in a cancellable worker. Recovery
provenance remains in the report/manifest beside the new version-2 recording;
original profile/context values are retained and legacy unknowns stay explicit.
Input I/O failure origin is tracked separately from format errors to prevent
cancellation/read failures becoming false repair success. See RECORDING_RECOVERY.

Recording playback access: `scancraft-recording::indexed` builds a bounded index
through the strict Reader, including footer validation, before exposing random
access. It owns the seekable input and stores frame offsets, predecessor order
and CRC fingerprints rather than image payloads. Selected records are rechecked;
source immutability and filesystem identity guards are caller responsibilities.

`scancraft-workflow::recording_playback` wraps the index with read-only regular-file
and identity/timestamp guards plus bounded GREY/Y14/JPEG display decoding.
`scancraft-app::playback_ui` moves the source into a single background indexing/
selection job and receives it with one decoded view. No camera handles or source
writes occur. Cancellation/error clears the old image; idle views do not schedule
redraw. Source changes are checked per operation, not monitored continuously.

`scancraft-workflow::recording_reconstruction` selects explicit left/right GREY
frames through guarded indexed sources, cross-checks declared calibration/context,
and calls the shared image pipeline used by PNG inputs. Prepared results retain
selected raw frames plus exact calibration/request and source sidecar for atomic
bundle publication. The GUI uses the existing geometry worker/document protection.
No hardware/calibration write, automatic synchronization or binding attestation is
introduced; original whole recordings remain referenced external sources.

`scancraft-geometry::project_origin` retains bounded descriptive initial source
metadata separately from reversible geometry/measurement history. Origin-bearing
projects use length/CRC framed ORPROJ05; v2–4 remain unchanged without origins.
`Prepared::create_editor` supplies identical origin data to the native live
proposal and published project, including deterministic retained-artifact hashes.
The geometry layer performs no external-file resolution or calibration activation.
Origins survive edits but do not establish current per-point or merge lineage.

`scancraft-workflow::source_verification` performs explicitly requested, read-only,
bounded streamed CRC/length checks against initial-origin retained artifacts.
Per-file metadata guards and final rechecks distinguish mismatches and changing
sources; cancellation remains fatal to the operation. CLI and the native geometry
worker share it. Loading the project does not invoke it, and verification does not
change the origin directory, history or qualification flags. It verifies artifact
byte references, not cryptographic authenticity or reconstruction semantics.

`scancraft-workflow::project_bundle` exports a new edited-project copy plus the five
initial-origin retained artifacts. It preflights serialized bounds, verifies and
streams sources, creates a source-location-adjusted Editor clone, saves and drops
that clone, reloads/re-serializes the complete saved state, verifies copied inputs,
and atomically publishes without overwrite after final source checks. CLI and
native background geometry jobs share it; active documents are not replaced or
marked saved. Absolute directory references are descriptive; explicit project-folder
selection supports checking moved bundles without automatic file access on load.

Point-cloud controls/status now have bounded vertical scrollers. Progress/cancel
and the viewport sit outside the controls scroller so expanded provenance and
saved measurement panels cannot consume the entire default-window view.

`scancraft-workflow::recording_archive` tees bounded source reads into new artifacts
while the strict Reader validates complete original files. Whole-file byte/CRC
checks and a second saved-file replay precede source identity rechecks and atomic
publication. Original headers are copied rather than normalized. Independent
`ArchiveUi` jobs integrate native progress/cancellation/close handling; CLI/startup
share the workflow. Archives retain full supplied raw files, separately from the
selected-frame project bundle format; no device/exposure pairing is attested.

`Capture::open_recording` adds bounded setup observations before video STREAMON;
plain open remains free of these extra queries. The device layer exposes typed
Serialize data; recording accepts it generically without a hardware dependency.
GUI and CLI producers attach it to existing capture context without replacing trial
metadata. Actual stream/control GETs use the selected video FD; USB descriptors
remain separate non-atomic, unbound observations. Public evidence redacts context.

`stream_group::SettingsLease` snapshots all selected reported settings before any
stereo workers start, then restores all formats before all rates after workers stop
and verifies every entry. Restoration failures propagate and block successful
publication; Drop retries best-effort. The exact kernel/device coupling behind the
observed concurrent restoration mismatch is not claimed to be isolated.

`scancraft-workflow::scan_project` composes strict complete-recording archival and
selected-frame project-bundle export under one parent staging/publish transaction.
It reuses guarded indexed playback and the reconstruction lineage encoder to match
selected source frames, then checks canonical retained-frame fingerprints. Nested
project export receives its final descriptive location separately from its staging
path. The container's fixed relative entries survive folder relocation; project
loading remains explicit. Native ArchiveUi runs this optional saved-document path
on its existing cancellable worker; neither camera acquisition nor the current
editing pane is modified. It does not infer historical whole-file identity or
calibration/synchronization from selected-frame membership.

`scan_project_load` explicitly consumes the fixed scan-folder layout without following
manifest-supplied arbitrary paths. It compares nested manifests, requires exact known
artifact inventories, hashes bounded regular files with cooperative cancellation,
checks complete recording replay and selected-frame membership, and retains open
file/directory identity stamps until return. Its editor relocates the verified source
directory in memory. CloudJobs uses this loader when an explicit import names a
folder; the existing document transaction, unsaved-work guard and preview pipeline
remain in effect. Standalone project import continues to avoid external source reads.
Checks are point-in-time integrity against CRC manifests, with no authenticity,
historical capture identity or physical calibration attestation.

ORPROJ06 adds independent typed `recording_references::Recordings` metadata to Editor.
It requires an initial origin and validates two fixed full-recording entries against
its frame counts/versions/selected indices. Project parsing remains filesystem-free;
older formats retain compatibility and cannot carry v6-only metadata. All editor
clones/edits/histories preserve the references. Scan export derives fingerprints
from verified archive copies; scan load cross-checks or supplies legacy references
and relocates them in memory. Explicit native/CLI verification hashes the full
files, replays and checks initial selected-frame membership with identity guards.
It never retargets references or mutates the active document. Fingerprints describe
supplied recordings, not authenticated historical originals or physical calibration.

Point-to-plane registration now has an explicit quadratic/Huber loss layer. Huber
IRLS weights scale the existing centered SVD system, with a separate exact robust
objective update guard. Backend diagnostics retain raw plane RMS and label mean
rho and downweighted-count separately. Tracker's opt-in Huber method requires
normals, validates its millimeter delta, and retains independent Euclidean RMS,
coverage and motion gates. Session normal handling and offline sequence settings
share that method; default point-to-point and plain point-to-plane remain available.
The independent SciPy nonlinear comparator is optional tooling, not app runtime.


Capture completion recovery: `scancraft-app/src/capture_worker.rs` owns the one-shot
result channel, pending outcome and thread handle. The GUI polls without joining
an unfinished worker; disconnected senders and panics produce explicit terminal
errors, and restart controls return only after the old thread finishes. A received
result alone does not prove cleanup completed. App shutdown still joins owned
capture cleanup synchronously; no driver-stall shutdown bound is established.
Hardware-free regressions exercise panic/disconnection and partial-file retention.
