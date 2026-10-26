//! Viewport gesture state; geometry selection itself always runs on the full cloud worker.
use eframe::egui;
use scancraft_geometry::selection::{
    OrthographicProjection, ScreenShape, SelectionMode, Visibility,
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Navigate,
    Rectangle,
    Polygon,
    Lasso,
    Brush,
}
struct Gesture {
    tool: Tool,
    points: Vec<egui::Pos2>,
    projection: OrthographicProjection,
    viewport: egui::Rect,
}
pub struct ScreenRequest {
    pub projection: OrthographicProjection,
    pub shape: ScreenShape,
    pub mode: SelectionMode,
    pub visibility: Visibility,
}
pub struct ScreenTools {
    pub tool: Tool,
    mode: SelectionMode,
    frontmost: bool,
    radius: f64,
    gesture: Option<Gesture>,
    apply: bool,
    status: String,
}
impl Default for ScreenTools {
    fn default() -> Self {
        Self {
            tool: Tool::Navigate,
            mode: SelectionMode::Replace,
            frontmost: false,
            radius: 12.,
            gesture: None,
            apply: false,
            status: String::new(),
        }
    }
}
impl ScreenTools {
    pub fn active(&self) -> bool {
        self.gesture.is_some()
    }
    pub fn cancel(&mut self) {
        self.gesture = None;
        self.apply = false;
    }
    pub fn toolbar(&mut self, ui: &mut egui::Ui, enabled: bool) {
        let before = self.tool;
        ui.add_enabled_ui(enabled, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Screen tool:");
                for (label, tool) in [
                    ("Navigate", Tool::Navigate),
                    ("Rectangle", Tool::Rectangle),
                    ("Polygon", Tool::Polygon),
                    ("Lasso", Tool::Lasso),
                    ("Brush", Tool::Brush),
                ] {
                    ui.selectable_value(&mut self.tool, tool, label);
                }
                egui::ComboBox::from_id_salt("screen-selection-mode")
                    .selected_text(match self.mode {
                        SelectionMode::Replace => "Replace",
                        SelectionMode::Add => "Add",
                        SelectionMode::Subtract => "Subtract",
                        SelectionMode::Intersect => "Intersect",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.mode, SelectionMode::Replace, "Replace");
                        ui.selectable_value(&mut self.mode, SelectionMode::Add, "Add");
                        ui.selectable_value(&mut self.mode, SelectionMode::Subtract, "Subtract");
                        ui.selectable_value(&mut self.mode, SelectionMode::Intersect, "Intersect");
                    });
                ui.checkbox(&mut self.frontmost,"Frontmost per pixel").on_hover_text("Chooses the nearest point in each logical-pixel cell using the complete cloud.");
                if self.tool == Tool::Brush {
                    ui.label("Radius (px):");
                    ui.add(egui::DragValue::new(&mut self.radius).range(1. ..=200.));
                }
                if self.tool == Tool::Polygon
                    && ui
                        .add_enabled(
                            self.gesture.as_ref().is_some_and(|g| g.points.len() >= 3),
                            egui::Button::new("Apply polygon"),
                        )
                        .clicked()
                {
                    self.apply = true;
                }
                if ui.button("Cancel gesture").clicked() {
                    self.cancel();
                }
            });
        });
        if before != self.tool {
            self.cancel();
        }
        let instruction = if self.status.is_empty() {
            "Drag to select. Polygon: click vertices, then Enter, double-click or Apply. Esc cancels. Right-drag orbits between gestures."
        } else {
            &self.status
        };
        ui.add(egui::Label::new(instruction).truncate());
    }
    fn begin(
        &mut self,
        point: egui::Pos2,
        projection: OrthographicProjection,
        viewport: egui::Rect,
    ) {
        self.status.clear();
        self.gesture = Some(Gesture {
            tool: self.tool,
            points: vec![point],
            projection,
            viewport,
        });
    }
    fn finish(&mut self) -> Option<ScreenRequest> {
        self.apply = false;
        let gesture = self.gesture.take()?;
        let coords: Vec<_> = gesture
            .points
            .iter()
            .map(|p| [f64::from(p.x), f64::from(p.y)])
            .collect();
        let shape = match gesture.tool {
            Tool::Rectangle => {
                let a = *coords.first()?;
                let b = *coords.last()?;
                ScreenShape::Rectangle {
                    min: [a[0].min(b[0]), a[1].min(b[1])],
                    max: [a[0].max(b[0]), a[1].max(b[1])],
                }
            }
            Tool::Polygon | Tool::Lasso => {
                if coords.len() < 3 {
                    self.status = "A polygon/lasso needs at least three vertices.".into();
                    return None;
                }
                ScreenShape::Polygon(coords)
            }
            Tool::Brush => ScreenShape::Brush {
                centers: coords,
                radius: self.radius,
            },
            Tool::Navigate => return None,
        };
        Some(ScreenRequest {
            projection: gesture.projection,
            shape,
            mode: self.mode,
            visibility: if self.frontmost {
                Visibility::FrontmostPerPixel { cell_size: 1. }
            } else {
                Visibility::Through
            },
        })
    }
    pub fn interact(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        painter: &egui::Painter,
        projection: Option<OrthographicProjection>,
        enabled: bool,
    ) -> Option<ScreenRequest> {
        if !enabled {
            self.cancel();
            return None;
        }
        if self
            .gesture
            .as_ref()
            .is_some_and(|g| g.viewport != response.rect)
        {
            self.cancel();
            self.status = "Gesture cancelled because the viewport changed size.".into();
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.cancel();
            return None;
        }
        let pointer = response
            .interact_pointer_pos()
            .or_else(|| ui.input(|i| i.pointer.latest_pos()));
        let primary = egui::PointerButton::Primary;
        let mut complete = false;
        if self.tool == Tool::Polygon {
            if response.clicked_by(primary)
                && let (Some(point), Some(projection)) = (pointer, projection)
            {
                let near_last = self
                    .gesture
                    .as_ref()
                    .and_then(|g| g.points.last())
                    .is_some_and(|last| last.distance_sq(point) <= 36.);
                response.request_focus();
                if self.gesture.is_none() {
                    self.begin(point, projection, response.rect);
                } else if let Some(g) = &mut self.gesture
                    && g.points.last().is_none_or(|p| p.distance_sq(point) > 1.)
                {
                    g.points.push(point);
                }
                if response.double_clicked_by(primary)
                    && near_last
                    && self.gesture.as_ref().is_some_and(|g| g.points.len() >= 3)
                {
                    complete = true;
                }
            }
            if self.apply
                || (self.active()
                    && response.has_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                complete = true;
            }
        } else if matches!(self.tool, Tool::Rectangle | Tool::Lasso | Tool::Brush) {
            if response.drag_started_by(primary)
                && let (Some(point), Some(projection)) =
                    (ui.input(|i| i.pointer.press_origin()), projection)
            {
                response.request_focus();
                self.begin(point, projection, response.rect);
            }
            if (response.dragged_by(primary) || response.drag_stopped_by(primary))
                && let (Some(point), Some(g)) = (pointer, &mut self.gesture)
            {
                if g.tool == Tool::Rectangle {
                    if g.points.len() == 1 {
                        g.points.push(point);
                    } else {
                        g.points[1] = point;
                    }
                } else if g.points.last().is_none_or(|p| {
                    p.distance_sq(point) >= 4.
                        || (response.drag_stopped_by(primary) && p.distance_sq(point) > 0.)
                }) {
                    g.points.push(point);
                }
                if response.drag_stopped_by(primary) {
                    complete = true;
                }
            }
            if response.drag_stopped_by(primary) && self.active() {
                complete = true;
            }
            if response.clicked_by(primary)
                && self.tool == Tool::Brush
                && let (Some(point), Some(projection)) = (pointer, projection)
            {
                self.begin(point, projection, response.rect);
                complete = true;
            }
        }
        if self.gesture.as_ref().is_some_and(|g| g.points.len() > 8192) {
            self.cancel();
            self.status = "Stroke exceeded 8192 vertices; use a shorter stroke.".into();
            return None;
        }
        if complete {
            return self.finish();
        }
        let painter = painter.with_clip_rect(response.rect);
        let stroke = egui::Stroke::new(1.5_f32, egui::Color32::LIGHT_BLUE);
        if let Some(g) = &self.gesture {
            match g.tool {
                Tool::Rectangle => {
                    if let (Some(a), Some(b)) = (g.points.first(), g.points.last()) {
                        let r = egui::Rect::from_two_pos(*a, *b);
                        painter.add(egui::Shape::closed_line(
                            vec![
                                r.left_top(),
                                r.right_top(),
                                r.right_bottom(),
                                r.left_bottom(),
                            ],
                            stroke,
                        ));
                    }
                }
                Tool::Polygon | Tool::Lasso => {
                    painter.add(egui::Shape::closed_line(g.points.clone(), stroke));
                }
                Tool::Brush => {
                    let color = egui::Color32::from_rgba_unmultiplied(80, 160, 255, 80);
                    for segment in g.points.windows(2) {
                        painter.line_segment(
                            [segment[0], segment[1]],
                            egui::Stroke::new(self.radius as f32 * 2., color),
                        );
                    }
                }
                Tool::Navigate => (),
            }
        }
        if self.tool == Tool::Brush
            && response.hovered()
            && let Some(point) = pointer
        {
            painter.circle_stroke(point, self.radius as f32, stroke);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Harness {
        ctx: egui::Context,
        time: f64,
    }
    impl Harness {
        fn new() -> Self {
            Self {
                ctx: egui::Context::default(),
                time: 0.,
            }
        }
        fn frame(
            &mut self,
            tools: &mut ScreenTools,
            events: Vec<egui::Event>,
            size: f32,
        ) -> Option<ScreenRequest> {
            self.time += 0.1;
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(size, size));
            let mut request = None;
            let _ = self.ctx.run(
                egui::RawInput {
                    screen_rect: Some(rect),
                    events,
                    time: Some(self.time),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let response = ui.interact(
                            rect,
                            egui::Id::new("gesture-view"),
                            egui::Sense::click_and_drag(),
                        );
                        let projection = OrthographicProjection::new(
                            ([-1.; 3], [1.; 3]),
                            [0.; 2],
                            1.,
                            [0., 0., f64::from(size), f64::from(size)],
                            [0.; 2],
                        )
                        .unwrap();
                        request =
                            tools.interact(ui, &response, ui.painter(), Some(projection), true);
                    });
                },
            );
            request
        }
        fn click(&mut self, t: &mut ScreenTools, point: [f32; 2]) -> Option<ScreenRequest> {
            self.frame(t, vec![button(point, true)], 400.);
            self.frame(t, vec![button(point, false)], 400.)
        }
    }
    fn button(p: [f32; 2], pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: egui::pos2(p[0], p[1]),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        }
    }
    fn movement(p: [f32; 2]) -> egui::Event {
        egui::Event::PointerMoved(egui::pos2(p[0], p[1]))
    }
    #[test]
    fn rectangle_mouse_gesture_preserves_press_and_release_coordinates() {
        let mut h = Harness::new();
        let mut t = ScreenTools {
            tool: Tool::Rectangle,
            ..Default::default()
        };
        h.frame(&mut t, vec![], 400.);
        h.frame(&mut t, vec![button([100., 100.], true)], 400.);
        h.frame(&mut t, vec![movement([200., 220.])], 400.);
        let request = h
            .frame(&mut t, vec![button([200., 220.], false)], 400.)
            .unwrap();
        let ScreenShape::Rectangle { min, max } = request.shape else {
            panic!("wrong shape")
        };
        assert_eq!(min, [100., 100.]);
        assert_eq!(max, [200., 220.]);
        assert!(!t.active());
    }
    #[test]
    fn polygon_clicks_and_enter_apply_without_duplicate_vertices() {
        let mut h = Harness::new();
        let mut t = ScreenTools {
            tool: Tool::Polygon,
            ..Default::default()
        };
        h.frame(&mut t, vec![], 400.);
        for point in [[100., 100.], [300., 100.], [200., 300.]] {
            assert!(h.click(&mut t, point).is_none());
        }
        let request = h
            .frame(
                &mut t,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                }],
                400.,
            )
            .unwrap();
        let ScreenShape::Polygon(points) = request.shape else {
            panic!("wrong shape")
        };
        assert_eq!(points, [[100., 100.], [300., 100.], [200., 300.]]);
    }
    #[test]
    fn brush_click_is_a_single_disc() {
        let mut h = Harness::new();
        let mut t = ScreenTools {
            tool: Tool::Brush,
            ..Default::default()
        };
        h.frame(&mut t, vec![], 400.);
        let request = h.click(&mut t, [200., 200.]).unwrap();
        let ScreenShape::Brush { centers, radius } = request.shape else {
            panic!("wrong shape")
        };
        assert_eq!(centers, [[200., 200.]]);
        assert_eq!(radius, 12.);
    }
    #[test]
    fn freehand_and_brush_drags_include_release_position() {
        let mut h = Harness::new();
        let mut t = ScreenTools {
            tool: Tool::Lasso,
            ..Default::default()
        };
        h.frame(&mut t, vec![], 400.);
        h.frame(&mut t, vec![button([100., 100.], true)], 400.);
        for point in [[200., 100.], [200., 200.], [100., 200.]] {
            h.frame(&mut t, vec![movement(point)], 400.);
        }
        let request = h
            .frame(&mut t, vec![button([100.5, 100.5], false)], 400.)
            .unwrap();
        let ScreenShape::Polygon(points) = request.shape else {
            panic!("wrong shape")
        };
        assert_eq!(points[0], [100., 100.]);
        assert_eq!(points.last(), Some(&[100.5, 100.5]));
        let mut h = Harness::new();
        let mut t = ScreenTools {
            tool: Tool::Brush,
            ..Default::default()
        };
        h.frame(&mut t, vec![], 400.);
        h.frame(&mut t, vec![button([100., 100.], true)], 400.);
        h.frame(&mut t, vec![movement([120., 100.])], 400.);
        let request = h
            .frame(&mut t, vec![button([121., 100.], false)], 400.)
            .unwrap();
        let ScreenShape::Brush { centers, .. } = request.shape else {
            panic!("wrong shape")
        };
        assert_eq!(centers.last(), Some(&[121., 100.]));
    }
    #[test]
    fn escape_and_viewport_resize_cancel_gestures() {
        let mut h = Harness::new();
        let mut t = ScreenTools {
            tool: Tool::Polygon,
            ..Default::default()
        };
        h.frame(&mut t, vec![], 400.);
        h.click(&mut t, [100., 100.]);
        assert!(t.active());
        h.frame(
            &mut t,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
            400.,
        );
        assert!(!t.active());
        h.click(&mut t, [150., 150.]);
        assert!(t.active());
        assert!(h.frame(&mut t, vec![], 500.).is_none());
        assert!(!t.active());
    }
    #[test]
    fn short_lasso_is_controlled() {
        let mut t = ScreenTools {
            tool: Tool::Lasso,
            ..Default::default()
        };
        let projection = OrthographicProjection::new(
            ([-1.; 3], [1.; 3]),
            [0.; 2],
            1.,
            [0., 0., 400., 400.],
            [0.; 2],
        )
        .unwrap();
        t.begin(egui::pos2(1., 1.), projection, egui::Rect::EVERYTHING);
        assert!(t.finish().is_none());
        assert!(!t.active());
    }
}
