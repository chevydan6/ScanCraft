# Terminal recording stream errors

After `Writer::push` encounters an output I/O error, that writer rejects later
pushes and `finish` without issuing more output operations. The first failure
retains its original I/O error; subsequent attempts identify an unusable writer.
A partial frame cannot be repaired by appending another frame or a footer.
Previously a caller could catch the error and continue writing at an unknown
byte boundary. Capture/publication callers must retain the failed artifact as
incomplete rather than claim a finished recording.

Frame validation and ordering errors that occur before output starts do not
invalidate the writer. The caller may correct that frame and retry; sequence,
timestamp and frame-count state still describe only successfully written frames.
Construction/finish errors already return no reusable writer. These guarantees
cover operations issued by this recording API; an underlying writer's own
destructor behavior is unchanged.

After any `Reader::read_frame` error, the reader similarly rejects subsequent
reads without accessing the input again. This includes I/O interruption that
escapes `read_exact`, CRC/format/order errors, and missing/corrupt footers.
The synchronous reader has no resumable partial-record state, so a later healthy
underlying stream does not make continuing safe. Reopen the source from a known
start to replay again. Successfully validated EOF remains repeatable `Ok(None)`.
Normal `Interrupted` retries performed internally by `read_exact` are unchanged.

Four integration tests inject recoverable input/output failures at six byte
positions spanning record length, CRC and payload. They assert terminal errors
without additional stream calls; test correctable frame rejection/order state;
and check persistent errors after CRC/footer/truncation failures. Faults are
synthetic and do not qualify disk hardware or acquisition reliability. The
existing genuine 60-frame saved recording still replays/decodes successfully.
Evidence uses the `recording-fail-stop-` prefix.
