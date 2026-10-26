# Bounded native frame tracking prototype

`scancraft_geometry::tracking::Tracker` accepts caller-selected clouds of 3..512
finite, noncollinear points in millimeters and strictly increasing frame IDs.
It is a CPU prototype, not a qualified live-scanning implementation. Nothing
activates it automatically on the camera stream. It requires genuine calibrated
clouds before it can be used for scanner tracking.

The first valid frame deliberately anchors a new session at identity. Each
subsequent cloud is registered against the last accepted cloud with local
point-to-point ICP, starting at identity. The accepted transform maps current
frame coordinates into reference-frame coordinates. World pose is composed as
`world_from_reference * reference_from_current`. No fusion or global pose
optimization is performed. Reset explicitly discards this session coordinate
anchor; callers must represent that as a new session/world.

Config requires positive finite correspondence distance, residual limit,
bidirectional coverage fraction, maximum relative translation and rotation,
iteration count (2..90), and convergence tolerance. RMS limit cannot exceed
correspondence distance; overlap cannot exceed one. Iterations and 512-point
cloud caps bound nearest-neighbor work to about 24 million distance comparisons
per attempt plus validation/final verification, with bounded memory. The caller
must choose tracking samples; the tracker does not silently discard cloud points.
Nearest-neighbor matching now uses an exact balanced spatial index; the point/iteration caps still bound worst-case work. No live throughput claim follows.

After convergence, correspondences are recomputed at the final pose. Inlier RMS,
fraction of source points with close neighbors, and fraction of distinct target
points covered are checked. Both coverage fractions must pass, preventing a
small perfect subset from masquerading as broad overlap. Excessive relative
motion, nonconvergence and high residuals cause loss. Rejected frames expose no
new pose and retain the last accepted cloud/pose. Later frames can recover
against that same reference; there is no silent identity re-anchoring or fusion
of rejected observations. Relative-motion gates during recovery apply against
the last accepted reference, not wall-clock velocity.

Malformed input, duplicate/out-of-order frame IDs, degeneracy and cancellation
return errors without changing tracker state or consuming the frame ID. Valid
but rejected tracking attempts consume their ID and set state to `Lost`.
`last_accepted()` always identifies the retained reference. `icp_with` adds
cooperative cancellation to the existing reference ICP; the old `icp` API keeps
its behavior using a default work control. Cancellation checks occur during
correspondence loops and before returning a fit; tracker publication checks
again before changing state.

Limitations: local minima and symmetric scenes can pass these gates. A low
residual is not proof of globally correct correspondence or physical accuracy.
There is no prediction, robust trimmed loss, multiscale search, point-to-plane
ICP, relocalization map, confidence-weighted fusion, temporal exposure check,
trajectory persistence, GUI tracking display or acquisition connection yet.
Zero-point geometry must be handled upstream, not treated as a tracking cloud.

Three Rust regressions check a 20-frame rotating/translating synthetic trajectory
against known world poses (translation and rotation-matrix error <1e-10), loss
and recovery without re-anchoring, partial-overlap/motion/residual rejections,
invalid and degenerate clouds, cancellation, frame order and explicit reset.
These are test tolerances, not measured physical accuracy. All 170 workspace
tests, warning-denying Clippy, formatting and release workspace build pass.
Evidence: `evidence/tracking-tests.log` and `evidence/tracking-build.log`.

Next connect this API to recorded calibrated cloud sequences and explicit
tracking status in the native workflow, qualify indexed registration, add robust losses and integrate fusion, then qualify genuine data and live throughput.
