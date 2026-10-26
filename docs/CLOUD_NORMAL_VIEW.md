# Point-cloud normal inspection

The native cloud workbench now offers **Show point normals** and an arrow-length
control in display pixels. `scancraft-app --cloud-normals PROJECT_OR_PLY_FILE`
opens geometry with this overlay selected; ordinary imports keep the default
off. Geometry startup does not start camera acquisition. Declared file units
and the existing unsaved-document import policy still apply.

Preview preparation counts finite, normalizable normals across the full cloud
and copies normalized directions into the bounded display sample. Source
normal values, positions, colors, selection and edit history remain unchanged.
Zero or unrepresentably large normal magnitudes are unsupported for display.
The overlay samples at most 2,000 directions from the existing at-most-50,000
point preview. A cloud with sparse normals may have fewer displayed arrows than
its full-cloud supported-normal count.

Arrows rotate with the orthographic view and foreshorten along the viewing
direction. Their length is a display aid, not a physical distance. Direction
projection avoids adding a small normal to a huge world coordinate, and arrows
are clipped to the viewport. Directions pointing almost directly toward/away
from the viewer can have no visible arrow. The overlay draws on top for
inspection; it is not an occlusion/visibility qualification. Picking continues
to use projected source points and original indices, never arrow endpoints.

Tests cover display-only normalization/zero normals, source preservation,
sampling limits, foreshortening, huge coordinate origins and point picking
with the overlay enabled. The synthetic plane in
`evidence/cloud-normal-preview.ply` has analytically defined unit normals.
`tools/check_workbench_render.py INPUT NEW_OUTPUT --cloud-normals` exercises the
release app in its own authenticated Xvfb display, captures its framebuffer,
checks for camera-device descriptors and terminates only its own processes.
Evidence is under `evidence/cloud-normal-render/`. Software-rendered UI checks
do not qualify GPU throughput, scanner calibration, normal accuracy on real
scans or full interactive usability.
