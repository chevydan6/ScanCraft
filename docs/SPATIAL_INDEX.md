# Exact native spatial queries for registration

`scancraft_geometry::spatial::SpatialIndex` builds a borrowed, balanced median
three-axis tree for up to one million finite points. Splitting cycles axes and
uses original index to order equal coordinates. The index stores nodes and a
borrow of immutable source positions; it cannot outlive that source. Empty
indexes are valid. Queries reject nonfinite positions; cancellation produces
an error without a partial answer. Build and query are cooperative.

Nearest-neighbor search evaluates Euclidean distances with chained `hypot`,
avoiding squared-distance overflow where a finite distance is representable.
It searches the near subtree first, then the far subtree when the split-plane
distance can equal or improve the best result. Equality is included because
an equidistant lower original index must not be pruned. Exact distance ties
choose the lowest source index, including coincident points. If every distance
is outside finite f64 range, the answer is None. No approximation is enabled.

The existing CPU point-to-point ICP now builds this index once per target.
The tracker also uses it to independently recompute final coverage and residuals.
Existing point/iteration caps remain; a balanced tree does not guarantee cheap
queries for every degenerate distribution. This improves search, not ICP's
local-minimum, initialization or physical-geometry limitations. No source points
are sampled or dropped by indexing.

Two new regressions compare 500 queries over 4,099 deterministic points against
an independent brute-force loop, plus exact ties, duplicates, empty input,
large finite distances, invalid values and cancellation. Existing synthetic
registration/trajectory/fusion/session tests continue to pass. All 181 workspace
tests, Clippy, formatting and release workspace build pass.

Reproduce the synthetic microbenchmark with:

```
cargo run --release -p scancraft-geometry --example spatial_benchmark
```

On the current x86_64 Ryzen 6900HS host, three rounds over 10,000 synthetic
positions and 1,000 queries produced identical answers to the scalar brute-force
`hypot` baseline. Build took 1.15–1.48 ms; indexed queries 2.15–2.18 ms; baseline
queries approximately 349 ms. The ratio including tree build was 95–105× for
this specific workload. These timings are neither the whole ICP speedup nor a
comparison against CrealityScan, and do not qualify 20 FPS live reconstruction.
Target/query sizes, random seed, exact agreement and raw round timings are
retained in `evidence/spatial-index-benchmark.json`. No new dependencies or
reference runtime are introduced.
