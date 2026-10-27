Audited source: `ced7697fa0a4e2a262ae255efffa53ca577f117f`. Confirmed/inference/recommendation labels and audit scope are defined in ARCHITECTURE.md.

# Executive summary

ScanCraft is a substantial experimental Rust Linux scanning workbench, not yet a
qualified professional CrealityScan alternative. The source has12 workspace crates,
338 locked external packages, real structural Raptor acquisition/recording history,
numerical stereo/calibration/registration modules, an extensive editing/mesh GUI,
raw/session/project workflows and supervised native helper processes. The decisive
missing evidence is a correctly bound, calibrated physical Raptor scan through
accurate geometry, reliable tracking/surface reconstruction and repeated complete
export/reopen qualification. See ARCHITECTURE and PERFORMANCE_AND_VALIDATION.

1. **What has Astra actually built?** The audited branch implements the workbench
   and native/data-processing components above. “Astra” is the development role;
   Git contributor identities do not prove which model wrote every line. Existing
   Linux UVC/V4L2 transport is reused with workstation-specific requirements, not
   replaced by an independently qualified universal Raptor kernel driver. Synthetic
   parent/helper scans and offline outputs work within tested scope; M2 and full
   release requirements remain incomplete (README, MISSION, acceptance, crate map).
2. **Where did algorithms originate?** Existing libraries supply linear algebra,
   serialization, codecs, CRC and UI. Several independent local implementations
   explicitly cite Zhang/OpenCV/mathematical methods; factory diagnostics interpret
   observed behavior without a public vendor source upstream. Many standard local
   geometric methods lack a recorded upstream source/revision. Similarity to PCL,
   Open3D or VCG does not prove copying or a port. Missing historical attribution
   is reported as unknown, not invented (ALGORITHM_PROVENANCE, DEPENDENCIES).
3. **Why this architecture?** The native-Rust mission, separation of capture/data/
   GUI ownership, retained original bytes and no-overwrite/cancellation contracts
   are documented. Process isolation addresses observed review lifecycle concerns.
   Per-algorithm Rust-vs-C++/UI/solver alternatives and selection benchmarks are
   largely undocumented; inferred advantages are not retroactive rationale
   (DESIGN_DECISIONS, AGENTS, CAPTURE_PROCESS_ISOLATION).
4. **Which mature components are already used?** nalgebra, serde/serde_json,
   crc32fast, image, eframe/egui/glow/winit, libc/V4L2 bindings and the Linux driver;
   optional nusb is experimental. Python OpenCV/NumPy/SciPy already serve as
   numerical oracles. The five compared C++ geometry libraries are not production
   reconstruction dependencies (manifests, DEPENDENCIES, compare tools).
5. **What is unnecessarily recreated?** Algorithm-family overlap is extensive,
   but unnecessary duplication is not proven. No common integration/maintenance
   study or performance/accuracy comparison justifies wholesale replacement.
   Missing full-board detection, global relocalization and arbitrary-cloud surface
   reconstruction are stronger candidates for evaluated reuse than replacing
   functioning small methods (OPEN_SOURCE_COMPARISON).
6. **What would integration gain/lose?** Potentially mature missing functionality
   and lower algorithm maintenance; costs include FFI/process/data mapping,
   build/packaging, cancellation/ownership, units/provenance and exact license
   obligations. No external speed/accuracy superiority or measurable Rust advantage
   is established. VCG/CCCoreLib/application licenses must be inspected per selected
   file/linkage, not assumed MIT-compatible unchanged (comparison/license sources).
7. **Biggest weaknesses?** Unqualified physical mode/camera/optics/scale/timing;
   failed historical rejection proxy and setup-specific portability; local tracking
   without global recovery; centroid fusion/organized meshing rather than general
   scan-surface reconstruction; absent full performance/memory/end-to-end evidence;
   stale architecture summaries and gaps in historical design/provenance records
   (performance/maturity matrix, README, acceptance, source modules).
8. **What most improves delivery likelihood?** Finish one trustworthy real native
   mode, qualify acquisition/restoration and calibrated geometry with retained
   ground truth, then validate tracking and a surface pipeline through save/reopen.
   Keep correctness/provenance/reference tests and review ownership; evaluate mature
   components for concrete missing capabilities. Broad editing expansion is lower
   priority than these gates. Physical calibration remains deferred, and no audit
   recommendation is implemented without authorization (RECOMMENDATIONS).

## Reproducibility and report locations

The exact audited source is the commit named above; current development may move
independently. Reports are in this directory: ARCHITECTURE.md,
ALGORITHM_PROVENANCE.md, DESIGN_DECISIONS.md, OPEN_SOURCE_COMPARISON.md,
DEPENDENCIES.md, PERFORMANCE_AND_VALIDATION.md and RECOMMENDATIONS.md.
The audit includes a complete144-source-module map,30 major algorithm-family
entries plus public-function/unit-test index, and all338 locked external package
records cross-checked against retained metadata/notices. External comparisons
record observed source revisions and primary-source links; they do not import code.

Reproduce source at an isolated worktree of the audited commit, inspect linked
files/manifests, use the pinned toolchain and a dedicated target directory, run
`tools/check.sh` plus named oracle commands, and compare actual artifacts. This
report references existing validation and does not claim an independent repeat of
all historical tests/private hardware trials or a new legal clearance of all Git
history. No firmware, calibration, source algorithms, reviewer-owned records,
GitHub protections or main-branch state is changed by this audit.
