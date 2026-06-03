use crate::model::{GeometryObject, GeometryPreset, GeometryShapeKind, GeometrySpec, VietnameseInputMode};
use crate::ui::{CrosshairApp, MouseCaptureKind, MouseMoveAbsoluteCaptureTarget, UiLanguage};
use eframe::egui::{self, Button, ComboBox, Frame, Grid, TextEdit};

impl CrosshairApp {
    pub(crate) fn render_geometry_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        let mut changed = false;
        let mut remove_preset_id = None;
        let mut request_screen_color_pick = false;
        let mut pending_screen_color_target: Option<(u32, u32, bool)> = None;
        let mut clear_preview_target = false;
        let mut begin_mouse_move_absolute_capture_target = None;

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
            Self::show_preset_card(ui, false, |ui| {
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
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if Self::sound_style_remove_button(ui)
                            .on_hover_text(Self::tr_lang(language, "Delete preset", "Delete preset"))
                            .clicked()
                        {
                            remove_preset_id = Some(preset.id);
                            if self.geometry_preview_target.is_some_and(|(preview_preset_id, _)| preview_preset_id == preset.id)
                            {
                                clear_preview_target = true;
                            }
                            if self.geometry_preset_preview_target == Some(preset.id) {
                                self.geometry_preset_preview_target = None;
                                let _ = self.overlay_tx.send(crate::overlay::OverlayCommand::PreviewGeometryPreset(None));
                            }
                        }
                        if Self::sound_style_toggle_button(
                            ui,
                            if preset.collapsed {
                                Self::tr_lang(language, "Show", "Show")
                            } else {
                                Self::tr_lang(language, "Hide", "Hide")
                            },
                        )
                        .clicked()
                        {
                            preset.collapsed = !preset.collapsed;
                            changed = true;
                        }
                        if ui
                            .add_sized(
                                [92.0, 24.0],
                                Button::new(Self::tr_lang(language, "+ Object", "+ Object")),
                            )
                            .clicked()
                        {
                            let object_id = self.state.next_geometry_object_id.max(1);
                            self.state.next_geometry_object_id = object_id + 1;
                            preset
                                .objects
                                .push(GeometryObject::new(object_id, GeometryShapeKind::Point));
                            preset.collapsed = false;
                            changed = true;
                        }
                        let preview_all_active = self.geometry_preset_preview_target == Some(preset.id);
                        let preview_all_btn = Button::new(Self::material_icon_text(
                            if preview_all_active { 0xe8f5 } else { 0xe8f4 },
                            16.0,
                        ));
                        if ui
                            .add_sized([24.0, 24.0], preview_all_btn)
                            .on_hover_text(if preview_all_active { "Stop Preview All" } else { "Preview All" })
                            .clicked()
                        {
                            if preview_all_active {
                                self.geometry_preset_preview_target = None;
                                let _ = self.overlay_tx.send(crate::overlay::OverlayCommand::PreviewGeometryPreset(None));
                            } else {
                                self.geometry_preset_preview_target = Some(preset.id);
                                let _ = self.overlay_tx.send(crate::overlay::OverlayCommand::PreviewGeometryPreset(Some(preset.id)));
                            }
                        }
                    });
                });

                if preset.collapsed {
                    return;
                }

                let mut remove_object_id = None;
                for object in &mut preset.objects {
                    ui.add_space(6.0);
                    let mut frame = Frame::group(ui.style());
                    if object.enabled {
                        frame = frame
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 255, 170)))
                            .fill(egui::Color32::from_rgba_unmultiplied(0, 255, 170, 5));
                    }
                    frame.inner_margin(8).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let preview_active =
                                self.geometry_preview_target == Some((preset.id, object.id));
                            if ui.checkbox(&mut object.enabled, "").changed() {
                                changed = true;
                                if !object.enabled && preview_active {
                                    self.geometry_preview_target = None;
                                    self.geometry_preview_sent = None;
                                    let _ = self.overlay_tx.send(crate::overlay::OverlayCommand::PreviewGeometrySpec(None));
                                }
                            }
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

                            let preview_btn = Button::new(Self::material_icon_text(
                                if preview_active { 0xe8f5 } else { 0xe8f4 },
                                16.0,
                            ));
                            if ui
                                .add_enabled(object.enabled, preview_btn)
                                .on_hover_text(if preview_active { "Stop preview" } else { "Preview" })
                                .clicked()
                            {
                                if preview_active {
                                    self.geometry_preview_target = None;
                                    self.geometry_preview_sent = None;
                                    let _ = self.overlay_tx.send(crate::overlay::OverlayCommand::PreviewGeometrySpec(None));
                                } else {
                                    self.geometry_preview_target = Some((preset.id, object.id));
                                    self.geometry_preview_sent = Some(object.spec.clone());
                                    let _ = self.overlay_tx.send(
                                        crate::overlay::OverlayCommand::PreviewGeometrySpec(
                                            Some(object.spec.clone()),
                                        ),
                                    );
                                }
                            }

                            if ui
                                .add_sized(
                                    [24.0, 24.0],
                                    Button::new(Self::material_icon_text(0xe5cd, 16.0)),
                                )
                                .on_hover_text(Self::tr_lang(language, "Delete object", "Delete object"))
                                .clicked()
                            {
                                remove_object_id = Some(object.id);
                                if self.geometry_preview_target == Some((preset.id, object.id)) {
                                    clear_preview_target = true;
                                }
                            }
                        });

                            ui.add_space(6.0);
                            changed |= Self::render_geometry_spec_editor(
                                ui,
                                language,
                                preset.id,
                                object.id,
                                false,
                                &mut object.spec,
                                &mut self.vision_manual_color,
                                &mut self.vision_manual_color_hex,
                                &mut request_screen_color_pick,
                                &mut pending_screen_color_target,
                                &mut begin_mouse_move_absolute_capture_target,
                                self.state.vietnamese_input_enabled,
                                self.state.vietnamese_input_mode,
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

        if clear_preview_target {
            self.geometry_preview_target = None;
            self.geometry_preview_sent = None;
            let _ = self
                .overlay_tx
                .send(crate::overlay::OverlayCommand::PreviewGeometrySpec(None));
        } else if let Some((preview_preset_id, preview_object_id)) = self.geometry_preview_target {
            let preview_spec = self
                .state
                .geometry_presets
                .iter()
                .find(|preset| preset.id == preview_preset_id)
                .and_then(|preset| preset.objects.iter().find(|object| object.id == preview_object_id))
                .map(|object| object.spec.clone());
            if preview_spec.is_none() {
                self.geometry_preview_target = None;
                self.geometry_preview_sent = None;
            }
            if self.geometry_preview_sent != preview_spec {
                self.geometry_preview_sent = preview_spec.clone();
                let _ = self
                    .overlay_tx
                    .send(crate::overlay::OverlayCommand::PreviewGeometrySpec(preview_spec));
            }
        }

        if let Some(preview_preset_id) = self.geometry_preset_preview_target {
            let exists = self
                .state
                .geometry_presets
                .iter()
                .any(|preset| preset.id == preview_preset_id);
            if !exists {
                self.geometry_preset_preview_target = None;
                let _ = self
                    .overlay_tx
                    .send(crate::overlay::OverlayCommand::PreviewGeometryPreset(None));
            }
        }

        if request_screen_color_pick {
            self.geometry_color_pick_target = pending_screen_color_target;
            self.begin_image_search_capture(
                ui.ctx(),
                crate::ui::VisionCaptureTarget::GeometryColor,
                crate::ui::VisionCaptureMode::ColorSample,
            );
        }

        if let Some(target) = begin_mouse_move_absolute_capture_target {
            self.begin_mouse_move_absolute_capture(ui.ctx(), target);
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
        allow_color_expression: bool,
        spec: &mut GeometrySpec,
        manual_color: &mut crate::model::RgbaColor,
        manual_color_hex: &mut String,
        request_screen_color_pick: &mut bool,
        pending_screen_color_target: &mut Option<(u32, u32, bool)>,
        begin_mouse_move_absolute_capture_target: &mut Option<MouseMoveAbsoluteCaptureTarget>,
        vietnamese_input_enabled: bool,
        vietnamese_input_mode: VietnameseInputMode,
    ) -> bool {
        let mut changed = false;

        Grid::new((preset_id, object_id, "geometry-spec-grid"))
            .num_columns(5)
            .spacing([10.0, 6.0])
            .min_col_width(72.0)
            .show(ui, |ui| {
                match spec.shape {
                    GeometryShapeKind::Point => {
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            0,
                            "X",
                            &mut spec.x1_expr,
                            "Y",
                            &mut spec.y1_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            255,
                            "Size",
                            &mut spec.radius_expr,
                            "Opacity",
                            &mut spec.opacity_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                    }
                    GeometryShapeKind::Line | GeometryShapeKind::Arrow => {
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            0,
                            "X1",
                            &mut spec.x1_expr,
                            "Y1",
                            &mut spec.y1_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            1,
                            "X2",
                            &mut spec.x2_expr,
                            "Y2",
                            &mut spec.y2_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        if spec.shape == GeometryShapeKind::Arrow {
                            changed |= Self::geometry_expr_pair_row(
                                ui,
                                preset_id,
                                object_id,
                                255,
                                "Head",
                                &mut spec.arrow_head_size_expr,
                                "Thickness",
                                &mut spec.thickness_expr,
                                begin_mouse_move_absolute_capture_target,
                            );
                        } else {
                            changed |= Self::geometry_expr_pair_row(
                                ui,
                                preset_id,
                                object_id,
                                255,
                                "Thickness",
                                &mut spec.thickness_expr,
                                "Opacity",
                                &mut spec.opacity_expr,
                                begin_mouse_move_absolute_capture_target,
                            );
                        }
                    }
                    GeometryShapeKind::Circle => {
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            0,
                            "CX",
                            &mut spec.x1_expr,
                            "CY",
                            &mut spec.y1_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            255,
                            "Radius",
                            &mut spec.radius_expr,
                            "Thickness",
                            &mut spec.thickness_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        changed |= Self::geometry_fill_mode_row(ui, language, &mut spec.filled);
                    }
                    GeometryShapeKind::Rectangle => {
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            0,
                            "X",
                            &mut spec.x1_expr,
                            "Y",
                            &mut spec.y1_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            255,
                            "W",
                            &mut spec.width_expr,
                            "H",
                            &mut spec.height_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            255,
                            "Thickness",
                            &mut spec.thickness_expr,
                            "Opacity",
                            &mut spec.opacity_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        changed |= Self::geometry_fill_mode_row(ui, language, &mut spec.filled);
                    }
                    GeometryShapeKind::Label => {
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            0,
                            "X",
                            &mut spec.x1_expr,
                            "Y",
                            &mut spec.y1_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        ui.label("Text");
                        let text_id = ui.make_persistent_id((preset_id, object_id, "label-text"));
                        let response = Self::render_interpolated_text_edit(
                            ui,
                            &mut spec.text,
                            text_id,
                            154.0, // normal width
                            360.0, // expanded width
                            24.0,  // normal height
                            24.0,  // expanded height
                            "Text", // hint
                            false, // multiline_on_focus
                        );
                        changed |= response.changed();
                        Self::apply_vietnamese_input_if_changed(
                            &response,
                            vietnamese_input_enabled,
                            vietnamese_input_mode,
                            &mut spec.text,
                        );
                        ui.label("");
                        ui.label("");
                        ui.add_space(24.0);
                        ui.end_row();
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            255,
                            "Size",
                            &mut spec.font_size_expr,
                            "Opacity",
                            &mut spec.opacity_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                    }
                    GeometryShapeKind::Ellipse => {
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            0,
                            "CX",
                            &mut spec.x1_expr,
                            "CY",
                            &mut spec.y1_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            255,
                            "RX",
                            &mut spec.radius_x_expr,
                            "RY",
                            &mut spec.radius_y_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            255,
                            "Thickness",
                            &mut spec.thickness_expr,
                            "Opacity",
                            &mut spec.opacity_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        changed |= Self::geometry_fill_mode_row(ui, language, &mut spec.filled);
                    }
                    GeometryShapeKind::Polyline | GeometryShapeKind::Polygon => {
                        let mut points: Vec<(String, String)> = spec.points_expr
                            .split(';')
                            .filter(|s| !s.is_empty())
                            .map(|pair| {
                                if let Some((x, y)) = pair.split_once(',') {
                                    (x.trim().to_owned(), y.trim().to_owned())
                                } else {
                                    (pair.trim().to_owned(), String::new())
                                }
                            })
                            .collect();

                        let mut points_changed = false;
                        let mut remove_point_idx = None;
                        for (idx, (x_val, y_val)) in points.iter_mut().enumerate() {
                            ui.label(format!("P{}", idx + 1));
                            let response_x = ui.add_sized([154.0, 24.0], TextEdit::singleline(x_val));
                            points_changed |= response_x.changed();

                            ui.label("Y");
                            let response_y = ui.add_sized([154.0, 24.0], TextEdit::singleline(y_val));
                            points_changed |= response_y.changed();

                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                if ui
                                    .add_sized(
                                        [24.0, 24.0],
                                        Button::new(Self::material_icon_text(0xe55f, 16.0)),
                                    )
                                    .on_hover_text("Pick coordinates from screen")
                                    .clicked()
                                {
                                    *begin_mouse_move_absolute_capture_target = Some(MouseMoveAbsoluteCaptureTarget {
                                        group_id: None,
                                        preset_id,
                                        step_index: object_id as usize,
                                        capture_kind: MouseCaptureKind::GeometryPrimaryPos,
                                        extra_cond_index: Some(idx),
                                        is_hold_stop: false,
                                    });
                                }
                                if ui
                                    .add_sized(
                                        [24.0, 24.0],
                                        Button::new(Self::material_icon_text(0xe5cd, 16.0)),
                                    )
                                    .on_hover_text("Delete point")
                                    .clicked()
                                {
                                    remove_point_idx = Some(idx);
                                }
                            });
                            ui.end_row();
                        }

                        if let Some(idx) = remove_point_idx {
                            points.remove(idx);
                            points_changed = true;
                        }

                        // Add new point button
                        ui.add_space(1.0);
                        if ui.button("+ Add Point").clicked() {
                            points.push(("960".to_owned(), "540".to_owned()));
                            points_changed = true;
                        }
                        ui.label("");
                        ui.label("");
                        ui.add_space(24.0);
                        ui.end_row();

                        if points_changed {
                            spec.points_expr = points
                                .iter()
                                .map(|(x, y)| format!("{},{}", x, y))
                                .collect::<Vec<_>>()
                                .join(";");
                            changed = true;
                        }

                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            255,
                            "Thickness",
                            &mut spec.thickness_expr,
                            "Opacity",
                            &mut spec.opacity_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        if spec.shape == GeometryShapeKind::Polygon {
                            changed |= Self::geometry_fill_mode_row(ui, language, &mut spec.filled);
                        }
                    }
                    GeometryShapeKind::Arc => {
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            0,
                            "CX",
                            &mut spec.x1_expr,
                            "CY",
                            &mut spec.y1_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            255,
                            "RX",
                            &mut spec.radius_x_expr,
                            "RY",
                            &mut spec.radius_y_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            255,
                            "Start",
                            &mut spec.start_angle_expr,
                            "End",
                            &mut spec.end_angle_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                        changed |= Self::geometry_expr_pair_row(
                            ui,
                            preset_id,
                            object_id,
                            255,
                            "Thickness",
                            &mut spec.thickness_expr,
                            "Opacity",
                            &mut spec.opacity_expr,
                            begin_mouse_move_absolute_capture_target,
                        );
                    }
                }

                let stroke_label = if matches!(
                    spec.shape,
                    GeometryShapeKind::Circle
                        | GeometryShapeKind::Rectangle
                        | GeometryShapeKind::Ellipse
                        | GeometryShapeKind::Polygon
                ) {
                    "Stroke"
                } else {
                    "Color"
                };

                changed |= Self::geometry_color_row(
                    ui,
                    preset_id,
                    object_id,
                    stroke_label,
                    &mut spec.stroke_color,
                    &mut spec.stroke_color_expr,
                    manual_color,
                    manual_color_hex,
                    allow_color_expression,
                    request_screen_color_pick,
                    pending_screen_color_target,
                    false,
                );

                if spec.filled
                    && matches!(
                        spec.shape,
                        GeometryShapeKind::Circle
                            | GeometryShapeKind::Rectangle
                            | GeometryShapeKind::Ellipse
                            | GeometryShapeKind::Polygon
                    )
                {
                    changed |= Self::geometry_color_row(
                        ui,
                        preset_id,
                        object_id,
                        "Fill",
                        &mut spec.fill_color,
                        &mut spec.fill_color_expr,
                        manual_color,
                        manual_color_hex,
                        allow_color_expression,
                        request_screen_color_pick,
                        pending_screen_color_target,
                        true,
                    );
                }
            });

        changed
    }

    fn geometry_expr_pair_row(
        ui: &mut egui::Ui,
        preset_id: u32,
        object_id: u32,
        pair_index: u8,
        label_a: &str,
        expr_a: &mut String,
        label_b: &str,
        expr_b: &mut String,
        begin_mouse_move_absolute_capture_target: &mut Option<MouseMoveAbsoluteCaptureTarget>,
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
        if pair_index != 255 {
            let capture_kind = if pair_index == 1 {
                MouseCaptureKind::GeometrySecondaryPos
            } else {
                MouseCaptureKind::GeometryPrimaryPos
            };
            if ui
                .add_sized(
                    [24.0, 24.0],
                    Button::new(Self::material_icon_text(0xe55f, 16.0)),
                )
                .on_hover_text("Pick coordinates from screen")
                .clicked()
            {
                *begin_mouse_move_absolute_capture_target = Some(MouseMoveAbsoluteCaptureTarget {
                    group_id: None,
                    preset_id,
                    step_index: object_id as usize,
                    capture_kind,
                    extra_cond_index: None,
                    is_hold_stop: false,
                });
            }
        } else {
            ui.add_space(24.0);
        }
        ui.end_row();
        changed
    }

    fn geometry_fill_mode_row(
        ui: &mut egui::Ui,
        language: UiLanguage,
        filled: &mut bool,
    ) -> bool {
        let mut changed = false;
        ui.label(Self::tr_lang(language, "Mode", "Mode"));
        ComboBox::from_id_salt(ui.next_auto_id())
            .width(154.0)
            .selected_text(if *filled {
                Self::tr_lang(language, "Filled", "Filled")
            } else {
                Self::tr_lang(language, "Outline", "Outline")
            })
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(
                        filled,
                        false,
                        Self::tr_lang(language, "Outline", "Outline"),
                    )
                    .changed();
                changed |= ui
                    .selectable_value(
                        filled,
                        true,
                        Self::tr_lang(language, "Filled", "Filled"),
                    )
                    .changed();
            });
        ui.add_space(154.0);
        ui.end_row();
        changed
    }

    fn geometry_color_row(
        ui: &mut egui::Ui,
        preset_id: u32,
        object_id: u32,
        label: &str,
        color: &mut crate::model::RgbaColor,
        expr: &mut String,
        manual_color: &mut crate::model::RgbaColor,
        manual_color_hex: &mut String,
        allow_color_expression: bool,
        request_screen_color_pick: &mut bool,
        pending_screen_color_target: &mut Option<(u32, u32, bool)>,
        is_fill: bool,
    ) -> bool {
        let mut changed = false;
        let color_tooltip = format!(
            "#{:02X}{:02X}{:02X}{:02X} rgba({}, {}, {}, {})",
            color.r, color.g, color.b, color.a, color.r, color.g, color.b, color.a
        );
        ui.label(label);
        ui.horizontal(|ui| {
            if allow_color_expression {
                let expr_response = ui.add_sized(
                    [176.0, 24.0],
                    TextEdit::singleline(expr).hint_text("{A} or #RRGGBB"),
                );
                changed |= expr_response.changed();
                expr_response.on_hover_text("Optional color expression. Example: {A} or #BAD1C4");
            }

            let _swatch_response = ui
                .scope(|ui| {
                    Self::image_search_target_color_swatch(ui, Some(*color), egui::vec2(24.0, 24.0));
                })
                .response
                .on_hover_text(color_tooltip.clone());

            let popup_id =
                ui.make_persistent_id((preset_id, object_id, label, "geometry-color-popup"));
            let mut popup_open = ui
                .ctx()
                .data(|data| data.get_temp::<bool>(popup_id))
                .unwrap_or(false);

            let palette_button = ui
                .add_sized([24.0, 24.0], Button::new(Self::material_icon_text(0xe40a, 16.0)))
                .on_hover_text("Choose color");
            if palette_button.clicked() {
                *manual_color = *color;
                *manual_color_hex = format!(
                    "{:02X}{:02X}{:02X}{:02X}",
                    color.r, color.g, color.b, color.a
                );
                popup_open = true;
            }

            let popup_response = egui::Popup::from_response(&palette_button)
                .id(popup_id)
                .open_bool(&mut popup_open)
                .align(egui::RectAlign::BOTTOM_START)
                .layout(egui::Layout::top_down_justified(egui::Align::Min))
                .width(220.0)
                .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
                .show(|ui| {
                    ui.set_min_width(220.0);
                    let mut color32 = egui::Color32::from_rgba_unmultiplied(
                        manual_color.r,
                        manual_color.g,
                        manual_color.b,
                        manual_color.a,
                    );
                    if egui::color_picker::color_picker_color32(
                        ui,
                        &mut color32,
                        egui::color_picker::Alpha::BlendOrAdditive,
                    ) {
                        manual_color.r = color32.r();
                        manual_color.g = color32.g();
                        manual_color.b = color32.b();
                        manual_color.a = color32.a();
                        *manual_color_hex = format!(
                            "{:02X}{:02X}{:02X}{:02X}",
                            manual_color.r, manual_color.g, manual_color.b, manual_color.a
                        );
                        *color = *manual_color;
                        expr.clear();
                        changed = true;
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("#");
                        let hex_response = ui.add(
                            TextEdit::singleline(manual_color_hex)
                                .hint_text("RRGGBB or RRGGBBAA")
                                .desired_width(132.0),
                        );
                        if hex_response.changed() {
                            let hex = manual_color_hex.trim().trim_start_matches('#');
                            if (hex.len() == 6 || hex.len() == 8)
                                && let Ok(color_value) = u32::from_str_radix(hex, 16)
                            {
                                let (r, g, b, a) = if hex.len() == 6 {
                                    (
                                        ((color_value >> 16) & 0xFF) as u8,
                                        ((color_value >> 8) & 0xFF) as u8,
                                        (color_value & 0xFF) as u8,
                                        255,
                                    )
                                } else {
                                    (
                                        ((color_value >> 24) & 0xFF) as u8,
                                        ((color_value >> 16) & 0xFF) as u8,
                                        ((color_value >> 8) & 0xFF) as u8,
                                        (color_value & 0xFF) as u8,
                                    )
                                };
                                *manual_color = crate::model::RgbaColor { r, g, b, a };
                                *color = *manual_color;
                                expr.clear();
                                changed = true;
                            }
                        }
                    });
                });

            if popup_open
                && let Some(pointer_pos) = ui.ctx().pointer_hover_pos()
            {
                let mut keep_open_rect = palette_button.rect.expand(10.0);
                if let Some(popup) = &popup_response {
                    keep_open_rect = keep_open_rect.union(popup.response.rect.expand(10.0));
                }
                if !keep_open_rect.contains(pointer_pos) {
                    popup_open = false;
                }
            }
            ui.ctx()
                .data_mut(|data| data.insert_temp(popup_id, popup_open));

            let screen_pick_response = ui
                .add_sized([24.0, 24.0], Button::new(Self::material_icon_text(0xe3b8, 16.0)))
                .on_hover_text("Pick from screen");
            if screen_pick_response.clicked() {
                *request_screen_color_pick = true;
                *pending_screen_color_target = Some((preset_id, object_id, is_fill));
            }
        });
        ui.end_row();
        changed
    }
}
