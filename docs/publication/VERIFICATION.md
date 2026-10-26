# Rebranding verification

- `cargo test --locked --workspace`: 497 passed on the reduced publication tree,
  including existing-format recording/project loads and synthetic reconstruction.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `cargo build --locked --release --workspace`: passed.
- Python tooling unittest discovery: 19 passed using the existing NumPy/OpenCV
  reference environment. System Python initially lacked cv2; no test was waived.
- Private Xvfb launch, three captures: app alive; legacy project opened with three
  points; visually inspected; no camera file descriptors. This is a startup/render
  check, not complete interactive GUI qualification.
- `scancraft-cli discover`: returned `[]`. No scanner was available for live USB,
  acquisition, laser or calibration validation; none is claimed newly verified.
- Native help/executable entry points work. Raw/project format magic is unchanged.

Evidence hashes and scan results are in VALIDATION.json. Raw local build/test logs
and screenshots remain outside the public tree to avoid developer paths. The
commands above are reproducible; hardware qualification still needs actual devices.

Outstanding: physical calibration/mode binding, genuine Raptor 3D geometry,
reliable complete live scanning, long-duration transport/drop-rate and accuracy
qualification. Statistical filtering remains heuristic and has quadratic behavior
for coincident points. Do not treat passing synthetic tests as professional scanning
qualification. Prioritize end-to-end calibrated geometry and transport reliability
before expanding generic editing tools.
