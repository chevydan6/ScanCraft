# Integrity-checked single-image export

`scancraft-cli export-image RECORDING NEW_OUTPUT.PNG` now defers all image
publication until every recording frame and the terminal footer validate.
It decodes the recording as before and retains only its first decoded image
for export. Empty recordings, a corrupt later payload, a bad footer and a
truncated recording fail without creating an output or staging directory.

Previously, the first image was written inside the replay loop. A later CRC
or footer failure could return an error while leaving a plausible PNG. That
artifact was not proof of complete source integrity. Existing historical
images must be re-exported or verified against successful replay when used
as evidence; this change cannot retroactively validate them.

After validation, the encoder writes PNG to a newly created staging
directory beside the destination, syncs the file, then uses the shared Linux
RENAME_NOREPLACE publication primitive. An existing file, directory or dangling
symlink is never replaced, including a target appearing after the initial
check. Encoding/publication failures retain staging with its path in the error.
Successful publication removes the empty staging directory and syncs the
destination parent. Errors after publication explicitly identify that state.
The source recording is unchanged. This exports raw decoded orientation; no
camera reflection, calibration binding or dimensional qualification is inferred.

Three CLI integration tests cover exact pixels/full-recording success/source
preservation, later-payload/footer corruption/truncation with no artifact, and
preservation of existing files and dangling symlinks. Real saved-frame smoke
evidence is `evidence/verified-image-export-replay-check.json`; tests and build
logs use the `verified-image-export-` prefix. Shared no-overwrite publication
remains covered by its existing tests; these tests do not qualify all possible
filesystem failures or long-running acquisition.
