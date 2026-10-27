# Roadmap

1. Complete native acquisition: decoding, timestamps, controls, cold-start,
   reconnection, multiple cameras, explicit dropped-frame/error accounting.
2. Complete recording schema for device/settings/calibration metadata; replay
   datasets, corruption handling, recovery tools, reproducible regression runs.
3. Deliver a tested Rust desktop camera workbench with diagnostics and storage.
4. While physical calibration is deferred: independently implement and test
   geometry, filtering, selections, undo, project formats and file interchange.
5. Resume calibrated reconstruction when physical calibration and ground truth
   are available. Establish first genuine point cloud and quantify error.
6. Tracking/fusion, meshing, repair, measurement and complete GUI workflows.
7. Profile, optimize, qualify all mission gates, audit licenses, package and
   prepare publication without proprietary assets or private captures.

No milestone is complete merely because a module or interface exists.
Experimental → Alpha (real point cloud) → Beta (complete workflow) → RC
(qualification) → Stable 1.0 (all approved gates and 10 end-to-end trials).

Calibration projection/triangulation and sparse matched-pixel reconstruction
are implemented with synthetic tests. Next connect genuine validated stereo
recordings to image correspondence extraction; physical calibration remains
deferred and no scanner geometry is qualified.

Reusable undistortion/rotation and masked bilinear GREY resampling now have
synthetic OpenCV comparison and a full-resolution CPU benchmark. Next image
work: correspondence extraction with ambiguity rejection and validated
stereo timing; connect to sparse reconstruction without guessing calibration.

Sparse integer rectified-image matching now has explicit ambiguity/texture and
reverse checks, an image-to-cloud synthetic test and independent OpenCV
comparison. Next compute and propagate genuine calibrated stereo rectification
into this path, then connect validated recordings and GUI controls. Feature
selection, subpixel refinement, robust dense matching and live throughput remain.

Calibrated rectification now connects paired remapping and matching to
original-frame triangulation with source record and index lineage retained.
Next expose the offline pipeline through native tools/GUI, validate actual
recording pairs and calibration when physical calibration resumes, and qualify
image quality, matching/geometry accuracy and performance on real scenes.

The offline native CLI now exposes the calibrated image pipeline, diagnostics
and PLY/editable-project output. Next connect genuine timestamped recording
pairs and calibration to this workflow, then expose it in the GUI with automatic
seed selection, progress/cancellation and real-scene performance validation.
Physical calibration remains deferred at the user's request.

Concurrent native raw recording and whole-file-verified frame selection now
connect sensor data to offline processing inputs. Next resolve dark/banded IR
optical content and interpret the running acquisition soak before claiming mode
qualification. Calibration and verified exposure pairing remain prerequisites
for real scanner reconstruction.

Read-only standard exposure/gain snapshots are implemented. After the current
soak terminates, inspect those settings and the left stream's accepted rate and
rejections, then design controlled reversible optical/exposure/transport tests.
Darkness/banding has no established cause yet.

Controlled isolated/dual rate comparisons and selected image inspection narrow
the intermittent high-rate issue. A device-scoped kernel queue experiment is
compiled and policy-tested, pending privileged runtime testing with rollback.
Do not redefine success around 30 FPS or infer a fixed bandwidth ceiling.

CLI reconstruction projects now open directly in the native workbench, with a
real startup-render check and exact import/edit/undo regression. Next integrate
reconstruction jobs and provenance into GUI workflows while continuing actual
transport, calibration and optical-data validation.

Offline reconstruction is now integrated into the GUI with shared bounded I/O,
progress, cancellation, output publication and unsaved-work guards. Next connect
validated sensor recordings/calibration, automatic feature selection and live
tracking/fusion; retain the unresolved transport/optical and kernel-test work.
Standalone projects still require their bundle/raw inputs for full provenance.

Native camera positioning now has exact raw intensity statistics and optional
display-only IR contrast stretch, with actual rendered-camera and exact raw
snapshot preservation evidence. Continue toward controlled scene positioning,
active-mode/calibration binding, real correspondences and genuine geometry;
display enhancement does not qualify focus, illumination or depth. Native
scoped gain/flood trials restore controls, but the current blurred scene still
prevents optical validation. Physical calibration remains deferred.

Experimental raw-GREY laser stripe centroids now have bounded native extraction
and analytic tests. Next expose diagnostics on verified recordings and validate
real stripe content, orientation, identity and calibration before triangulation.
No native blue-laser scanning mode is qualified by this image primitive.

Recorded-frame stripe diagnostics now retain source integrity and settings and
have run on genuine raw IR data. Next validate controlled laser observations;
zero accepted candidates on an unverified illumination scene does not establish
laser-mode behavior. Continue calibration binding and reconstruction integration.

Automatic textured grid sampling is integrated into the shared CLI/GUI stereo
workflow with exact seed lineage and synthetic plane tests. Next validate
calibration binding and real correspondences, then improve spatial coverage,
subpixel matching and live registration/fusion without weakening quality checks.

Actual factory lens fields now have full integer-grid numerical inverse-domain
checks and an independent aggregate OpenCV reference at an explicit test radius.
Do not promote that radius to physical calibration coverage. Translation units,
transform direction, active-mode binding and real correspondence checks remain
before genuine geometry; physical calibration stays deferred.

The reference's synthetic rectification rotations confirm the numerical role
of the ordered relative pose on five finite cases. Physical sensor assignment
and translation scale still require stronger evidence; retain independent
native degenerate-baseline handling instead of reproducing reference NaNs.

A bounded native frame tracker now has explicit pose acceptance, loss/recovery
and retained-reference semantics with synthetic trajectory tests. Next connect
recorded genuine calibrated cloud sequences, expose tracking state, improve
spatial indexing/robust registration and add fusion. No camera/live mode is
qualified by the offline state-machine tests.

Bounded native voxel fusion now consumes explicit accepted world poses and
exports clouds, with synthetic tracker/fusion and atomic-failure tests. Next
couple tracker acceptance/integration into an explicit session workflow, retain
trajectory and fusion status, and exercise genuine calibrated cloud sequences.
Do not reuse a fusion map after re-anchoring into another session world.

Coupled ScanSession and offline `fuse-sequence` now provide lifecycle,
transactional tracking/fusion and editable exports from explicit cloud inputs.
Next connect validated genuine reconstruction sequences and native GUI session
status, retain resumable trajectory/fusion provenance, and qualify live scanning.

Native GUI sequence fusion now uses shared offline preparation/publication,
background cancellation and unsaved-work protection. Actual synthetic startup
render and byte-identical CLI/GUI outputs are verified. Next integrate genuine
calibrated reconstruction streams, resumable session state and live acquisition;
full interactive scanning and physical qualification remain required.

Exact spatial indexing now accelerates ICP and tracker verification with
brute-force equivalence tests and a scoped CPU microbenchmark. The reconnected
scanner's short capture is clean but scene content remains unsuitable for real
correspondence validation. Continue robust registration and genuine image-mode/
calibration binding; do not infer physical calibration or live qualification.

Organized source-grid triangulation and native STL/OBJ export now have synthetic
geometry/topology/integrity tests and a runnable example. Next connect calibrated
stereo grid lineage, add mesh viewport/project support, then general unordered
surface reconstruction, selective repair and simplification. M5 stays open.

Stereo reconstruction now optionally exports a surface from explicit accepted
seed-grid lineage; CLI/GUI synthetic bundles match exactly. Next validate real
calibration/image binding, add native mesh viewing/editable persistence and
repair, then general unordered surface reconstruction and qualification.

Native Meshes inspection now loads validated mesh JSON off-thread and previews
shading/wireframe with source preservation. Next add a proper depth-buffer
viewport, mesh editing/undo/project persistence and repair, then full interaction,
real geometry and large-data qualification. Current sampled depth-sorted preview
is explicitly experimental.

Native mesh plane clipping and bounded exact undo/redo now work in backend/CLI,
with original-preserving exports and inspected synthetic result. Next connect
interactive preview/apply/undo to the mesh pane, add editable mesh persistence
and selective repair, and qualify real geometry and numerical/physical error.

Interactive plane preview/apply and saved-version mesh undo/redo are now connected
to durable original-preserving publication. Next persist full mesh edit history,
add selection/repair/simplification and a depth-buffer viewport, then qualify
pointer workflows, large models and real calibrated geometry.

Checksummed mesh backend projects now preserve exact current/undo/redo state;
CLI chained trimming and reopened undo/redo are verified. Next hydrate/save the
native GUI's complete saved-version navigation through projects, retain broader
operation/provenance state and continue repair/selection/qualification work.

Full GUI mesh history save/reopen now restores current/undo/redo without original
files and supports further edits from validated snapshots. Next persist broader
tool/provenance state, add mesh selection/repair/simplification and improve the
viewport, while continuing genuine calibration/mode and physical qualification.

Selective duplicate-face/unused-vertex cleanup is connected to reversible GUI/CLI
edits and persisted history. Next add selection and broader repair/simplification
with topology/precision safeguards, qualify interaction/large-data behavior and
continue genuine calibrated acquisition/reconstruction work. No general repair
or watertightness milestone follows from this limited cleanup.

Explicit edge-connected mesh component analysis/keep/remove now supports CLI and
reversible GUI previews, including deliberate retention of small components.
Next add full face selection and broader repair/simplification, improve viewport
and memory behavior, and continue genuine calibrated geometry qualification.

Native fixed line-pattern activation now has real on/off dual-camera evidence
and verified restoration. Next identify projected lines, establish active image/
factory calibration binding and translation units, and reconstruct genuine
validated geometry. Do not equate observed patterns with calibrated blue-laser
scanning; physical calibration remains deferred.

Constrained native mesh simplification and smoothing now support reversible GUI
previews and saved history. Explicit-unit triangle OBJ and ASCII/binary STL import
now preserves raw sources and publishes editable projects. Next extend supported
mesh attributes/formats and selection, compare independent tools and qualify
large-data behavior. Genuine calibrated reconstruction and end-to-end hardware
qualification remain required; calibration stays deferred by user request.

A native dual-IR ten-minute trial now passes duration but fails the rejection
proxy (0.505717% per camera), and a warm reopen exposes a left startup fragment
burst. Next isolate the shared truncation mechanism with controlled rate/transport
comparisons and unchanged validation, then repeat the failed gate after a justified
fix. Keep calibration deferred; this does not replace genuine geometry milestones.

Lower-rate counterbalance does not establish a transport remedy. New rejection
phase counters and independent image/metadata diagnostics now preserve ongoing
failures after startup sample saturation. Next isolate the shared truncation and
startup framing mechanisms; the prepared queue-depth candidate remains runtime
untested. Preserve full validation and qualification targets while advancing the
remaining native reconstruction/editing/release requirements.

Stereo process termination now cooperatively unwinds capture and preserves scoped
cleanup, with two real signal/state-restoration tests. Next extend equivalent
cancellation to separate single-camera CLI paths and qualify interrupted recording
and illumination controls. The USB truncation and deferred calibrated geometry
work remain required; this cleanup fix is not a transport remedy.

Single-camera validate/capture now share cooperative termination, with twelve
hardware interruption cases and three successful post-cancellation recording/
replay checks. Next qualify additional cancellation/publication timings and
illumination restoration, while continuing transport isolation and the remaining
native reconstruction, editing and release requirements.

Interrupted recordings now have GUI/CLI validated-prefix recovery and original-
preserving publication, including genuine IR/color replay/decode evidence. Next
integrate broader recording/session playback and provenance into projects, qualify
large recovery workloads and additional failure timings, and continue transport
isolation and deferred calibrated reconstruction. Stable release gates remain open.

Indexed raw recording access now validates the complete file once and seeks
selected frames with original fingerprint checks. Next wire a read-only file
identity guard, bounded decoder and cancellable background worker into native
GUI frame navigation. Keep recovered-source provenance visible. The index alone
does not detect later modifications to unselected bytes.

Native manual recording image playback is now wired through guarded filesystem
access and bounded decoding into cancellable GUI jobs and indexed navigation.
Next connect saved session/paired-recording provenance and calibrated offline
processing to this view; timed/synchronized replay and actual calibrated scanner
geometry remain unqualified. Calibration remains deferred by user instruction.

Explicit recorded stereo frame indices now feed shared image reconstruction via
CLI and native GUI, retaining selected raw inputs and lineage in published bundles.
Next integrate source/session lineage into editable project persistence and extend
paired replay/diagnostics; resume genuine calibrated reconstruction only when
physical calibration is available. No synthetic result passes M2 or hardware
accuracy/registration/live gates. Calibration remains deferred.

Initial recorded-reconstruction provenance now persists in v5 editable projects
and survives native save/reopen/edit histories. Next implement raw/session bundle
relocation/verification and multi-source lineage; source metadata alone is not a
self-contained raw-scan project. Genuine calibrated geometry, live tracking and
full qualification remain required; calibration stays deferred.

Explicit source-bundle verification now works on user-chosen relocated directories
through CLI and background native UI, preserving documents/references. Next add
persistent verified-reference relocation and self-contained raw/session bundles,
then multi-source lineage. Byte identity alone does not qualify calibration,
synchronization, physical geometry, live scanning or the full mission.

Edited projects now export with their five retained initial frame artifacts through
verified atomic native GUI/CLI workflows. Undo/measurement histories and originals
are preserved, and moved exported folders can be explicitly verified. Next extend
the container to full recording/session data, meshes and multi-source lineage,
then persistent relocation support. This selected-frame copy is not the full raw-
scan/session deliverable. Calibration stays deferred; genuine geometry and all
remaining live/hardware/accuracy/release gates remain required.

Complete supplied left/right recordings now archive byte-exactly through native
GUI/CLI with whole-file validation, original preservation and genuine IR evidence.
Next associate these complete recordings with editable project/session lineage,
recovery provenance and multi-session data. Keep exposure completeness and pairing
unverified until independently established. Calibrated geometry, live tracking
and all remaining release gates still apply; calibration stays deferred.

Recording now retains reported setup settings/descriptors and unknowns; stereo uses
pre-worker group snapshots and verified two-phase restoration. Next associate full
recordings with editable session lineage and extend genuinely bound mode/calibration
metadata when independently established. Do not interpret setup values as per-frame
or physical attestation. Investigate remaining transport loss and continue genuine
calibrated geometry/live/release requirements; calibration stays deferred.

Saved projects now package with both complete supplied recordings and checked
selected-frame membership through native GUI/CLI. Next implement explicit container
load/verification and persistent recording references, recovery report inclusion,
mesh and multi-session provenance, then resumable/live session workflows. Earlier
projects cannot retrospectively attest unselected source bytes. Continue transport
isolation and genuine calibration/geometry qualification; calibration stays deferred.

Fixed scan-folder containers now explicitly verify/reopen in native background
import and CLI, relocating source references in memory and preserving histories.
Next retain richer recording/session references across subsequent saves, embed
recovery provenance, add mesh and multi-session containers and resumable lifecycle.
Full original-capture identity, calibrated geometry, live operation, transport
reliability and all remaining qualification/release gates remain required.

Full-recording references now persist through standalone project edits, saves and
reopening, with explicit native/CLI verification and legacy container upgrade in
memory. Next include recovery provenance and multi-session/mesh state, connect
recorded reconstruction sequences and resumable scanning, and continue transport
isolation and genuine mode/calibration/geometry qualification. Calibration remains
deferred; metadata persistence does not qualify live scanning or physical accuracy.

Optional Huber plane-residual weighting now reaches offline tracking/fusion through
native GUI/CLI settings with independent quality gates unchanged and synthetic
SciPy comparison. Next add stronger correspondence/global alignment strategies,
calibrated real-sequence evaluation, trajectory optimization and live integration.
Do not interpret local robust convergence as verified physical or global alignment.
Continue transport isolation and deferred genuine calibration/geometry requirements.

Supplied-observation calibration now includes pinhole initialization and joint
Brown5/intrinsic/target-pose refinement through an offline diagnostic CLI. Next add
joint stereo fitting, target/subpixel detection, coverage/uncertainty and explicit
device/mode binding. Physical calibration stays deferred. Supplied observations
and small residuals do not qualify Raptor reconstruction or remove hardware trials.

Fixed-intrinsic stereo fitting and its diagnostic CLI now connect known synthetic
lenses/target poses to relative pose estimation and held-out triangulation. Next
advance real source/mode/optical binding, target/subpixel detection and coverage/
uncertainty; simultaneous lens/rig refinement remains distinct future work.
Physical calibration remains deferred; no synthetic fit removes physical trials.

Seeded subpixel target-corner refinement now has an offline image/association CLI
and independent synthetic NumPy/OpenCV comparison. Next connect robust full-board
detection/indexing, multi-view coverage and uncertainty to the existing numerical
calibration workflows. Supplied associations still require verification; no real
calibration, optics binding or M2 reconstruction qualification follows from this
component. Physical calibration stays deferred.
