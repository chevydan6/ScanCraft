# Native sphere measurement

The cloud workbench provides **Fit sphere to selected** next to plane fitting.
Select at least four points spanning a sphere in 3D. The background worker reports
center, radius, diameter, geometric RMS and maximum absolute radial residual in
the cloud's declared/imported millimeter coordinates. It does not modify the
cloud, selection, normals, edit history or dirty state. Fit-only results are diagnostic text. Explicit [saved snapshots](MEASUREMENT_SNAPSHOTS.md)
retain fitted values and source coordinates; no parametric CAD is produced.

`measurement::fit_sphere` independently implements centered/scaled algebraic
least squares. For normalized positions u, the design row is `[2u.x,2u.y,2u.z,1]`
and the right-hand side is `u dot u`. Solving for c,d gives radius
`sqrt(d+c dot c)` in normalized coordinates. Translation/scale are restored only
after solving. The separately computed radial residual is `distance(p,center)-r`;
its RMS is geometric, rather than the algebraic system's residual.

A bounded-iteration SVD must have all four singular values above `1e-8` of the
largest. Planar, collinear, repeated or poorly conditioned samples are refused,
as are invalid/nonfinite results and unrepresentable radius/diameter/residuals.
The API reports the design condition number and sample count alongside the fit.
It accepts 4..100,000 finite points and checks cancellation while preparing and
evaluating the bounded system. It returns no partial fit on error/cancellation.

This is a uniform-weight algebraic fit, not robust outlier rejection or nonlinear
geometric-error minimization. A low residual does not prove adequate physical
coverage, a correct object model, calibrated dimensions or scanner accuracy.
Biased/noisy selections and small spherical patches still require independent
assessment. The numeric conditioning gate is not a physical uncertainty bound.
No reference-model comparison, cylinder fitting or real-object qualification is
claimed by this primitive.

Three analytical geometry tests cover known spheres, scales down to 1e-100,
coordinate origins around 1e12, known noisy radial residuals, degenerate and
nonfinite inputs, source retention and cancellation. A worker test checks the
reported diameter and that successful/failed/cancelled measurements preserve
selection, geometry and redo history. Evidence uses the `sphere-fit-` prefix.
