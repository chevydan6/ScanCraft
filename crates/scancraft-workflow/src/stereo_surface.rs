//! Mesh only explicitly declared seed-lattice adjacency; never reorder an arbitrary cloud.
use scancraft_depth::pipeline::StereoReport;
use scancraft_geometry::{
    mesh::{GridMesh, from_grid},
    work::WorkControl,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceConfig {
    pub spacing: u32,
    pub maximum_edge_mm: f64,
}
pub struct StereoSurface {
    pub grid: GridMesh,
    pub diagnostics: serde_json::Value,
    pub vertex_cloud_indices: Vec<usize>,
}
pub fn build(
    report: &StereoReport,
    config: SurfaceConfig,
    work: &WorkControl,
) -> Result<StereoSurface, String> {
    work.check()?;
    if config.spacing == 0 || !config.maximum_edge_mm.is_finite() || config.maximum_edge_mm <= 0. {
        return Err("invalid stereo meshing spacing or edge limit".into());
    }
    if report
        .automatic_sampling
        .is_some_and(|s| s.spacing != config.spacing)
    {
        return Err("mesh spacing must equal automatic sampling spacing".into());
    }
    let camera = &report.rectification.geometry().left;
    let origin = report.matching.config.patch_radius;
    let border = origin.checked_mul(2).ok_or("mesh border overflow")?;
    if report.seeds.len() > 100_000 {
        return Err("mesh seed budget exceeded".into());
    }
    let nx = camera
        .width
        .checked_sub(border)
        .filter(|n| *n > 0)
        .ok_or("mesh grid has no interior columns")?;
    let ny = camera
        .height
        .checked_sub(border)
        .filter(|n| *n > 0)
        .ok_or("mesh grid has no interior rows")?;
    let width = (nx - 1) / config.spacing + 1;
    let height = (ny - 1) / config.spacing + 1;
    let count = u64::from(width) * u64::from(height);
    if width < 2 || height < 2 || count > 1_000_000 {
        return Err(
            "stereo mesh lattice must be 2x2 or larger and at most one million cells".into(),
        );
    }
    let slot = |pixel: [u32; 2]| -> Result<usize, String> {
        let x = pixel[0]
            .checked_sub(origin)
            .ok_or("seed outside mesh lattice")?;
        let y = pixel[1]
            .checked_sub(origin)
            .ok_or("seed outside mesh lattice")?;
        if x >= nx
            || y >= ny
            || !x.is_multiple_of(config.spacing)
            || !y.is_multiple_of(config.spacing)
        {
            return Err("seed is not on the declared stereo mesh lattice".into());
        }
        Ok((y / config.spacing) as usize * width as usize + (x / config.spacing) as usize)
    };
    // Validate even rejected seeds so the declaration cannot silently hide layout conflicts.
    for (i, seed) in report.seeds.iter().enumerate() {
        if i.is_multiple_of(1024) {
            work.check()?;
        }
        slot(*seed)?;
    }
    let mut samples = vec![None; count as usize];
    let mut seed_indices = vec![None; count as usize];
    let mut cloud_indices = vec![None; count as usize];
    if let Some(reconstruction) = &report.reconstruction {
        for (cloud_index, point) in reconstruction.accepted.iter().enumerate() {
            work.check()?;
            let mapped = report
                .mapped_matches
                .get(point.source_index)
                .ok_or("invalid reconstructed source lineage")?;
            let seed = *report
                .seeds
                .get(mapped.seed_index)
                .ok_or("invalid mapped seed lineage")?;
            let index = slot(seed)?;
            if samples[index].is_some() {
                return Err("duplicate accepted mesh sample".into());
            }
            samples[index] = Some(point.geometry.position_mm);
            seed_indices[index] = Some(mapped.seed_index);
            cloud_indices[index] = Some(cloud_index);
        }
    }
    let grid = from_grid(width, height, &samples, config.maximum_edge_mm, work)?;
    let vertex_seed_indices: Vec<_> = seed_indices.into_iter().flatten().collect();
    let topology = grid.mesh.topology(work)?;
    let diagnostics = serde_json::json!({"schema_version":1,"units":"millimeters","coordinate_frame":"original left camera frame",
        "lattice_origin_rectified_pixel":[origin,origin],"spacing_rectified_pixels":config.spacing,
        "grid_size":[width,height],"maximum_edge_mm":config.maximum_edge_mm,
        "vertex_seed_indices":vertex_seed_indices,"vertices":grid.mesh.vertices.len(),"triangles":grid.mesh.triangles.len(),
        "incomplete_cells":grid.incomplete_cells,"rejected_triangles":grid.rejected_triangles,"topology":topology,
        "synchronization_verified":false,"physical_accuracy_qualified":false,"watertightness_qualified":false});
    Ok(StereoSurface {
        grid,
        diagnostics,
        vertex_cloud_indices: cloud_indices.into_iter().flatten().collect(),
    })
}
