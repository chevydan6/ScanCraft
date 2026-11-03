# Experimental live stereo GUI

The Live scan tab uses the same native acquisition, recorded-frame validation,
stereo processing, tracking, fusion and bundle publication path as the CLI.
It requires explicit calibration, reconstruction settings, scan settings and a
new output directory. Physical source binding, exposure synchronization and
Raptor dimensional accuracy remain unqualified. Physical calibration is deferred.

## Controls and data ownership

Start runs the native adapter on a background worker. Pause reconstruction keeps
raw capture running and consumes pair identities without reconstruction or fusion.
Resume processes subsequent pairs; it does not retrospectively reconstruct paused
frames. The configured finite capture limit (maximum 1,000 frames per camera)
continues to apply while paused. Stop and save stops producers, joins owners,
drains processing queues, verifies raw recordings, restores settings and publishes
the bundle. Abort cancels publication and retains incomplete staging for diagnosis.
Existing synchronous driver calls and joins have no guaranteed shutdown deadline
(SCAN-REV-005); responsive controls are not proof of bounded driver shutdown.

A single-slot preview mailbox prevents rendering backlog. Camera thumbnails are
at most 320×240; fused-cloud previews sample at most 50,000 points without first
allocating a full cloud. Skipped display previews are separate from processing
queue drops. Raw frames are appended before queue delivery. Completion replaces
a stale pending snapshot with the final preview. The GUI owns its preview
separately from editable documents. Opening the completed project uses the
existing import/unsaved-change guard, never automatic document replacement.

Window close requests graceful stop and waits for worker completion. Failure
cancels pending close so its diagnostic remains visible. Cooperative cleanup
cannot interrupt an indefinitely blocked kernel call.

## Module boundaries and rationale

`scancraft-acquisition` factors the prior native CLI adapter and device lease
ownership into one local Rust crate so the GUI and CLI share cleanup behavior.
The workflow runner remains device-neutral and accepts injected sources. This is
orchestration reuse, not replacement of reconstruction algorithms. No external
crate version changes are required. GUI `serde_json` uses the existing locked
workspace version, promoted from test-only to runtime use.

`workflow/live_stereo/control.rs` owns atomic control/counters and the bounded
snapshot mailbox. `geometry/fusion.rs::preview_cloud` samples fusion output;
full cloud export retains its existing path. `app/live_ui.rs` polls the reusable
worker and reuses the existing point renderer. CLI previews remain disabled.

## Validation and outstanding qualification

The development gate passed 571 Rust tests, frozen formatting, strict all-target
Clippy and release compilation. Added regressions cover pause/resume, raw retention,
graceful stop versus abort, final snapshot replacement, preview bounds, dirty
project handover and window-close/error ownership. Final publication requires
review of the exact committed changes and a final gate after subsequent edits.

A private authenticated X display ran the actual release GUI with explicit
`--synthetic-live-scan NEW_OUTPUT`. Screenshots showed pause while raw counts rose
from 16/16 to 31/31 with six fused points unchanged, resume, stop/save at 46/46,
and successful handover of the six-point project to the editor. The saved report
contains 46 candidate pairs, 16 paused pairs, zero processing drops and a true
graceful-stop flag. Inspected process descriptors contained no camera/USB device.
These are synthetic interaction results, not hardware, accuracy, throughput or
realistic dense-scan measurements. The diagnostic deliberately paces 100 fixture
frames at 500 ms; nominal fixture timestamps do not measure actual acquisition.

Private test captures and outputs are retained locally, not distributed as scanner
data. A second private-display run sent WM_DELETE_WINDOW during active capture. The
application saved five frames per side with graceful stop requested and exited
zero. This injected-source result does not establish physical shutdown timing.

### Completion ordering regression

Independent finding SCAN-REV-014 identified stale final counters in addition to
the final mailbox race. The GUI now polls/joins before reading counters and
draining the mailbox. A synchronized test releases the actual one-pair fixture
runner at the poll boundary: the prior ordering deterministically displayed
zero recorded frames instead of one per side. The corrected test also checks
the final three-point preview, thumbnails, tracking and completed-project path.
This regression exercises the GUI consumer, not just the runner mailbox.
