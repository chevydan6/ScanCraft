//! Bounded triangle OBJ/STL import with explicit units and exact-welding policy.
use crate::{mesh::Mesh, work::WorkControl};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Obj,
    Stl,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Units {
    Millimeters,
    Inches,
    Meters,
}
impl Units {
    fn factor(self) -> f64 {
        match self {
            Self::Millimeters => 1.,
            Self::Inches => 25.4,
            Self::Meters => 1000.,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub format: Format,
    pub units: Units,
    pub weld_exact: bool,
}
impl Config {
    pub fn from_labels(format: &str, units: &str, welding: &str) -> Result<Self, String> {
        Ok(Self {
            format: match format {
                "obj" => Format::Obj,
                "stl" => Format::Stl,
                _ => return Err("expected obj or stl".into()),
            },
            units: match units {
                "mm" => Units::Millimeters,
                "in" => Units::Inches,
                "m" => Units::Meters,
                _ => return Err("expected explicit mm, in or m units".into()),
            },
            weld_exact: match welding {
                "separate" => false,
                "weld-exact" => true,
                _ => return Err("expected separate or weld-exact".into()),
            },
        })
    }
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub source_vertex_records: usize,
    pub result_vertices: usize,
    pub triangles: usize,
    pub merged_exact_vertices: usize,
    pub facet_normals_recomputed: bool,
}
struct Builder<'a> {
    mesh: Mesh,
    keys: BTreeMap<[u64; 3], u32>,
    records: usize,
    config: &'a Config,
    work: &'a WorkControl,
}
impl Builder<'_> {
    fn vertex(&mut self, p: [f64; 3]) -> Result<u32, String> {
        self.work.check()?;
        self.records += 1;
        let p = p.map(|x| x * self.config.units.factor());
        if !p.iter().all(|x| x.is_finite()) {
            return Err("nonfinite or overflowing imported vertex".into());
        }
        let key = p.map(|x| if x == 0. { 0 } else { x.to_bits() });
        if self.config.weld_exact
            && let Some(&id) = self.keys.get(&key)
        {
            return Ok(id);
        }
        if self.mesh.vertices.len() == 1_000_000 {
            return Err("import exceeds one million result vertices".into());
        }
        let id = self.mesh.vertices.len() as u32;
        self.mesh.vertices.push(p);
        if self.config.weld_exact {
            self.keys.insert(key, id);
        }
        Ok(id)
    }
    fn face(&mut self, t: [u32; 3]) -> Result<(), String> {
        self.work.check()?;
        if self.mesh.triangles.len() == 2_000_000 {
            return Err("import exceeds two million triangles".into());
        }
        self.mesh.triangles.push(t);
        Ok(())
    }
}
fn triple(tokens: &[&str]) -> Result<[f64; 3], String> {
    if tokens.len() != 3 {
        return Err("expected exactly three coordinates".into());
    }
    let mut p = [0_f64; 3];
    for (out, token) in p.iter_mut().zip(tokens) {
        *out = token.parse().map_err(|_| "invalid coordinate")?;
    }
    if !p.iter().all(|x| x.is_finite()) {
        return Err("nonfinite coordinate or normal".into());
    }
    Ok(p)
}
pub fn decode(bytes: &[u8], config: &Config, work: &WorkControl) -> Result<(Mesh, Report), String> {
    work.check()?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("mesh import exceeds 128 MiB".into());
    }
    let mut b = Builder {
        mesh: Mesh {
            vertices: Vec::new(),
            triangles: Vec::new(),
        },
        keys: BTreeMap::new(),
        records: 0,
        config,
        work,
    };
    match config.format {
        Format::Obj => obj(bytes, &mut b)?,
        Format::Stl => stl(bytes, &mut b)?,
    }
    if b.mesh.triangles.is_empty() {
        return Err("mesh import contains no triangles".into());
    }
    b.mesh.validate(work)?;
    let report = Report {
        source_vertex_records: b.records,
        result_vertices: b.mesh.vertices.len(),
        triangles: b.mesh.triangles.len(),
        merged_exact_vertices: b.records - b.mesh.vertices.len(),
        facet_normals_recomputed: config.format == Format::Stl,
    };
    Ok((b.mesh, report))
}
fn obj(bytes: &[u8], b: &mut Builder<'_>) -> Result<(), String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "OBJ must be UTF-8 text")?;
    let mut indices = Vec::new();
    for (line_number, line) in text.lines().enumerate() {
        b.work.check()?;
        if line.len() > 4096 {
            return Err("OBJ line exceeds 4096 bytes".into());
        }
        let tokens: Vec<_> = line.split('#').next().unwrap().split_whitespace().collect();
        let Some(&kind) = tokens.first() else {
            continue;
        };
        let error = |s: &str| format!("OBJ line {}: {s}", line_number + 1);
        match kind {
            "v" => {
                if indices.len() == 1_000_000 {
                    return Err(error("source vertex budget exceeded"));
                }
                indices.push(b.vertex(triple(&tokens[1..]).map_err(|e| error(&e))?)?);
            }
            "f" => {
                if tokens.len() != 4 {
                    return Err(error(
                        "only triangular faces are supported; triangulate polygons explicitly",
                    ));
                }
                let mut face = [0; 3];
                for (out, token) in face.iter_mut().zip(&tokens[1..]) {
                    let raw:i64=token.parse().map_err(|_|error("expected vertex-only face indices; texture/normal attributes are unsupported"))?;
                    let index = if raw > 0 {
                        raw - 1
                    } else if raw < 0 {
                        indices.len() as i64 + raw
                    } else {
                        -1
                    };
                    if index < 0 || index >= indices.len() as i64 {
                        return Err(error("face index is zero, forward, or out of range"));
                    }
                    *out = indices[index as usize];
                }
                b.face(face)?;
            }
            _ => {
                return Err(error(
                    "unsupported OBJ record; attributes/groups/materials are not silently discarded",
                ));
            }
        }
    }
    Ok(())
}
fn stl(bytes: &[u8], b: &mut Builder<'_>) -> Result<(), String> {
    if bytes.len() >= 84 {
        let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
        if count.checked_mul(50).and_then(|x| x.checked_add(84)) == Some(bytes.len()) {
            if count > 2_000_000 {
                return Err("STL triangle budget exceeded".into());
            }
            for facet in bytes[84..].as_chunks::<50>().0 {
                b.work.check()?;
                if facet[48..50] != [0, 0] {
                    return Err("binary STL attributes/color are unsupported".into());
                }
                let read = |offset| {
                    f32::from_le_bytes(facet[offset..offset + 4].try_into().unwrap()) as f64
                };
                if !(0..3).all(|i| read(i * 4).is_finite()) {
                    return Err("nonfinite STL facet normal".into());
                }
                let mut t = [0; 3];
                for (i, index) in t.iter_mut().enumerate() {
                    *index = b.vertex(std::array::from_fn(|axis| read(12 + i * 12 + axis * 4)))?;
                }
                b.face(t)?;
            }
            return Ok(());
        }
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "binary STL count/length mismatch or invalid ASCII STL")?;
    for line in text.lines() {
        b.work.check()?;
        if line.len() > 4096 {
            return Err("STL line exceeds 4096 bytes".into());
        }
    }
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let first = lines.next().ok_or("empty STL")?;
    if first.len() > 4096 || first.split_whitespace().next() != Some("solid") {
        return Err("invalid STL header or binary count/length".into());
    }
    let mut closed = false;
    while let Some(line) = lines.next() {
        b.work.check()?;
        if line.len() > 4096 {
            return Err("STL line exceeds 4096 bytes".into());
        }
        let tokens: Vec<_> = line.split_whitespace().collect();
        if tokens.first() == Some(&"endsolid") {
            closed = true;
            break;
        }
        if tokens.len() != 5 || tokens[..2] != ["facet", "normal"] {
            return Err("expected STL facet normal".into());
        }
        triple(&tokens[2..])?;
        expect(&mut lines, &["outer", "loop"])?;
        let mut t = [0; 3];
        for index in &mut t {
            b.work.check()?;
            let line = lines.next().ok_or("truncated STL vertex")?;
            if line.len() > 4096 {
                return Err("STL line exceeds 4096 bytes".into());
            }
            let tokens: Vec<_> = line.split_whitespace().collect();
            if tokens.first() != Some(&"vertex") {
                return Err("expected STL vertex".into());
            }
            *index = b.vertex(triple(&tokens[1..])?)?;
        }
        expect(&mut lines, &["endloop"])?;
        expect(&mut lines, &["endfacet"])?;
        b.face(t)?;
    }
    if !closed || lines.next().is_some() {
        return Err("missing STL endsolid or trailing content".into());
    }
    Ok(())
}
fn expect<'a>(lines: &mut impl Iterator<Item = &'a str>, tokens: &[&str]) -> Result<(), String> {
    let line = lines.next().ok_or("truncated STL")?;
    if line.len() > 4096 || !line.split_whitespace().eq(tokens.iter().copied()) {
        return Err("unexpected STL record".into());
    }
    Ok(())
}
