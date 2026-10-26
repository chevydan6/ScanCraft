use scancraft_geometry::{
    mesh::Mesh,
    mesh_normals::{self, Config},
    work::WorkControl,
};
fn config() -> Config {
    Config {
        maximum_deviation_rad: 0.3,
        viewpoint_mm: None,
    }
}
#[test]
fn planar_normals_preserve_mesh_and_orient_only_when_explicit() {
    let mesh = Mesh {
        vertices: vec![
            [0., 0., 5.],
            [2., 0., 5.],
            [2., 1., 5.],
            [0., 1., 5.],
            [20., 20., 20.],
        ],
        triangles: vec![[0, 1, 2], [0, 2, 3]],
    };
    let original = mesh.clone();
    let report = mesh_normals::estimate(&mesh, config(), &WorkControl::default()).unwrap();
    assert_eq!(report.estimated, 4);
    assert_eq!(report.unsupported, 1);
    assert_eq!(report.normals[4], None);
    for normal in report.normals[..4].iter().flatten() {
        assert_eq!(*normal, [0., 0., 1.]);
    }
    let mut c = config();
    c.viewpoint_mm = Some([0., 0., 0.]);
    let report = mesh_normals::estimate(&mesh, c, &WorkControl::default()).unwrap();
    assert!(report.normals[..4].iter().flatten().all(|n| n[2] == -1.));
    assert_eq!(mesh, original);
}
#[test]
fn sharp_shared_corners_are_unsupported_instead_of_blended_past_gate() {
    let mesh = Mesh {
        vertices: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 4.]],
        triangles: vec![[0, 1, 2], [1, 0, 3]],
    };
    let report = mesh_normals::estimate(&mesh, config(), &WorkControl::default()).unwrap();
    assert_eq!(report.normals[0], None);
    assert_eq!(report.normals[1], None);
    assert!(report.normals[2].is_some());
    assert!(report.normals[3].is_some());
    let mut wide = config();
    wide.maximum_deviation_rad = 0.9;
    let smooth = mesh_normals::estimate(&mesh, wide, &WorkControl::default()).unwrap();
    let normal = smooth.normals[0].unwrap();
    // Equal corner angles, unequal triangle areas: angle weighting gives equal Y/Z.
    assert!(normal[0].abs() < 1e-12);
    assert!((normal[1] - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
    assert!((normal[2] - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
}
#[test]
fn duplicate_winding_errors_invalid_config_and_cancellation_are_refused() {
    let mesh = Mesh {
        vertices: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        triangles: vec![[0, 1, 2], [0, 1, 3]],
    };
    assert!(mesh_normals::estimate(&mesh, config(), &WorkControl::default()).is_err());
    let mut duplicate = mesh.clone();
    duplicate.triangles = vec![[0, 1, 2], [0, 1, 2]];
    assert!(mesh_normals::estimate(&duplicate, config(), &WorkControl::default()).is_err());
    let mut invalid = config();
    invalid.maximum_deviation_rad = f64::NAN;
    assert!(mesh_normals::estimate(&mesh, invalid, &WorkControl::default()).is_err());
    let work = WorkControl::default();
    work.cancel();
    assert!(mesh_normals::estimate(&mesh, config(), &work).is_err());
}
