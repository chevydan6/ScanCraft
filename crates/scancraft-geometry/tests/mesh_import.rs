use scancraft_geometry::{
    mesh::Mesh,
    mesh_import::{Config, Format, Units, decode},
    work::WorkControl,
};
fn config(format: Format) -> Config {
    Config {
        format,
        units: Units::Millimeters,
        weld_exact: false,
    }
}
fn source() -> Mesh {
    Mesh {
        vertices: vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        triangles: vec![[0, 1, 2], [0, 2, 3]],
    }
}
#[test]
fn native_obj_roundtrip_keeps_indices_and_explicit_units() {
    let source = source();
    let work = WorkControl::default();
    let mut bytes = Vec::new();
    source.write_obj(&mut bytes, &work).unwrap();
    let (mesh, r) = decode(&bytes, &config(Format::Obj), &work).unwrap();
    assert_eq!(mesh, source);
    assert_eq!(r.merged_exact_vertices, 0);
    for (unit, factor) in [(Units::Inches, 25.4), (Units::Meters, 1000.)] {
        let mut c = config(Format::Obj);
        c.units = unit;
        let (mesh, _) = decode(&bytes, &c, &work).unwrap();
        assert_eq!(mesh.vertices[2], [factor, factor, 0.]);
    }
}
#[test]
fn ascii_stl_exact_welding_is_explicit_and_normals_are_recomputed() {
    let source = source();
    let work = WorkControl::default();
    let mut bytes = Vec::new();
    source.write_stl(&mut bytes, &work).unwrap();
    let mut c = config(Format::Stl);
    let (separate, r) = decode(&bytes, &c, &work).unwrap();
    assert_eq!(separate.vertices.len(), 6);
    assert_eq!(separate.topology(&work).unwrap().boundary_edges, 6);
    assert!(r.facet_normals_recomputed);
    c.weld_exact = true;
    let (welded, r) = decode(&bytes, &c, &work).unwrap();
    assert_eq!(welded, source);
    assert_eq!(r.merged_exact_vertices, 2);
    assert_eq!(welded.topology(&work).unwrap().boundary_edges, 4);
}
fn binary() -> Vec<u8> {
    let mut bytes = vec![0; 80];
    bytes[..5].copy_from_slice(b"solid");
    bytes.extend(1_u32.to_le_bytes());
    for x in [0_f32, 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0.] {
        bytes.extend(x.to_le_bytes());
    }
    bytes.extend([0, 0]);
    bytes
}
#[test]
fn solid_prefixed_binary_stl_and_all_truncated_prefixes_are_controlled() {
    let bytes = binary();
    let work = WorkControl::default();
    let (mesh, _) = decode(&bytes, &config(Format::Stl), &work).unwrap();
    assert_eq!(mesh.triangles, vec![[0, 1, 2]]);
    assert_eq!(mesh.vertices[2], [0., 1., 0.]);
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end], &config(Format::Stl), &work).is_err());
    }
    let mut colored = bytes.clone();
    colored[132] = 1;
    assert!(decode(&colored, &config(Format::Stl), &work).is_err());
    let mut invalid = bytes;
    invalid[96..100].copy_from_slice(&f32::INFINITY.to_le_bytes());
    assert!(decode(&invalid, &config(Format::Stl), &work).is_err());
}
#[test]
fn obj_negative_indices_signed_zero_and_exact_not_proximity_welding() {
    let bytes = b"v -0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 0\nv 0.0000000001 0 0\nf -5 -4 -3\nf 4 2 3\n";
    let mut c = config(Format::Obj);
    c.weld_exact = true;
    let (mesh, r) = decode(bytes, &c, &WorkControl::default()).unwrap();
    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(r.merged_exact_vertices, 1);
    assert_eq!(mesh.triangles[0], mesh.triangles[1]);
    assert_eq!(
        mesh.topology(&WorkControl::default())
            .unwrap()
            .duplicate_faces,
        1
    );
}
#[test]
fn invalid_indices_polygons_attributes_nonfinite_and_degeneracy_are_rejected() {
    let base = "v 0 0 0\nv 1 0 0\nv 0 1 0\n";
    let work = WorkControl::default();
    for suffix in [
        "f 0 1 2",
        "f 1 2 4",
        "f -4 -2 -1",
        "f 1 2 3 1",
        "f 1/1 2/2 3/3",
        "vn 0 0 1",
        "mtllib x",
        "v 0 0 0 1",
        "f 1 1 2",
    ] {
        assert!(
            decode(
                format!("{base}{suffix}\n").as_bytes(),
                &config(Format::Obj),
                &work
            )
            .is_err(),
            "{suffix}"
        );
    }
    for vertex in ["NaN", "inf", "1e309"] {
        assert!(
            decode(
                format!("v {vertex} 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3").as_bytes(),
                &config(Format::Obj),
                &work
            )
            .is_err()
        );
    }
    let mut c = config(Format::Obj);
    c.units = Units::Meters;
    assert!(decode(b"v 1e307 0 0\nv 0 1 0\nv 0 0 1\nf 1 2 3", &c, &work).is_err());
    assert!(decode(&vec![b'x'; 4097], &c, &work).is_err());
}
#[test]
fn malformed_ascii_stl_and_cancelled_inputs_are_rejected() {
    let work = WorkControl::default();
    let source = source();
    let mut bytes = Vec::new();
    source.write_stl(&mut bytes, &work).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    for bad in [
        text.replace("outer loop", "outer invalid"),
        text.replace("endfacet", "endface"),
        text.replace("endsolid", "missing"),
        format!("{text}\ntrailing"),
    ] {
        assert!(decode(bad.as_bytes(), &config(Format::Stl), &work).is_err());
    }
    assert!(
        decode(
            format!("{}\n{text}", " ".repeat(4097)).as_bytes(),
            &config(Format::Stl),
            &work
        )
        .is_err()
    );
    work.cancel();
    assert!(decode(text.as_bytes(), &config(Format::Stl), &work).is_err());
}
