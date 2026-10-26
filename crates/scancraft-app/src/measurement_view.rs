//! Historical analytic fits; these lines are not acquired surface geometry.
use eframe::egui;
use scancraft_geometry::{
    measurement_snapshot::{ResultValue, Snapshot},
    selection::OrthographicProjection,
};

type RingBasis = ([f64; 3], [f64; 3]);
fn rings(result: &ResultValue) -> ([f64; 3], f64, Vec<RingBasis>) {
    match result {
        ResultValue::Distance { .. }
        | ResultValue::Angle { .. }
        | ResultValue::PointPlane { .. }
        | ResultValue::Cylinder { .. } => ([0.; 3], 0., Vec::new()),
        ResultValue::Sphere { fit } => (
            fit.center_mm,
            fit.radius_mm,
            vec![
                ([1., 0., 0.], [0., 1., 0.]),
                ([1., 0., 0.], [0., 0., 1.]),
                ([0., 1., 0.], [0., 0., 1.]),
            ],
        ),
        ResultValue::Circle { fit, .. } => {
            let n = nalgebra::Vector3::from(fit.normal);
            let axis = (0..3)
                .min_by(|&a, &b| n[a].abs().total_cmp(&n[b].abs()))
                .unwrap();
            let mut reference = nalgebra::Vector3::zeros();
            reference[axis] = 1.;
            let u = n.cross(&reference).normalize();
            let v = n.cross(&u).normalize();
            (fit.center_mm, fit.radius_mm, vec![(u.into(), v.into())])
        }
    }
}
fn offset(radius: f64, u: [f64; 3], v: [f64; 3], step: usize) -> [f64; 3] {
    let angle = std::f64::consts::TAU * (step % 64) as f64 / 64.;
    std::array::from_fn(|a| radius * (u[a] * angle.cos() + v[a] * angle.sin()))
}
/// Offsets from the retained axis point, not world-coordinate additions.
fn cylinder_segments(fit: &scancraft_geometry::cylinder::Cylinder) -> Vec<([f64; 3], [f64; 3])> {
    let axis = nalgebra::Vector3::from(fit.axis_direction).normalize();
    let index = (0..3)
        .min_by(|&a, &b| axis[a].abs().total_cmp(&axis[b].abs()))
        .unwrap();
    let mut reference = nalgebra::Vector3::zeros();
    reference[index] = 1.;
    let u = axis.cross(&reference).normalize();
    let v = axis.cross(&u);
    let point = |t: f64, step: usize| -> [f64; 3] {
        let radial = offset(fit.radius_mm, u.into(), v.into(), step);
        std::array::from_fn(|a| axis[a] * t + radial[a])
    };
    let mut segments = Vec::with_capacity(132);
    for t in fit.axial_range_mm {
        for step in 0..64 {
            segments.push((point(t, step), point(t, step + 1)));
        }
    }
    for step in [0, 16, 32, 48] {
        segments.push((
            point(fit.axial_range_mm[0], step),
            point(fit.axial_range_mm[1], step),
        ));
    }
    segments
}
// Greedy placement is bounded and deterministic. Omit a label rather than draw
// it over another; the saved-measurement panel always retains its full value.
fn place_label(
    view: egui::Rect,
    anchor: egui::Pos2,
    size: egui::Vec2,
    occupied: &[egui::Rect],
) -> Option<egui::Rect> {
    let area = view.shrink(4.);
    if !area.is_positive()
        || !anchor.is_finite()
        || !size.is_finite()
        || size.x <= 0.
        || size.y <= 0.
        || size.x > area.width()
        || size.y > area.height()
    {
        return None;
    }
    let x = (anchor.x + 6.).clamp(area.left(), area.right() - size.x);
    let desired = (anchor.y - size.y - 6.).clamp(area.top(), area.bottom() - size.y);
    for step in 0usize..=64 {
        let shift = step.div_ceil(2) as f32 * (size.y + 4.) * if step % 2 == 0 { 1. } else { -1. };
        let y = (desired + shift).clamp(area.top(), area.bottom() - size.y);
        let candidate = egui::Rect::from_min_size(egui::pos2(x, y), size);
        if occupied.iter().all(|r| !r.expand(2.).intersects(candidate)) {
            return Some(candidate);
        }
    }
    None
}
pub fn draw(
    records: &[Snapshot],
    projection: &OrthographicProjection,
    painter: &egui::Painter,
    rect: egui::Rect,
) {
    let painter = painter.with_clip_rect(rect);
    let color = egui::Color32::from_rgb(220, 160, 255);
    let mut labels = Vec::new();
    for record in records.iter().take(16) {
        if let ResultValue::Cylinder { fit, .. } = &record.result {
            for (a, b) in cylinder_segments(fit) {
                if let (Some(a), Some(b)) = (
                    projection.project_offset(fit.axis_point_mm, a),
                    projection.project_offset(fit.axis_point_mm, b),
                ) {
                    let a = egui::pos2(a.xy[0] as f32, a.xy[1] as f32);
                    let b = egui::pos2(b.xy[0] as f32, b.xy[1] as f32);
                    if a.is_finite() && b.is_finite() {
                        painter.line_segment([a, b], egui::Stroke::new(1_f32, color));
                    }
                }
            }
            if let Some(p) = projection.project(fit.axis_point_mm)
                && projection.in_viewport(p.xy)
            {
                labels.push((
                    egui::pos2(p.xy[0] as f32, p.xy[1] as f32),
                    format!("{} (historical cylinder; sample span)", record.label),
                ));
            }
            continue;
        }
        if let ResultValue::PointPlane { fit, probe, .. } = &record.result {
            if let (Some(a), Some(b)) = (
                projection.project(record.source_points_mm[*probe]),
                projection.project(fit.foot_mm),
            ) {
                let a = egui::pos2(a.xy[0] as f32, a.xy[1] as f32);
                let b = egui::pos2(b.xy[0] as f32, b.xy[1] as f32);
                if a.is_finite() && b.is_finite() {
                    painter.line_segment([a, b], egui::Stroke::new(1_f32, color));
                }
                if projection.in_viewport([f64::from(a.x), f64::from(a.y)]) {
                    labels.push((
                        a,
                        format!(
                            "{}: {:.6} mm to plane (historical)",
                            record.label, fit.distance_mm
                        ),
                    ));
                }
            }
            continue;
        }
        let direct = match record.result {
            ResultValue::Distance { distance_mm } => Some((
                vec![(0, 1)],
                0,
                format!("{}: {distance_mm:.6} mm (historical)", record.label),
            )),
            ResultValue::Angle { degrees, vertex } => Some((
                (0..3)
                    .filter(|&i| i != vertex)
                    .map(|i| (vertex, i))
                    .collect(),
                vertex,
                format!("{}: {degrees:.6} degrees (historical)", record.label),
            )),
            _ => None,
        };
        if let Some((edges, label_index, label)) = direct {
            for (a, b) in edges {
                if let (Some(a), Some(b)) = (
                    projection.project(record.source_points_mm[a]),
                    projection.project(record.source_points_mm[b]),
                ) {
                    let a = egui::pos2(a.xy[0] as f32, a.xy[1] as f32);
                    let b = egui::pos2(b.xy[0] as f32, b.xy[1] as f32);
                    if a.is_finite() && b.is_finite() {
                        painter.line_segment([a, b], egui::Stroke::new(1_f32, color));
                    }
                }
            }
            if let Some(p) = projection.project(record.source_points_mm[label_index])
                && projection.in_viewport(p.xy)
            {
                labels.push((egui::pos2(p.xy[0] as f32, p.xy[1] as f32), label));
            }
            continue;
        }
        let (center, radius, rings) = rings(&record.result);
        for (u, v) in rings {
            for step in 0..64 {
                let points = [step, step + 1].map(|i| {
                    projection
                        .project_offset(center, offset(radius, u, v, i))
                        .map(|p| egui::pos2(p.xy[0] as f32, p.xy[1] as f32))
                });
                if let [Some(a), Some(b)] = points
                    && a.is_finite()
                    && b.is_finite()
                {
                    painter.line_segment([a, b], egui::Stroke::new(1_f32, color));
                }
            }
        }
        if let Some(p) = projection.project(center)
            && projection.in_viewport(p.xy)
        {
            labels.push((
                egui::pos2(p.xy[0] as f32, p.xy[1] as f32),
                format!("{} (historical fit)", record.label),
            ));
        }
    }
    let mut occupied = Vec::new();
    for (anchor, text) in labels {
        let galley = painter.layout(
            text,
            egui::FontId::proportional(12.),
            color,
            (rect.width() - 16.).max(1.),
        );
        if let Some(label_rect) =
            place_label(rect, anchor, galley.size() + egui::vec2(6., 4.), &occupied)
        {
            painter.line_segment(
                [anchor, label_rect.clamp(anchor)],
                egui::Stroke::new(1_f32, color.gamma_multiply(0.6)),
            );
            painter.rect_filled(label_rect, 2., egui::Color32::from_rgb(13, 20, 30));
            painter.galley(label_rect.min + egui::vec2(3., 2.), galley, color);
            occupied.push(label_rect);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_geometry::{measurement_snapshot::Snapshot, work::WorkControl};
    #[test]
    fn cylinder_outline_preserves_radius_sample_span_and_local_offsets() {
        let axis = nalgebra::Vector3::new(1., 2., 3.).normalize();
        let mut fit = scancraft_geometry::cylinder::Cylinder {
            axis_point_mm: [0.; 3],
            axis_direction: axis.into(),
            radius_mm: 3.,
            diameter_mm: 6.,
            geometric_rms_mm: 0.,
            maximum_absolute_residual_mm: 0.,
            axial_range_mm: [-6., 6.],
            jacobian_condition_number: 1.,
            points: 6,
            iterations: 1,
            starts_converged: 1,
        };
        let segments = cylinder_segments(&fit);
        assert_eq!(segments.len(), 132);
        for &(a, b) in &segments[..128] {
            for point in [a, b] {
                let p = nalgebra::Vector3::from(point);
                let t = p.dot(&axis);
                assert!((t.abs() - 6.).abs() < 1e-12);
                assert!(((p - axis * t).norm() - 3.).abs() < 1e-12);
            }
        }
        for &(a, b) in &segments[128..] {
            assert!((scancraft_geometry::distance_mm(a, b).unwrap() - 12.).abs() < 1e-12);
        }
        fit.axis_point_mm = [1e100; 3];
        assert_eq!(cylinder_segments(&fit), segments);
    }
    #[test]
    fn coincident_labels_are_separated_and_stay_inside_viewport() {
        let view = egui::Rect::from_min_size(egui::pos2(10., 20.), egui::vec2(500., 500.));
        let mut placed = Vec::new();
        for _ in 0..16 {
            let r = place_label(view, view.right_bottom(), egui::vec2(200., 20.), &placed).unwrap();
            assert!(view.shrink(4.).contains_rect(r));
            assert!(
                placed
                    .iter()
                    .all(|old: &egui::Rect| !old.expand(2.).intersects(r))
            );
            placed.push(r);
        }
    }
    #[test]
    fn corner_labels_are_clamped_and_placement_is_deterministic() {
        let view = egui::Rect::from_min_size(egui::pos2(0., 0.), egui::vec2(500., 300.));
        for anchor in [
            view.left_top(),
            view.left_bottom(),
            view.right_top(),
            view.right_bottom(),
        ] {
            let a = place_label(view, anchor, egui::vec2(200., 20.), &[]).unwrap();
            assert!(view.shrink(4.).contains_rect(a));
            assert_eq!(
                Some(a),
                place_label(view, anchor, egui::vec2(200., 20.), &[])
            );
        }
    }
    #[test]
    fn crowded_or_invalid_view_omits_labels_instead_of_overlapping() {
        let view = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100., 30.));
        let size = egui::vec2(80., 20.);
        let first = place_label(view, view.center(), size, &[]).unwrap();
        assert!(place_label(view, view.center(), size, &[first]).is_none());
        assert!(place_label(view, view.center(), egui::vec2(200., 20.), &[]).is_none());
        assert!(place_label(view, egui::pos2(f32::NAN, 0.), size, &[]).is_none());
    }
    #[test]
    fn circle_ring_preserves_radius_and_plane() {
        let points = (0..8)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 8.;
                [3. * a.cos(), 0., 3. * a.sin()]
            })
            .collect();
        let snapshot =
            Snapshot::circle("circle".into(), points, 1e-10, &WorkControl::default()).unwrap();
        let (_, r, bases) = rings(&snapshot.result);
        assert_eq!(bases.len(), 1);
        for (u, v) in bases {
            for i in 0..=64 {
                let p = offset(r, u, v, i);
                assert!(p[1].abs() < 1e-10);
                assert!((p.iter().map(|x| x * x).sum::<f64>().sqrt() - 3.).abs() < 1e-10);
            }
        }
    }
    #[test]
    fn sphere_has_three_closed_great_circles() {
        let points = vec![
            [2., 0., 0.],
            [-2., 0., 0.],
            [0., 2., 0.],
            [0., -2., 0.],
            [0., 0., 2.],
            [0., 0., -2.],
        ];
        let snapshot = Snapshot::sphere("sphere".into(), points, &WorkControl::default()).unwrap();
        let (_, r, bases) = rings(&snapshot.result);
        assert_eq!(bases.len(), 3);
        for (u, v) in bases {
            assert_eq!(offset(r, u, v, 0), offset(r, u, v, 64));
            for i in 0..64 {
                let p = offset(r, u, v, i);
                assert!((p.iter().map(|x| x * x).sum::<f64>() - 4.).abs() < 1e-10);
            }
        }
    }
}
