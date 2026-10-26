# ScanCraft source-publication review

Review date: 2026-10-26. This is an engineering source-distribution review, not
legal certification or physical scanning qualification.

## Scope and history

The publication tree derives from development commit
`8bf67feb1ef30a02e4bb0d61dc5b03d6d4ffc74f`, including the existing uncommitted
statistical-outlier preview/filter. A separate publication directory preserves the
original repository, Git history, private research and unfinished work unchanged.
The public repository starts with a fresh source snapshot: no original branches,
tags, reflogs, commit author addresses or historical blobs are pushed.

The original architecture audit remains in the private development repository;
it describes its pinned historical revision, not the renamed publication tree.
The rebrand changes crate/import/executable/UI names and documentation, not geometry
algorithms or acquisition protocols. Existing format magic/schema bytes are retained.
The one regenerated synthetic source bundle removes absolute developer home paths
using the existing reconstruction command; its manifest/checksums are regenerated.

## Included materials and provenance

- Rust implementation: project-local code declared MIT by the original workspace;
  old copyright notice retained, ScanCraft contributors added. No translated C++
  implementation or proprietary runtime is introduced. Mathematical methods and
  vendor-observation provenance remain documented in subsystem source/docs.
- Dependencies: Cargo.lock plus DEPENDENCIES.json cover 338 external packages,
  including optional/platform-specific packages. Original notice texts were collected
  from installed crates and exact upstream revisions where needed. Manifest license
  declarations are evidence, not an assertion that every transitive source was
  independently authored or individually legally certified.
- Embedded egui fonts: original Hack/emoji, OFL and Ubuntu Font License notices
  collected. No new logo, copied application artwork or vendor UI assets added.
- Linux kernel experiment: retained source patch/test and GPL-2.0-only license,
  separate from the MIT workspace. It is not linked into or automatically installed
  with ScanCraft. Rebranding does not grant MIT rights to Linux code.
- Oracle harnesses: independently written offline research tools referring to
  user-supplied private software. No vendor instructions, DLL, firmware, shader
  source, calibration dump or captured proprietary asset is bundled. They are not
  production dependencies, and cannot be fully rerun from public inputs alone.
- Evidence: only synthetic test inputs and generated test products retained; see
  evidence/README.md. Numerical comparison results do not establish a clean-room
  legal process or physical device accuracy. No unsupported authorship claim is made.

Historical hardware logs, captures, screenshots, build logs, private oracle inputs,
virtual environments, compiled binaries and the original audit archive are excluded.
Private paths were removed; attribution contacts in third-party license notices
are intentionally preserved. Binary fixture files are explicitly allowlisted and
synthetic; they are not vendor executables or firmware. Original hardware evidence
referenced by older documents is not included and is not current release validation.

## License decisions

Keep MIT for the independently maintained application: this preserves the existing
license and permits independent/possible ecosystem reuse without changing architecture.
MIT requires preservation of its copyright/permission notice; see LICENSE and
https://choosealicense.com/licenses/mit/. Dependencies keep their own licenses;
Apache, font and kernel obligations are not erased by the root MIT license.

No native OpenCV/Open3D/PCL/CCCoreLib/VCGlib reconstruction library was added.
Reference Python dependencies are separately installed and not vendored. The Linux
GUI's selected dependency tree excludes dispatch, hexf-parse and both r-efi versions;
their declared licenses and absent upstream standalone notice texts are recorded.
Their source/binaries are not distributed here. Any additional-platform or binary
release requires its own selected-graph license/notice review. See THIRD_PARTY_NOTICES.md.

## Publication gates

The clean source snapshot and its sole Git history are scanned with Gitleaks and a
supplementary path/key/binary inventory. Scan results, test counts and executable
hashes are recorded in VALIDATION.json. A zero finding is evidence from these checks,
not proof that an arbitrary secret/provenance issue could never exist.

Publication requires authenticated GitHub repository creation/push access. The
connected account is known, but authentication in a connector that lacks repository
creation does not provide a local Git transport credential. Never put tokens in
this repository, chat, command-line URLs or reports.
