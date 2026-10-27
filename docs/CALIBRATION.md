# Camera geometry and calibration records

The `scancraft-calibration` crate provides independent Rust camera mathematics,
versioned calibration records, and bounded decoders for observed factory-data
schemas. [Factory-data diagnostics](FACTORY_CALIBRATION.md) extract named lens,
pose and laser fields; extraction does not establish their active-mode binding,
physical units, camera assignment or physical accuracy. The crate now provides
an explicit [planar pinhole initializer](PLANAR_CALIBRATION_INITIALIZER.md) from
supplied metric correspondences and explicit [Brown5 nonlinear refinement](BROWN_CALIBRATION_REFINEMENT.md)
with an offline CLI workflow. Target-corner detection, joint stereo estimation,
physical lens coverage and a qualified Raptor optical model remain incomplete.
Genuine calibrated Raptor geometry remains unverified, and physical calibration
remains deferred by user request.

## Coordinates and model

Camera coordinates use +X right, +Y down, +Z forward. Translations and 3D
positions are millimeters; image coordinates are pixels with integer pixel
centers starting at (0,0). `right_from_left` transforms a left-camera point
into right-camera coordinates: `p_right = R * p_left + t_mm`. The right camera
origin in left coordinates is `-R.transpose() * t_mm`. Rotations must be
right-handed and orthonormal. No implicit reflection, scaling or axis change
is applied.

The implemented model is pinhole plus five Brown distortion coefficients:
k1/k2/k3 radial and p1/p2 tangential, with optional intrinsic skew. This is a
subset of the models described by the [OpenCV camera geometry reference](https://docs.opencv.org/4.13.0/d9/d0c/group__calib3d.html).
Unsupported rational, fisheye, thin-prism and tilted-sensor models must be
implemented and validated if actual scanner evidence requires them. The
current model must not be assumed to fit the Raptor.

`valid_radius` specifies the supported undistorted normalized-image disk.
It must come from calibration coverage, not a guessed device specification.
Validation finds cubic minima at endpoints and stationary points. A
conservative bound on the symmetric tangential Jacobian ensures the full
mapping remains strongly monotone over that disk. Numerically folded or
insufficiently conditioned models fail validation. Inverse distortion uses
bounded damped Newton iterations and the analytic Jacobian; failure or an
out-of-domain observation returns an error rather than an invented ray.

```mermaid
flowchart LR
    Pixels[Distorted pixel pair] --> Inverse[Inverse intrinsics and distortion]
    Inverse --> Rays[Camera rays]
    Rays --> Pose[Right ray transformed to left frame]
    Pose --> Closest[Closest points between rays]
    Closest --> Checks[Parallax, cheirality, range, gap and reprojection checks]
    Checks --> Point[Accepted 3D point in millimeters]
```

A prepared stereo model validates once and caches extrinsic matrices before
processing correspondences. Its input limits are explicit: minimum ray angle,
left Z depth range, maximum ray gap and maximum reprojection error. These are
algorithm policies, not manufacturer claims. Image preparation and matching
live in `scancraft-image`, reconstruction in `scancraft-depth`, and tracking/fusion
in the `session`, `tracking`, and `fusion` modules of `scancraft-geometry`;
`scancraft-workflow` orchestrates these components separately from camera mathematics.
See [offline stereo](OFFLINE_STEREO.md), [scan sessions](SCAN_SESSIONS.md), and
[recorded sequences](RECORDED_STEREO_SEQUENCE.md). Their synthetic/offline
validation does not establish synchronized Raptor exposures or physical accuracy.

## Persistence and provenance

Calibration files use RAPCAL01 magic, little-endian u32 JSON-body length,
u32 CRC32, and a versioned JSON body capped at one MiB. Every coefficient,
unit/convention marker and declared binding is checked before use. Unknown
fields and schemas, corrupt/truncated files and trailing bytes are errors.
CRC provides accidental-corruption detection, not authentication.

Records explicitly distinguish synthetic fixtures, estimated observations,
device readouts and imports. The capture-binding guard rejects known synthetic
fixtures and mismatched declared serial, firmware, mode or image sizes. A
matching declaration does not establish physical provenance or accuracy. The
offline application workflow accepts explicit calibration records for recorded
stereo reconstruction, including [GUI orchestration](GUI_RECONSTRUCTION.md) and
[recorded-sequence projects](SEQUENCE_PROJECT_REFERENCES.md). This is an
implemented declared-calibration workflow, not qualified calibrated live Raptor
scanning. The application does not automatically convert diagnostic factory
fields or mirror hypotheses into a physically qualified record.

## Independent numerical comparison

The synthetic reference example creates two rotated, asymmetric cameras and
60 known points. They are explicitly labeled synthetic and are not scanner
parameters. A separate test-only OpenCV 5.0.0 environment compares projection,
inverse distortion, and triangulation against the Rust result. The Rust ray
midpoint method and OpenCV's triangulation are independent implementations;
only clean, exact correspondences are used for this agreement check.

```sh
cargo run --release -p scancraft-calibration --example synthetic_reference_cases > evidence/synthetic-calibration-reference-cases.json
python3 -m venv .venv-reference
.venv-reference/bin/python -m pip install -r tools/reference-requirements.txt
.venv-reference/bin/python tools/compare_calibration_opencv.py evidence/synthetic-calibration-reference-cases.json evidence/calibration-opencv-comparison.json
```

OpenCV and NumPy are not production application dependencies. Their isolated
environment is excluded from git. The observed maximum 3D discrepancy was
about 1.08e-8 mm for these synthetic cases; this is numerical agreement,
**not physical scanner accuracy or dimensional qualification**.
