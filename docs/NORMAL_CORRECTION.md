# Explicit selected normal reversal

The cloud workbench now provides **Flip selected normals**. Select points with
the existing point/box/screen tools, inspect their directions with the normal
overlay, and apply the command. It reverses only the selected normal vectors.
Positions, colors, point order, vector magnitudes and unselected attributes are
unchanged. No viewpoint, camera binding or outward orientation is inferred.

The geometry API is `normal_edit::flip_selected`. Empty, duplicate or out-of-range
selections and selected points with missing/zero normals reject the entire
proposal. Nonfinite attributes fail ordinary cloud validation. It does not skip
bad selections, silently normalize stored normals or estimate missing normals.
All selections validate before the source is cloned; cancellation returns no
proposal. Double reversal restores normal bits, including signed zero.

The background worker prepares a preview before committing through the existing
Editor history. Selection remains intact after a successful reversal, making
inspection and repeated correction possible. Undo/redo and saved/reopened
projects retain the corrected attributes and prior geometry. Failed/cancelled
edits leave document/history unchanged. The operation marks the document dirty
and participates in existing unsaved-document handling.

Normal reversal is an attribute correction, not mesh winding repair, a physical
movement, surface smoothing or scanner calibration. It can affect downstream
normal averaging/rendering; point-to-plane squared residuals alone cannot
distinguish normal sign. Use a deliberate orientation policy for real datasets.

Two geometry tests cover selected-only changes, geometry/color preservation,
bit-exact involution, missing/zero normals, invalid selections and cancellation.
A workbench worker test exercises preview/commit, selection retention,
undo/redo, failed/cancelled proposals, save/reopen and subsequent undo.
The release UI startup is separately rendered in an authenticated test display;
the image verifies the command is present, not a mouse-driven edit workflow.
Evidence uses `normal-flip-`; no scanner device state is changed or physical
normal accuracy qualified.
