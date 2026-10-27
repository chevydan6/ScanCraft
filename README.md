# ScanCraft | ArtCraft Ecosystem Candidate

ScanCraft is an independent, open-source 3D scanning application in development,
with native Linux/Rust support for Creality Raptor acquisition and extensible
calibration, reconstruction, point-cloud and mesh workflows.

**ScanCraft is not officially affiliated with, endorsed by, accepted by, or adopted
by ArtCraft.** “ArtCraft Ecosystem Candidate” describes the project's aspiration
for future evaluation, not an official candidate designation. It is independently
maintained and does not require ArtCraft. Creality hardware names identify supported
hardware; this project is not an official Creality product.

**Experimental:** native camera acquisition and offline geometry tools exist;
genuine calibrated Raptor reconstruction and a complete live scanning workflow
are not yet qualified. This is not currently a professional CrealityScan replacement.

## Start here

- [Build and install](docs/BUILD.md)
- [Architecture](ARCHITECTURE.md), [current status](PROJECT_STATUS.md), [roadmap](ROADMAP.md)
- [Compatibility and migration](docs/MIGRATION.md)
- [Offline planar calibration fitting](docs/BROWN_CALIBRATION_REFINEMENT.md) (experimental; supplied observations only)
- [Contribution guide](CONTRIBUTING.md), [security reporting](SECURITY.md)
- [Rebranding verification](docs/publication/VERIFICATION.md)
- [Publication review](docs/publication/PUBLICATION_AUDIT.md)
- [License and dependency notices](THIRD_PARTY_NOTICES.md)
- [Future ecosystem evaluation](docs/ARTCRAFT_READINESS.md)

## Build and run

Requires a Rust toolchain, Linux V4L2 development headers and libclang (for
build-time UAPI bindings), and graphics libraries for the desktop app.

```sh
cargo build --release --workspace
cargo run --release -p scancraft-app
cargo run -p scancraft-cli -- discover
cargo run -p scancraft-cli -- capture 0 left.raptor 100
cargo run -p scancraft-cli -- replay left.raptor
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Close Creality Scan before native capture. The local bridge lock is honored
on the development workstation. Interfaces: 0 left IR, 2 right IR, 4 color.
Captures preserve original image bytes and kernel UVC metadata paired by
sequence. The tested workstation needs its existing Raptor Y14/metadata kernel
patch and device permissions; installation on an unmodified kernel is not
qualified. GUI preview may drop images when its bounded queue is full; the
count is displayed. Recording never silently drops a valid captured frame to
keep up with the UI. Disk I/O backpressure can cause device sequence gaps,
which are counted separately. Recordings use CRC-32 for accidental corruption,
not authentication, and retain incomplete `.partial` files on failure.

## Current scope

Acquisition, image preview, raw recording/replay and diagnostics, plus imported
point-cloud viewing, box trimming, voxel downsampling, undo/redo, PLY interchange
and versioned editable projects. No scanner-generated point clouds yet. Calibration
is deferred at the user's request. No geometric scale or accuracy claims.
The historical Python prototype under `prototypes/` is not production code.
Private hardware captures are excluded from git. Do not distribute vendor
binaries or private scans. Read PROJECT_STATUS.md and ACCEPTANCE_RESULTS.md
for the distinction between implemented features and verified gates.

## Development hardware status

The experimental direct USB diagnostic is separate from the V4L2 application
and is disabled by default after a failed interface-restoration test. It is
not a supported acquisition backend. The scanner recovered after a physical
USB/power reconnection; see PROJECT_STATUS.md. The native application uses
the proven V4L2 path and still rejects occasional incomplete IR frames.

Point-cloud operations run in a background worker with progress and Cancel.
Click a rendered point to select it; Ctrl-click toggles another point. With
two points selected, the workbench displays their distance in imported units
converted to millimeters. Unsaved geometry changes are guarded on import and
close. Rendering samples at most 50,000 points; box selection and edits operate
on the complete cloud. Full viewport latency and interactive workflows are
not qualified yet.

Screen selection tools: choose Rectangle, Polygon, Lasso or Brush. Drag with
the primary mouse button; Polygon uses vertex clicks followed by Enter, a
nearby double-click, or Apply polygon. Escape cancels. Replace/add/subtract/
intersect apply to the current selection. Navigate mode retains individual
point picking. Right-drag orbits between gestures; the view is frozen during
a gesture, and resizing cancels it so the projection cannot silently change.
Frontmost per pixel chooses the nearest point in each logical pixel cell.
Selection uses the full cloud even when preview rendering is sampled.

Camera mathematics are documented in docs/CALIBRATION.md. The Rust library
and test-only OpenCV comparison currently validate synthetic geometry only.
No calibrated Raptor reconstruction is available. Native timing diagnostics:
`cargo run --release -p scancraft-cli -- stereo-probe 100` (close other scanner
sessions first). Reported rejections count V4L2 buffers, which can include
spurious fragments rather than one complete physical exposure each.

The `scancraft-image` library provides GREY image preparation and explicit validity
masks for future calibrated matching; see docs/IMAGE_PREPARATION.md. Current
verification is synthetic and the camera GUI does not apply calibration yet.

Sparse rectified stereo matching is available as a Rust library component, with
synthetic image-to-point-cloud and OpenCV reference tests. It requires explicit
ideal stereo geometry and now runs in the offline reconstruction GUI; live scanning remains unfinished;
see docs/STEREO_MATCHING.md.

The Rust library now connects calibrated stereo rectification, image remapping,
sparse matching and original-camera triangulation. See docs/STEREO_RECTIFICATION.md
for coordinate/provenance contracts and synthetic validation. It is not yet
connected to physical Raptor calibration or live scanning. The offline GUI uses this pipeline.

Run `scancraft-cli reconstruct-stereo CALIBRATION LEFT.PNG RIGHT.PNG REQUEST.JSON
NEW_RESULT_DIRECTORY` for explicit offline calibrated reconstruction. It saves
diagnostics, PLY and an editable project when valid points exist. A reproducible
synthetic example and exact input/output contracts are documented in
[offline reconstruction](docs/OFFLINE_RECONSTRUCTION.md). Actual Raptor calibration
and physical scan qualification remain unfinished.

`scancraft-cli stereo-capture 100 NEW_DIRECTORY` saves both native IR streams
(about 461 MB for 100 frames per camera). `extract-frame FILE INDEX NEW_DIRECTORY`
exports an integrity-checked raw selection and PNG. See docs/STEREO_RECORDING.md.
Current real images are dark and left samples show banding; usable optical data
and exposure synchronization remain unverified.

`scancraft-cli controls` snapshots standard camera exposure/gain/image controls
without changing them. It honors the shared scanner lock; wait for capture to
finish. See docs/STANDARD_CONTROLS.md for raw units and limitations.

Stereo diagnostics/recording accept `--fps FPS` for explicit supported IR rates.
Current rate/optical findings and the uninstalled kernel experiment are tracked
in docs/ACQUISITION_RATE_INVESTIGATION.md; no new scanning mode is qualified.

Open a reconstruction project directly in the workbench:
`scancraft /path/to/result/project.orproj`. This opens geometry without starting
a camera; project type is recognized by content. Unitless PLY requires an explicit
conversion choice. See docs/WORKBENCH_PROJECT_OPEN.md for verified behavior.

The Point clouds tab now offers **Reconstruct from stereo images**, with a
background worker, progress/cancellation and saved diagnostics/PLY/project.
The explicit input-file workflow can also start with `scancraft-app --reconstruct
CALIBRATION LEFT.PNG RIGHT.PNG SETTINGS NEW_RESULT_FOLDER`. See
[GUI reconstruction](docs/GUI_RECONSTRUCTION.md). Current validation is synthetic;
actual Raptor calibration and live scanning remain unfinished.
