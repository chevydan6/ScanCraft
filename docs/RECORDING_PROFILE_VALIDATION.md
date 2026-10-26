# Recording profile validation

Reader construction now validates the stream profile immediately after decoding
the header, for both legacy RAPTOR01 and checksummed RAPTOR02. Writer
construction applies the same checks before writing any bytes. Previously these
fields were only exercised indirectly during frame validation; an empty file
could therefore report integrity PASS with an unusable profile.

The accepted profile requires nonzero width, height and FPS, one of the existing
GREY/Y14/MJPG formats, and at most 16,777,216 declared pixels. The product is
computed in u64, including for maximal u32 dimensions, without allocating image
storage. The pixel limit bounds declared decoded size to 64 MiB at four bytes
per pixel. Existing header/frame byte limits remain unchanged. This is an input
resource limit, not proof that downstream algorithms support every accepted
resolution: image-processing modules may have stricter limits. FPS is checked
for nonzero representation, not physical timing validity or device support.

This deliberately rejects previously accepted invalid/oversized profiles without
changing the binary format. Current native Raptor modes fit the limit. Other
future formats/resolutions need an explicit supported-size review. A valid
empty recording remains readable with zero frames; image export still requires
an image. Header CRC correctness alone does not imply semantic validity.

Two Rust tests construct both header versions independently of Writer, including
CRC-valid invalid v2 headers. They cover zero dimensions/rate, unknown format,
maximal u32 products, the pixel-limit boundary, no writer-side output on failure,
and supported empty profiles. A release CLI audit checks twelve synthetic header
cases and successfully replays/decodes the original saved 60-frame left-IR
recording. Evidence: `evidence/recording-profile-cli-audit.json` and
`recording-profile-` test/build logs. This does not qualify capture reliability,
physical synchronization, calibration or reconstruction accuracy.
