# Independent cylinder numerical comparison

Eight deterministic synthetic cases were compared against an independent SciPy
optimizer: long and squat clean walls, full/partial noisy walls (90 and 45 degree
coverage), skewed height sampling, elliptical form error and tapered form error.
All eight pass the predeclared comparison thresholds. This is numerical agreement
between two local solvers, not a physical uncertainty, global-optimality proof,
scanner validation or independent certification.

The production implementation remains Rust. The optional Python/SciPy harness is
not an application, reconstruction or ordinary Cargo-test dependency. It uses a
separate five-parameter stereographic axis/offset parameterization, three-point
numerical derivatives and the Trust Region Reflective optimizer, rather than the
native analytic tangent Jacobian/damped iteration. Seven starts use independent
NumPy PCA/projected initialization plus fixed directions. It does not seed from
the native fit. Each reference candidate must terminate successfully and pass
rank and undamped-stationarity checks before minimum-cost selection. The generic
optimizer behavior is documented in the primary
[SciPy least_squares reference](https://docs.scipy.org/doc/scipy/reference/generated/scipy.optimize.least_squares.html).

Native and reference fits use the same unweighted radial residual objective.
Axis signs are compared modulo reversal. Axis-line locations are compared at the
point nearest the common input centroid, avoiding false disagreement from axial
point gauge choices. Native RMS is independently recomputed in NumPy.

Comparison limits, chosen before the first run, are:

- Diameter difference <=1e-5 times max(1 mm, reference diameter).
- Axis angle <=1e-5 rad.
- Axis location difference <=1e-5 times max(1 mm, reference radius, sample extent).
- RMS difference <=1e-7 times max(1 mm, reference RMS).
- Native RMS recomputation difference <=1e-7 times max(1 mm, sample extent).
- Native unit-axis norm error <=1e-8 (explicit invariant added during harness
  hardening; no geometric-agreement limit was relaxed).

These are diagnostic numerical gates, not the mission's physical qualification
thresholds. Intentional 0.1 mm diameter corruption and nonunit axis corruption
both fail. Reference failures and native rejections remain failed/inconclusive
cases in the report, rather than being dropped. The initial and final runs use
identical input and native binary hashes; no cases were selected away.

## Observed differences

| Case | Diameter difference (mm) | Axis angle (rad) | RMS difference (mm) |
|---|---:|---:|---:|
| long_clean | 1.776e-15 | 1.717e-16 | 5.672e-16 |
| squat_clean | 0.000e+00 | 1.669e-15 | 5.821e-17 |
| full_radial_noise | 1.092e-11 | 3.613e-10 | 2.671e-16 |
| partial_90_noise | 1.287e-10 | 1.243e-13 | 1.388e-17 |
| partial_45_noise | 2.487e-09 | 1.052e-13 | 5.399e-17 |
| asymmetric_height | 2.450e-09 | 2.371e-11 | 5.031e-17 |
| elliptic_form | 1.776e-15 | 4.765e-12 | 2.776e-17 |
| tapered_form | 2.132e-14 | 1.871e-10 | 2.776e-17 |

Maximum axis-line location difference at the centroid was 4.17e-9 mm; maximum
native RMS recomputation difference was 3.72e-16 mm. These small floating-point
comparison differences say nothing about achievable scanner resolution.

## Reproduction

The recorded environment is Python 3.14.8, NumPy 2.5.4 and SciPy 1.18.1. Use an
isolated environment; pinned optional requirements are in
`tools/cylinder-reference-requirements.txt`.

```
python3 -m venv /tmp/scancraft-reference-venv
/tmp/scancraft-reference-venv/bin/python -m pip install -r tools/cylinder-reference-requirements.txt
cargo build --release -p scancraft-geometry --example cylinder_reference
/tmp/scancraft-reference-venv/bin/python tools/check_cylinder_reference.py NEW_OUTPUT_FOLDER
```

The Rust diagnostic accepts a strict bounded 8 MiB millimeter request with up to
64 named cases and reports each native fit/rejection. The harness pins random
seed 20261026, records all generated inputs, and checks their CRC against the
native report. Reports retain input SHA-256, native executable SHA-256 (unchanged
across execution), harness SHA-256, package versions, per-start diagnostics,
metrics, thresholds and sensitivity checks. Times are observations, not qualified
benchmarks. The tool writes only a newly created output folder and opens no
scanner devices.

Evidence: `evidence/cylinder-reference-comparison/{inputs,native,report}.json`,
`cylinder-reference-initial-report.json`, `cylinder-reference-fault-checks.json`
and `cylinder-reference-{tests,clippy,release,build}.log`. Standard Rust test count
remains 370, all passing; this comparison adds a separate numerical evidence gate.

## NIST practice-data limit

[NIST provides practice data and reference fits](https://www.nist.gov/pml/sensor-science/dimensional-metrology/algorithm-testing)
for self-testing. Its [instructions](https://www.nist.gov/document/instructionsfornist-l2-reference-pairspdf)
explicitly distinguish that use from testing by an independent body. Official
archive downloads returned HTTP 403 through this execution runtime; see
`evidence/cylinder-reference-nist-access.json`. Zero NIST cases were executed.
The synthetic SciPy comparison must not be described as NIST testing or
certification. NIST/reference-dataset coverage, broader adversarial validation,
performance qualification and calibrated physical measurements remain open.
