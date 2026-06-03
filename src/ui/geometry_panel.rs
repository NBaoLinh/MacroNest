use crate::model::{GeometryObject, GeometryPreset, GeometryShapeKind, GeometrySpec};
use crate::ui::CrosshairApp;
use eframe::egui::{self, Button, ComboBox, Frame, Grid, TextEdit};

impl CrosshairApp {
    pub(crate) fn render_geometry_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        let mut changed = false;
        let mut remove_preset_id = None;
        let mut request_screen_color_pick = false;

        ui.add_space(2.0);
        ui.horizontal(|ui| {
            if ui
                .button(Self::tr_lang(language, "+ Add geometry preset", "+ Add geometry preset"))
                .clicked()
            {
                let id = self.state.next_geometry_preset_id.max(1);
                self.state.next_geometry_preset_id = id + 1;
                self.state
                    .geometry_presets
                    .push(GeometryPreset::new(id));
                changed = true;
            }
        });

        ui.add_space(8.0);

        for preset_index in 0..self.state.geometry_presets.len() {
            let preset = &mut self.state.geometry_presets[preset_index];
            Self::show_preset_card(ui, preset.enabled, |ui| {
                ui.horizontal(|ui| {
                    let name_width = Self::preset_header_name_width(ui);
                    let response =
                        ui.add_sized([name_width, 24.0], TextEdit::singleline(&mut preset.name));
                    Self::apply_vietnamese_input_if_changed(
                        &response,
                        self.state.vietnamese_input_enabled,
                        self.state.vietnamese_input_mode,
                        &mut preset.name,
                    );
                    changed |= response.changed();

                    ui.checkbox(&mut preset.enabled, Self::tr_lang(language, "On", "On"));
                    changed |= ui
                        .add_sized(
                            [118.0, 24.0],
                            Button::new(Self::tr_lang(
                                language,
                                if preset.collapsed { "Show" } else { "Hide" },
                                if preset.collapsed { "Show" } else { "Hide" },
                            )),
                        )
                        .clicked();
                    if ui
                        .add_sized([28.0, 24.0], Button::new("X"))
                        .on_hover_text(Self::tr_lang(language, "Delete preset", "Delete preset"))
                        .clicked()
                    {
                        remove_preset_id = Some(preset.id);
                    }
                    if ui
                        .add_sized([92.0, 24.0], Button::new(Self::tr_lang(language, "+ Object", "+ Object")))
                        .clicked()
                    {
                        let object_id = self.state.next_geometry_object_id.max(1);
                        self.state.next_geometry_object_id = object_id + 1;
                        preset.objects.push(GeometryObject::new(object_id, GeometryShapeKind::Point));
                        preset.collapsed = false;
                        changed = true;
                    }
                    if ui
                        .add_sized([24.0, 24.0], Button::new(if preset.collapsed { ">" } else { "v" }))
                        .clicked()
                    {
                        preset.collapsed = !preset.collapsed;
                        changed = true;
                    }
                });

                if preset.collapsed {
                    return;
                }

                let mut remove_object_id = None;
                for object in &mut preset.objects {
                    ui.add_space(6.0);
                    Frame::group(ui.style())
                        .inner_margin(8)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut object.enabled, "");
                                let response = ui.add_sized(
                                    [180.0, 24.0],
                                    TextEdit::singleline(&mut object.name),
                                );
                                Self::apply_vietnamese_input_if_changed(
                                    &response,
                                    self.state.vietnamese_input_enabled,
                                    self.state.vietnamese_input_mode,
                                    &mut object.name,
                                );
                                changed |= response.changed();

                                ComboBox::from_id_salt((preset.id, object.id, "shape"))
                                    .width(132.0)
                                    .selected_text(Self::geometry_shape_label(object.spec.shape))
                                    .show_ui(ui, |ui| {
                                        for shape in Self::geometry_shapes() {
                                            changed |= ui
                                                .selectable_value(
                                                    &mut object.spec.shape,
                                                    shape,
                                                    Self::geometry_shape_label(shape),
                                                )
                                                .changed();
                                        }
                                    });

                                if ui
                                    .add_sized([28.0, 24.0], Button::new("X"))
                                    .on_hover_text(Self::tr_lang(language, "Delete object", "Delete object"))
                                    .clicked()
                                {
                                    remove_object_id = Some(object.id);
                                }
                            });

                            ui.add_space(6.0);
                            changed |= Self::render_geometry_spec_editor(
                                ui,
                                language,
                                preset.id,
                                object.id,
                                &mut object.spec,
                                &mut self.vision_manual_color,
                                &mut self.vision_manual_color_hex,
                                &mut request_screen_color_pick,
                            );
                        });
                }

                if let Some(object_id) = remove_object_id {
                    preset.objects.retain(|object| object.id != object_id);
                    changed = true;
                }
            });
        }

        if let Some(preset_id) = remove_preset_id {
            self.state
                .geometry_presets
                .retain(|preset| preset.id != preset_id);
            changed = true;
        }

        if changed {
            self.sync_geometry_presets();
            self.persist();
        }

        if request_screen_color_pick {
            self.begin_image_search_capture(
                ui.ctx(),
                crate::ui::VisionCaptureTarget::GeometryColor,
                crate::ui::VisionCaptureMode::ColorSample,
            );
        }
    }

    pub(crate) fn geometry_shapes() -> [GeometryShapeKind; 10] {
        [
            GeometryShapeKind::Point,
            GeometryShapeKind::Line,
            GeometryShapeKind::Circle,
            GeometryShapeKind::Rectangle,
            GeometryShapeKind::Label,
            GeometryShapeKind::Ellipse,
            GeometryShapeKind::Arrow,
            GeometryShapeKind::Polyline,
            GeometryShapeKind::Polygon,
            GeometryShapeKind::Arc,
        ]
    }

    pub(crate) fn geometry_shape_label(shape: GeometryShapeKind) -> &'static str {
        match shape {
            GeometryShapeKind::Point => "Point",
            GeometryShapeKind::Line => "Line",
            GeometryShapeKind::Circle => "Circle",
            GeometryShapeKind::Rectangle => "Rectangle",
            GeometryShapeKind::Label => "Label",
            GeometryShapeKind::Ellipse => "Ellipse",
            GeometryShapeKind::Arrow => "Arrow",
            GeometryShapeKind::Polyline => "Polyline",
            GeometryShapeKind::Polygon => "Polygon",
            GeometryShapeKind::Arc => "Arc",
        }
    }

    pub(crate) fn render_geometry_spec_editor(
        ui: &mut egui::Ui,
        language: crate::model::UiLanguage,
        preset_id: u32,
        object_id: u32,
        spec: &mut GeometrySpec,
        manual_color: &mut crate::model::RgbaColor,
        manual_color_hex: &mut String,
        request_screen_color_pick: &mut bool,
    ) -> bool {
        let mut changed = false;

        Grid::new((preset_id, object_id, "geometry-spec-grid"))
            .num_columns(4)
            .spacing([10.0, 6.0])
            .min_col_width(72.0)
            .show(ui, |ui| {
                match spec.shape {
                    GeometryShapeKind::Point => {
                        changed |= Self::geometry_expr_pair_row(ui, "X", &mut spec.x1_expr, "Y", &mut spec.y1_expr);
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            "Radius",
                            &mut spec.radius_expr,
                            "Thickness",
                            &mut spec.thickness_expr,
                        );
                    }
                    GeometryShapeKind::Line | GeometryShapeKind::Arrow => {
                        changed |= Self::geometry_expr_pair_row(ui, "X1", &mut spec.x1_expr, "Y1", &mut spec.y1_expr);
                        changed |= Self::geometry_expr_pair_row(ui, "X2", &mut spec.x2_expr, "Y2", &mut spec.y2_expr);
                        if spec.shape == GeometryShapeKind::Arrow {
                            changed |= Self::geometry_expr_pair_row(
                                ui,
                                "Head",
                                &mut spec.arrow_head_size_expr,
                                "Thickness",
                                &mut spec.thickness_expr,
                            );
                        } else {
                            changed |= Self::geometry_expr_pair_row(
                                ui,
                                "Thickness",
                                &mut spec.thickness_expr,
                                "Opacity",
                                &mut spec.opacity_expr,
                            );
                        }
                    }
                    GeometryShapeKind::Circle => {
                        changed |= Self::geometry_expr_pair_row(ui, "CX", &mut spec.x1_expr, "CY", &mut spec.y1_expr);
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            "Radius",
                            &mut spec.radius_expr,
                            "Thickness",
                            &mut spec.thickness_expr,
                        );
                        changed |= ui
                            .checkbox(&mut spec.filled, Self::tr_lang(language, "Filled", "Filled"))
                            .changed();
                        ui.end_row();
                    }
                    GeometryShapeKind::Rectangle => {
                        changed |= Self::geometry_expr_pair_row(ui, "X", &mut spec.x1_expr, "Y", &mut spec.y1_expr);
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            "W",
                            &mut spec.width_expr,
                            "H",
                            &mut spec.height_expr,
                        );
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            "Thickness",
                            &mut spec.thickness_expr,
                            "Opacity",
                            &mut spec.opacity_expr,
                        );
                        changed |= ui
                            .checkbox(&mut spec.filled, Self::tr_lang(language, "Filled", "Filled"))
                            .changed();
                        ui.end_row();
                    }
                    GeometryShapeKind::Label => {
                        changed |= Self::geometry_expr_pair_row(ui, "X", &mut spec.x1_expr, "Y", &mut spec.y1_expr);
                        ui.label("Text");
                        let response = ui.add_sized([360.0, 24.0], TextEdit::singleline(&mut spec.text));
                        changed |= response.changed();
                        ui.end_row();
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            "Size",
                            &mut spec.font_size_expr,
                            "Opacity",
                            &mut spec.opacity_expr,
                        );
                    }
                    GeometryShapeKind::Ellipse => {
                        changed |= Self::geometry_expr_pair_row(ui, "CX", &mut spec.x1_expr, "CY", &mut spec.y1_expr);
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            "RX",
                            &mut spec.radius_x_expr,
                            "RY",
                            &mut spec.radius_y_expr,
                        );
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            "Thickness",
                            &mut spec.thickness_expr,
                            "Opacity",
                            &mut spec.opacity_expr,
                        );
                        changed |= ui
                            .checkbox(&mut spec.filled, Self::tr_lang(language, "Filled", "Filled"))
                            .changed();
                        ui.end_row();
                    }
                    GeometryShapeKind::Polyline | GeometryShapeKind::Polygon => {
                        ui.label("Points");
                        let response =
                            ui.add_sized([520.0, 24.0], TextEdit::singleline(&mut spec.points_expr));
                        changed |= response.changed();
                        ui.end_row();
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            "Thickness",
                            &mut spec.thickness_expr,
                            "Opacity",
                            &mut spec.opacity_expr,
                        );
                        if spec.shape == GeometryShapeKind::Polygon {
                            changed |= ui
                                .checkbox(&mut spec.filled, Self::tr_lang(language, "Filled", "Filled"))
                                .changed();
                            ui.end_row();
                        }
                    }
                    GeometryShapeKind::Arc => {
                        changed |= Self::geometry_expr_pair_row(ui, "CX", &mut spec.x1_expr, "CY", &mut spec.y1_expr);
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            "RX",
                            &mut spec.radius_x_expr,
                            "RY",
                            &mut spec.radius_y_expr,
                        );
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            "Start",
                            &mut spec.start_angle_expr,
                            "End",
                            &mut spec.end_angle_expr,
                        );
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            "Thickness",
                            &mut spec.thickness_expr,
                            "Opacity",
                            &mut spec.opacity_expr,
                        );
                    }
                }

                changed |= Self::geometry_color_row(
                    ui,
                    language,
                    "Stroke",
                    &mut spec.stroke_color,
                    &mut spec.stroke_color_expr,
                    manual_color,
                    manual_color_hex,
                    request_screen_color_pick,
                );

                if matches!(
                    spec.shape,
                    GeometryShapeKind::Circle
                        | GeometryShapeKind::Rectangle
                        | GeometryShapeKind::Ellipse
                        | GeometryShapeKind::Polygon
                ) {
                    changed |= Self::geometry_color_row(
                        ui,
                        language,
                        "Fill",
                        &mut spec.fill_color,
                        &mut spec.fill_color_expr,
                        manual_color,
                        manual_color_hex,
                        request_screen_color_pick,
                    );
                }
            });

        changed
    }

    fn geometry_expr_pair_row(
        ui: &mut egui::Ui,
        label_a: &str,
        expr_a: &mut String,
        label_b: &str,
        expr_b: &mut String,
    ) -> bool {
        let mut changed = false;
        ui.label(label_a);
        changed |= ui
            .add_sized([154.0, 24.0], TextEdit::singleline(expr_a))
            .changed();
        ui.label(label_b);
        changed |= ui
            .add_sized([154.0, 24.0], TextEdit::singleline(expr_b))
            .changed();
        ui.end_row();
        changed
    }

    fn geometry_color_row(
        ui: &mut egui::Ui,
        language: crate::model::UiLanguage,
        label: &str,
        color: &mut crate::model::RgbaColor,
        expr: &mut String,
        manual_color: &mut crate::model::RgbaColor,
        manual_color_hex: &mut String,
        request_screen_color_pick: &mut bool,
    ) -> bool {
        let mut changed = false;
        ui.label(label);
        changed |= ui
            .add_sized([208.0, 24.0], TextEdit::singleline(expr))
            .changed();
        changed |= Self::edit_rgba_color(ui, color).changed();
        if ui
            .add_sized([86.0, 24.0], Button::new(Self::tr_lang(language, "Pick screen", "Pick screen")))
            .clicked()
        {
            *request_screen_color_pick = true;
        }
        if ui
            .add_sized([88.0, 24.0], Button::new(Self::tr_lang(language, "Use picker", "Use picker")))
            .clicked()
        {
            *color = *manual_color;
            *manual_color_hex = format!("{:02X}{:02X}{:02X}", color.r, color.g, color.b);
            changed = true;
        }
        ui.end_row();
        changed
    }
}
