# Paired remapping concurrency

`StereoImageMaps::apply` uses one scoped helper for the left map while the caller
processes the right map when both output maps contain at least320×240pixels and
Rust reports at least two available CPUs. Smaller maps, an unavailable CPU hint,
one CPU or a failed thread spawn use the unchanged sequential path. The public
`apply_sequential` method remains an explicit reference/fallback. No new crate
or geometry algorithm is introduced; all interpolation math and coordinates stay
in the existing UndistortMap implementation.

Both inputs are checked before the large-map job starts. Both processing results
must succeed before either image is returned, and the helper is joined even when
the right map returns an error. A helper panic becomes an error. Source buffers
and maps are borrowed immutably; outputs remain independent. WorkControl is
shared for cooperative cancellation; per-map progress may interleave and is not
an aggregate pair fraction. This does not change device ownership or establish
an ioctl/shutdown deadline.

## Measured policy and limits

The release paired_remap_benchmark calls the actual production API and the explicit
sequential reference on the same synthetic maps/inputs. Twenty rounds alternate
measurement order after one warmup per mode. Every output intensity bit and
validity mask is compared with the reference outside the measured interval.
Allocation and thread spawn/join are included; map construction, comparison and
output destruction are excluded. No other owned build ran during the final runs.
The CPU governor and general OS scheduling were not fixed.

Ryzen9 6900HS,16availableCPUs, median milliseconds per pair:

| Output size | Sequential | Adaptive production |
|---|---:|---:|
|64×32|0.070407|0.070407|
|320×240|2.891|1.558|
|640×480|10.627|5.590|
|1920×1200|89.930|43.795|

A separate child restricted to one CPU reported one available CPU and used the
sequential fallback (full-size median93.529versus93.691ms). A child with
RLIMIT_NPROC=0 could not create threads, still reported16CPUs, and completed all
maps with exact matching output through the spawn-failure fallback. Only test
children’s affinity/limits changed; other processes were untouched.

The320×240policy crossover is based on representative sizes on this workstation,
not a universal optimum. The exploratory always-threaded64×32case was slower;
small maps therefore remain sequential. Full-size map coordinate buffers total
110,592,000bytes; this is not RSS. Additional helper stack/thread overhead was
not profiled. Results do not establish hardware latency, sustained frame rate,
physical precision, GPU/SIMD benefit or memory-growth qualification.

## Reproduction and validation

```sh
cargo run --frozen --release -p scancraft-image --example paired_remap_benchmark
```

Source: crates/scancraft-image/src/stereo.rs and
crates/scancraft-image/examples/paired_remap_benchmark.rs.
Three regressions compare rotated/distorted large maps bit-for-bit, retain the
small-map contract and reject wrong-sized/cancelled inputs without partial
results. The full frozen gate passes583Rusttests, formatting, strict all-target
Clippy and release build. The benchmark also exercises single-CPU and actual
thread-denial fallback in isolated test children. Measurements, toolchain,
source/binary hashes and limits are in benchmarks/paired-remap-2026-10-27.json.
Physical Raptor geometry/accuracy and SCAN-REV-005 remain unqualified.
