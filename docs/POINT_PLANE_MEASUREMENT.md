# Point-to-fitted-plane measurement

Expand **Point-to-plane measurement** in the point-cloud workbench. Select one
probe and at least three reference points, set the probe's current source index,
and set the maximum allowed reference-plane RMS in millimeters. The displayed
first selected indices help identify the probe. **Measure point to plane** reports
a read-only result; **Save point-plane measurement** also records historical
evidence in the project and marks it dirty. The probe is excluded from the fit.

The result includes absolute and signed distance, projected foot, unit normal,
reference-plane RMS and reference-point count. The sign follows a numerical
convention: the largest-magnitude normal component is positive. It does not imply
an outward-facing or scanner-facing surface. The RMS limit is a user fit gate,
not an accuracy guarantee. No robust outlier rejection or physical qualification
is claimed; select and inspect an appropriate reference patch first.

`scancraft_geometry::plane_measurement::measure` accepts 4..100001 finite coordinates
(one explicit probe plus 3..100000 plane points), a finite nonnegative RMS limit,
and cancellation control. It centers and scales the reference samples before
PCA, avoiding world-coordinate covariance overflow. It rejects collinear samples
(middle covariance eigenvalue <= largest times 1e-12), ambiguous normals (middle
minus smallest <= largest times 1e-10), nonrepresentable results and RMS failure.
Signed distance and residuals are evaluated in the centered coordinate system.
The geometric output remains in the declared millimeter coordinates. Extreme
finite data with unrepresentable intermediate coordinate differences is rejected.

Snapshot schema 3 adds `point_plane`, storing the fitted plane origin, normal,
foot, distances, RMS, source count, user RMS limit and a probe index into retained
coordinates. Earlier schema 1 sphere/circle and schema 2 direct measurements keep
loading; the project envelope remains version 3/4. Older apps reject this unknown
snapshot kind. Validation checks finite fields, count/index bounds, unit normal,
signed/absolute agreement, RMS gate and probe/normal/foot consistency within a
floating-point rounding allowance. It does not refit or authenticate the reference
samples; stored plane fits have the same structural-evidence limitations as other
fitted snapshots. Project CRC is an integrity check, not authentication.

The existing rename/remove and separate snapshot undo/redo operations apply,
including restoration after reopening. Later cloud edits do not update the
retained evidence. Optional historical outlines draw the probe-to-foot segment
with a distance label. They do not draw an infinite plane or participate in point
selection. An empty current cloud has no viewport projection; its evidence still
appears in the saved-measurement panel.

Evidence: `evidence/point-plane-measurement-{tests,clippy,release}.log`, synthetic
`point-plane-measurement-preview.orproj`, its Rust generator example
`point_plane_measurement_fixture`, and `point-plane-measurement-render/`.
Tests cover exact distance/foot/sign with excluded probe, reordered probe,
tilted planes, 1e-100..1e200 scales, distant origins, noisy reference RMS gates,
collinearity/isotropy, invalid inputs, cancellation, source retention and history
reopening. Worker checks cover source-to-local indexing, read-only state,
saving, cancellation and preserved geometry/selection. Native rendering is a
synthetic startup/reopen check, not full interaction or physical qualification.
