# Raptor SCR observations

## Contract and provenance

`scancraft_device::scr_observations::observe` is an independently written passive
parser of Linux UVC metadata. It returns all SCR-bearing records, including raw
six-byte SCR values, record indices and optional numeric vendor light types. It
performs no device operations. Existing frame validation remains unchanged.

Confirmed static protocol research identified this numeric tag table:
`ff:0, a2:1, b1:2, b2:3, b4:4, c7:6, c8:7`. SCR byte 3 overrides byte 0 only
when byte 3 is c7 or c8. Unknown values remain unknown. No human-readable scanning
mode names or physical camera assignments are established by this table.

The Windows metadata correspondence is supported by Microsoft's
[UVC extensions specification, sections 2.2.3.1 and 2.2.3.4.5](https://learn.microsoft.com/en-us/windows-hardware/drivers/stream/uvc-extensions-1-5):
the USB video metadata item has an eight-byte item header, followed by start and
end timestamps. The first timestamp begins with four PTS bytes and six SCR bytes.
Thus item offset 12 is the first payload SCR, matching the six-byte field used by
the investigated SDK. Linux UVC records place optional PTS before optional SCR.
No vendor source, binary, disassembly or private capture is included here.

## Evidence and limitations

Private read-only examination of 100 frames from each of four genuine native
recordings (400 total) verified recording CRCs and found SCR byte 0 a2 throughout.
This supports that tag's occurrence, not the correctness of every table entry,
mode names, reconstruction accuracy or current hardware connectivity. These
private captures are not redistribution fixtures.

Synthetic tests cover all table entries, unknown values, both override values,
PTS present/absent, missing SCR, conflicting records, malformed lengths,
truncation and UVC error flags. Per-record results deliberately preserve
conflicts rather than silently selecting a frame-wide mode. Windows uses the
first payload's SCR; equivalence of arbitrary later payload tags is unqualified.

`scancraft-cli recording-scr RECORDING` streams through the verified recording
footer and emits aggregate per-record numeric-type counts, missing-SCR frame
counts, unknown-tag counts and conflicting-known-type frame counts. It emits no
capture context, raw image or raw metadata bytes. Corruption fails the command
without publishing partial JSON. Memory remains bounded per frame.

The API is not wired into live mode gating. It must
not be used to select calibration or silently discard frames without separate
qualification. Hardware mode activation and physical accuracy remain outstanding.
