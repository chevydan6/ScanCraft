# Contributing to ScanCraft

ScanCraft is independent and welcomes reproducible fixes and numerical evidence.
Discuss changes to calibration, device controls and reconstruction contracts before
large rewrites. Keep scanner I/O separate from geometry and UI. Submit small changes
with regression coverage for correctness-sensitive behavior; run the locked build,
tests, clippy and formatting checks in docs/BUILD.md.

Contributions are offered under MIT unless their directory explicitly states
another license. Preserve upstream notices and record algorithm sources, exact
revisions, licenses and modifications. Mathematical similarity alone is not code
provenance. Do not submit proprietary code, firmware, vendor shaders, DLLs, private
captures, device serial numbers, credentials or third-party assets without a
redistributable license. Prefer deterministic synthetic test fixtures.

Report measured behavior with input hashes, command, hardware/toolchain, units and
limitations. Distinguish synthetic checks from genuine calibrated hardware results.
Do not advertise incomplete workflows as operational. ArtCraft integration proposals
must preserve standalone builds and must not imply official acceptance.
