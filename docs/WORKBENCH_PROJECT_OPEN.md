# Opening reconstruction outputs in the native workbench

The app now accepts one geometry file as a startup argument:

```sh
scancraft /path/to/result/project.orproj
# Or run the workspace binary directly:
./target/release/scancraft evidence/offline-cli-verified/project.orproj
./target/release/scancraft --help
```

File startup selects the point-cloud tab and loads through the cancellable
background worker. It does not start camera acquisition. Only the existing
explicit `--smoke-test` mode starts the hardware smoke capture. Use `-- FILE`
for a filename that would otherwise look like an option. Unknown options and
multiple filenames produce a usage error before creating a window.

Import now recognizes the `ORPROJ` content header, rather than relying on a
filename extension. This fixes CLI `.orproj` output being sent to the PLY parser
while the workbench's own saves used `.orproject`. Both extensions and renamed
project files load through the same version/length/CRC/history checks. PLY is
still recognized and validated by its own parser. Current project schema and
save filenames are unchanged.

File-declared units take precedence. Startup imports do not supply an implicit
conversion for PLY that omits units: the existing document is preserved and the
UI reports the import error. Choose the conversion under **If file omits units**
and press **Import** to retry. GUI imports use that visible explicit fallback
choice. Versioned project units remain millimeters and ignore the PLY fallback.

Import failure or cancellation does not replace the current cloud, selection or
undo history. Unsaved-work guards for interactive import/close remain in place.
A loaded reconstruction project can use the existing selection, trimming,
filtering, undo/redo and project/PLY save tools. The standalone project contains
geometry/history; retain its reconstruction bundle/report and raw inputs for
full acquisition provenance. The app now exposes offline stereo reconstruction; live scanner reconstruction remains unfinished.

## Verification

New tests load the actual redistributable CLI project fixture under `.orproj`,
`.orproject` and an unrelated extension, verify millimeter coordinates, delete
one point and restore it exactly through undo. Separate tests verify renamed
project history, corrupt-import preservation, explicit conversion for unitless
PLY and startup argument parsing distinct from hardware capture.

The release binary was also launched on a separate authenticated Xvfb display
with software rendering. The captured real window shows the imported three-point
synthetic reconstruction, correct millimeter bounds and the point-cloud tab.
No scanner device file descriptors were open. The test app/display were stopped
after capture; the user's desktop session was not used or unlocked.

Artifacts: evidence/workbench-project-startup/window.png, capture.json and
inspection.json. This is a startup-render check, not a complete interactive
editing, hardware GPU, performance or real-scanner qualification.

Reproduce with Xvfb, xauth and ImageMagick `import` available:

```sh
python3 tools/check_workbench_render.py evidence/offline-cli-verified/project.orproj NEW_CAPTURE_DIRECTORY
```

The script uses an authenticated private display, opens only the supplied
geometry, checks that the child app has no scanner device descriptors, captures
the root window and cleans up its own child processes. It refuses an existing
capture directory. Inspect the screenshot; process survival alone cannot prove
an import succeeded. The workstation's Xvfb was downloaded and signature-checked,
then extracted in the user cache without a system package installation. The
optional test tool is not a production runtime dependency.
