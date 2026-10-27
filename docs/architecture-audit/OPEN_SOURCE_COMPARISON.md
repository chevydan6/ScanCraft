Audited source: `ced7697fa0a4e2a262ae255efffa53ca577f117f`. Confirmed/inference/recommendation labels and audit scope are defined in ARCHITECTURE.md.

# Comparison with mature open-source components

## Interpretation

**Confirmed local capabilities** below reference the audited implementation.
**External capabilities** reference primary upstream documentation/source. These
are functionality comparisons, not measured speed/accuracy rankings. No common
ScanCraft-vs-library hardware dataset/benchmark was found. External current branch
heads were recorded read-only on 2026-10-27 for reproducible source/license
inspection; they are not integrated dependencies or claimed sources of local code.

| Project/source snapshot | Evidence and selected license facts |
|---|---|
| OpenCV4.x fa3037533ed05f5018cdd22a20f5b2ee7a38e8ed | [License](https://github.com/opencv/opencv/blob/fa3037533ed05f5018cdd22a20f5b2ee7a38e8ed/LICENSE): Apache-2.0. [calib3d](https://docs.opencv.org/4.x/d9/d0c/group__calib3d.html) and feature docs expose calibration/board detection/rectification/stereo/corner methods |
| Open3D main b6c5e196384ad71e75b6e6f9c5da22d046221f1d | [MIT license](https://github.com/isl-org/Open3D/blob/b6c5e196384ad71e75b6e6f9c5da22d046221f1d/LICENSE); [pipeline documentation](https://www.open3d.org/docs/release/tutorial/pipelines/index.html) (release docs identify0.19.0, not a promise of exact main API) |
| PCL master dd4edf371e058ab914d385c2ff417eea68c2691d | [BSD-style license terms](https://github.com/PointCloudLibrary/pcl/blob/dd4edf371e058ab914d385c2ff417eea68c2691d/LICENSE.txt); [registration module](https://pointclouds.org/documentation/group__registration.html) |
| CCCoreLib master35bfaac17eca59d45084fbb92d5117c1c63aab23 | [RegistrationTools.h](https://github.com/CloudCompare/CCCoreLib/blob/35bfaac17eca59d45084fbb92d5117c1c63aab23/include/RegistrationTools.h) declares LGPL-2.0-or-later; repository LICENSE includes Library GPLv2. Do not assume an LGPL2.1-only label. [CloudCompare application license](https://github.com/CloudCompare/CloudCompare/blob/master/license.txt) is separately GPL2-or-later |
| VCGlib main5cae2abc2f9056785b0537dcbe156c40da2aea20 | [README capabilities](https://github.com/cnr-isti-vclab/vcglib/blob/5cae2abc2f9056785b0537dcbe156c40da2aea20/README.md). Root [LICENSE](https://github.com/cnr-isti-vclab/vcglib/blob/5cae2abc2f9056785b0537dcbe156c40da2aea20/LICENSE.txt) contains GPLv3 while [clean.h](https://github.com/cnr-isti-vclab/vcglib/blob/5cae2abc2f9056785b0537dcbe156c40da2aea20/vcg/complex/algorithms/clean.h) declares GPL2-or-later. Inspect exact selected headers/dependencies; do not collapse differing file notices into one unsupported license claim |

## Subsystem evaluation

| Subsystem and local source | Existing overlap | Mature capability that could add value | Integration/maintenance tradeoff and measurable result |
|---|---|---|---|
| Linux acquisition (device/acquisition) | V4L2 capture, metadata, session locks, controls, settings restoration | Generic OpenCV video APIs do not by themselves supply Raptor initialization, mode binding, metadata or cleanup qualification | Keep device-specific native code; no evidence external generic capture replaces the transport work |
| Camera/target preparation (calibration; image/corners) | Brown5, rectification, triangulation, supplied-observation fits and seeded corners | OpenCV has mature full-board/marker detection and calibration variants absent here | Existing Python oracles already reuse it. Offline adapter could reduce detector development; FFI/runtime packaging must retain provenance, pixel/axis/unit conventions and cancellation |
| Stereo/laser image observations (depth/matching, image/laser) | Sparse ZNCC/subpixel and row-run stripe centroids | OpenCV offers dense stereo and broader image primitives; none automatically supplies scanner line identity/optics | Prototype an offline oracle on identical recorded pairs before runtime adoption; no measured accuracy/throughput comparison exists |
| Neighborhoods/filtering (spatial/filtering/statistical_filter) | Exact KD-tree, voxel/radius/statistical/duplicate filters | Open3D/PCL/CCCoreLib offer broader indexes, filters, normals and processing policies | May save maintenance for new capabilities; conversion/copies/order/attribute preservation and dependencies cost work. No current Rust speed advantage established |
| Registration/tracking (registration/point_plane/tracking) | Local point/plane ICP, Huber option and explicit quality gates | Open3D global/pose-graph pipelines; PCL registration families; CCCoreLib registration/constraints | Strong candidates for reference experiments and missing relocalization capabilities. Same starting pose, correspondence rules, thresholds and failure labels required before ranking |
| Fusion (fusion/session) | Bounded voxel centroids and retained accepted-frame pose/state | Open3D volumetric integration/SLAM facilities exceed centroid fusion | Could reduce implementing a volumetric pipeline, but must bind metric calibrated inputs, retain session provenance and measure memory/latency. Not a drop-in replacement |
| Surface reconstruction (mesh::from_grid, stereo_surface) | Organized adjacent samples only | Open3D Poisson/ball pivoting and PCL surface algorithms; VCG meshing/isosurfaces | Substantially broader functionality for unorganized scans; selected method still needs topology/detail/scale/failure qualification. Highest-value evaluation when trustworthy clouds exist |
| Mesh editing/repair (mesh_* modules) | Basic topology, components, caps, smoothing and constrained shortest-edge collapse | VCGlib/MeshLab tooling has QEM/fairing/repair/analysis breadth; Open3D also supplies mesh operations | Reuse could avoid building all repair features. GPL/LGPL/application-vs-library distinctions, templates, attributes and error contracts must be addressed; not proof local edits should be replaced |
| Measurement/deviation (measurement/cylinder/section) | Distances, frames, plane/circle/sphere/cylinder fits and mesh sections | CCCoreLib/CloudCompare distance/scalar-field workflows and VCG geometry analysis | Useful independent validation and missing deviation workflows; physical uncertainty/ground truth still needed regardless of library |
| Rendering/UI/persistence (app/workflow/formats) | egui/glow, original-index editing, custom source/history/project formats | External libraries provide visualizers and serializers but do not preserve ScanCraft-specific edit/source/restoration contracts automatically | Replacing the workbench is not justified by this audit; improve measured bottlenecks through explicit owned adapters if needed |

## Gains, losses and licensing constraints

**Inference:** using existing implementations for still-missing full-board detection,
global registration or arbitrary-cloud reconstruction could reduce algorithm
implementation/testing effort. Actual integration adds C++ build/ABI/exception
management, allocations/copies, unit and coordinate transforms, attribute/index
mapping, threading/cancellation policy, packaging and upstream compatibility work.
An offline process adapter can isolate failures and keep production Rust optional,
but introduces file/IPC/provenance costs. A narrow reference-tool dependency is a
lower-risk first experiment than replacing the core.

The application MIT declaration does not relicense imported source. Apache/MIT/
BSD notice and, where applicable, patent terms must be retained. LGPL/GPL code
has different source/relinking/combined-distribution conditions depending on
selected files and linkage. A license choice for a combined distribution must
be checked before integration; no such distribution is authorized or implemented
by this audit. Neither “header-only” nor process boundaries alone establish a
universal exemption. Preserve THIRD_PARTY_NOTICES and the existing separate kernel
license. No new upstream code or binaries are copied here.

**Finding:** substantial algorithm-family overlap is confirmed. “Unnecessarily
recreated” is not established: the native-Rust requirement is documented, while
per-algorithm alternatives/FFI experiments and common performance/accuracy trials
are missing. No external implementation is claimed faster or more accurate.
Require a concrete missing capability, identical datasets and an explicit decision
record before recommending replacement. See DESIGN_DECISIONS/RECOMMENDATIONS.
