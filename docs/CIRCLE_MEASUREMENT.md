# Native 3D circle measurement

The point-cloud workbench now offers **Fit circle to selected** for at least
three selected points, with an explicit **Circle plane RMS limit (mm)**.
The default 0.05 mm is an editable selection tolerance, not a scanner accuracy
claim. The command reports center, diameter, supporting-plane RMS and geometric
curve RMS in imported coordinates. It is read-only and preserves selection,
geometry and edit/redo history. Fit-only results are diagnostic text; explicit
[saved snapshots](MEASUREMENT_SNAPSHOTS.md) retain values and source coordinates.
Automatic hole recognition remains unfinished.

`measurement::fit_circle(points, maximum_plane_rms_mm, work)` centers and scales
the data, fits a PCA supporting plane, constructs an orthonormal in-plane basis,
then solves the three-column algebraic circle system with bounded SVD. The
design condition number must be below 1e8; collinear/repeated or inadequately
conditioned selections fail. The supporting-plane RMS must satisfy the caller's
finite nonnegative limit. The result includes a deterministic plane normal,
center/radius/diameter, sample count, condition number, geometric RMS and maximum
absolute residual. The normal's sign is a numeric convention, not an outward
surface or physical camera orientation claim.

Geometric residual is distance to the 3D circle curve:
`hypot(in_plane_distance_to_center-radius, signed_plane_distance)`.
Out-of-plane error is therefore retained even when the caller allows a loose
planarity gate. Center and scale are restored only after the normalized solve;
unrepresentable results/residuals fail. Input is limited to 3..100,000 finite
points and computation checks cancellation around bounded numerical solves and
in processing loops. No partial measurement is returned on failure.

This is PCA plus algebraic least squares, not a joint nonlinear geometric fit,
robust estimator, uncertainty bound or proof that the selection describes a
physical hole. Small arcs, noise and outliers can bias a result despite a low
residual. Physical diameter/center accuracy requires calibrated source geometry
and independent ground truth. Cylinder fitting, parametric CAD and full scanner
qualification are not provided by this measurement primitive.

Three analytical tests cover tilted circles, tiny scales/distant origins, an
explicit nonplanarity gate and known 3D residual, degenerate/nonfinite inputs
and cancellation. A worker test verifies reported diameter and unchanged
selection/history with redo still available after success or invalid settings.
Evidence uses the `circle-fit-` prefix. The private-display render verifies
control presence/empty-selection state, not a mouse-driven measurement workflow.
