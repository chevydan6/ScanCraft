//! Reproducible exact-query microbenchmark, not live-scanner qualification.
use scancraft_geometry::{
    spatial::{Neighbor, SpatialIndex},
    work::WorkControl,
};
use std::{hint::black_box, time::Instant};
fn main() -> Result<(), String> {
    let mut state = 0x12345678u32;
    let mut random = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        f64::from(state) / f64::from(u32::MAX) * 1000. - 500.
    };
    let points: Vec<_> = (0..10000).map(|_| [random(), random(), random()]).collect();
    let queries: Vec<_> = (0..1000).map(|_| [random(), random(), random()]).collect();
    let work = WorkControl::default();
    let mut rounds = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        let index = SpatialIndex::build(black_box(&points), &work)?;
        let build = start.elapsed().as_secs_f64();
        let start = Instant::now();
        let indexed: Vec<_> = queries
            .iter()
            .map(|p| index.nearest(black_box(*p), &work))
            .collect::<Result<_, _>>()?;
        let query = start.elapsed().as_secs_f64();
        let start = Instant::now();
        let brute: Vec<_> = queries
            .iter()
            .map(|p| {
                points
                    .iter()
                    .enumerate()
                    .map(|(i, q)| Neighbor {
                        index: i,
                        distance_mm: (p[0] - q[0]).hypot(p[1] - q[1]).hypot(p[2] - q[2]),
                    })
                    .min_by(|a, b| {
                        a.distance_mm
                            .total_cmp(&b.distance_mm)
                            .then(a.index.cmp(&b.index))
                    })
            })
            .collect();
        let brute_time = start.elapsed().as_secs_f64();
        if indexed != brute {
            return Err("indexed/brute result mismatch".into());
        }
        black_box((&indexed, &brute));
        rounds.push(serde_json::json!({"build_seconds":build,"indexed_query_seconds":query,"brute_query_seconds":brute_time,"query_speed_ratio":brute_time/query,"ratio_including_build":brute_time/(query+build)}));
    }
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"scope":"CPU synthetic exact-query microbenchmark; not ICP accuracy or live throughput qualification","architecture":std::env::consts::ARCH,"points":points.len(),"queries":queries.len(),"rounds":rounds,"all_results_exactly_match_brute_force":true,"physical_accuracy_qualified":false})).map_err(|e|e.to_string())?);
    Ok(())
}
