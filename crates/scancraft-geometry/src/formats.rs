//! Bounded ASCII PLY and versioned editable project persistence.
use crate::{Cloud, Editor, Point};
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Read, Write};
fn invalid(s: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s.into())
}
fn line(input: &mut impl BufRead) -> io::Result<String> {
    let mut out = Vec::new();
    loop {
        let bytes = input.fill_buf()?;
        if bytes.is_empty() {
            if out.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "missing PLY line",
                ));
            }
            break;
        }
        let n = bytes
            .iter()
            .position(|b| *b == b'\n')
            .map_or(bytes.len(), |p| p + 1);
        if out.len() + n > 4096 {
            return Err(invalid("PLY line exceeds 4096 bytes"));
        }
        let done = bytes[n - 1] == b'\n';
        out.extend_from_slice(&bytes[..n]);
        input.consume(n);
        if done {
            break;
        }
    }
    String::from_utf8(out)
        .map(|s| s.trim().to_string())
        .map_err(|e| invalid(e.to_string()))
}

/// Unknown file units require an explicit caller-provided conversion to mm.
pub fn read_ply(
    mut input: impl BufRead,
    unspecified_units_to_mm: Option<f64>,
) -> io::Result<Cloud> {
    if line(&mut input)? != "ply" || line(&mut input)? != "format ascii 1.0" {
        return Err(invalid("only ASCII PLY 1.0 is supported"));
    }
    let mut count = None;
    let mut props = Vec::new();
    let mut unit = None;
    let mut vertex = false;
    let mut ended = false;
    for _ in 0..256 {
        let l = line(&mut input)?;
        let words: Vec<_> = l.split_whitespace().collect();
        match words.as_slice() {
            ["end_header"] => {
                ended = true;
                break;
            }
            ["comment", "units", value] => {
                unit = Some(match *value {
                    "millimeters" | "mm" => 1.,
                    "meters" | "m" => 1000.,
                    "inches" | "inch" => 25.4,
                    _ => return Err(invalid("unsupported PLY units")),
                });
            }
            ["comment", ..] | ["obj_info", ..] => (),
            ["element", "vertex", n] => {
                if count.is_some() {
                    return Err(invalid("duplicate vertex element"));
                }
                let n: usize = n.parse().map_err(|_| invalid("invalid vertex count"))?;
                if n > 10_000_000 {
                    return Err(invalid("point count exceeds limit"));
                }
                count = Some(n);
                vertex = true;
            }
            ["element", _, "0"] => vertex = false,
            ["property", kind, name] if vertex => {
                if ![
                    "float", "float32", "double", "float64", "uchar", "uint8", "int", "uint",
                    "short", "ushort", "char",
                ]
                .contains(kind)
                {
                    return Err(invalid("unsupported PLY property type"));
                }
                if props.contains(&name.to_string()) || props.len() >= 32 {
                    return Err(invalid("duplicate or too many PLY properties"));
                }
                props.push(name.to_string());
            }
            _ => {
                return Err(invalid(
                    "unsupported PLY header; meshes/lists require a mesh importer",
                ));
            }
        }
    }
    if !ended {
        return Err(invalid("PLY header exceeds limit"));
    }
    let scale = unit
        .or(unspecified_units_to_mm)
        .ok_or_else(|| invalid("PLY has no units; choose units explicitly"))?;
    if !scale.is_finite() || scale <= 0. {
        return Err(invalid("invalid unit scale"));
    }
    let count = count.ok_or_else(|| invalid("missing vertex element"))?;
    let indices = |names: [&str; 3]| -> io::Result<Option<[usize; 3]>> {
        let p = names.map(|name| props.iter().position(|s| s == name));
        match p {
            [Some(a), Some(b), Some(c)] => Ok(Some([a, b, c])),
            [None, None, None] => Ok(None),
            _ => Err(invalid("partial vector/color property")),
        }
    };
    let xyz = indices(["x", "y", "z"])?.ok_or_else(|| invalid("missing XYZ"))?;
    let normals = indices(["nx", "ny", "nz"])?;
    let colors = indices(["red", "green", "blue"])?;
    let mut points = Vec::with_capacity(count);
    for _ in 0..count {
        let l = line(&mut input)?;
        let words: Vec<_> = l.split_whitespace().collect();
        if words.len() != props.len() {
            return Err(invalid("PLY row property count mismatch"));
        }
        let values: Vec<f64> = words
            .iter()
            .map(|s| s.parse::<f64>().map_err(|_| invalid("invalid PLY number")))
            .collect::<io::Result<_>>()?;
        if !values.iter().all(|v| v.is_finite()) {
            return Err(invalid("non-finite PLY number"));
        }
        let position = xyz.map(|i| values[i] * scale);
        let normal = normals.map(|n| n.map(|i| values[i]));
        let color = if let Some(c) = colors {
            let mut out = [0; 3];
            for a in 0..3 {
                let v = values[c[a]];
                if !(0. ..=255.).contains(&v) || v.fract() != 0. {
                    return Err(invalid("invalid RGB component"));
                }
                out[a] = v as u8;
            }
            Some(out)
        } else {
            None
        };
        points.push(Point {
            position,
            normal,
            color,
        });
    }
    if !line_remainder_empty(&mut input)? {
        return Err(invalid("unexpected trailing PLY data"));
    }
    let cloud = Cloud { points };
    cloud.validate().map_err(invalid)?;
    Ok(cloud)
}
fn line_remainder_empty(input: &mut impl BufRead) -> io::Result<bool> {
    let mut remaining = 0usize;
    loop {
        let b = input.fill_buf()?;
        if b.is_empty() {
            return Ok(true);
        }
        remaining += b.len();
        if remaining > 4096 || b.iter().any(|c| !c.is_ascii_whitespace()) {
            return Ok(false);
        }
        let n = b.len();
        input.consume(n);
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Project {
    version: u32,
    units: String,
    editor: Editor,
}
pub fn save_project(editor: &Editor, mut out: impl Write) -> io::Result<()> {
    editor.validate_history().map_err(invalid)?;
    #[derive(Serialize)]
    struct View<'a> {
        version: u32,
        units: &'static str,
        editor: &'a Editor,
    }
    let view = View {
        version: if editor.live_session().is_some() {
            8
        } else if editor.sequence().is_some() {
            7
        } else if editor.recordings().is_some() {
            6
        } else if editor.origin().is_some() {
            5
        } else if editor.has_measurement_history() {
            4
        } else if editor.measurements().is_empty() {
            2
        } else {
            3
        },
        units: "millimeters",
        editor,
    };
    let mut count = ChecksumWriter {
        hash: crc32fast::Hasher::new(),
        length: 0,
    };
    serde_json::to_writer(&mut count, &view)?;
    out.write_all(if view.version == 2 {
        b"ORPROJ02"
    } else if view.version == 3 {
        b"ORPROJ03"
    } else if view.version == 4 {
        b"ORPROJ04"
    } else if view.version == 5 {
        b"ORPROJ05"
    } else if view.version == 6 {
        b"ORPROJ06"
    } else if view.version == 7 {
        b"ORPROJ07"
    } else {
        b"ORPROJ08"
    })?;
    out.write_all(&(count.length as u32).to_le_bytes())?;
    out.write_all(&count.hash.finalize().to_le_bytes())?;
    let mut bounded = BoundedWriter {
        output: &mut out,
        remaining: 64 * 1024 * 1024,
    };
    serde_json::to_writer(&mut bounded, &view)?;
    bounded.flush()
}
struct ChecksumWriter {
    hash: crc32fast::Hasher,
    length: usize,
}
impl Write for ChecksumWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.length + bytes.len() > 64 * 1024 * 1024 {
            return Err(invalid(
                "project exceeds 64 MiB limit; reduce history or split the cloud",
            ));
        }
        self.length += bytes.len();
        self.hash.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct BoundedWriter<W: Write> {
    output: W,
    remaining: usize,
}
impl<W: Write> Write for BoundedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(invalid(
                "project exceeds 64 MiB limit; reduce history or split the cloud",
            ));
        }
        let n = self.output.write(bytes)?;
        self.remaining -= n;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}
pub fn load_project(input: impl Read) -> io::Result<Editor> {
    let mut bytes = Vec::new();
    input.take(64 * 1024 * 1024 + 17).read_to_end(&mut bytes)?;
    if bytes.len() < 16
        || bytes.len() > 64 * 1024 * 1024 + 16
        || !matches!(
            &bytes[..8],
            b"ORPROJ02"
                | b"ORPROJ03"
                | b"ORPROJ04"
                | b"ORPROJ05"
                | b"ORPROJ06"
                | b"ORPROJ07"
                | b"ORPROJ08"
        )
    {
        return Err(invalid("unsupported or oversized project file"));
    }
    let length = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let expected = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    if length != bytes.len() - 16 || crc32fast::hash(&bytes[16..]) != expected {
        return Err(invalid("project length/checksum mismatch"));
    }
    let p: Project = serde_json::from_slice(&bytes[16..])?;
    let version = match &bytes[..8] {
        b"ORPROJ02" => 2,
        b"ORPROJ03" => 3,
        b"ORPROJ04" => 4,
        b"ORPROJ05" => 5,
        b"ORPROJ06" => 6,
        b"ORPROJ07" => 7,
        _ => 8,
    };
    if p.version != version
        || p.units != "millimeters"
        || (version == 2 && !p.editor.measurements().is_empty())
        || (version < 4 && p.editor.has_measurement_history())
        || (version < 5 && p.editor.origin().is_some())
        || ((5..=6).contains(&version) && p.editor.origin().is_none())
        || (version < 6 && p.editor.recordings().is_some())
        || (version == 6 && p.editor.recordings().is_none())
        || (version < 8 && p.editor.live_session().is_some())
        || (version == 8 && p.editor.live_session().is_none())
        || (version < 7 && p.editor.sequence().is_some())
        || (version == 7
            && (p.editor.sequence().is_none()
                || p.editor.origin().is_some()
                || p.editor.recordings().is_some()))
    {
        return Err(invalid("unsupported project schema or units"));
    }
    p.editor.validate_history().map_err(invalid)?;
    Ok(p.editor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::write_ply;
    fn sample() -> Cloud {
        Cloud {
            points: vec![Point {
                position: [1.2345678901234567, -1e-20, 1e20],
                color: Some([1, 2, 255]),
                normal: Some([0., 1., 0.]),
            }],
        }
    }
    #[test]
    fn ply_exact_roundtrip() {
        let c = sample();
        let mut b = Vec::new();
        write_ply(&c, &mut b).unwrap();
        assert_eq!(read_ply(b.as_slice(), None).unwrap(), c);
    }
    #[test]
    fn corrupt_and_oversized_ply_rejected() {
        assert!(
            read_ply(
                b"ply\nformat ascii 1.0\nelement vertex 10000001\n".as_slice(),
                Some(1.)
            )
            .is_err()
        );
        assert!(read_ply(vec![b'a'; 5000].as_slice(), Some(1.)).is_err());
        let mut b = Vec::new();
        write_ply(&sample(), &mut b).unwrap();
        b.truncate(b.len() - 10);
        assert!(read_ply(b.as_slice(), None).is_err());
    }
    #[test]
    fn unknown_units_require_choice() {
        let data=b"ply\nformat ascii 1.0\nelement vertex 1\nproperty double x\nproperty double y\nproperty double z\nend_header\n1 2 3\n";
        assert!(read_ply(data.as_slice(), None).is_err());
        let c = read_ply(data.as_slice(), Some(1000.)).unwrap();
        assert_eq!(c.points[0].position, [1000., 2000., 3000.]);
    }
    #[test]
    fn project_roundtrip_restores_undo() {
        let c = sample();
        let mut e = Editor::new(c.clone(), 8).unwrap();
        e.trim(&[0], false).unwrap();
        let mut b = Vec::new();
        save_project(&e, &mut b).unwrap();
        let mut loaded = load_project(b.as_slice()).unwrap();
        assert_eq!(loaded.cloud.points.len(), 0);
        assert!(loaded.undo());
        assert_eq!(loaded.cloud, c);
        assert!(loaded.redo());
        assert!(loaded.cloud.points.is_empty());
    }
    #[test]
    fn project_preserves_difficult_f64_bits_and_undo_history() {
        let mut values = vec![
            400.00000000001523,
            -0.0,
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            f64::MAX,
            1. + f64::EPSILON,
        ];
        let mut bits = 0x12345678abcdef01u64;
        for _ in 0..512 {
            bits ^= bits << 13;
            bits ^= bits >> 7;
            bits ^= bits << 17;
            let v = f64::from_bits(bits);
            if v.is_finite() {
                values.push(v);
            }
        }
        let cloud = Cloud {
            points: values
                .chunks(3)
                .map(|v| Point {
                    position: std::array::from_fn(|i| v[i % v.len()]),
                    color: None,
                    normal: None,
                })
                .collect(),
        };
        let mut editor = Editor::new(cloud.clone(), 8).unwrap();
        editor.trim(&[0], false).unwrap();
        let mut bytes = Vec::new();
        save_project(&editor, &mut bytes).unwrap();
        let mut loaded = load_project(bytes.as_slice()).unwrap();
        assert!(loaded.undo());
        assert_eq!(loaded.cloud.points.len(), cloud.points.len());
        for (expected, actual) in cloud.points.iter().zip(&loaded.cloud.points) {
            assert_eq!(
                expected.position.map(f64::to_bits),
                actual.position.map(f64::to_bits)
            );
        }
    }
    #[test]
    fn project_corruption_and_all_truncations_rejected() {
        let e = Editor::new(sample(), 8).unwrap();
        let mut original = Vec::new();
        save_project(&e, &mut original).unwrap();
        for n in 0..original.len() {
            assert!(load_project(&original[..n]).is_err());
        }
        for n in 0..original.len() {
            let mut b = original.clone();
            b[n] ^= 1;
            assert!(load_project(b.as_slice()).is_err());
        }
    }
    #[test]
    fn invalid_project_rejected() {
        assert!(
            load_project(b"{\"version\":99,\"units\":\"mm\",\"editor\":{}}".as_slice()).is_err()
        );
    }
}
