use crate::ai;
use crate::hotkey;
use crate::model::*;
use crate::ui::{
    CrosshairApp, MATERIAL_ICONS_FONT, MacroActionSubmenuKind, MacroGroupFavoriteFilter,
    MacroStepDragPayload, MouseMoveAbsoluteCaptureTarget,
};
use eframe::egui::{self, *};

#[derive(Clone, Copy, PartialEq, Eq)]
enum VariableValueKind {
    Neutral,
    Number,
    Text,
}

#[derive(Clone)]
struct MacroStepHoverPreview {
    source_id: egui::Id,
    title: String,
    kind: MacroStepHoverPreviewKind,
}

#[derive(Clone)]
enum MacroStepHoverPreviewKind {
    MacroPreset {
        mode_label: String,
        preset_name: String,
        steps: Vec<MacroStep>,
    },
    StepToggle {
        mode_label: String,
        preset_name: String,
        steps: Vec<MacroStep>,
        selected_steps: Vec<u32>,
    },
    Vision {
        action_label: String,
        preset_name: String,
        preset: VisionPreset,
        trigger_macro_name: Option<String>,
        move_cursor: bool,
        wait_until_found: bool,
        trigger_macro_enabled: bool,
    },
    Hud {
        preset_name: String,
        preset: HudPreset,
        text: String,
        duration_override_ms: u64,
        timed_override: bool,
    },
    Crosshair {
        profile_name: String,
        style: CrosshairStyle,
        disabled: bool,
    },
    MouseMoveAbsolute {
        x: i32,
        y: i32,
    },
    WindowResize {
        preset_name: String,
        preset: WindowPreset,
    },
    Pin {
        preset_name: String,
        preset: PinPreset,
        disabled: bool,
        disable_all: bool,
    },
    FocusWindow {
        window_title: String,
    },
    Generic {
        lines: Vec<String>,
    },
}

#[derive(Clone)]
enum HoverPreviewRequest {
    MacroPreset {
        source_id: egui::Id,
        preset_id: u32,
        mode_label: String,
    },
    StepToggle {
        source_id: egui::Id,
        preset_id: u32,
        mode_label: String,
        selected_steps: Vec<u32>,
    },
    Vision {
        source_id: egui::Id,
        preset_id: u32,
        action_label: String,
        move_cursor: bool,
        wait_until_found: bool,
        trigger_macro_enabled: bool,
        trigger_macro_preset_id: Option<u32>,
    },
    Hud {
        source_id: egui::Id,
        preset_id: u32,
        text_override: String,
        duration_override_ms: u64,
        timed_override: bool,
    },
    Crosshair {
        source_id: egui::Id,
        profile_name: String,
        disabled: bool,
    },
    MouseMoveAbsolute {
        source_id: egui::Id,
        x: i32,
        y: i32,
    },
    WindowResize {
        source_id: egui::Id,
        preset_id: u32,
    },
    Pin {
        source_id: egui::Id,
        preset_id: u32,
        disabled: bool,
        disable_all: bool,
    },
    FocusWindow {
        source_id: egui::Id,
        window_title: String,
    },
    Generic {
        source_id: egui::Id,
        title: String,
        lines: Vec<String>,
    },
}

impl CrosshairApp {
    fn rgba_to_color32(color: RgbaColor) -> Color32 {
        Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
    }

    fn loop_is_infinite(step: &MacroStep) -> bool {
        matches!(
            step.key.trim().to_ascii_lowercase().as_str(),
            "infinite" | "inf" | "forever" | "-1"
        )
    }
    fn render_macro_action_button(
        ui: &mut egui::Ui,
        language: UiLanguage,
        current: &MacroAction,
        candidate: MacroAction,
        action_hover_id: egui::Id,
        is_submenu_item: bool,
    ) -> egui::Response {
        let inner = ui.allocate_ui_with_layout(
            vec2(58.0, 42.0),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                let label_color = if *current == candidate {
                    ui.visuals().strong_text_color()
                } else {
                    ui.visuals().text_color()
                };
                let response = ui.add_sized(
                    [34.0, 24.0],
                    Button::new(Self::macro_action_icon_text(candidate))
                        .selected(*current == candidate),
                );
                if !is_submenu_item && (response.hovered() || response.clicked()) {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(action_hover_id, true));
                }
                ui.label(
                    RichText::new(Self::macro_action_short_label(candidate, language))
                        .size(9.0)
                        .color(label_color),
                );
                response
            },
        );
        let response = inner.inner;
        if !is_submenu_item {
            Self::show_instant_hover_tooltip(
                ui,
                &response,
                format!(
                    "{}\n{}",
                    Self::macro_action_label(candidate),
                    Self::macro_action_tooltip(candidate, language)
                ),
            );
        }
        response
    }
    fn render_macro_action_option(
        ui: &mut egui::Ui,
        language: UiLanguage,
        current: &mut MacroAction,
        candidate: MacroAction,
        live_sync: &mut bool,
        action_hover_id: egui::Id,
        is_submenu_item: bool,
    ) {
        let response =
            Self::render_macro_action_button(ui, language, current, candidate, action_hover_id, is_submenu_item);
        if response.clicked() {
            *current = candidate;
            *live_sync = true;
            ui.close();
        }
    }
    fn clear_macro_action_submenus(ui: &mut egui::Ui, id_source: impl std::hash::Hash + Copy) {
        let owner_id = ui.make_persistent_id("macro-action-submenu-owner");
        let active_mouse_click_popup_key_id =
            ui.make_persistent_id((id_source, "mouse-click-active-submenu-key"));
        let mouse_popup_id = ui.make_persistent_id((id_source, "mouse-submenu-popup"));
        let image_popup_id = ui.make_persistent_id((id_source, "image-search-submenu-popup"));
        let timer_popup_id = ui.make_persistent_id((id_source, "timer-submenu-popup"));
        let if_popup_id = ui.make_persistent_id((id_source, "if-submenu-popup"));
        ui.ctx().data_mut(|data| {
            data.insert_temp(owner_id, None::<MacroActionSubmenuKind>);
            data.insert_temp(active_mouse_click_popup_key_id, None::<&'static str>);
            data.insert_temp(mouse_popup_id, false);
            data.insert_temp(image_popup_id, false);
            data.insert_temp(timer_popup_id, false);
            data.insert_temp(if_popup_id, false);
        });
        egui::Popup::close_id(ui.ctx(), mouse_popup_id);
        egui::Popup::close_id(ui.ctx(), image_popup_id);
        egui::Popup::close_id(ui.ctx(), timer_popup_id);
        egui::Popup::close_id(ui.ctx(), if_popup_id);
        for (_, _, _, popup_key) in Self::mouse_click_action_groups().iter().copied() {
            let child_popup_id = ui.make_persistent_id((id_source, popup_key, "popup"));
            ui.ctx().data_mut(|data| data.insert_temp(child_popup_id, false));
            egui::Popup::close_id(ui.ctx(), child_popup_id);
        }
        ui.ctx().request_repaint();
    }
    fn clear_mouse_click_submenus(ui: &mut egui::Ui, id_source: impl std::hash::Hash + Copy) {
        let active_mouse_click_popup_key_id =
            ui.make_persistent_id((id_source, "mouse-click-active-submenu-key"));
        ui.ctx()
            .data_mut(|data| data.insert_temp(active_mouse_click_popup_key_id, None::<&'static str>));
        for (_, _, _, popup_key) in Self::mouse_click_action_groups().iter().copied() {
            let child_popup_id = ui.make_persistent_id((id_source, popup_key, "popup"));
            ui.ctx().data_mut(|data| data.insert_temp(child_popup_id, false));
            egui::Popup::close_id(ui.ctx(), child_popup_id);
        }
        ui.ctx().request_repaint();
    }
    fn close_inactive_mouse_click_submenus(
        ui: &mut egui::Ui,
        id_source: impl std::hash::Hash + Copy,
        active_popup_key: Option<&'static str>,
    ) {
        for (_, _, _, popup_key) in Self::mouse_click_action_groups().iter().copied() {
            if Some(popup_key) != active_popup_key {
                let child_popup_id = ui.make_persistent_id((id_source, popup_key, "popup"));
                ui.ctx().data_mut(|data| data.insert_temp(child_popup_id, false));
            }
        }
    }
    fn render_expression_help_box(ui: &mut egui::Ui, language: UiLanguage) {
        let fill = Color32::from_rgba_unmultiplied(0, 170, 255, 18);
        let stroke = egui::Stroke::new(1.0, Color32::from_rgb(0, 170, 255));
        egui::Frame::group(ui.style())
            .fill(fill)
            .stroke(stroke)
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(Self::material_icon_text(0xe88f, 14.0).color(Color32::from_rgb(0, 170, 255)));
                    ui.label(
                        egui::RichText::new(Self::tr_lang(
                            language,
                            "EXPRESSION HELP",
                            "HƯỚNG DẪN BIỂU THỨC",
                        ))
                        .strong()
                        .color(Color32::from_rgb(0, 170, 255)),
                    );
                });
                ui.add_space(2.0);
                ui.label(Self::tr_lang(
                    language,
                    "You can write math expressions and use variables in {}. Math operators + - * / and parentheses () are supported.\nExample: {100 + (A - B) * 2}",
                    "Bạn có thể viết biểu thức toán và dùng biến trong {}. Hỗ trợ các phép toán + - * / và dấu ngoặc ().\nVí dụ: {100 + (A - B) * 2}",
                ));
                ui.add_space(4.0);
                ui.label(egui::RichText::new(Self::tr_lang(language, "Supported Functions:", "Hàm hỗ trợ:")).strong());
                ui.label(egui::RichText::new("• random(min, max)").monospace());
                ui.label(egui::RichText::new("• min(a, b)  •  max(a, b)").monospace());
                ui.label(egui::RichText::new("• abs(a)").monospace());
            });
    }

    fn macro_step_preview_summary_line(step: &MacroStep, language: UiLanguage) -> String {
        let mut parts = Vec::new();
        if step.enabled {
            parts.push("[x]".to_owned());
        } else {
            parts.push("[ ]".to_owned());
        }
        parts.push(Self::macro_action_short_label(step.action, language).to_owned());
        let key = step.key.trim();
        if !key.is_empty() {
            parts.push(key.to_owned());
        }
        if matches!(step.action, MacroAction::MouseMoveAbsolute | MacroAction::MouseMoveRelative) {
            parts.push(format!("({}, {})", step.x, step.y));
        }
        if step.action == MacroAction::ShowHud && !step.text_override.trim().is_empty() {
            parts.push(step.text_override.trim().to_owned());
        }
        if step.action == MacroAction::ShowHud && step.timed_override && step.duration_override_ms > 0 {
            parts.push(format!("{} ms", step.duration_override_ms));
        } else if !step.delay_expr.trim().is_empty() {
            parts.push(format!("{} ms", step.get_delay_ms()));
        } else if step.delay_ms > 0 {
            parts.push(format!("{} ms", step.delay_ms));
        }
        parts.join("  ")
    }

    fn macro_step_preview_visible_entries(steps: &[MacroStep]) -> Vec<(usize, &MacroStep)> {
        steps
            .iter()
            .enumerate()
            .filter(|(_, step)| !matches!(step.action, MacroAction::LoopStart | MacroAction::LoopEnd))
            .collect()
    }

    fn render_macro_step_hover_preview_list(
        ui: &mut egui::Ui,
        language: UiLanguage,
        source_id: egui::Id,
        steps: &[MacroStep],
        selected_steps: &[u32],
    ) {
        let offset_id = source_id.with("macro-hover-preview-offset");
        let visible_lines = 5usize;
        let visible_steps = Self::macro_step_preview_visible_entries(steps);
        let max_offset = visible_steps.len().saturating_sub(visible_lines) as i32;
        let mut offset = ui
            .ctx()
            .data(|data| data.get_temp::<i32>(offset_id))
            .unwrap_or(0)
            .clamp(0, max_offset.max(0));
        let start = offset as usize;
        let end = (start + visible_lines).min(visible_steps.len());
        if visible_steps.is_empty() {
            ui.label(Self::tr_lang(language, "No steps.", "Không có step nào."));
            return;
        }
        for (step_no, step) in visible_steps[start..end].iter() {
            let is_selected = selected_steps.contains(&(*step_no as u32));
            let line = format!("{}. {}", step_no, Self::macro_step_preview_summary_line(step, language));
            let text = if is_selected {
                RichText::new(line)
                    .strong()
                    .color(Color32::from_rgb(0, 255, 170))
            } else {
                RichText::new(line).monospace()
            };
            ui.label(text);
        }
        if visible_steps.len() > visible_lines {
            ui.add_space(2.0);
            ui.label(
                RichText::new(format!("{} / {}", start + 1, visible_steps.len()))
                    .weak()
                    .monospace(),
            );
        }
        ui.ctx().data_mut(|data| data.insert_temp(offset_id, offset));
    }

    fn render_hover_preview_screen_canvas(
        ui: &mut egui::Ui,
        max_height: f32,
    ) -> (egui::Rect, egui::Rect, f32) {
        let screen_size = Self::screen_size();
        let available_width = ui.available_width().clamp(320.0, 720.0);
        let aspect = if screen_size.y > 0.0 {
            screen_size.x / screen_size.y
        } else {
            16.0 / 9.0
        };
        let mut desired_width = available_width;
        let mut desired_height = desired_width / aspect;
        if desired_height > max_height {
            desired_height = max_height;
            desired_width = desired_height * aspect;
        }
        let (canvas_rect, _) = ui.allocate_exact_size(vec2(desired_width, desired_height), Sense::hover());
        let draw_rect = canvas_rect.shrink(4.0);
        let scale = (draw_rect.width() / screen_size.x.max(1.0))
            .min(draw_rect.height() / screen_size.y.max(1.0))
            .max(0.0001);
        let preview_size = vec2(screen_size.x * scale, screen_size.y * scale);
        let preview_rect = egui::Rect::from_center_size(draw_rect.center(), preview_size);
        ui.painter().rect_filled(
            preview_rect,
            8.0,
            Color32::from_rgba_premultiplied(18, 24, 22, 220),
        );
        ui.painter().rect_stroke(
            preview_rect,
            8.0,
            egui::Stroke::new(1.0, Color32::from_rgb(104, 148, 124)),
            egui::StrokeKind::Outside,
        );
        (canvas_rect, preview_rect, scale)
    }

    fn render_hover_preview_panel(
        ui: &mut egui::Ui,
        language: UiLanguage,
        preview: Option<&MacroStepHoverPreview>,
    ) {
        if let Some(preview) = preview {
            let fill = Color32::from_rgba_unmultiplied(20, 24, 26, 242);
            let stroke = egui::Stroke::new(1.0, Color32::from_rgb(94, 176, 122));
            egui::Frame::popup(ui.style())
                .fill(fill)
                .stroke(stroke)
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.label(RichText::new(&preview.title).strong());
                    ui.add_space(4.0);
                    match &preview.kind {
                        MacroStepHoverPreviewKind::MacroPreset {
                            mode_label,
                            preset_name,
                            steps,
                        } => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(mode_label).strong());
                                ui.label(RichText::new(preset_name).strong().color(Color32::from_rgb(124, 240, 164)));
                            });
                            ui.add_space(4.0);
                            Self::render_macro_step_hover_preview_list(
                                ui,
                                language,
                                preview.source_id,
                                steps,
                                &[],
                            );
                        }
                        MacroStepHoverPreviewKind::StepToggle {
                            mode_label,
                            preset_name,
                            steps,
                            selected_steps,
                        } => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(mode_label).strong());
                                ui.label(RichText::new(preset_name).strong().color(Color32::from_rgb(124, 240, 164)));
                            });
                            ui.add_space(4.0);
                            Self::render_macro_step_hover_preview_list(
                                ui,
                                language,
                                preview.source_id,
                                steps,
                                selected_steps,
                            );
                        }
                        MacroStepHoverPreviewKind::Vision {
                            action_label,
                            preset_name,
                            preset,
                            trigger_macro_name,
                            move_cursor,
                            wait_until_found,
                            trigger_macro_enabled,
                        } => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(action_label).strong());
                                ui.label(RichText::new(preset_name).strong().color(Color32::from_rgb(124, 240, 164)));
                            });
                            ui.add_space(6.0);
                            let (_, preview_rect, scale) = Self::render_hover_preview_screen_canvas(ui, 360.0);
                            let colors = Self::image_search_target_colors(preset);
                            let mut swatch_pos = preview_rect.left_top() + vec2(10.0, 10.0);
                            for color in colors.iter().take(6) {
                                let swatch = egui::Rect::from_min_size(swatch_pos, vec2(14.0, 14.0));
                                ui.painter().rect_filled(swatch, 3.0, Self::rgba_to_color32(*color));
                                ui.painter().rect_stroke(
                                    swatch,
                                    3.0,
                                    egui::Stroke::new(1.0, Color32::BLACK),
                                    egui::StrokeKind::Outside,
                                );
                                swatch_pos.x += 18.0;
                            }
                            let region_rect = if let (Some(x), Some(y), Some(w), Some(h)) = (
                                preset.search_region_screen_x,
                                preset.search_region_screen_y,
                                preset.search_region_width,
                                preset.search_region_height,
                            ) {
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        preview_rect.left() + (x.max(0) as f32 * scale),
                                        preview_rect.top() + (y.max(0) as f32 * scale),
                                    ),
                                    vec2(
                                        (w.max(1) as f32 * scale).max(10.0),
                                        (h.max(1) as f32 * scale).max(10.0),
                                    ),
                                )
                            } else {
                                preview_rect.shrink2(vec2(preview_rect.width() * 0.18, preview_rect.height() * 0.18))
                            };
                            let region_fill = if preset.show_search_region_overlay {
                                Color32::from_rgba_unmultiplied(0, 255, 170, 28)
                            } else {
                                Color32::TRANSPARENT
                            };
                            if region_fill != Color32::TRANSPARENT {
                                if preset.search_region_is_circle {
                                    let radius = 0.5 * region_rect.width().min(region_rect.height());
                                    ui.painter().circle_filled(region_rect.center(), radius, region_fill);
                                } else {
                                    ui.painter().rect_filled(region_rect, 6.0, region_fill);
                                }
                            }
                            if preset.search_region_is_circle {
                                let radius = 0.5 * region_rect.width().min(region_rect.height());
                                ui.painter().circle_stroke(
                                    region_rect.center(),
                                    radius,
                                    egui::Stroke::new(2.0, Color32::from_rgb(0, 255, 170)),
                                );
                            } else {
                                ui.painter().rect_stroke(
                                    region_rect,
                                    6.0,
                                    egui::Stroke::new(2.0, Color32::from_rgb(0, 255, 170)),
                                    egui::StrokeKind::Outside,
                                );
                            }
                            if let Some(color) = preset.target_color {
                                let swatch = egui::Rect::from_min_size(
                                    region_rect.left_top() + vec2(6.0, 6.0),
                                    vec2(14.0, 14.0),
                                );
                                ui.painter().rect_filled(swatch, 3.0, Self::rgba_to_color32(color));
                                ui.painter().rect_stroke(
                                    swatch,
                                    3.0,
                                    egui::Stroke::new(1.0, Color32::BLACK),
                                    egui::StrokeKind::Outside,
                                );
                            }
                            ui.add_space(6.0);
                            ui.label(format!(
                                "{} {}",
                                Self::tr_lang(language, "Tolerance:", "Sai số:"),
                                preset.color_tolerance
                            ));
                            ui.label(format!(
                                "{} {}",
                                Self::tr_lang(language, "Scan rate:", "Tốc độ quét:"),
                                preset.color_scan_rate_hz
                            ));
                            ui.label(format!(
                                "{} {}",
                                Self::tr_lang(language, "Move offset:", "Độ lệch di chuyển:"),
                                format!("{}, {}", preset.move_offset_x, preset.move_offset_y)
                            ));
                            if preset.is_pixel_counter && !preset.pixel_counter_variable_name.trim().is_empty() {
                                ui.label(format!(
                                    "{} {}",
                                    Self::tr_lang(language, "Pixel variable:", "Biến pixel:"),
                                    preset.pixel_counter_variable_name
                                ));
                            }
                            if *trigger_macro_enabled {
                                ui.label(format!(
                                    "{} {}",
                                    Self::tr_lang(language, "Trigger macro:", "Kích hoạt macro:"),
                                    trigger_macro_name.as_deref().unwrap_or("-")
                                ));
                            }
                            if *move_cursor {
                                ui.label(Self::tr_lang(language, "Move cursor on match", "Di chuột khi tìm thấy"));
                            }
                            if *wait_until_found {
                                ui.label(Self::tr_lang(language, "Wait until found", "Chờ cho tới khi tìm thấy"));
                            }
                        }
                        MacroStepHoverPreviewKind::Hud {
                            preset_name,
                            preset,
                            text,
                            duration_override_ms,
                            timed_override,
                        } => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(preset_name).strong().color(Color32::from_rgb(124, 240, 164)));
                                ui.label(format!(
                                    "{} {}",
                                    Self::tr_lang(language, "Duration:", "Thời gian:"),
                                    if *timed_override {
                                        format!("{} ms", duration_override_ms)
                                    } else {
                                        Self::tr_lang(language, "until macro ends", "đến khi macro kết thúc").to_owned()
                                    }
                                ));
                            });
                            ui.add_space(6.0);
                            let mut preview_preset = preset.clone();
                            let resolved_text = crate::overlay::interpolate_variables(text.trim());
                            preview_preset.text = if resolved_text.is_empty() {
                                Self::tr_lang(language, "No text", "Không có text").to_owned()
                            } else {
                                resolved_text
                            };
                            Self::render_hud_rect_editor(
                                ui,
                                (preview.source_id, "hover-hud-preview"),
                                &mut preview_preset,
                            );
                            ui.add_space(4.0);
                            ui.label(format!(
                                "{} {}",
                                Self::tr_lang(language, "Position:", "Vị trí:"),
                                format!("{}, {}", preset.x, preset.y)
                            ));
                            ui.label(format!(
                                "{} {}x{}",
                                Self::tr_lang(language, "Size:", "Kích thước:"),
                                preset.width,
                                preset.height
                            ));
                        }
                        MacroStepHoverPreviewKind::Crosshair {
                            profile_name,
                            style,
                            disabled,
                        } => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(profile_name).strong().color(Color32::from_rgb(124, 240, 164)));
                                if *disabled {
                                    ui.label(RichText::new(Self::tr_lang(language, "Disabled", "Đang tắt")).color(Color32::from_rgb(255, 120, 120)).strong());
                                }
                            });
                            ui.add_space(6.0);
                            let (_, preview_rect, scale) = Self::render_hover_preview_screen_canvas(ui, 340.0);
                            let center = egui::pos2(
                                preview_rect.left() + style.x_offset as f32 * scale,
                                preview_rect.top() + style.y_offset as f32 * scale,
                            );
                            let arm_color = Self::rgba_to_color32(style.color);
                            let outline_color = Self::rgba_to_color32(style.outline_color);
                            let thickness = (style.thickness.max(1.0) * scale).max(1.0);
                            let gap = style.gap.max(0.0) * scale;
                            let h_len = style.horizontal_length.max(0.0) * scale;
                            let v_len = style.vertical_length.max(0.0) * scale;
                            let outline = if style.outline_enabled {
                                style.outline_thickness.max(0.0) * scale
                            } else {
                                0.0
                            };
                            let fill_rect = |painter: &egui::Painter, x: f32, y: f32, w: f32, h: f32, color: Color32| {
                                painter.rect_filled(
                                    egui::Rect::from_min_size(egui::pos2(x, y), vec2(w, h)),
                                    0.0,
                                    color,
                                );
                            };
                            if outline > 0.0 {
                                fill_rect(
                                    ui.painter(),
                                    center.x - gap - h_len - outline,
                                    center.y - thickness / 2.0 - outline,
                                    h_len + outline * 2.0,
                                    thickness + outline * 2.0,
                                    outline_color,
                                );
                                fill_rect(
                                    ui.painter(),
                                    center.x + gap - outline,
                                    center.y - thickness / 2.0 - outline,
                                    h_len + outline * 2.0,
                                    thickness + outline * 2.0,
                                    outline_color,
                                );
                                fill_rect(
                                    ui.painter(),
                                    center.x - thickness / 2.0 - outline,
                                    center.y - gap - v_len - outline,
                                    thickness + outline * 2.0,
                                    v_len + outline * 2.0,
                                    outline_color,
                                );
                                fill_rect(
                                    ui.painter(),
                                    center.x - thickness / 2.0 - outline,
                                    center.y + gap - outline,
                                    thickness + outline * 2.0,
                                    v_len + outline * 2.0,
                                    outline_color,
                                );
                            }
                            fill_rect(
                                ui.painter(),
                                center.x - gap - h_len,
                                center.y - thickness / 2.0,
                                h_len,
                                thickness,
                                arm_color,
                            );
                            fill_rect(
                                ui.painter(),
                                center.x + gap,
                                center.y - thickness / 2.0,
                                h_len,
                                thickness,
                                arm_color,
                            );
                            fill_rect(
                                ui.painter(),
                                center.x - thickness / 2.0,
                                center.y - gap - v_len,
                                thickness,
                                v_len,
                                arm_color,
                            );
                            fill_rect(
                                ui.painter(),
                                center.x - thickness / 2.0,
                                center.y + gap,
                                thickness,
                                v_len,
                                arm_color,
                            );
                            if style.center_dot {
                                let dot = style.center_dot_size.max(1.0) * scale;
                                ui.painter().rect_filled(
                                    egui::Rect::from_center_size(center, vec2(dot, dot)),
                                    0.0,
                                    arm_color,
                                );
                            }
                            ui.add_space(4.0);
                            ui.label(format!(
                                "{} {}, {}",
                                Self::tr_lang(language, "Gap/Thickness:", "Khoảng cách/Độ dày:"),
                                style.gap,
                                style.thickness
                            ));
                            ui.label(format!(
                                "{} {}, {}",
                                Self::tr_lang(language, "Lengths:", "Độ dài:"),
                                style.horizontal_length,
                                style.vertical_length
                            ));
                        }
                        MacroStepHoverPreviewKind::MouseMoveAbsolute { x, y } => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(Self::tr_lang(language, "Mouse move target", "Điểm di chuyển chuột")).strong());
                            });
                            ui.add_space(6.0);
                            let (_, preview_rect, scale) = Self::render_hover_preview_screen_canvas(ui, 320.0);
                            let px = preview_rect.left() + (*x as f32 * scale);
                            let py = preview_rect.top() + (*y as f32 * scale);
                            ui.painter().circle_filled(egui::pos2(px, py), 4.5, Color32::from_rgb(0, 255, 170));
                            ui.painter().line_segment(
                                [egui::pos2(px - 8.0, py), egui::pos2(px + 8.0, py)],
                                egui::Stroke::new(1.0, Color32::from_rgb(0, 255, 170)),
                            );
                            ui.painter().line_segment(
                                [egui::pos2(px, py - 8.0), egui::pos2(px, py + 8.0)],
                                egui::Stroke::new(1.0, Color32::from_rgb(0, 255, 170)),
                            );
                            ui.add_space(4.0);
                            ui.label(format!("X: {}  Y: {}", x, y));
                        }
                        MacroStepHoverPreviewKind::WindowResize {
                            preset_name,
                            preset,
                        } => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(preset_name).strong().color(Color32::from_rgb(124, 240, 164)));
                            });
                            ui.add_space(4.0);
                            let mut preview_preset = preset.clone();
                            let mut live_sync = false;
                            Self::render_window_preset_preview(ui, language, &mut preview_preset, None, &mut live_sync);
                        }
                        MacroStepHoverPreviewKind::Pin {
                            preset_name,
                            preset,
                            disabled,
                            disable_all,
                        } => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(preset_name).strong().color(Color32::from_rgb(124, 240, 164)));
                                if *disabled {
                                    ui.label(RichText::new(Self::tr_lang(language, "Disabled", "Đang tắt")).color(Color32::from_rgb(255, 120, 120)).strong());
                                }
                                if *disable_all {
                                    ui.label(RichText::new(Self::tr_lang(language, "All", "Tất cả")).weak());
                                }
                            });
                            ui.add_space(6.0);
                            let (_, preview_rect, scale) = Self::render_hover_preview_screen_canvas(ui, 320.0);
                            let target_rect = egui::Rect::from_min_size(
                                egui::pos2(
                                    preview_rect.left() + preset.x.max(0) as f32 * scale,
                                    preview_rect.top() + preset.y.max(0) as f32 * scale,
                                ),
                                vec2(
                                    preset.width.max(1) as f32 * scale,
                                    preset.height.max(1) as f32 * scale,
                                ),
                            );
                            let source_preview_size = vec2(
                                (preset.source_width.max(1) as f32 * scale).min(preview_rect.width() * 0.36),
                                (preset.source_height.max(1) as f32 * scale).min(preview_rect.height() * 0.36),
                            );
                            let source_rect = egui::Rect::from_min_size(
                                preview_rect.left_top() + vec2(10.0, 10.0),
                                vec2(source_preview_size.x.max(44.0), source_preview_size.y.max(28.0)),
                            );
                            ui.painter().rect_stroke(
                                source_rect,
                                6.0,
                                egui::Stroke::new(1.5, Color32::from_rgb(0, 191, 255)),
                                egui::StrokeKind::Outside,
                            );
                            ui.painter().rect_stroke(
                                target_rect,
                                6.0,
                                egui::Stroke::new(1.5, Color32::from_rgb(0, 255, 170)),
                                egui::StrokeKind::Outside,
                            );
                            ui.painter().text(
                                source_rect.center_top() + vec2(0.0, -12.0),
                                egui::Align2::CENTER_CENTER,
                                Self::tr_lang(language, "Source", "Nguồn"),
                                egui::FontId::proportional(11.0),
                                Color32::from_rgb(0, 191, 255),
                            );
                            ui.painter().text(
                                target_rect.center_top() + vec2(0.0, -12.0),
                                egui::Align2::CENTER_CENTER,
                                Self::tr_lang(language, "Pin box", "Khung ghim"),
                                egui::FontId::proportional(11.0),
                                Color32::from_rgb(0, 255, 170),
                            );
                            ui.add_space(4.0);
                            ui.label(format!(
                                "{} {}, {}",
                                Self::tr_lang(language, "Target box:", "Khung ghim:"),
                                preset.width,
                                preset.height
                            ));
                            ui.label(format!(
                                "{} {}, {}",
                                Self::tr_lang(language, "Source box:", "Khung nguồn:"),
                                preset.source_width,
                                preset.source_height
                            ));
                            ui.label(format!(
                                "{} {}",
                                Self::tr_lang(language, "Overlay style:", "Kiểu ghim:"),
                                match preset.overlay_style {
                                    PinOverlayStyle::Rectangle => Self::tr_lang(language, "Rectangle", "Chữ nhật"),
                                    PinOverlayStyle::Circle => Self::tr_lang(language, "Circle", "Tròn"),
                                    PinOverlayStyle::HorizontalBar => Self::tr_lang(language, "Bar", "Thanh ngang"),
                                }
                            ));
                        }
                        MacroStepHoverPreviewKind::FocusWindow { window_title } => {
                            ui.label(format!(
                                "{} {}",
                                Self::tr_lang(language, "Focused window:", "Cửa sổ focus:"),
                                window_title
                            ));
                        }
                        MacroStepHoverPreviewKind::Generic { lines } => {
                            for line in lines {
                                ui.label(line);
                            }
                        }
                    }
                });
        }
    }

    fn render_hover_preview_popup(
        ctx: &egui::Context,
        language: UiLanguage,
        preview: Option<&MacroStepHoverPreview>,
        anchor_pos: egui::Pos2,
    ) -> bool {
        let Some(preview) = preview else {
            return false;
        };
        let popup_id = egui::Id::new(("macro-hover-preview-popup", preview.source_id));
        let mut pos = anchor_pos + vec2(16.0, 18.0);
        let content_rect = ctx.content_rect();
        let estimated_size = match &preview.kind {
            MacroStepHoverPreviewKind::MacroPreset { .. }
            | MacroStepHoverPreviewKind::StepToggle { .. } => vec2(360.0, 180.0),
            MacroStepHoverPreviewKind::Vision { .. } => vec2(560.0, 440.0),
            MacroStepHoverPreviewKind::Hud { .. } => vec2(600.0, 520.0),
            MacroStepHoverPreviewKind::Crosshair { .. } => vec2(560.0, 420.0),
            MacroStepHoverPreviewKind::MouseMoveAbsolute { .. } => vec2(560.0, 380.0),
            MacroStepHoverPreviewKind::WindowResize { .. } => vec2(560.0, 440.0),
            MacroStepHoverPreviewKind::Pin { .. } => vec2(560.0, 420.0),
            MacroStepHoverPreviewKind::FocusWindow { .. } => vec2(240.0, 110.0),
            MacroStepHoverPreviewKind::Generic { .. } => vec2(240.0, 120.0),
        };
        if pos.x + estimated_size.x > content_rect.right() {
            pos.x = (anchor_pos.x - estimated_size.x - 16.0).max(content_rect.left() + 6.0);
        }
        if pos.y + estimated_size.y > content_rect.bottom() {
            pos.y = (anchor_pos.y - estimated_size.y - 16.0).max(content_rect.top() + 6.0);
        }
        let area_response = egui::Area::new(popup_id)
            .order(egui::Order::Tooltip)
            .fixed_pos(pos)
            .interactable(true)
            .show(ctx, |ui| {
                Self::render_hover_preview_panel(ui, language, Some(preview));
            });
        let popup_hovered = area_response.response.hovered();
        if popup_hovered {
            if matches!(
                &preview.kind,
                MacroStepHoverPreviewKind::MacroPreset { .. }
                    | MacroStepHoverPreviewKind::StepToggle { .. }
            ) {
                let scroll_y = ctx.input(|i| i.raw_scroll_delta.y);
                if scroll_y.abs() > 0.0 {
                    let offset_id = preview.source_id.with("macro-hover-preview-offset");
                    let line_count = match &preview.kind {
                        MacroStepHoverPreviewKind::MacroPreset { steps, .. } => {
                            Self::macro_step_preview_visible_entries(steps).len() as i32
                        }
                        MacroStepHoverPreviewKind::StepToggle { steps, .. } => {
                            Self::macro_step_preview_visible_entries(steps).len() as i32
                        }
                        _ => 0,
                    };
                    let visible_lines = 5i32;
                    let max_offset = (line_count - visible_lines).max(0);
                    let mut offset = ctx
                        .data(|data| data.get_temp::<i32>(offset_id))
                        .unwrap_or(0);
                    let delta = if scroll_y > 0.0 { -1 } else { 1 };
                    offset = (offset + delta).clamp(0, max_offset);
                    ctx.data_mut(|data| data.insert_temp(offset_id, offset));
                    ctx.request_repaint();
                }
            }
        }
        popup_hovered
    }

    fn build_hover_preview_request(
        language: UiLanguage,
        source_id: egui::Id,
        step: &MacroStep,
    ) -> Option<HoverPreviewRequest> {
        match step.action {
            MacroAction::TriggerMacroPreset => Some(HoverPreviewRequest::MacroPreset {
                source_id,
                preset_id: step.key.trim().parse::<u32>().ok().unwrap_or(0),
                mode_label: Self::tr_lang(language, "Trigger macro", "Kích hoạt macro").to_owned(),
            }),
            MacroAction::EnableMacroPreset => Some(HoverPreviewRequest::MacroPreset {
                source_id,
                preset_id: step.key.trim().parse::<u32>().ok().unwrap_or(0),
                mode_label: Self::tr_lang(language, "Enable macro", "Bật macro").to_owned(),
            }),
            MacroAction::DisableMacroPreset => Some(HoverPreviewRequest::MacroPreset {
                source_id,
                preset_id: step.key.trim().parse::<u32>().ok().unwrap_or(0),
                mode_label: Self::tr_lang(language, "Disable macro", "Tắt macro").to_owned(),
            }),
            MacroAction::EnableStep | MacroAction::DisableStep => {
                let (preset_id, steps) = {
                    let parts: Vec<&str> = step.key.split('|').collect();
                    if parts.len() == 2 {
                        (
                            parts[0].trim().parse::<u32>().ok().unwrap_or(0),
                            parts[1]
                                .split(',')
                                .filter_map(|value| value.trim().parse::<u32>().ok())
                                .collect::<Vec<_>>(),
                        )
                    } else {
                        (
                            step.key.trim().parse::<u32>().ok().unwrap_or(0),
                            step.key
                                .split(',')
                                .filter_map(|value| value.trim().parse::<u32>().ok())
                                .collect::<Vec<_>>(),
                        )
                    }
                };
                Some(HoverPreviewRequest::StepToggle {
                    source_id,
                    preset_id,
                    mode_label: if step.action == MacroAction::EnableStep {
                        Self::tr_lang(language, "Enable steps", "Bật step").to_owned()
                    } else {
                        Self::tr_lang(language, "Disable steps", "Tắt step").to_owned()
                    },
                    selected_steps: steps,
                })
            }
            _ => None,
        }
    }

    fn resolve_hover_preview_request(
        &self,
        group_id: u32,
        request: HoverPreviewRequest,
    ) -> Option<MacroStepHoverPreview> {
        match request {
            HoverPreviewRequest::MacroPreset {
                source_id,
                preset_id,
                mode_label,
            } => {
                let group = self.state.macro_groups.iter().find(|group| group.id == group_id)?;
                let preset = group.presets.iter().find(|preset| preset.id == preset_id)?.clone();
                let preset_label = Self::format_macro_trigger_ui(self.state.ui_language, &preset);
                let title = format!("{}: {}", mode_label, preset_label.clone());
                Some(MacroStepHoverPreview {
                    source_id,
                    title,
                    kind: MacroStepHoverPreviewKind::MacroPreset {
                        mode_label,
                        preset_name: preset_label,
                        steps: preset.steps,
                    },
                })
            }
            HoverPreviewRequest::StepToggle {
                source_id,
                preset_id,
                mode_label,
                selected_steps,
            } => {
                let group = self.state.macro_groups.iter().find(|group| group.id == group_id)?;
                let preset = group.presets.iter().find(|preset| preset.id == preset_id)?.clone();
                let preset_label = Self::format_macro_trigger_ui(self.state.ui_language, &preset);
                let title = format!("{}: {}", mode_label, preset_label.clone());
                Some(MacroStepHoverPreview {
                    source_id,
                    title,
                    kind: MacroStepHoverPreviewKind::StepToggle {
                        mode_label,
                        preset_name: preset_label,
                        steps: preset.steps,
                        selected_steps,
                    },
                })
            }
            HoverPreviewRequest::Vision {
                source_id,
                preset_id,
                action_label,
                move_cursor,
                wait_until_found,
                trigger_macro_enabled,
                trigger_macro_preset_id,
            } => {
                let preset = self
                    .state
                    .vision_presets
                    .iter()
                    .find(|preset| preset.id == preset_id)?
                    .clone();
                let trigger_macro_name = if trigger_macro_enabled {
                    trigger_macro_preset_id.and_then(|macro_id| {
                        self.state
                            .macro_groups
                            .iter()
                            .find(|group| group.id == group_id)
                            .and_then(|group| {
                                group
                                    .presets
                                    .iter()
                                    .find(|macro_preset| macro_preset.id == macro_id)
                                    .map(|macro_preset| {
                                        Self::format_macro_trigger_ui(self.state.ui_language, macro_preset)
                                    })
                            })
                    })
                } else {
                    None
                };
                let preset_name = preset.name.clone();
                let title = format!("{}: {}", action_label, preset_name.clone());
                Some(MacroStepHoverPreview {
                    source_id,
                    title,
                    kind: MacroStepHoverPreviewKind::Vision {
                        action_label,
                        preset_name,
                        preset,
                        trigger_macro_name,
                        move_cursor,
                        wait_until_found,
                        trigger_macro_enabled,
                    },
                })
            }
            HoverPreviewRequest::Hud {
                source_id,
                preset_id,
                text_override,
                duration_override_ms,
                timed_override,
            } => {
                let preset = self
                    .state
                    .hud_presets
                    .iter()
                    .find(|preset| preset.id == preset_id)?
                    .clone();
                let text = if text_override.trim().is_empty() {
                    preset.text.clone()
                } else {
                    text_override
                };
                let preset_name = preset.name.clone();
                let title = format!("HUD: {}", preset_name.clone());
                Some(MacroStepHoverPreview {
                    source_id,
                    title,
                    kind: MacroStepHoverPreviewKind::Hud {
                        preset_name,
                        preset,
                        text,
                        duration_override_ms,
                        timed_override,
                    },
                })
            }
            HoverPreviewRequest::Crosshair {
                source_id,
                profile_name,
                disabled,
            } => {
                let profile = self
                    .state
                    .profiles
                    .iter()
                    .find(|profile| profile.name.eq_ignore_ascii_case(&profile_name))
                    .cloned()
                    .unwrap_or_else(|| ProfileRecord {
                        name: profile_name.clone(),
                        enabled: true,
                        collapsed: true,
                        style: CrosshairStyle::default(),
                        target_window_title: None,
                        extra_target_window_titles: Vec::new(),
                    });
                let title = format!(
                    "{}: {}",
                    Self::tr_lang(
                        self.state.ui_language,
                        "Crosshair",
                        "Tâm ngắm",
                    ),
                    profile.name.clone()
                );
                Some(MacroStepHoverPreview {
                    source_id,
                    title,
                    kind: MacroStepHoverPreviewKind::Crosshair {
                        profile_name: profile.name,
                        style: profile.style,
                        disabled,
                    },
                })
            }
            HoverPreviewRequest::MouseMoveAbsolute { source_id, x, y } => Some(MacroStepHoverPreview {
                source_id,
                title: Self::tr_lang(self.state.ui_language, "Mouse move absolute", "Di chuyển chuột tuyệt đối").to_owned(),
                kind: MacroStepHoverPreviewKind::MouseMoveAbsolute { x, y },
            }),
            HoverPreviewRequest::WindowResize {
                source_id,
                preset_id,
            } => {
                let preset = self
                    .state
                    .window_presets
                    .iter()
                    .find(|preset| preset.id == preset_id)?
                    .clone();
                let preset_name = preset.name.clone();
                let title = format!(
                    "{}: {}",
                    Self::tr_lang(self.state.ui_language, "Resize", "Kích thước"),
                    preset_name.clone()
                );
                Some(MacroStepHoverPreview {
                    source_id,
                    title,
                    kind: MacroStepHoverPreviewKind::WindowResize {
                        preset_name,
                        preset,
                    },
                })
            }
            HoverPreviewRequest::Pin {
                source_id,
                preset_id,
                disabled,
                disable_all,
            } => {
                let preset = self
                    .state
                    .pin_presets
                    .iter()
                    .find(|preset| preset.id == preset_id)?
                    .clone();
                let preset_name = preset.name.clone();
                let title = format!(
                    "{}: {}",
                    Self::tr_lang(self.state.ui_language, "Pin", "Ghim"),
                    preset_name.clone()
                );
                Some(MacroStepHoverPreview {
                    source_id,
                    title,
                    kind: MacroStepHoverPreviewKind::Pin {
                        preset_name,
                        preset,
                        disabled,
                        disable_all,
                    },
                })
            }
            HoverPreviewRequest::FocusWindow {
                source_id,
                window_title,
            } => Some(MacroStepHoverPreview {
                source_id,
                title: Self::tr_lang(self.state.ui_language, "Window focus", "Focus cửa sổ").to_owned(),
                kind: MacroStepHoverPreviewKind::FocusWindow { window_title },
            }),
            HoverPreviewRequest::Generic {
                source_id,
                title,
                lines,
            } => Some(MacroStepHoverPreview {
                source_id,
                title,
                kind: MacroStepHoverPreviewKind::Generic { lines },
            }),
        }
    }

    fn mouse_macro_actions() -> &'static [MacroAction] {
        &[
            MacroAction::MouseLeftClick,
            MacroAction::MouseLeftDown,
            MacroAction::MouseLeftUp,
            MacroAction::MouseRightClick,
            MacroAction::MouseRightDown,
            MacroAction::MouseRightUp,
            MacroAction::MouseMiddleClick,
            MacroAction::MouseMiddleDown,
            MacroAction::MouseMiddleUp,
            MacroAction::MouseX1Click,
            MacroAction::MouseX1Down,
            MacroAction::MouseX1Up,
            MacroAction::MouseX2Click,
            MacroAction::MouseX2Down,
            MacroAction::MouseX2Up,
            MacroAction::MouseWheelUp,
            MacroAction::MouseWheelDown,
            MacroAction::MouseMoveAbsolute,
            MacroAction::MouseMoveRelative,
            MacroAction::LockMouse,
            MacroAction::UnlockMouse,
            MacroAction::PlayMousePathPreset,
        ]
    }
    fn macro_action_is_mouse(action: MacroAction) -> bool {
        Self::mouse_macro_actions().contains(&action)
    }
    fn mouse_click_action_groups() -> &'static [(MacroAction, MacroAction, MacroAction, &'static str)] {
        &[
            (
                MacroAction::MouseLeftClick,
                MacroAction::MouseLeftDown,
                MacroAction::MouseLeftUp,
                "mouse-left-click",
            ),
            (
                MacroAction::MouseRightClick,
                MacroAction::MouseRightDown,
                MacroAction::MouseRightUp,
                "mouse-right-click",
            ),
            (
                MacroAction::MouseMiddleClick,
                MacroAction::MouseMiddleDown,
                MacroAction::MouseMiddleUp,
                "mouse-middle-click",
            ),
            (
                MacroAction::MouseX1Click,
                MacroAction::MouseX1Down,
                MacroAction::MouseX1Up,
                "mouse-x1-click",
            ),
            (
                MacroAction::MouseX2Click,
                MacroAction::MouseX2Down,
                MacroAction::MouseX2Up,
                "mouse-x2-click",
            ),
        ]
    }
    fn mouse_leaf_action_groups() -> &'static [MacroAction] {
        &[
            MacroAction::MouseWheelUp,
            MacroAction::MouseWheelDown,
            MacroAction::MouseMoveAbsolute,
            MacroAction::MouseMoveRelative,
            MacroAction::LockMouse,
            MacroAction::UnlockMouse,
            MacroAction::PlayMousePathPreset,
        ]
    }
    fn if_action_groups() -> &'static [MacroAction] {
        &[MacroAction::IfStart, MacroAction::Else, MacroAction::IfEnd]
    }
    fn render_mouse_click_action_group_option(
        ui: &mut egui::Ui,
        language: UiLanguage,
        id_source: impl std::hash::Hash + Copy,
        current: &mut MacroAction,
        live_sync: &mut bool,
        base_action: MacroAction,
        down_action: MacroAction,
        up_action: MacroAction,
        popup_key: &'static str,
    ) {
        let selected = matches!(*current, action if action == base_action || action == down_action || action == up_action);
        let popup_id = ui.make_persistent_id((id_source, popup_key, "popup"));
        let popup_rect_id = ui.make_persistent_id((id_source, popup_key, "rect"));
        let active_mouse_click_popup_key_id =
            ui.make_persistent_id((id_source, "mouse-click-active-submenu-key"));
        let mut open = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(popup_id))
            .unwrap_or(false);
        let inner = ui.allocate_ui_with_layout(
            vec2(58.0, 42.0),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                let label_color = if selected {
                    ui.visuals().strong_text_color()
                } else {
                    ui.visuals().text_color()
                };
                let response = ui.add_sized(
                    [34.0, 24.0],
                    Button::new(Self::macro_action_icon_text(base_action)).selected(selected),
                );
                if response.hovered() || response.clicked() {
                    Self::clear_mouse_click_submenus(ui, id_source);
                    open = true;
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(active_mouse_click_popup_key_id, Some(popup_key))
                    });
                }
                if response.clicked() {
                    *current = base_action;
                    *live_sync = true;
                    ui.close();
                }
                let _popup_response = egui::Popup::from_response(&response)
                    .id(popup_id)
                    .open_bool(&mut open)
                    .align(egui::RectAlign::TOP_START)
                    .layout(egui::Layout::top_down_justified(egui::Align::Min))
                    .width(140.0)
                    .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
                    .show(|ui| {
                        let rect = ui.max_rect();
                        ui.ctx().data_mut(|data| data.insert_temp(popup_rect_id, rect));
                        egui::Grid::new((id_source, popup_key, "grid"))
                            .num_columns(2)
                            .spacing([6.0, 6.0])
                            .show(ui, |ui| {
                                Self::render_macro_action_option(
                                    ui,
                                    language,
                                    current,
                                    down_action,
                                    live_sync,
                                    popup_id,
                                    true,
                                );
                                Self::render_macro_action_option(
                                    ui,
                                    language,
                                    current,
                                    up_action,
                                    live_sync,
                                    popup_id,
                                    true,
                                );
                            });
                    });
                let popup_rect: Option<egui::Rect> =
                    ui.ctx().data(|data| data.get_temp(popup_rect_id));
                if open {
                    if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
                        let mut keep_open_rect = response.rect.expand(2.0);
                        if let Some(rect) = popup_rect {
                            keep_open_rect = keep_open_rect.union(rect.expand(2.0));
                        }
                        if !keep_open_rect.contains(pointer_pos) {
                            open = false;
                            ui.ctx().request_repaint();
                        }
                    } else {
                        open = false;
                        ui.ctx().request_repaint();
                    }
                }
                let active_popup_key = ui
                    .ctx()
                    .data(|data| data.get_temp::<Option<&'static str>>(active_mouse_click_popup_key_id))
                    .flatten();
                if let Some(active_popup_key) = active_popup_key {
                    if active_popup_key != popup_key {
                        open = false;
                    }
                }
                ui.ctx().data_mut(|data| data.insert_temp(popup_id, open));
                ui.label(
                    RichText::new(Self::macro_action_short_label(base_action, language))
                        .size(9.0)
                        .color(label_color),
                );
                response
            },
        );
        let response = inner.inner;
        if !open {
            Self::show_instant_hover_tooltip(
                ui,
                &response,
                Self::macro_action_tooltip(base_action, language),
            );
        }
    }
    fn render_if_action_group_option(
        ui: &mut egui::Ui,
        language: UiLanguage,
        id_source: impl std::hash::Hash + Copy,
        current: &mut MacroAction,
        live_sync: &mut bool,
        action_hover_id: egui::Id,
    ) {
        let selected = matches!(*current, MacroAction::IfStart | MacroAction::Else | MacroAction::IfEnd);
        let owner_id = ui.make_persistent_id("macro-action-submenu-owner");
        let popup_id = ui.make_persistent_id((id_source, "if-submenu-popup"));
        let popup_rect_id = ui.make_persistent_id((id_source, "if-submenu-rect"));
        let mouse_popup_id = ui.make_persistent_id((id_source, "mouse-submenu-popup"));
        let image_popup_id = ui.make_persistent_id((id_source, "image-search-submenu-popup"));
        let timer_popup_id = ui.make_persistent_id((id_source, "timer-submenu-popup"));
        let active_owner = ui
            .ctx()
            .data(|data| data.get_temp::<MacroActionSubmenuKind>(owner_id));
        let mut open = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(popup_id))
            .unwrap_or(false);
        if active_owner.is_some_and(|kind| kind != MacroActionSubmenuKind::If) {
            open = false;
        }
        let inner = ui.allocate_ui_with_layout(
            vec2(58.0, 42.0),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                let label_color = if selected {
                    ui.visuals().strong_text_color()
                } else {
                    ui.visuals().text_color()
                };
                let response = ui.add_sized(
                    [34.0, 24.0],
                    Button::new(Self::macro_action_icon_text(MacroAction::IfStart)).selected(selected),
                );
                if response.hovered() || response.clicked() {
                    Self::clear_macro_action_submenus(ui, id_source);
                    open = true;
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(owner_id, MacroActionSubmenuKind::If));
                    ui.ctx().data_mut(|data| data.insert_temp(action_hover_id, true));
                    ui.ctx().data_mut(|data| data.insert_temp(mouse_popup_id, false));
                    ui.ctx().data_mut(|data| data.insert_temp(image_popup_id, false));
                    ui.ctx().data_mut(|data| data.insert_temp(timer_popup_id, false));
                }
                let _popup_response = egui::Popup::from_response(&response)
                    .id(popup_id)
                    .open_bool(&mut open)
                    .align(egui::RectAlign::BOTTOM_START)
                    .layout(egui::Layout::top_down_justified(egui::Align::Min))
                    .width(176.0)
                    .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
                    .show(|ui| {
                        let rect = ui.max_rect();
                        ui.ctx().data_mut(|data| data.insert_temp(popup_rect_id, rect));
                        egui::Grid::new((id_source, "if-action-grid"))
                            .num_columns(2)
                            .spacing([6.0, 6.0])
                            .show(ui, |ui| {
                                for action in Self::if_action_groups().iter().copied() {
                                    Self::render_macro_action_option(
                                        ui,
                                        language,
                                        current,
                                        action,
                                        live_sync,
                                        action_hover_id,
                                        true,
                                    );
                                }
                            });
                    });
                let popup_rect: Option<egui::Rect> =
                    ui.ctx().data(|data| data.get_temp(popup_rect_id));
                if open {
                    if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
                        let mut keep_open_rect = response.rect.expand(2.0);
                        if let Some(rect) = popup_rect {
                            keep_open_rect = keep_open_rect.union(rect.expand(2.0));
                        }
                        if !keep_open_rect.contains(pointer_pos) {
                            open = false;
                            ui.ctx()
                                .data_mut(|data| data.insert_temp(owner_id, None::<MacroActionSubmenuKind>));
                            ui.ctx().request_repaint();
                        }
                    } else {
                        open = false;
                        ui.ctx()
                            .data_mut(|data| data.insert_temp(owner_id, None::<MacroActionSubmenuKind>));
                        ui.ctx().request_repaint();
                    }
                }
                ui.ctx().data_mut(|data| data.insert_temp(popup_id, open));
                ui.label(
                    RichText::new(Self::tr_lang(language, "IF", ""))
                        .size(9.0)
                        .color(label_color),
                );
                response
            },
        );
        let response = inner.inner;
        if !open {
            Self::show_instant_hover_tooltip(
                ui,
                &response,
                Self::macro_action_tooltip(MacroAction::IfStart, language),
            );
        }
    }
    fn render_mouse_action_group_option(
        ui: &mut egui::Ui,
        language: UiLanguage,
        id_source: impl std::hash::Hash + Copy,
        current: &mut MacroAction,
        live_sync: &mut bool,
        action_hover_id: egui::Id,
    ) {
        let selected = Self::macro_action_is_mouse(*current);
        let owner_id = ui.make_persistent_id("macro-action-submenu-owner");
        let active_mouse_click_popup_key_id =
            ui.make_persistent_id((id_source, "mouse-click-active-submenu-key"));
        let popup_id = ui.make_persistent_id((id_source, "mouse-submenu-popup"));
        let image_popup_id = ui.make_persistent_id((id_source, "image-search-submenu-popup"));
        let timer_popup_id = ui.make_persistent_id((id_source, "timer-submenu-popup"));
        let active_owner = ui
            .ctx()
            .data(|data| data.get_temp::<MacroActionSubmenuKind>(owner_id));
        let top_level_hovered = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(action_hover_id))
            .unwrap_or(false);
        let mut open = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(popup_id))
            .unwrap_or(false);
        if active_owner.is_some_and(|kind| kind != MacroActionSubmenuKind::Mouse) {
            open = false;
        }
        if top_level_hovered {
            open = false;
            Self::clear_macro_action_submenus(ui, id_source);
        }
        let inner = ui.allocate_ui_with_layout(
            vec2(58.0, 42.0),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                let response = ui.add_sized(
                    [34.0, 24.0],
                    Button::new(Self::material_icon_text(0xe323, 18.0)).selected(selected),
                );
                if response.hovered() || response.clicked() {
                    Self::clear_macro_action_submenus(ui, id_source);
                    open = true;
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(owner_id, MacroActionSubmenuKind::Mouse));
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(active_mouse_click_popup_key_id, None::<&'static str>)
                    });
                }
                let popup_rect_id = ui.make_persistent_id((id_source, "mouse-submenu-rect"));
                let popup_response = egui::Popup::from_response(&response)
                    .id(popup_id)
                    .open_bool(&mut open)
                    .align(egui::RectAlign::BOTTOM_START)
                    .layout(egui::Layout::top_down_justified(egui::Align::Min))
                    .width(372.0)
                    .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
                    .show(|ui| {
                        let rect = ui.max_rect();
                        ui.ctx().data_mut(|data| data.insert_temp(popup_rect_id, rect));
                        egui::Grid::new((id_source, "mouse-action-grid"))
                            .num_columns(6)
                            .spacing([6.0, 6.0])
                            .show(ui, |ui| {
                                let mut item_index = 0usize;
                                for (base_action, down_action, up_action, popup_key) in
                                    Self::mouse_click_action_groups().iter().copied()
                                {
                                    Self::render_mouse_click_action_group_option(
                                        ui,
                                        language,
                                        id_source,
                                        current,
                                        live_sync,
                                        base_action,
                                        down_action,
                                        up_action,
                                        popup_key,
                                    );
                                    item_index += 1;
                                    if item_index % 6 == 0 {
                                        ui.end_row();
                                    }
                                }
                                for action in Self::mouse_leaf_action_groups().iter().copied() {
                                    let leaf_response = Self::render_macro_action_button(
                                        ui,
                                        language,
                                        current,
                                        action,
                                        action_hover_id,
                                        true,
                                    );
                                    if leaf_response.hovered() || leaf_response.clicked() {
                                        Self::clear_mouse_click_submenus(ui, id_source);
                                        ui.ctx().data_mut(|data| {
                                            data.insert_temp(active_mouse_click_popup_key_id, None::<&'static str>)
                                        });
                                    }
                                    if leaf_response.clicked() {
                                        *current = action;
                                        *live_sync = true;
                                        ui.close();
                                    }
                                    item_index += 1;
                                    if item_index % 6 == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
                let active_mouse_click_popup_key = ui
                    .ctx()
                    .data(|data| data.get_temp::<Option<&'static str>>(active_mouse_click_popup_key_id))
                    .flatten();
                Self::close_inactive_mouse_click_submenus(
                    ui,
                    id_source,
                    active_mouse_click_popup_key,
                );
                let popup_rect: Option<egui::Rect> = ui.ctx().data(|data| data.get_temp(popup_rect_id));
                if open {
                    if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
                        let mut keep_open_rect = response.rect.expand(2.0);
                        if let Some(rect) = popup_rect {
                            keep_open_rect = keep_open_rect.union(rect.expand(2.0));
                            if rect.contains(pointer_pos) {
                                ui.ctx().data_mut(|data| {
                                    data.insert_temp(owner_id, MacroActionSubmenuKind::Mouse)
                                });
                            }
                        }
                        for (_, _, _, popup_key) in Self::mouse_click_action_groups().iter().copied() {
                            let child_popup_rect_id =
                                ui.make_persistent_id((id_source, popup_key, "rect"));
                            if let Some(rect) =
                                ui.ctx().data(|data| data.get_temp::<egui::Rect>(child_popup_rect_id))
                            {
                                keep_open_rect = keep_open_rect.union(rect.expand(2.0));
                            }
                        }
                        if !keep_open_rect.contains(pointer_pos) {
                            open = false;
                            Self::clear_macro_action_submenus(ui, id_source);
                        }
                    } else {
                        open = false;
                        Self::clear_macro_action_submenus(ui, id_source);
                    }
                }
                ui.ctx().data_mut(|data| data.insert_temp(popup_id, open));
                let label_color = if selected {
                    ui.visuals().strong_text_color()
                } else {
                    ui.visuals().text_color()
                };
                ui.label(
                    RichText::new(Self::tr_lang(language, "Mouse", "Chuá»™t"))
                        .size(9.0)
                        .color(label_color),
                );
                response
            },
        );
        let response = inner.inner;
        if !open {
            Self::show_instant_hover_tooltip(
                ui,
                &response,
                Self::tr_lang(
                    language,
                    "Mouse\nOpen mouse click, wheel, and move actions.",
                    "Chuá»™t\nMá»Ÿ cÃƒÂ¡c action click, lÃ„Æ’n vÃƒÂ  di chuyÃ¡Â»Æ’n chuá»™t.",
                ),
            );
        }
    }
    fn image_search_macro_actions() -> &'static [MacroAction] {
        &[
            MacroAction::StartVisionSearch,
            MacroAction::ScanVisionOnce,
            MacroAction::TriggerVisionMove,
            MacroAction::StopVision,
        ]
    }
    fn macro_action_is_image_search(action: MacroAction) -> bool {
        Self::image_search_macro_actions().contains(&action)
    }
    fn render_image_search_action_group_option(
        ui: &mut egui::Ui,
        language: UiLanguage,
        id_source: impl std::hash::Hash + Copy,
        current: &mut MacroAction,
        live_sync: &mut bool,
        action_hover_id: egui::Id,
    ) {
        let selected = Self::macro_action_is_image_search(*current);
        let owner_id = ui.make_persistent_id("macro-action-submenu-owner");
        let popup_id = ui.make_persistent_id((id_source, "image-search-submenu-popup"));
        let mouse_popup_id = ui.make_persistent_id((id_source, "mouse-submenu-popup"));
        let timer_popup_id = ui.make_persistent_id((id_source, "timer-submenu-popup"));
        let active_owner = ui
            .ctx()
            .data(|data| data.get_temp::<MacroActionSubmenuKind>(owner_id));
        let top_level_hovered = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(action_hover_id))
            .unwrap_or(false);
        let mut open = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(popup_id))
            .unwrap_or(false);
        if active_owner.is_some_and(|kind| kind != MacroActionSubmenuKind::ImageSearch) {
            open = false;
        }
        if top_level_hovered {
            open = false;
            ui.ctx()
                .data_mut(|data| data.insert_temp(owner_id, None::<MacroActionSubmenuKind>));
            ui.ctx().request_repaint();
        }
        let inner = ui.allocate_ui_with_layout(
            vec2(58.0, 42.0),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                let response = ui.add_sized(
                    [34.0, 24.0],
                    Button::new(Self::material_icon_text(0xe8b6, 18.0)).selected(selected),
                );
                if response.hovered() || response.clicked() {
                    Self::clear_macro_action_submenus(ui, id_source);
                    open = true;
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(owner_id, MacroActionSubmenuKind::ImageSearch)
                    });
                }
                let popup_rect_id = ui.make_persistent_id((id_source, "image-search-submenu-rect"));
                let popup_response = egui::Popup::from_response(&response)
                    .id(popup_id)
                    .open_bool(&mut open)
                    .align(egui::RectAlign::BOTTOM_START)
                    .layout(egui::Layout::top_down_justified(egui::Align::Min))
                    .width(220.0)
                    .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
                    .show(|ui| {
                        let rect = ui.max_rect();
                        ui.ctx().data_mut(|data| data.insert_temp(popup_rect_id, rect));
                        egui::Grid::new((id_source, "image-search-action-grid"))
                            .num_columns(3)
                            .spacing([6.0, 6.0])
                            .show(ui, |ui| {
                                for (index, action) in Self::image_search_macro_actions()
                                    .iter()
                                    .copied()
                                    .enumerate()
                                {
                                    Self::render_macro_action_option(
                                        ui,
                                        language,
                                        current,
                                        action,
                                        live_sync,
                                        action_hover_id,
                                        true,
                                    );
                                    if (index + 1) % 3 == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
                let popup_rect: Option<egui::Rect> = ui.ctx().data(|data| data.get_temp(popup_rect_id));
                if open {
                    if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
                        let mut keep_open_rect = response.rect.expand(2.0);
                        if let Some(rect) = popup_rect {
                            keep_open_rect = keep_open_rect.union(rect.expand(2.0));
                            if rect.contains(pointer_pos) {
                                ui.ctx().data_mut(|data| {
                                    data.insert_temp(owner_id, MacroActionSubmenuKind::ImageSearch)
                                });
                            }
                        }
                        if !keep_open_rect.contains(pointer_pos) {
                            open = false;
                            ui.ctx().data_mut(|data| data.insert_temp(owner_id, None::<MacroActionSubmenuKind>));
                            ui.ctx().request_repaint();
                        }
                    } else {
                        open = false;
                        ui.ctx().data_mut(|data| data.insert_temp(owner_id, None::<MacroActionSubmenuKind>));
                        ui.ctx().request_repaint();
                    }
                }
                ui.ctx().data_mut(|data| data.insert_temp(popup_id, open));
                let label_color = if selected {
                    ui.visuals().strong_text_color()
                } else {
                    ui.visuals().text_color()
                };
                ui.label(
                    RichText::new(Self::tr_lang(language, "Image", "Image"))
                        .size(9.0)
                        .color(label_color),
                );
                response
            },
        );
        let response = inner.inner;
        if !open {
            Self::show_instant_hover_tooltip(
                ui,
                &response,
                Self::tr_lang(
                    language,
                    "Image\nOpen image search start, trigger, and stop actions.",
                    "Image\nMá»Ÿ cÃƒÂ¡c action báº¯t Ã„â€˜Ã¡ÂºÂ§u, trigger vÃƒÂ  dÃ¡Â»Â«ng image search.",
                ),
            );
        }
    }
    fn timer_macro_actions() -> &'static [MacroAction] {
        &[
            MacroAction::StartTimerPreset,
            MacroAction::PauseTimerPreset,
            MacroAction::StopTimerPreset,
        ]
    }
    fn macro_action_is_timer(action: MacroAction) -> bool {
        Self::timer_macro_actions().contains(&action)
    }
    fn render_timer_action_group_option(
        ui: &mut egui::Ui,
        language: UiLanguage,
        id_source: impl std::hash::Hash + Copy,
        current: &mut MacroAction,
        live_sync: &mut bool,
        action_hover_id: egui::Id,
    ) {
        let selected = Self::macro_action_is_timer(*current);
        let owner_id = ui.make_persistent_id("macro-action-submenu-owner");
        let popup_id = ui.make_persistent_id((id_source, "timer-submenu-popup"));
        let mouse_popup_id = ui.make_persistent_id((id_source, "mouse-submenu-popup"));
        let image_popup_id = ui.make_persistent_id((id_source, "image-search-submenu-popup"));
        let active_owner = ui
            .ctx()
            .data(|data| data.get_temp::<MacroActionSubmenuKind>(owner_id));
        let top_level_hovered = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(action_hover_id))
            .unwrap_or(false);
        let mut open = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(popup_id))
            .unwrap_or(false);
        if active_owner.is_some_and(|kind| kind != MacroActionSubmenuKind::Timer) {
            open = false;
        }
        if top_level_hovered {
            open = false;
            ui.ctx()
                .data_mut(|data| data.insert_temp(owner_id, None::<MacroActionSubmenuKind>));
            ui.ctx().request_repaint();
        }
        let inner = ui.allocate_ui_with_layout(
            vec2(58.0, 42.0),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                let response = ui.add_sized(
                    [34.0, 24.0],
                    Button::new(Self::material_icon_text(0xe425, 18.0)).selected(selected),
                );
                if response.hovered() || response.clicked() {
                    Self::clear_macro_action_submenus(ui, id_source);
                    open = true;
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(owner_id, MacroActionSubmenuKind::Timer));
                }
                let popup_rect_id = ui.make_persistent_id((id_source, "timer-submenu-rect"));
                let popup_response = egui::Popup::from_response(&response)
                    .id(popup_id)
                    .open_bool(&mut open)
                    .align(egui::RectAlign::BOTTOM_START)
                    .layout(egui::Layout::top_down_justified(egui::Align::Min))
                    .width(220.0)
                    .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
                    .show(|ui| {
                        let rect = ui.max_rect();
                        ui.ctx().data_mut(|data| data.insert_temp(popup_rect_id, rect));
                        egui::Grid::new((id_source, "timer-action-grid"))
                            .num_columns(3)
                            .spacing([6.0, 6.0])
                            .show(ui, |ui| {
                                for (index, action) in
                                    Self::timer_macro_actions().iter().copied().enumerate()
                                {
                                    Self::render_macro_action_option(
                                        ui,
                                        language,
                                        current,
                                        action,
                                        live_sync,
                                        action_hover_id,
                                        true,
                                    );
                                    if (index + 1) % 3 == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
                let popup_rect: Option<egui::Rect> = ui.ctx().data(|data| data.get_temp(popup_rect_id));
                if open {
                    if let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
                        let mut keep_open_rect = response.rect.expand(2.0);
                        if let Some(rect) = popup_rect {
                            keep_open_rect = keep_open_rect.union(rect.expand(2.0));
                            if rect.contains(pointer_pos) {
                                ui.ctx().data_mut(|data| {
                                    data.insert_temp(owner_id, MacroActionSubmenuKind::Timer)
                                });
                            }
                        }
                        if !keep_open_rect.contains(pointer_pos) {
                            open = false;
                            ui.ctx().data_mut(|data| data.insert_temp(owner_id, None::<MacroActionSubmenuKind>));
                            ui.ctx().request_repaint();
                        }
                    } else {
                        open = false;
                        ui.ctx().data_mut(|data| data.insert_temp(owner_id, None::<MacroActionSubmenuKind>));
                        ui.ctx().request_repaint();
                    }
                }
                ui.ctx().data_mut(|data| data.insert_temp(popup_id, open));
                let label_color = if selected {
                    ui.visuals().strong_text_color()
                } else {
                    ui.visuals().text_color()
                };
                ui.label(
                    RichText::new(Self::tr_lang(language, "Timer", "Háº¹n giá»"))
                        .size(9.0)
                        .color(label_color),
                );
                response
            },
        );
        let response = inner.inner;
        if !open {
            Self::show_instant_hover_tooltip(
                ui,
                &response,
                Self::tr_lang(
                    language,
                    "Timer\nOpen start, pause, and stop timer actions.",
                    "HÃ¡ÂºÂ¹n giÃ¡Â»Â\nMá»Ÿ cÃƒÂ¡c action báº¯t Ã„â€˜Ã¡ÂºÂ§u, tÃ¡ÂºÂ¡m dÃ¡Â»Â«ng vÃƒÂ  dÃ¡Â»Â«ng hÃ¡ÂºÂ¹n giÃ¡Â»Â.",
                ),
            );
        }
    }
    fn render_custom_preset_step_draft_popup(
        ui: &mut egui::Ui,
        response: &egui::Response,
        anchor_response: &egui::Response,
        step: &mut MacroStep,
        id_source: impl std::hash::Hash,
        step_index: Option<usize>,
        language: UiLanguage,
        command_presets: &[CommandPreset],
        is_generating: bool,
    ) -> (
        bool,
        Option<(Option<usize>, String, String, bool)>,
        Option<(Option<usize>, String, String, bool, bool)>,
        Option<u32>,
    ) {
        let mut changed = false;
        let mut save_request = None;
        let mut save_and_open_ai_request = None;
        let mut open_ai_preset_id = None;
        if step.action != MacroAction::TriggerCommandPreset {
            return (false, None, None, None);
        }
        let popup_id = ui.make_persistent_id((id_source, "custom-preset-draft-popup"));
        let pointer_pos = ui.input(|input| input.pointer.hover_pos());
        let mut open = response.hovered()
            || ui
                .ctx()
                .data(|data| data.get_temp::<bool>(popup_id))
                .unwrap_or(false);
        let resolved_preset = command_presets
            .iter()
            .find(|preset| {
                let key = step.key.trim();
                if key.is_empty() {
                    return false;
                }
                preset.id.to_string() == key || preset.name.trim().eq_ignore_ascii_case(key)
            })
            .cloned();
        if step.command_preset_command.trim().is_empty() {
            if let Some(preset) = resolved_preset.as_ref() {
                step.command_preset_command = preset.command.clone();
            }
        }
        let is_saved_custom_preset = resolved_preset.as_ref().is_some_and(|preset| {
            preset.command.trim() == step.command_preset_command.trim()
                && !step.command_preset_command.trim().is_empty()
                && preset.use_powershell == step.command_preset_use_powershell
        });
        if open {
            let popup_size = vec2(320.0, 132.0);
            let mut pos = anchor_response.rect.right_top() + vec2(2.0, 0.0);
            let screen_rect = ui.ctx().content_rect();
            if pos.x + popup_size.x > screen_rect.right() {
                pos.x = anchor_response.rect.left() - popup_size.x - 2.0;
            }
            let area = egui::Area::new(popup_id)
                .order(egui::Order::Foreground)
                .fixed_pos(pos)
                .interactable(true);
            let area_response = area.show(ui.ctx(), |ui| {
                let mut frame = egui::Frame::popup(ui.style());
                let (fill, stroke_color) = if step.command_preset_use_powershell {
                    (
                        Color32::from_rgba_premultiplied(20, 35, 55, 245),
                        Color32::from_rgb(90, 190, 255),
                    )
                } else {
                    (
                        Color32::from_rgba_premultiplied(45, 30, 15, 245),
                        Color32::from_rgb(255, 170, 75),
                    )
                };
                frame = frame
                    .fill(fill)
                    .stroke(egui::Stroke::new(1.5, stroke_color));
                frame.show(ui, |ui| {
                    ui.set_min_width(320.0);
                    let mut trigger_ai = false;
                    ui.horizontal(|ui| {
                        ui.label(Self::tr_lang(language, "Custom command", "Custom command"));
                        if is_generating {
                            let (rect, _resp) = ui
                                .allocate_exact_size(egui::vec2(36.0, 20.0), egui::Sense::hover());
                            Self::draw_spinning_wand(ui, rect, Color32::from_rgb(255, 220, 100));
                        } else {
                            let ai_btn = egui::Button::new(Self::ai_badge_text(true))
                                .fill(Self::ai_badge_fill())
                                .stroke(Self::ai_badge_stroke());
                            if ui
                                .add(ai_btn)
                                .on_hover_text(Self::tr_lang(
                                    language,
                                    "Generate or edit command with AI",
                                    "Tạo hoặc sửa câu lệnh bằng AI",
                                ))
                                .clicked()
                            {
                                trigger_ai = true;
                            }
                        }
                        ui.add_space(8.0);
                        changed |= ui
                            .radio_value(&mut step.command_preset_use_powershell, false, "CMD")
                            .changed();
                        ui.add_space(4.0);
                        changed |= ui
                            .radio_value(
                                &mut step.command_preset_use_powershell,
                                true,
                                "PowerShell",
                            )
                            .changed();
                    });
                    if trigger_ai {
                        if let Some(preset) = resolved_preset.as_ref() {
                            open_ai_preset_id = Some(preset.id);
                        } else {
                            let preset_name = if step.key.trim().is_empty() {
                                "Custom Command Step".to_owned()
                            } else {
                                step.key.trim().to_owned()
                            };
                            let command_text =
                                ai::normalize_command_text(&step.command_preset_command);
                            save_and_open_ai_request = Some((
                                step_index,
                                preset_name,
                                command_text,
                                step.command_preset_use_powershell,
                                true, // is_ad_hoc
                            ));
                        }
                    }
                    let is_dark_theme = ui.visuals().dark_mode;
                    let hint_color = if is_dark_theme {
                        Color32::from_rgba_unmultiplied(140, 140, 140, 150)
                    } else {
                        Color32::from_rgba_unmultiplied(100, 100, 100, 150)
                    };
                    let command_changed = Self::render_expandable_command_text_edit(
                        ui,
                        &mut step.command_preset_command,
                        ui.id().with((step_index, "command-preset-cmd")),
                        "shutdown /s /t 0",
                    )
                    .changed();
                    if command_changed {
                        changed = true;
                    }
                    if resolved_preset.is_none() {
                        ui.horizontal(|ui| {
                            ui.label(Self::tr_lang(language, "Preset name:", "Tên preset:"));
                            let name_changed = ui
                                .add(
                                    TextEdit::singleline(&mut step.key)
                                        .hint_text(Self::tr_lang(
                                            language,
                                            "Enter name...",
                                            "Nhập tên...",
                                        ))
                                        .desired_width(180.0),
                                )
                                .changed();
                            if name_changed {
                                changed = true;
                            }
                        });
                        ui.add_space(2.0);
                    }
                    if !is_saved_custom_preset {
                        ui.horizontal(|ui| {
                            let save_enabled = !step.key.trim().is_empty()
                                && !ai::normalize_command_text(&step.command_preset_command)
                                    .trim()
                                    .is_empty();
                            let btn_text = if resolved_preset.is_some() {
                                Self::tr_lang(language, "Update custom preset", "Cập nhật preset")
                            } else {
                                Self::tr_lang(
                                    language,
                                    "Save as custom preset",
                                    "Lưu thành preset mới",
                                )
                            };
                            if ui
                                .add_enabled(save_enabled, egui::Button::new(btn_text))
                                .clicked()
                            {
                                let save_name = resolved_preset
                                    .as_ref()
                                    .map(|p| p.name.clone())
                                    .unwrap_or_else(|| step.key.trim().to_owned());
                                save_request = Some((
                                    step_index,
                                    save_name,
                                    ai::normalize_command_text(&step.command_preset_command),
                                    step.command_preset_use_powershell,
                                ));
                            }
                        });
                    }
                });
            });
            let icon_pos = area_response.response.rect.right_top() + vec2(-24.0, 12.0);
            ui.painter().text(
                icon_pos,
                egui::Align2::CENTER_CENTER,
                char::from_u32(0xeb8e).unwrap_or('?').to_string(),
                egui::FontId::new(15.0, FontFamily::Name(MATERIAL_ICONS_FONT.into())),
                ui.visuals().weak_text_color(),
            );
            let popup_rect = area_response.response.rect.expand(12.0);
            let hover_popup = pointer_pos.is_some_and(|pos| popup_rect.contains(pos));
            open = response.hovered() || hover_popup;
            ui.ctx().data_mut(|data| data.insert_temp(popup_id, open));
            return (
                changed,
                save_request,
                save_and_open_ai_request,
                open_ai_preset_id,
            );
        }
        ui.ctx().data_mut(|data| data.insert_temp(popup_id, open));
        (
            changed,
            save_request,
            save_and_open_ai_request,
            open_ai_preset_id,
        )
    }
    pub(crate) fn render_macro_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        let timer_names: Vec<String> = self.state.timer_presets.iter().map(|t| t.name.clone()).collect();
        let mut suggestion_names = std::collections::HashSet::new();
        for (idx, _name) in timer_names.iter().enumerate() {
            suggestion_names.insert(format!("Timer{}", idx + 1));
        }
        for name in Self::builtin_variable_suggestions() {
            suggestion_names.insert(name.to_string());
        }
        for name in self.collect_all_macro_referenced_variables() {
            if !name.contains('.') {
                suggestion_names.insert(name);
            }
        }
        {
            let vars = crate::overlay::RUNTIME_VARIABLES.lock();
            for name in vars.keys() {
                if !name.contains('.') {
                    suggestion_names.insert(name.clone());
                }
            }
        }
        let mut suggestion_names: Vec<String> = suggestion_names.into_iter().collect();
        suggestion_names.sort();
        let mut writable_suggestion_names = std::collections::HashSet::new();
        {
            let vars = crate::overlay::RUNTIME_VARIABLES.lock();
            for name in vars.keys() {
                if !name.contains('.') {
                    writable_suggestion_names.insert(name.clone());
                }
            }
        }
        for (idx, _name) in timer_names.iter().enumerate() {
            writable_suggestion_names.insert(format!("Timer{}", idx + 1));
        }
        let mut writable_suggestion_names: Vec<String> = writable_suggestion_names.into_iter().collect();
        writable_suggestion_names.sort();
        ui.memory_mut(|mem| {
            mem.data.insert_temp(egui::Id::new("macro_variable_suggestion_names"), suggestion_names);
            mem.data.insert_temp(egui::Id::new("macro_variable_writable_suggestion_names"), writable_suggestion_names);
            mem.data.insert_temp(egui::Id::new("macro_variable_suggestion_committed"), false);
        });
        let any_popup_open = ui.memory(|mem| mem.data.get_temp::<bool>(egui::Id::new("any_popup_open"))).unwrap_or(false);
        let mut enter_pressed = false;
        let mut arrow_up_pressed = false;
        let mut arrow_down_pressed = false;
        if any_popup_open {
            ui.input_mut(|i| {
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                    enter_pressed = true;
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                    arrow_up_pressed = true;
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                    arrow_down_pressed = true;
                }
            });
        }
        ui.memory_mut(|mem| {
            mem.data.insert_temp(egui::Id::new("enter_pressed"), enter_pressed);
            mem.data.insert_temp(egui::Id::new("arrow_up_pressed"), arrow_up_pressed);
            mem.data.insert_temp(egui::Id::new("arrow_down_pressed"), arrow_down_pressed);
            mem.data.insert_temp(egui::Id::new("any_popup_open"), false);
        });
        let active_folder_for_controls = if self.macro_folders_panel_open {
            self.active_macro_folder_view.filter(|folder_id| {
                self.state
                    .macro_folders
                    .iter()
                    .any(|folder| folder.id == *folder_id)
            })
        } else {
            None
        };
        if self.active_macro_folder_view.is_some() && active_folder_for_controls.is_none() {
            self.active_macro_folder_view = None;
        }
        let active_folder_name = if self.macro_folders_panel_open {
            self.active_macro_folder_view.and_then(|folder_id| {
                self.state
                    .macro_folders
                    .iter()
                    .find(|folder| folder.id == folder_id)
                    .map(|folder| folder.name.clone())
            })
        } else {
            None
        };
        let paste_target_folder = if active_folder_name.is_some() {
            self.active_macro_folder_view
        } else {
            None
        };
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let master_label = if self.state.macros_master_enabled {
                    Self::tr_lang(language, "Macro On", "Macro On")
                } else {
                    Self::tr_lang(language, "Macro Off", "Macro Off")
                };
                let master_fill = if self.state.macros_master_enabled {
                    Color32::from_rgb(44, 132, 74)
                } else {
                    Color32::from_rgb(74, 78, 86)
                };
                let master_stroke = if self.state.macros_master_enabled {
                    Color32::from_rgb(124, 240, 164)
                } else {
                    Color32::from_rgb(156, 162, 172)
                };
                if ui
                    .add_sized(
                        [120.0, 28.0],
                        Button::new(RichText::new(master_label).color(Color32::WHITE))
                            .fill(master_fill)
                            .stroke(egui::Stroke::new(1.0, master_stroke)),
                    )
                    .clicked()
                {
                    self.state.macros_master_enabled = !self.state.macros_master_enabled;
                    self.sync_macro_master_enabled();
                    self.persist();
                }
                let macro_hotkey_capture_target = CaptureRequest::MacrosMasterHotkey;
                let macro_hotkey_capture_active =
                    self.capture_target.as_ref() == Some(&macro_hotkey_capture_target);
                let macro_hotkey_preview = if macro_hotkey_capture_active
                    && let Some(pending) = self.capture_hotkey_combo_keys.as_ref()
                {
                    Some(Self::hotkey_binding_from_combo_keys(pending.clone()))
                } else {
                    self.state.macros_master_hotkey.clone()
                };
                let macro_hotkey_capture_button_text = if macro_hotkey_capture_active {
                    Self::capture_button_text(language, true)
                } else {
                    Self::material_icon_text(0xe312, 18.0)
                };
                if ui
                    .add_sized(
                        if macro_hotkey_capture_active {
                            [104.0, 28.0]
                        } else {
                            [28.0, 28.0]
                        },
                        Button::new(macro_hotkey_capture_button_text)
                            .fill(if macro_hotkey_capture_active {
                                Color32::from_rgba_premultiplied(72, 156, 116, 120)
                            } else {
                                ui.visuals().faint_bg_color
                            })
                            .stroke(egui::Stroke::new(
                                1.0,
                                if macro_hotkey_capture_active {
                                    Color32::from_rgb(126, 224, 182)
                                } else {
                                    ui.visuals().widgets.noninteractive.bg_stroke.color
                                },
                            )),
                    )
                    .on_hover_text(Self::tr_lang(
                        language,
                        "Capture macro hotkey",
                        "Bat macro hotkey",
                    ))
                    .clicked()
                {
                    if macro_hotkey_capture_active {
                        self.cancel_capture();
                    } else {
                        self.begin_capture(
                            macro_hotkey_capture_target,
                            Self::tr_lang(
                                language,
                                "Press a hotkey for Macro On / Off.",
                                "Nhan hotkey de bat / tat Macro.",
                            )
                            .to_owned(),
                        );
                    }
                }
                if let Some(binding) = macro_hotkey_preview.as_ref() {
                    let label = hotkey::format_binding(Some(binding));
                    if ui
                        .add(
                            Button::new(RichText::new(label).monospace()).min_size(vec2(0.0, 28.0)),
                        )
                        .on_hover_text(Self::tr_lang(
                            language,
                            "Click to remove this hotkey",
                            "Bấm để xóa hotkey này",
                        ))
                        .clicked()
                    {
                        self.state.macros_master_hotkey = None;
                        self.sync_macro_master_hotkey();
                        self.persist();
                    }
                }
            });
            if ui
                .add_sized(
                    [28.0, 28.0],
                    Button::new(Self::material_icon_text(0xe145, 18.0))
                        .fill(ui.visuals().faint_bg_color)
                        .stroke(egui::Stroke::new(
                            1.0,
                            ui.visuals().widgets.noninteractive.bg_stroke.color,
                        )),
                )
                .on_hover_text(Self::tr_lang(
                    language,
                    "Add macro group",
                    "Them macro group",
                ))
                .clicked()
            {
                if let Some(folder_id) = active_folder_for_controls {
                    self.add_macro_group_to_folder(folder_id);
                } else {
                    self.add_macro_group();
                }
                self.persist();
            }
            let share_icon = 0xe80d; // Material icon for share
            let share_fill = if self.show_share_buttons {
                Color32::from_rgba_premultiplied(0, 191, 255, 30)
            } else {
                ui.visuals().faint_bg_color
            };
            let share_stroke = if self.show_share_buttons {
                Color32::from_rgb(0, 191, 255)
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke.color
            };
            if ui
                .add_sized(
                    [28.0, 28.0],
                    Button::new(Self::material_icon_text(share_icon, 18.0))
                        .fill(share_fill)
                        .stroke(egui::Stroke::new(1.0, share_stroke)),
                )
                .on_hover_text(Self::tr_lang(
                    language,
                    "Toggle Import/Export buttons",
                    "Bật/Tắt hiển thị nút chia sẻ (Import/Export)",
                ))
                .clicked()
            {
                self.show_share_buttons = !self.show_share_buttons;
            }
            let paste_enabled = !self.macro_group_clipboard.is_empty();
            let paste_fill = if paste_enabled {
                Color32::from_rgb(84, 90, 102)
            } else {
                ui.visuals().faint_bg_color
            };
            let paste_stroke = if paste_enabled {
                ui.visuals().widgets.active.bg_stroke.color
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke.color
            };
            if ui
                .add_enabled(
                    paste_enabled,
                    Button::new(Self::material_icon_text(0xe14f, 18.0))
                        .min_size(egui::vec2(28.0, 28.0))
                        .fill(paste_fill)
                        .stroke(egui::Stroke::new(1.0, paste_stroke)),
                )
                .on_hover_text(Self::tr_lang(
                    language,
                    "Paste macro groups",
                    "DÃƒÂ¡n macro group",
                ))
                .clicked()
            {
                self.paste_macro_groups_into_folder(paste_target_folder);
            }
            let copy_enabled = !self.selected_macro_groups.is_empty();
            let copy_fill = if copy_enabled {
                Color32::from_rgb(84, 90, 102)
            } else {
                ui.visuals().faint_bg_color
            };
            let copy_stroke = if copy_enabled {
                ui.visuals().widgets.active.bg_stroke.color
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke.color
            };
            if ui
                .add_enabled(
                    copy_enabled,
                    Button::new(Self::material_icon_text(0xe14d, 18.0))
                        .min_size(egui::vec2(28.0, 28.0))
                        .fill(copy_fill)
                        .stroke(egui::Stroke::new(1.0, copy_stroke)),
                )
                .on_hover_text(Self::tr_lang(
                    language,
                    "Copy selected macro groups",
                    "Sao chÃƒÂ©p macro group Ãƒâ€žÃ¢â‚¬ËœÃƒÂ£ chÃ¡Â»Ân",
                ))
                .clicked()
            {
                self.copy_selected_macro_groups();
            }
            let cut_enabled = !self.selected_macro_groups.is_empty();
            let cut_fill = if cut_enabled {
                Color32::from_rgb(84, 90, 102)
            } else {
                ui.visuals().faint_bg_color
            };
            let cut_stroke = if cut_enabled {
                ui.visuals().widgets.active.bg_stroke.color
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke.color
            };
            if ui
                .add_enabled(
                    cut_enabled,
                    Button::new(Self::material_icon_text(0xe14e, 18.0))
                        .min_size(egui::vec2(28.0, 28.0))
                        .fill(cut_fill)
                        .stroke(egui::Stroke::new(1.0, cut_stroke)),
                )
                .on_hover_text(Self::tr_lang(
                    language,
                    "Cut selected macro groups",
                    "Cat macro group Ãƒâ€žÃ¢â‚¬ËœÃƒÂ£ chÃ¡Â»Ân",
                ))
                .clicked()
            {
                self.cut_selected_macro_groups();
            }
            let trash_enabled = !self.selected_macro_groups.is_empty();
            let trash_fill = if trash_enabled {
                Color32::from_rgb(84, 90, 102)
            } else {
                ui.visuals().faint_bg_color
            };
            let trash_stroke = if trash_enabled {
                ui.visuals().widgets.active.bg_stroke.color
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke.color
            };
            if ui
                .add_enabled(
                    trash_enabled,
                    Button::new(Self::material_icon_text(0xe872, 18.0))
                        .min_size(egui::vec2(28.0, 28.0))
                        .fill(trash_fill)
                        .stroke(egui::Stroke::new(1.0, trash_stroke)),
                )
                .on_hover_text(Self::tr_lang(
                    language,
                    "Delete selected macro groups",
                    "Xoa cac macro group da chon",
                ))
                .clicked()
            {
                self.remove_selected_macro_groups();
            }
            let star_filter_active = matches!(
                self.macro_groups_favorite_filter,
                MacroGroupFavoriteFilter::Star
            );
            if ui
                .add_sized(
                    [28.0, 28.0],
                    Button::new(Self::material_icon_text(0xe838, 18.0))
                        .fill(if star_filter_active {
                            Color32::from_rgb(124, 96, 28)
                        } else {
                            ui.visuals().faint_bg_color
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if star_filter_active {
                                Color32::from_rgb(255, 220, 96)
                            } else {
                                ui.visuals().widgets.noninteractive.bg_stroke.color
                            },
                        )),
                )
                .on_hover_text(Self::tr_lang(
                    language,
                    "Show star macros only",
                    "Chi nhÃƒÂ³m Ãƒâ€žÃ¢â‚¬ËœÃƒÂ£ favorite",
                ))
                .clicked()
            {
                self.macro_groups_favorite_filter = if star_filter_active {
                    MacroGroupFavoriteFilter::All
                } else {
                    MacroGroupFavoriteFilter::Star
                };
            }
            ui.group(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                // Render Global Constants on toolbar
                if !self.state.global_constants.is_empty() {
                    let max_show = 3;
                    for (i, (name, val)) in self.state.global_constants.iter().enumerate() {
                        if i >= max_show {
                            break;
                        }
                        let text = format!("{}={}", name, val);
                        let is_dark = self.state.ui_theme == UiThemeMode::Dark;
                        let bg_color = if is_dark {
                            Color32::from_rgba_premultiplied(0, 150, 200, 30)
                        } else {
                            Color32::from_rgba_premultiplied(0, 120, 180, 20)
                        };
                        let border_color = if is_dark {
                            Color32::from_rgba_premultiplied(0, 200, 255, 120)
                        } else {
                            Color32::from_rgba_premultiplied(0, 100, 150, 120)
                        };
                        let text_color = if is_dark {
                            Color32::from_rgb(140, 230, 255)
                        } else {
                            Color32::from_rgb(0, 80, 120)
                        };
                        egui::Frame::canvas(ui.style())
                            .fill(bg_color)
                            .stroke(egui::Stroke::new(1.0, border_color))
                            .rounding(4.0)
                            .inner_margin(egui::Margin::symmetric(5, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(text).monospace().size(11.0).color(text_color),
                                );
                            })
                            .response
                            .on_hover_text(Self::tr_lang(
                                language,
                                "Global Constant (Fixed Value)",
                                "Hằng số toàn cục (Giá trị cố định)",
                            ));
                    }
                    if self.state.global_constants.len() > max_show {
                        let remaining = self.state.global_constants.len() - max_show;
                        let mut tooltip_text = String::new();
                        for (i, (name, val)) in self.state.global_constants.iter().enumerate() {
                            if i >= max_show {
                                if !tooltip_text.is_empty() {
                                    tooltip_text.push('\n');
                                }
                                tooltip_text.push_str(&format!("{} = {}", name, val));
                            }
                        }
                        ui.label(
                            RichText::new(format!("+{} more", remaining))
                                .size(11.0)
                                .color(ui.visuals().weak_text_color())
                                .italics(),
                        )
                        .on_hover_text(tooltip_text);
                    }
                }
                ui.vertical(|ui| {
                    let edit_icon = 0xe3c9; // edit icon (bút chì chéo)
                    if ui
                        .add_sized(
                            [28.0, 28.0],
                            Button::new(Self::material_icon_text(edit_icon, 18.0)) // variable edit icon
                                .fill(if self.variable_inspector_open {
                                    Color32::from_rgba_premultiplied(72, 156, 116, 120)
                                } else {
                                    ui.visuals().faint_bg_color
                                })
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    if self.variable_inspector_open {
                                        Color32::from_rgb(126, 224, 182)
                                    } else {
                                        ui.visuals().widgets.noninteractive.bg_stroke.color
                                    },
                                )),
                        )
                        .on_hover_text(Self::tr_lang(
                            language,
                            "Global & Local Variables Manager (Real-time)",
                            "Trình quản lý biến toàn cục & cục bộ (Real-time)",
                        ))
                        .clicked()
                    {
                        self.variable_inspector_open = !self.variable_inspector_open;
                    }
                    if self.variable_inspector_open {
                        ui.add_space(4.0);
                        Self::render_expression_help_box(ui, language);
                    }
                });
            });
            if ui
                .add_sized(
                    [28.0, 28.0],
                    Button::new(Self::folder_icon_text(self.macro_folders_panel_open, 18.0))
                        .fill(if self.macro_folders_panel_open {
                            Color32::from_rgba_premultiplied(72, 156, 116, 120)
                        } else {
                            ui.visuals().faint_bg_color
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if self.macro_folders_panel_open {
                                Color32::from_rgb(126, 224, 182)
                            } else {
                                ui.visuals().widgets.noninteractive.bg_stroke.color
                            },
                        )),
                )
                .on_hover_text(Self::tr_lang(
                    language,
                    "Show / hide macro folders",
                    "Hien / an macro folder",
                ))
                .clicked()
            {
                self.macro_folders_panel_open = !self.macro_folders_panel_open;
                if !self.macro_folders_panel_open {
                    self.set_active_macro_folder_view(None);
                }
            }
            if self.macro_folders_panel_open {
                if let Some(folder_id) = self.active_macro_folder_view {
                    let folder_name = self
                        .state
                        .macro_folders
                        .iter()
                        .find(|f| f.id == folder_id)
                        .map(|f| f.name.clone())
                        .unwrap_or_default();
                    if ui
                        .add_sized(
                            [28.0, 28.0],
                            Button::new(Self::material_icon_text(0xe5c4, 18.0)) // arrow_back icon
                                .fill(ui.visuals().faint_bg_color)
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                                )),
                        )
                        .on_hover_text(Self::tr_lang(
                            language,
                            "Back to folder list",
                            "Quay lại danh sách thư mục",
                        ))
                        .clicked()
                    {
                        self.set_active_macro_folder_view(None);
                    }
                    ui.label(
                        RichText::new(folder_name)
                            .strong()
                            .color(ui.visuals().strong_text_color()),
                    );
                    ui.add_space(8.0);
                } else {
                    if ui
                        .add_sized(
                            [28.0, 28.0],
                            Button::new(Self::material_icon_text(0xe2cc, 18.0))
                                .fill(ui.visuals().faint_bg_color)
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                                )),
                        )
                        .on_hover_text(Self::tr_lang(language, "Add folder", "Them thu muc"))
                        .clicked()
                    {
                        self.add_macro_folder();
                        self.persist();
                        self.macro_folders_panel_open = true;
                        self.active_macro_folder_view = None;
                    }
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let response = ui.add_sized(
                    [260.0, 24.0],
                    TextEdit::singleline(&mut self.macro_preset_search_query).hint_text(
                        RichText::new(Self::tr_lang(
                            language,
                            "Search macro groups and presets",
                            "Tìm group macro và preset",
                        ))
                        .weak(),
                    ),
                );
                ui.label(Self::material_icon_text(0xe8b6, 18.0));
                Self::apply_vietnamese_input_if_changed(
                    &response,
                    self.state.vietnamese_input_enabled,
                    self.state.vietnamese_input_mode,
                    &mut self.macro_preset_search_query,
                );
            });
        });
        ui.add_space(8.0);
        if Self::is_copy_feedback_active(self.macro_group_export_feedback_until)
            || Self::is_copy_feedback_active(self.macro_preset_export_feedback_until)
            || Self::is_copy_feedback_active(self.macro_step_export_feedback_until)
        {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
        }
        let macro_panel_scroll_height = ui.available_height();
        let pending_macro_group_scroll_target = self.pending_macro_group_scroll_target.take();
        let mut pending_macro_group_scroll_consumed = false;
        let mut hover_preview: Option<MacroStepHoverPreview> = None;
        let mut hover_preview_request: Option<(u32, HoverPreviewRequest)> = None;
        let hover_preview_state_id = ui.make_persistent_id("macro-hover-preview-state");
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(macro_panel_scroll_height)
            .show(ui, |ui| {
        let mut release_folder_id = None;
        let mut delete_folder_id = None;
        let mut enter_folder_id = None;
        let mut begin_mouse_move_absolute_capture_target = None;
        let mut cancel_mouse_move_absolute_capture = false;
        let capture_target_snapshot = self.capture_target.clone();
        let capture_hotkey_combo_keys_snapshot = self.capture_hotkey_combo_keys.clone();
        let active_folder_name = if self.macro_folders_panel_open {
            self.active_macro_folder_view.and_then(|folder_id| {
                self.state
                    .macro_folders
                    .iter()
                    .find(|folder| folder.id == folder_id)
                    .map(|folder| folder.name.clone())
            })
        } else {
            None
        };
        if !self.macro_folders_panel_open {
            self.active_macro_folder_view = None;
        } else if self.active_macro_folder_view.is_some() && active_folder_name.is_none() {
            self.active_macro_folder_view = None;
        }
        if false {
            ui.horizontal_wrapped(|ui| {
            let master_label = if self.state.macros_master_enabled {
                Self::tr_lang(language, "Macro On", "Macro On")
            } else {
                Self::tr_lang(language, "Macro Off", "Macro Off")
            };
            let master_fill = if self.state.macros_master_enabled {
                Color32::from_rgb(44, 132, 74)
            } else {
                Color32::from_rgb(74, 78, 86)
            };
            let master_stroke = if self.state.macros_master_enabled {
                Color32::from_rgb(124, 240, 164)
            } else {
                Color32::from_rgb(156, 162, 172)
            };
            if ui
                .add_sized(
                    [120.0, 28.0],
                    Button::new(RichText::new(master_label).color(Color32::WHITE))
                        .fill(master_fill)
                        .stroke(egui::Stroke::new(1.0, master_stroke)),
                )
                .clicked()
            {
                self.state.macros_master_enabled = !self.state.macros_master_enabled;
                self.sync_macro_master_enabled();
                self.persist();
            }
            let capture_target = CaptureRequest::MacrosMasterHotkey;
            let capture_active = self.capture_target.as_ref() == Some(&capture_target);
            let hotkey_preview =
                if capture_active && let Some(pending) = self.capture_hotkey_combo_keys.as_ref() {
                    Some(Self::hotkey_binding_from_combo_keys(pending.clone()))
                } else {
                    self.state.macros_master_hotkey.clone()
                };
            let macro_hotkey_capture_button_text = if capture_active {
                Self::capture_button_text(language, true)
            } else {
                Self::material_icon_text(0xe312, 18.0)
            };
            if ui
                .add_sized(
                    if capture_active {
                        [104.0, 28.0]
                    } else {
                        [28.0, 28.0]
                    },
                    Button::new(macro_hotkey_capture_button_text)
                        .fill(if capture_active {
                            Color32::from_rgba_premultiplied(72, 156, 116, 120)
                        } else {
                            ui.visuals().faint_bg_color
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if capture_active {
                                Color32::from_rgb(126, 224, 182)
                            } else {
                                ui.visuals().widgets.noninteractive.bg_stroke.color
                            },
                        )),
                )
                .on_hover_text(Self::tr_lang(
                    language,
                    "Capture macro hotkey",
                    "Bat macro hotkey",
                ))
                .clicked()
            {
                if capture_active {
                    self.cancel_capture();
                } else {
                    self.begin_capture(
                        capture_target,
                        Self::tr_lang(
                            language,
                            "Press a hotkey for Macro On / Off.",
                            "Nhan hotkey de bat / tat Macro.",
                        )
                        .to_owned(),
                    );
                }
            }
            let star_filter_active = matches!(
                self.macro_groups_favorite_filter,
                MacroGroupFavoriteFilter::Star
            );
            if ui
                .add_sized(
                    [28.0, 28.0],
                    Button::new(Self::material_icon_text(0xe838, 18.0))
                        .fill(if star_filter_active {
                            Color32::from_rgb(124, 96, 28)
                        } else {
                            ui.visuals().faint_bg_color
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if star_filter_active {
                                Color32::from_rgb(255, 220, 96)
                            } else {
                                ui.visuals().widgets.noninteractive.bg_stroke.color
                            },
                        )),
                )
                .on_hover_text(Self::tr_lang(
                    language,
                    "Show star macros only",
                    "ChÃ¡Â»â€° hiÃ¡Â»â€¡n nhÃ³m Ã„â€˜ÃƒÂ£ favorite",
                ))
                .clicked()
            {
                self.macro_groups_favorite_filter = if star_filter_active {
                    MacroGroupFavoriteFilter::All
                } else {
                    MacroGroupFavoriteFilter::Star
                };
            }
            if ui
                .add_sized(
                    [28.0, 28.0],
                    Button::new(Self::folder_icon_text(self.macro_folders_panel_open, 18.0))
                        .fill(if self.macro_folders_panel_open {
                            Color32::from_rgba_premultiplied(72, 156, 116, 120)
                        } else {
                            ui.visuals().faint_bg_color
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if self.macro_folders_panel_open {
                                Color32::from_rgb(126, 224, 182)
                            } else {
                                ui.visuals().widgets.noninteractive.bg_stroke.color
                            },
                        )),
                )
                .on_hover_text(Self::tr_lang(
                    language,
                    "Show / hide macro folders",
                    "Hien / an macro folder",
                ))
                .clicked()
            {
                self.macro_folders_panel_open = !self.macro_folders_panel_open;
                if !self.macro_folders_panel_open {
                    self.set_active_macro_folder_view(None);
                }
            }
            // Render Global Constants on toolbar
            if !self.state.global_constants.is_empty() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let max_show = 3;
                    for (i, (name, val)) in self.state.global_constants.iter().enumerate() {
                        if i >= max_show {
                            break;
                        }
                        let text = format!("{}={}", name, val);
                        let is_dark = self.state.ui_theme == UiThemeMode::Dark;
                        let bg_color = if is_dark {
                            Color32::from_rgba_premultiplied(0, 150, 200, 30)
                        } else {
                            Color32::from_rgba_premultiplied(0, 120, 180, 20)
                        };
                        let border_color = if is_dark {
                            Color32::from_rgba_premultiplied(0, 200, 255, 120)
                        } else {
                            Color32::from_rgba_premultiplied(0, 100, 150, 120)
                        };
                        let text_color = if is_dark {
                            Color32::from_rgb(140, 230, 255)
                        } else {
                            Color32::from_rgb(0, 80, 120)
                        };
                        egui::Frame::canvas(ui.style())
                            .fill(bg_color)
                            .stroke(egui::Stroke::new(1.0, border_color))
                            .rounding(4.0)
                            .inner_margin(egui::Margin::symmetric(5, 2))
                            .show(ui, |ui| {
                                ui.label(RichText::new(text).monospace().size(11.0).color(text_color));
                            }).response.on_hover_text(Self::tr_lang(
                                language,
                                "Global Constant (Fixed Value)",
                                "Hằng số toàn cục (Giá trị cố định)",
                            ));
                    }
                    if self.state.global_constants.len() > max_show {
                        let remaining = self.state.global_constants.len() - max_show;
                        let mut tooltip_text = String::new();
                        for (i, (name, val)) in self.state.global_constants.iter().enumerate() {
                            if i >= max_show {
                                if !tooltip_text.is_empty() {
                                    tooltip_text.push('\n');
                                }
                                tooltip_text.push_str(&format!("{} = {}", name, val));
                            }
                        }
                        ui.label(
                            RichText::new(format!("+{} more", remaining))
                                .size(11.0)
                                .color(ui.visuals().weak_text_color())
                                .italics()
                        ).on_hover_text(tooltip_text);
                    }
                });
            }
            let variable_inspector_active = self.variable_inspector_open;
            if ui
                .add_sized(
                    [28.0, 28.0],
                    Button::new(Self::material_icon_text(0xe150, 18.0)) // variable add icon
                        .fill(if variable_inspector_active {
                            Color32::from_rgba_premultiplied(72, 156, 116, 120)
                        } else {
                            ui.visuals().faint_bg_color
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if variable_inspector_active {
                                Color32::from_rgb(126, 224, 182)
                            } else {
                                ui.visuals().widgets.noninteractive.bg_stroke.color
                            },
                        )),
                )
                .on_hover_text(Self::tr_lang(
                    language,
                    "Global & Local Variables Manager (Real-time)",
                    "Trình quản lý biến toàn cục & cục bộ (Real-time)",
                ))
                .clicked()
            {
                self.variable_inspector_open = !self.variable_inspector_open;
            }
            let trash_enabled = !self.selected_macro_groups.is_empty();
            if ui
                .add_enabled(
                    trash_enabled,
                    Button::new(Self::material_icon_text(0xe872, 18.0))
                        .min_size(egui::vec2(28.0, 28.0))
                        .fill(ui.visuals().faint_bg_color)
                        .stroke(egui::Stroke::new(
                            1.0,
                            ui.visuals().widgets.noninteractive.bg_stroke.color,
                        )),
                )
                .on_hover_text(Self::tr_lang(
                    language,
                    "Delete selected macro groups",
                    "XÃƒÂ³a cÃƒÂ¡c macro group Ã„â€˜ÃƒÂ£ chÃ¡Â»Ân",
                ))
                .clicked()
            {
                self.remove_selected_macro_groups();
            }
            if let Some(binding) = hotkey_preview.as_ref() {
                let label = hotkey::format_binding(Some(binding));
                if ui
                    .add(
                        Button::new(RichText::new(label).monospace())
                            .min_size(vec2(0.0, 28.0))
                            .fill(if capture_active {
                                Color32::from_rgba_premultiplied(72, 156, 116, 120)
                            } else {
                                ui.visuals().faint_bg_color
                            })
                            .stroke(egui::Stroke::new(
                                1.0,
                                if capture_active {
                                    Color32::from_rgb(126, 224, 182)
                                } else {
                                    ui.visuals().widgets.noninteractive.bg_stroke.color
                                },
                            )),
                    )
                    .on_hover_text(Self::tr_lang(
                        language,
                        "Click to remove the macro hotkey",
                        "Bam vao de xoa macro hotkey",
                    ))
                    .clicked()
                    && !capture_active
                {
                    self.state.macros_master_hotkey = None;
                    self.sync_macro_master_hotkey();
                    self.persist();
                }
            }
        });
        }
        ui.add_space(6.0);
        enum MacroDestructiveConfirmAction {
            DeleteFolder(u32),
            ReleaseFolder(u32),
            DeleteGroup(u32),
        }
        let mut destructive_confirm = None;
        if let Some(folder_id) = self.confirm_delete_folder_id {
            let group_count = self
                .state
                .macro_groups
                .iter()
                .filter(|group| group.folder_id == Some(folder_id))
                .count();
            let folder_name = self
                .state
                .macro_folders
                .iter()
                .find(|folder| folder.id == folder_id)
                .map(|folder| folder.name.clone())
                .unwrap_or_else(|| format!("Folder {folder_id}"));
            destructive_confirm = Some((
                MacroDestructiveConfirmAction::DeleteFolder(folder_id),
                Self::tr_lang(language, "Delete folder", "Delete folder"),
                format!(
                    "{} {folder_name} {} {group_count} {}?",
                    Self::tr_lang(language, "Delete", "Delete"),
                    Self::tr_lang(language, "and all", "vÃƒÂ  toÃƒÂ n bÃ¡Â»â„¢"),
                    Self::tr_lang(
                        language,
                        "macro group(s) inside it",
                        "macro group bÃªn trong",
                    ),
                ),
                Self::tr_lang(language, "Yes, Delete All", "Yes, Delete All"),
                Self::tr_lang(language, "Cancel", "Cancel"),
            ));
        } else if let Some(folder_id) = self.confirm_release_folder_id {
            let group_count = self
                .state
                .macro_groups
                .iter()
                .filter(|group| group.folder_id == Some(folder_id))
                .count();
            let folder_name = self
                .state
                .macro_folders
                .iter()
                .find(|folder| folder.id == folder_id)
                .map(|folder| folder.name.clone())
                .unwrap_or_else(|| format!("Folder {folder_id}"));
            destructive_confirm = Some((
                MacroDestructiveConfirmAction::ReleaseFolder(folder_id),
                Self::tr_lang(language, "Release folder", "Release folder"),
                format!(
                    "{} {folder_name} {} {group_count} {}?",
                    Self::tr_lang(language, "Release", "Release"),
                    Self::tr_lang(language, "and move", "vÃƒÂ  chuyÃ¡Â»Æ’n"),
                    Self::tr_lang(
                        language,
                        "macro group(s) out of it",
                        "macro group ra khá»i nÃ³",
                    ),
                ),
                Self::tr_lang(language, "Yes, Release", "Yes, Release"),
                Self::tr_lang(language, "Cancel", "Cancel"),
            ));
        } else if let Some(group_id) = self.confirm_delete_macro_group_id {
            if let Some(group_name) = self
                .state
                .macro_groups
                .iter()
                .find(|group| group.id == group_id)
                .map(|group| group.name.clone())
            {
                destructive_confirm = Some((
                    MacroDestructiveConfirmAction::DeleteGroup(group_id),
                    Self::tr_lang(language, "Delete macro group", "Delete macro group"),
                    format!("{group_name}?"),
                    Self::tr_lang(language, "Yes, Delete", "Yes, Delete"),
                    Self::tr_lang(language, "Cancel", "Cancel"),
                ));
            } else {
                self.confirm_delete_macro_group_id = None;
            }
        }
        if let Some((action, title, message, confirm_label, cancel_label)) = destructive_confirm {
            if let Some(result) = self.render_blocking_confirmation_modal(
                ui.ctx(),
                "macro-destructive-confirm",
                &title,
                &message,
                &confirm_label,
                &cancel_label,
            ) {
                match (action, result) {
                    (MacroDestructiveConfirmAction::DeleteFolder(folder_id), true) => {
                        self.state
                            .macro_groups
                            .retain(|group| group.folder_id != Some(folder_id));
                        self.state
                            .macro_folders
                            .retain(|folder| folder.id != folder_id);
                        self.confirm_delete_folder_id = None;
                        self.confirm_release_folder_id = None;
                        if self.active_macro_folder_view == Some(folder_id) {
                            self.set_active_macro_folder_view(None);
                        }
                        self.persist_macro_presets();
                    }
                    (MacroDestructiveConfirmAction::DeleteFolder(_), false) => {
                        self.confirm_delete_folder_id = None;
                    }
                    (MacroDestructiveConfirmAction::ReleaseFolder(folder_id), true) => {
                        self.state
                            .macro_folders
                            .retain(|folder| folder.id != folder_id);
                        for group in &mut self.state.macro_groups {
                            if group.folder_id == Some(folder_id) {
                                group.folder_id = None;
                            }
                        }
                        self.confirm_release_folder_id = None;
                        if self.active_macro_folder_view == Some(folder_id) {
                            self.set_active_macro_folder_view(None);
                        }
                        self.persist_macro_presets();
                    }
                    (MacroDestructiveConfirmAction::ReleaseFolder(_), false) => {
                        self.confirm_release_folder_id = None;
                    }
                    (MacroDestructiveConfirmAction::DeleteGroup(group_id), true) => {
                        self.state.macro_groups.retain(|group| group.id != group_id);
                        self.selected_macro_groups.remove(&group_id);
                        self.macro_group_clipboard
                            .retain(|clipboard_group_id| *clipboard_group_id != group_id);
                        self.confirm_delete_macro_group_id = None;
                        self.persist_macro_presets();
                    }
                    (MacroDestructiveConfirmAction::DeleteGroup(_), false) => {
                        self.confirm_delete_macro_group_id = None;
                    }
                }
            }
        }
        let mut remove_group = None;
        let mut live_sync = false;
        let mut add_preset_to_group = None;
        let mut paste_preset_to_group: Option<u32> = None;
        ui.separator();
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum RenderItem {
            FolderHeader(u32),
            MacroGroup(usize),
        }
        let search_query = self.macro_preset_search_query.trim().to_owned();
        Self::sort_macro_groups(&mut self.state.macro_groups);
        let mut render_items = Vec::new();
        if self.macro_folders_panel_open {
            if self.state.macro_folders.is_empty() {
                ui.label(Self::tr_lang(
                    language,
                    "No folders yet. Macro groups can stay outside folders if you want.",
                    "Chưa có thư mục nào. Nếu muốn, macro group có thể nằm ngoài thư mục.",
                ));
            }
            if let Some(active_folder_id) = self.active_macro_folder_view {
                for (index, group) in self.state.macro_groups.iter().enumerate() {
                    if group.folder_id == Some(active_folder_id) {
                        if Self::macro_group_matches_search_query(group, &search_query) {
                            if match self.macro_groups_favorite_filter {
                                MacroGroupFavoriteFilter::All => true,
                                MacroGroupFavoriteFilter::Star => group.favorite,
                            } {
                                render_items.push(RenderItem::MacroGroup(index));
                            }
                        }
                    }
                }
            } else {
                for folder in &self.state.macro_folders {
                    render_items.push(RenderItem::FolderHeader(folder.id));
                }
            }
        } else {
            for (index, group) in self.state.macro_groups.iter().enumerate() {
                if group.folder_id.is_none() {
                    if Self::macro_group_matches_search_query(group, &search_query) {
                        if match self.macro_groups_favorite_filter {
                            MacroGroupFavoriteFilter::All => true,
                            MacroGroupFavoriteFilter::Star => group.favorite,
                        } {
                            render_items.push(RenderItem::MacroGroup(index));
                        }
                    }
                }
            }
        }
        if !self.macro_folders_panel_open && render_items.is_empty() {
            ui.label(Self::tr_lang(
                language,
                "No macro groups outside folders.",
                "KhÃƒÂ´ng cÃƒÂ³ macro group nÃƒÂ o ngoÃ i thÆ° má»¥c.",
            ));
        }
        let mut toggle_collapsed_folder_id: Option<u32> = None;
        let mut add_group_to_folder_id: Option<u32> = None;
        let mut renamed_folder: Option<(u32, String)> = None;
        let mut toggle_folder_enabled_id: Option<u32> = None;
        let mut pending_custom_preset_save: Option<(
            u32,
            u32,
            Option<usize>,
            String,
            String,
            bool,
        )> = None;
        let mut pending_custom_preset_save_and_open_ai: Option<(
            u32,
            u32,
            Option<usize>,
            String,
            String,
            bool,
            bool, // is_ad_hoc
        )> = None;
        let mut pending_open_ai_preset_id: Option<u32> = None;
        let command_presets_snapshot = self.state.command_presets.clone();
        for item in render_items {
            match item {
                RenderItem::FolderHeader(folder_id) => {
                    let folder = self.state.macro_folders.iter().find(|f| f.id == folder_id).unwrap();
                    let folder_group_count = self
                        .state
                        .macro_groups
                        .iter()
                        .filter(|group| group.folder_id == Some(folder.id))
                        .count();
                    let folder_has_enabled_content = self.state.macro_groups.iter().any(|group| {
                        group.folder_id == Some(folder.id)
                            && group.enabled
                            && group.presets.iter().any(|preset| preset.enabled)
                    });
                    let mut folder_name = folder.name.clone();
                    let rect_key = ui.make_persistent_id((folder_id, "folder-rect"));
                    let last_rect: Option<egui::Rect> = ui.ctx().data(|data| data.get_temp(rect_key));
                    let hovered = last_rect.map_or(false, |rect| ui.rect_contains_pointer(rect));
                    let mut delete_clicked = false;
                    let (inner_res, frame_response) = Self::show_folder_card(ui, folder_has_enabled_content, hovered, |ui| {
                        ui.horizontal(|ui| {
                            let icon_btn = ui.add_sized(
                                [28.0, 24.0],
                                Button::new(Self::folder_icon_text(false, 18.0)),
                            );
                            let name_response =
                                ui.add_sized([220.0, 24.0], TextEdit::singleline(&mut folder_name));
                            Self::apply_vietnamese_input_if_changed(
                                &name_response,
                                self.state.vietnamese_input_enabled,
                                self.state.vietnamese_input_mode,
                                &mut folder_name,
                            );
                            if name_response.changed() {
                                renamed_folder = Some((folder_id, folder_name.clone()));
                            }
                            ui.add_sized(
                                [96.0, 24.0],
                                egui::Label::new(match language {
                                    UiLanguage::Vietnamese => format!("{folder_group_count} nhóm"),
                                    _ => format!("{folder_group_count} group(s)"),
                                }),
                            );
                            let delete_response = ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let btn = Self::sound_style_remove_button(ui);
                                if btn.clicked() {
                                    delete_clicked = true;
                                }
                                btn
                            }).inner;
                            (icon_btn, name_response, delete_response)
                        })
                    });
                    let (icon_btn, name_response, delete_response) = inner_res.inner;
                    ui.ctx().data_mut(|data| data.insert_temp(rect_key, frame_response.rect));
                    let card_hovered = frame_response.hovered();
                    let pointer_in_widgets = ui.rect_contains_pointer(name_response.rect) || ui.rect_contains_pointer(delete_response.rect);
                    let card_clicked = card_hovered && ui.input(|i| i.pointer.any_click());
                    if card_hovered && !pointer_in_widgets {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if ((card_clicked && !pointer_in_widgets && !delete_clicked && !name_response.has_focus()) || icon_btn.clicked()) && !self.confirm_delete_folder_id.is_some() {
                        enter_folder_id = Some(folder_id);
                    }
                    if delete_clicked {
                        if folder_group_count > 0 {
                            self.confirm_delete_folder_id = Some(folder_id);
                        } else {
                            delete_folder_id = Some(folder_id);
                        }
                    }
                    ui.add_space(4.0);
                }
                RenderItem::MacroGroup(group_index) => {
                    let mut next_capture_target = None;
                    let mut cancel_active_capture = false;
                    let mut remove_step = None;
                    let mut insert_step_after = None;
                    let mut move_step_to: Option<(u32, Vec<usize>, usize)> = None;
                    let mut remove_preset = None;
                    let mut pending_step_selection = None;
                    let mut selection_after_paste = None;
                    let mut clear_step_selection = None;
                    let mut copy_selected_steps = None;
                    let mut delete_selected_steps = None;
                    let mut paste_step_after = None;
                    let mut copy_single_step = None;
                    let mut export_step: Option<(u32, usize)> = None;
                    let mut import_step_to: Option<(u32, u32, Option<usize>)> = None; // (group_id, preset_id, Option<step_index>)
                    let mut export_preset: Option<u32> = None;
                    let mut import_preset_to_group: Option<(u32, Option<u32>)> = None; // (group_id, Option<insert_after_preset_id>)
                    let mut export_group: Option<u32> = None;
                    let mut import_group_after: Option<u32> = None; // insert_after_group_id
                    let selected_steps_snapshot = self.selected_macro_steps.clone();
                let render_preset_indices = {
                    let group = &self.state.macro_groups[group_index];
                    let query = search_query.as_str();
                    if query.is_empty() || Self::contains_case_insensitive(&group.name, query) {
                        (0..group.presets.len()).collect::<Vec<_>>()
                    } else {
                        group
                            .presets
                            .iter()
                            .enumerate()
                            .filter(|(_, preset)| {
                                Self::macro_preset_matches_search_query(group, preset, query)
                            })
                            .map(|(index, _)| index)
                            .collect::<Vec<_>>()
                    }
                };
                if render_preset_indices.is_empty() {
                    continue;
                }
                {
                    let all_presets: Vec<(u32, String)> = self.state.macro_groups.iter().flat_map(|g| &g.presets).map(|p| (p.id, Self::format_macro_trigger_ui(language, p))).collect();
                    let all_preset_step_counts: Vec<(u32, i32)> = self
                        .state
                        .macro_groups
                        .iter()
                        .flat_map(|g| {
                            g.presets.iter().map(|p| {
                                (
                                    p.id,
                                    p.steps
                                        .iter()
                                        .filter(|step| !matches!(step.action, MacroAction::LoopStart | MacroAction::LoopEnd))
                                        .count() as i32,
                                )
                            })
                        })
                        .collect();
                    let group = &mut self.state.macro_groups[group_index];
                    let should_scroll_to_group = pending_macro_group_scroll_target == Some(group.id);
                    let folder_enabled = true;
                    Self::show_preset_card(ui, group.enabled && folder_enabled, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                            let star_icon = if group.favorite { 0xe838 } else { 0xe83a };
                            let star_fill = if group.favorite {
                                Color32::from_rgb(104, 82, 18)
                            } else {
                                Color32::from_rgba_premultiplied(52, 58, 70, 190)
                            };
                            let star_stroke = if group.favorite {
                                Color32::from_rgb(255, 220, 96)
                            } else {
                                Color32::from_rgb(102, 110, 122)
                            };
                            if ui
                                .add_sized(
                                    [28.0, 24.0],
                                    Button::new(Self::material_icon_text(star_icon, 15.0).color(
                                        if group.favorite {
                                            Color32::from_rgb(255, 224, 110)
                                        } else {
                                            Color32::from_rgb(208, 214, 224)
                                        },
                                    ))
                                    .fill(star_fill)
                                    .stroke(egui::Stroke::new(1.0, star_stroke)),
                                )
                                .on_hover_text(Self::tr_lang(language, "Star group", "Nhom sao"))
                                .clicked()
                            {
                                group.favorite = !group.favorite;
                                live_sync = true;
                            }
                            let mut selected = self.selected_macro_groups.contains(&group.id);
                            if ui.checkbox(&mut selected, "").changed() {
                                if selected {
                                    self.selected_macro_groups.insert(group.id);
                                } else {
                                    self.selected_macro_groups.remove(&group.id);
                                }
                            }
                            let has_group_inf_loop = self.state.macro_infinite_loop_warning_enabled
                                && group.enabled
                                && group.presets.iter().any(|preset| {
                                    preset.enabled
                                        && (
                                            (preset.trigger_mode == MacroTriggerMode::Press && !preset.stop_on_retrigger_immediate)
                                            || preset.trigger_mode == MacroTriggerMode::Release
                                        )
                                        && preset.steps.iter().any(|s| s.action == MacroAction::LoopStart && s.is_infinite_loop())
                                });
                            let has_group_vision_leak = group.enabled
                                && group.presets.iter().any(|preset| {
                                    preset.enabled
                                        && (preset.trigger_mode == MacroTriggerMode::Press || preset.trigger_mode == MacroTriggerMode::Release)
                                        && preset.steps.iter().any(|s| s.action == MacroAction::StartVisionSearch && s.enabled)
                                        && !preset.steps.iter().any(|s| s.action == MacroAction::StopVision && s.enabled)
                                });
                            if has_group_inf_loop || has_group_vision_leak {
                                ui.add_space(2.0);
                                let response = ui.add_sized([24.0, 24.0], egui::Button::new(
                                    Self::material_icon_text(0xe002, 18.0).color(Color32::from_rgb(255, 10, 10))
                                ).frame(false));
                                if response.contains_pointer() {
                                    egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), response.id.with("group-tip"), |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(Self::material_icon_text(0xe002, 14.0).color(Color32::from_rgb(255, 10, 10)));
                                            ui.label(RichText::new(Self::tr_lang(language, "CRITICAL WARNING", "CÃ¡ÂºÂ¢NH BÃƒÂO NGUY HIÃ¡Â»â€šM")).strong().color(Color32::from_rgb(255, 10, 10)));
                                        });
                                        if has_group_inf_loop {
                                            ui.label(Self::tr_lang(
                                                language,
                                                "This group contains one or more enabled infinite loop macros! Enabling this group could lead to persistent looping upon keypress.",
                                                "NhÃ³m macro nÃƒÂ y chÃ¡Â»Â©a mÃ¡Â»â„¢t hoáº·c nhiÃ¡Â»Âu macro bÃ¡Â»â€¹ lÃ¡ÂºÂ·p vÃƒÂ´ tÃ¡ÂºÂ­n Ä‘ang bÃ¡ÂºÂ­t! KÃƒÂ­ch hoáº¡t nhÃ³m nÃƒÂ y cÃƒÂ³ thá»ƒ dÃ¡ÂºÂ«n tÃ¡Â»â€ºi lÃ¡ÂºÂ·p vÃ„Â©nh viÃ¡Â»â€¦n khi bÃ¡ÂºÂ¥m phÃ­m."
                                            ));
                                        }
                                        if has_group_vision_leak {
                                            ui.label(Self::tr_lang(
                                                language,
                                                "This group contains one or more macros that start image search (Press/Release trigger) but never stop it! This could cause background CPU thread leaks.",
                                                "NhÃ³m macro nÃƒÂ y chÃ¡Â»Â©a mÃ¡Â»â„¢t hoáº·c nhiÃ¡Â»Âu macro báº¯t Ã„â€˜Ã¡ÂºÂ§u tÃƒÂ¬m áº£nhh (kÃƒÂ­ch hoáº¡t bÃ¡ÂºÂ±ng Nháº¥n/Tháº£) nhÃ†Â°ng khÃƒÂ´ng dÃ¡Â»Â«ng lÃ¡ÂºÂ¡i! Ã„ÂiÃ¡Â»Âu nÃƒÂ y cÃƒÂ³ thá»ƒ gÃƒÂ¢y chÃ¡ÂºÂ¡y luÃ¡Â»â€œng ngÃ¡ÂºÂ§m liÃƒÂªn tÃ¡Â»Â¥c hao CPU."
                                            ));
                                        }
                                    });
                                }
                                ui.add_space(2.0);
                            }
                            let name_width = Self::preset_header_name_width(ui);
                            if group.collapsed {
                                ui.add_sized(
                                    [name_width, 24.0],
                                    egui::Label::new(
                                        RichText::new(&group.name).size(17.0).strong(),
                                    ),
                                );
                            } else {
                                let response = ui.add_sized(
                                    [name_width, 24.0],
                                    TextEdit::singleline(&mut group.name)
                                        .font(egui::FontId::proportional(17.0))
                                        .margin(egui::Margin {
                                            left: 5,
                                            right: 3,
                                            top: 1,
                                            bottom: 0,
                                        })
                                        .text_color(ui.visuals().strong_text_color())
                                        .horizontal_align(egui::Align::Center),
                                );
                                Self::apply_vietnamese_input_if_changed(
                                    &response,
                                    self.state.vietnamese_input_enabled,
                                    self.state.vietnamese_input_mode,
                                    &mut group.name,
                                );
                                live_sync |= response.changed();
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    let is_active = group.enabled && folder_enabled;
                                    let enabled_icon = if is_active { 0xe5ca } else { 0xe835 };
                                    let enabled_fill = if is_active {
                                        Color32::from_rgba_premultiplied(72, 156, 116, 120)
                                    } else {
                                        ui.visuals().faint_bg_color
                                    };
                                    let enabled_stroke = if is_active {
                                        Color32::from_rgb(126, 224, 182)
                                    } else {
                                        ui.visuals().widgets.noninteractive.bg_stroke.color
                                    };
                                    if ui
                                        .add_sized(
                                            [36.0, 24.0],
                                            Button::new(Self::material_icon_text(
                                                enabled_icon,
                                                18.0,
                                            ))
                                            .fill(enabled_fill)
                                            .stroke(egui::Stroke::new(1.0, enabled_stroke)),
                                        )
                                        .on_hover_text(Self::tr_lang(
                                            language,
                                            if folder_enabled { "Enable / disable group" } else { "Folder containing this group is disabled" }, "",
                                        ))
                                        .clicked()
                                    {
                                        group.enabled = !group.enabled;
                                        live_sync = true;
                                    }
                                    if Self::sound_style_remove_button(ui).clicked() {
                                        remove_group = Some(group.id);
                                    }
                                    if Self::sound_style_toggle_button(
                                        ui,
                                        if group.collapsed {
                                            Self::tr_lang(language, "Show", "Show")
                                        } else {
                                            Self::tr_lang(language, "Hide", "Hide")
                                        },
                                    )
                                    .clicked()
                                    {
                                        group.collapsed = !group.collapsed;
                                        live_sync = true;
                                    }
                                    if !group.collapsed {
                                        let folder_popup_id =
                                            ui.make_persistent_id((group.id, "macro-group-folder-popup"));
                                        let mut folder_popup_open = ui
                                            .ctx()
                                            .data(|data| data.get_temp::<bool>(folder_popup_id))
                                            .unwrap_or(false);
                                        let folder_button = Self::sound_style_icon_button(
                                            ui,
                                            Self::folder_icon_text(group.folder_id.is_some(), 18.0).color(
                                                if group.folder_id.is_some() {
                                                    Color32::from_rgb(248, 214, 102)
                                                } else {
                                                    ui.visuals().widgets.inactive.fg_stroke.color
                                                },
                                            ),
                                        );
                                        if folder_button.clicked() {
                                            folder_popup_open = true;
                                        }
                                        let mut selected_folder_after_popup: Option<Option<u32>> = None;
                                        let popup_response = egui::Popup::from_response(&folder_button)
                                            .id(folder_popup_id)
                                            .open_bool(&mut folder_popup_open)
                                            .align(egui::RectAlign::BOTTOM_END)
                                            .layout(egui::Layout::top_down_justified(egui::Align::Min))
                                            .width(220.0)
                                            .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
                                            .show(|ui| {
                                                ui.set_min_width(220.0);
                                                ui.label(Self::tr_lang(language, "Folder", "Folder"));
                                                ui.separator();
                                                if ui
                                                    .selectable_label(
                                                        group.folder_id.is_none(),
                                                        Self::tr_lang(language, "No folder", "No folder"),
                                                    )
                                                    .clicked()
                                                {
                                                    selected_folder_after_popup = Some(None);
                                                }
                                                for folder in &self.state.macro_folders {
                                                    if ui
                                                        .selectable_label(
                                                            group.folder_id == Some(folder.id),
                                                            &folder.name,
                                                        )
                                                        .clicked()
                                                    {
                                                        selected_folder_after_popup = Some(Some(folder.id));
                                                    }
                                                }
                                            });
                                        if let Some(selected_folder) = selected_folder_after_popup {
                                            group.folder_id = selected_folder;
                                            live_sync = true;
                                            folder_popup_open = false;
                                        }
                                        if folder_popup_open
                                            && let Some(pointer_pos) = ui.ctx().pointer_hover_pos()
                                        {
                                            let mut keep_open_rect = folder_button.rect.expand(10.0);
                                            if let Some(popup) = &popup_response {
                                                keep_open_rect =
                                                    keep_open_rect.union(popup.response.rect.expand(10.0));
                                            }
                                            if !keep_open_rect.contains(pointer_pos) {
                                                folder_popup_open = false;
                                            }
                                        }
                                        ui.ctx().data_mut(|data| {
                                            data.insert_temp(folder_popup_id, folder_popup_open)
                                        });
                                        if Self::sound_style_icon_button(
                                            ui,
                                            Self::material_icon_text(0xe145, 18.0),
                                        )
                                        .on_hover_text(Self::tr_lang(language, "Add Preset", "Thêm Preset"))
                                        .clicked()
                                        {
                                            add_preset_to_group = Some(group.id);
                                        }
                                        if self.show_share_buttons {
                                            let group_export_feedback = Self::is_copy_feedback_active(
                                             self.macro_group_export_feedback_until,
                                         );
                                         let group_export_label = if group_export_feedback {
                                             Self::tr_lang(language, "Copied", "Copied")
                                         } else {
                                             Self::tr_lang(language, "Export", "Xuáº¥t")
                                         };
                                         let group_export_button = ui.add_sized(
                                             [84.0, 24.0],
                                             Button::new(group_export_label).fill(if group_export_feedback {
                                                 Color32::from_rgba_premultiplied(72, 156, 116, 140)
                                             } else {
                                                 ui.visuals().widgets.inactive.bg_fill
                                             })
                                             .stroke(egui::Stroke::new(
                                                 1.0,
                                                 if group_export_feedback {
                                                     Color32::from_rgb(126, 224, 182)
                                                 } else {
                                                     ui.visuals().widgets.inactive.bg_stroke.color
                                                 },
                                             )),
                                         );
                                         if group_export_button
                                             .on_hover_text(Self::tr_lang(
                                                 language,
                                                 "Copy Group Code",
                                                 "Sao chÃ©p mÃ£ nhÃ³m",
                                             ))
                                             .clicked()
                                         {
                                             export_group = Some(group.id);
                                         }
                                         if Self::sized_button(
                                             ui,
                                             84.0,
                                             Self::tr_lang(language, "Import", "Nháº­p"),
                                         )
                                         .on_hover_text(Self::tr_lang(language, "Import Preset", "Nháº­p preset"))
                                         .clicked()
                                         {
                                             import_preset_to_group = Some((group.id, None));
                                         }
                                        }
                                        ui.add_space(4.0);
                                         live_sync |= Self::render_multi_window_targets_with_duplicate_mode(
                                             ui,
                                             language,
                                             (group.id, "macro-group-window-target"),
                                             Self::tr_lang(language, "Any focused window", ""),
                                             &mut group.target_window_title,
                                             &mut group.extra_target_window_titles,
                                             &mut group.match_duplicate_window_titles,
                                             &self.open_windows,
                                         );
                                    }
                                },
                            );
                        });
                        if should_scroll_to_group {
                            ui.scroll_to_cursor(Some(egui::Align::Center));
                            pending_macro_group_scroll_consumed = true;
                        }
                        if group.collapsed {
                            return;
                        }
                        let binding_labels = Self::macro_group_binding_labels(group);
                        let group_preset_step_counts = group
                            .presets
                            .iter()
                            .map(|preset| (preset.id, preset.steps.len() as u32))
                            .collect::<Vec<_>>();
                        let group_preset_options = group
                            .presets
                            .iter()
                            .map(|preset_option| {
                                (
                                    preset_option.id,
                                    binding_labels
                                        .get(&preset_option.id)
                                        .cloned()
                                        .unwrap_or_else(|| {
                                            Self::format_macro_trigger_ui(language, preset_option)
                                        }),
                                )
                            })
                            .collect::<Vec<_>>();
                        let image_search_preset_options = self
                            .state
                            .vision_presets
                            .iter()
                            .map(|preset_option| (preset_option.id, preset_option.name.clone()))
                            .collect::<Vec<_>>();
                        for preset_index in render_preset_indices.iter().copied() {
                            let preset = &mut group.presets[preset_index];
                            Self::show_macro_preset_card(ui, group.enabled && folder_enabled, preset.enabled, |ui| {
                                ui.horizontal_top(|ui| {
                                    let available_width = ui.available_width();
                                    let right_width = 540.0;
                                    let left_width =
                                        (available_width - right_width - 8.0).max(260.0);
                                    let label_width = 72.0;
                                    let binding_width = (left_width - label_width - 6.0).max(160.0);
                                    ui.allocate_ui_with_layout(
                                        vec2(left_width, 0.0),
                                        egui::Layout::left_to_right(egui::Align::TOP),
                                        |ui| {
                                            ui.allocate_ui_with_layout(
                                                vec2(label_width, 0.0),
                                                egui::Layout::top_down(egui::Align::LEFT),
                                                |ui| {
                                                    ui.label(Self::tr_lang(
                                                         language,
                                                         if preset.trigger_mode
                                                             == MacroTriggerMode::Release
                                                         {
                                                             "Release"
                                                         } else {
                                                             "Trigger"
                                                         },
                                                         if preset.trigger_mode
                                                             == MacroTriggerMode::Release
                                                         {
                                                             "Tháº£"
                                                         } else {
                                                             "KÃƒÂ­ch hoáº¡t"
                                                         },
                                                     ));
                                                },
                                            );
                                            ui.allocate_ui_with_layout(
                                                vec2(binding_width, 0.0),
                                                egui::Layout::top_down(egui::Align::LEFT),
                                                |ui| {
                                                    live_sync |= Self::render_macro_trigger_chips(
                                                        ui,
                                                        language,
                                                        group.id,
                                                        preset,
                                                        capture_target_snapshot.as_ref(),
                                                        capture_hotkey_combo_keys_snapshot.as_ref(),
                                                    );
                                                },
                                            );
                                        },
                                    );
                                    let right_spacer =
                                        (ui.available_width() - right_width).max(0.0);
                                    if right_spacer > 0.0 {
                                        ui.add_space(right_spacer);
                                    }
                                    ui.allocate_ui_with_layout(
                                        vec2(right_width, 0.0),
                                        egui::Layout::right_to_left(egui::Align::TOP),
                                        |ui| {
                                            ui.spacing_mut().item_spacing.x = 4.0;
                                            if Self::sound_style_remove_button(ui).clicked() {
                                                remove_preset = Some(preset.id);
                                            }
                                            let paste_response = ui.add_enabled_ui(self.macro_preset_clipboard.is_some(), |ui| {
                                                ui.add_sized([60.0, 24.0], Button::new(Self::tr_lang(language, "Paste", "Paste")))
                                            }).inner;
                                            if paste_response.clicked() {
                                                paste_preset_to_group = Some(group.id);
                                            }
                                            if Self::sized_button(
                                                 ui,
                                                 60.0,
                                                 Self::tr_lang(language, "Copy", "Copy"),
                                             )
                                             .clicked()
                                             {
                                                 self.macro_preset_clipboard = Some(preset.clone());
                                                 self.status = "Copied macro preset.".to_owned();
                                             }
                                             if self.show_share_buttons {
                                                 let preset_export_feedback = Self::is_copy_feedback_active(
                                                  self.macro_preset_export_feedback_until,
                                              );
                                              let preset_export_label = if preset_export_feedback {
                                                  Self::tr_lang(language, "Copied", "Copied")
                                              } else {
                                                  Self::tr_lang(language, "Exp", "Exp")
                                              };
                                              let preset_export_button = ui.add_sized(
                                                  [60.0, 24.0],
                                                  Button::new(preset_export_label).fill(if preset_export_feedback {
                                                      Color32::from_rgba_premultiplied(72, 156, 116, 140)
                                                  } else {
                                                      ui.visuals().widgets.inactive.bg_fill
                                                  })
                                                  .stroke(egui::Stroke::new(
                                                      1.0,
                                                      if preset_export_feedback {
                                                          Color32::from_rgb(126, 224, 182)
                                                      } else {
                                                          ui.visuals().widgets.inactive.bg_stroke.color
                                                      },
                                                  )),
                                              );
                                              if preset_export_button
                                                  .on_hover_text(Self::tr_lang(
                                                      language,
                                                      "Export Preset Code",
                                                      "Sao chÃ©p mÃ£ preset",
                                                  ))
                                                  .clicked()
                                              {
                                                  export_preset = Some(preset.id);
                                              }
                                             if Self::sized_button(
                                                 ui,
                                                 46.0,
                                                 Self::tr_lang(language, "Imp", "Imp"),
                                             )
                                             .on_hover_text(Self::tr_lang(language, "Import Preset from Clipboard", "Nhập Preset từ clipboard"))
                                             .clicked()
                                             {
                                                 import_preset_to_group = Some((group.id, Some(preset.id)));
                                             }
                                             }
                                            if false {
                                            let mouse_trigger_options = [
                                                (
                                                    "MouseLeft",
                                                    Self::tr_lang(language, "Left Click", "Click TrÃ¡i"),
                                                ),
                                                (
                                                    "MouseRight",
                                                    Self::tr_lang(language, "Right Click", "Click Pháº£i"),
                                                ),
                                                (
                                                    "MouseMiddle",
                                                    Self::tr_lang(language, "Middle Click", "Click Giá»¯a"),
                                                ),
                                                ("MouseX1", Self::tr_lang(language, "Mouse X1", "NÃºt Phá»¥ 1 (X1)")),
                                                ("MouseX2", Self::tr_lang(language, "Mouse X2", "NÃºt Phá»¥ 2 (X2)")),
                                                (
                                                    "MouseWheelUp",
                                                    Self::tr_lang(language, "Wheel Up", "Cuá»™n LÃªn"),
                                                ),
                                                (
                                                    "MouseWheelDown",
                                                    Self::tr_lang(language, "Wheel Down", "Cuá»™n Xuá»‘ng"),
                                                ),
                                            ];
                                            let selected_mouse_key =
                                                Self::macro_trigger_bindings(preset)
                                                    .into_iter()
                                                    .rev()
                                                    .find(|binding| {
                                                        binding.combo_keys.iter().any(|key| {
                                                            hotkey::is_mouse_key_name(key)
                                                        })
                                                    })
                                                    .and_then(|binding| {
                                                        binding.combo_keys.into_iter().find(|key| {
                                                            hotkey::is_mouse_key_name(key)
                                                        })
                                                    });
                                            let selected_mouse_label = selected_mouse_key
                                                .as_deref()
                                                .and_then(|key| {
                                                    mouse_trigger_options.iter().find(
                                                        |(option_key, _)| {
                                                            option_key.eq_ignore_ascii_case(key)
                                                        },
                                                    )
                                                })
                                                .map(|(_, label)| *label)
                                                .unwrap_or_else(|| {
                                                    Self::tr_lang(language, "Mouse", "Mouse")
                                                });
                                            let mouse_trigger_response = ui
                                                .scope(|ui| {
                                                    ui.spacing_mut().interact_size.y = 24.0;
                                                    egui::ComboBox::from_id_salt((
                                                        group.id,
                                                        preset.id,
                                                        "mouse-trigger-dropdown",
                                                    ))
                                                    .width(96.0)
                                                    .selected_text(selected_mouse_label)
                                                    .show_ui(ui, |ui| {
                                                        for (option_key, option_label) in
                                                            mouse_trigger_options
                                                        {
                                                            if ui
                                                                .selectable_label(
                                                                    selected_mouse_key
                                                                        .as_ref()
                                                                        .is_some_and(|current| {
                                                                            current
                                                                            .eq_ignore_ascii_case(
                                                                                option_key,
                                                                            )
                                                                        }),
                                                                    option_label,
                                                                )
                                                                .clicked()
                                                            {
                                                                let binding =
                                                                    hotkey::parse_binding(
                                                                        option_key,
                                                                    )
                                                                    .unwrap_or_else(|| {
                                                                        HotkeyBinding {
                                                                            ctrl: false,
                                                                            alt: false,
                                                                            shift: false,
                                                                            win: false,
                                                                            key: option_key
                                                                                .to_owned(),
                                                                            combo_keys: vec![
                                                                                option_key
                                                                                    .to_owned(),
                                                                            ],
                                                                        }
                                                                    });
                                                                live_sync |=
                                                                    Self::macro_trigger_add_binding(
                                                                        preset, binding,
                                                                    );
                                                            }
                                                        }
                                                    })
                                                })
                                                .inner;
                                            mouse_trigger_response
                                                .response
                                                .on_hover_text(selected_mouse_label);
                                            }
                                            let capture_target = CaptureRequest::MacroPresetHotkey(
                                                group.id, preset.id,
                                            );
                                            if ui
                                                .add_sized(
                                                    [64.0, 24.0],
                                                    Button::new(Self::capture_button_text(
                                                        language,
                                                        capture_target_snapshot.as_ref()
                                                            == Some(&capture_target),
                                                    )),
                                                )
                                                .clicked()
                                            {
                                                if capture_target_snapshot.as_ref()
                                                    == Some(&capture_target)
                                                {
                                                    cancel_active_capture = true;
                                                } else {
                                                    next_capture_target = Some(capture_target);
                                                }
                                            }
                                            if Self::sized_button(
                                                ui,
                                                56.0,
                                                if preset.collapsed {
                                                    Self::tr_lang(language, "Show", "Show")
                                                } else {
                                                    Self::tr_lang(language, "Hide", "Hide")
                                                },
                                            )
                                            .clicked()
                                            {
                                                preset.collapsed = !preset.collapsed;
                                                live_sync = true;
                                            }
                                            let is_preset_enabled = preset.enabled;
                                            let is_preset_active = is_preset_enabled && group.enabled && folder_enabled;
                                            let enabled_icon = if is_preset_enabled { 0xe5ca } else { 0xe835 };
                                            let enabled_fill = if is_preset_active {
                                                Color32::from_rgba_premultiplied(72, 156, 116, 120)
                                            } else if is_preset_enabled {
                                                Color32::from_rgba_premultiplied(72, 156, 116, 50)
                                            } else {
                                                ui.visuals().faint_bg_color
                                            };
                                            let enabled_stroke = if is_preset_active {
                                                Color32::from_rgb(126, 224, 182)
                                            } else if is_preset_enabled {
                                                Color32::from_rgb(110, 180, 142)
                                            } else {
                                                ui.visuals().widgets.noninteractive.bg_stroke.color
                                            };
                                            if ui
                                                .add_sized(
                                                    [36.0, 24.0],
                                                    Button::new(Self::material_icon_text(
                                                        enabled_icon,
                                                        18.0,
                                                    ))
                                                    .fill(enabled_fill)
                                                    .stroke(egui::Stroke::new(1.0, enabled_stroke)),
                                                )
                                                .on_hover_text(Self::tr_lang(
                                                    language,
                                                    if !folder_enabled {
                                                        "Folder containing this preset is disabled"
                                                    } else if !group.enabled {
                                                        "Group containing this preset is disabled"
                                                    } else {
                                                        "Enable / disable preset"
                                                    },
                                                    "",
                                                ))
                                                .clicked()
                                            {
                                                preset.enabled = !preset.enabled;
                                                live_sync = true;
                                            }
                                             let has_preset_inf_loop = self.state.macro_infinite_loop_warning_enabled
                                                 && preset.enabled
                                                 && (
                                                     (preset.trigger_mode == MacroTriggerMode::Press && !preset.stop_on_retrigger_immediate)
                                                     || preset.trigger_mode == MacroTriggerMode::Release
                                                 )
                                                 && preset.steps.iter().any(|s| s.action == MacroAction::LoopStart && s.is_infinite_loop());
                                             let has_preset_vision_leak = preset.enabled
                                                 && (preset.trigger_mode == MacroTriggerMode::Press || preset.trigger_mode == MacroTriggerMode::Release)
                                                 && preset.steps.iter().any(|s| s.action == MacroAction::StartVisionSearch && s.enabled)
                                                 && !preset.steps.iter().any(|s| s.action == MacroAction::StopVision && s.enabled);
                                             if has_preset_inf_loop || has_preset_vision_leak {
                                                 ui.add_space(4.0);
                                                 let response = ui.add_sized([24.0, 24.0], egui::Button::new(
                                                     Self::material_icon_text(0xe002, 18.0).color(Color32::from_rgb(255, 90, 0))
                                                 ).frame(false));
                                                 if response.contains_pointer() {
                                                     egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), response.id.with("preset-tip"), |ui| {
                                                         ui.horizontal(|ui| {
                                                             ui.label(Self::material_icon_text(0xe002, 14.0).color(Color32::from_rgb(255, 90, 0)));
                                                             ui.label(RichText::new(Self::tr_lang(language, "MACRO WARNING", "Cáº¢NH BÃO MACRO")).strong().color(Color32::from_rgb(255, 90, 0)));
                                                         });
                                                         if has_preset_inf_loop {
                                                             ui.label(Self::tr_lang(
                                                                 language,
                                                                 "This macro contains an infinite loop and is active. Ensure you know how to stop it to avoid system hang!",
                                                                 "Macro nÃƒÂ y chÃ¡Â»Â©a vÃƒÂ²ng lÃ¡ÂºÂ·p vÃƒÂ´ hÃ¡ÂºÂ¡n vÃƒÂ  Ä‘ang á»Ÿ chÃ¡ÂºÂ¿ Ä‘á»™ tÃ¡Â»Â± kÃƒÂ­ch hoáº¡t. HÃƒÂ£y Ã„â€˜áº£m báº£o bÃ¡ÂºÂ¡n Ã„â€˜ÃƒÂ£ biáº¿t cÃƒÂ¡ch dÃ¡Â»Â«ng nÃƒÂ³ Ä‘á»Æ’ trÃƒÂ¡nh treo mÃƒÂ¡y!"
                                                             ));
                                                         }
                                                         if has_preset_vision_leak {
                                                             ui.label(Self::tr_lang(
                                                                 language,
                                                                 "This macro starts image search (Press/Release trigger) but does not contain a 'StopImageSearch' action! This could lead to a persistent background CPU thread. Add a 'StopImageSearch' step or change trigger to 'Hold'.",
                                                                 "Macro nÃƒÂ y báº¯t Ã„â€˜Ã¡ÂºÂ§u tÃƒÂ¬m kiÃ¡ÂºÂ¿m hÃƒÂ¬nh áº£nhh (chÃ¡ÂºÂ¿ Ä‘á»™ Nháº¥n/Tháº£) nhÃ†Â°ng khÃƒÂ´ng cÃƒÂ³ bÆ°á»›c dÃ¡Â»Â«ng tÃƒÂ¬m áº£nhh! Ã„ÂiÃ¡Â»Âu nÃƒÂ y cÃƒÂ³ thá»ƒ dÃ¡ÂºÂ«n tÃ¡Â»â€ºi luÃ¡Â»â€œng chÃ¡ÂºÂ¡y ngÃ¡ÂºÂ§m liÃƒÂªn tÃ¡Â»Â¥c gÃƒÂ¢y hao CPU. HÃƒÂ£y thÃƒÂªm bÆ°á»›c dÃ¡Â»Â«ng tÃƒÂ¬m áº£nhh hoáº·c Ã„â€˜á»•i trigger sang Giá»¯ (Hold)."
                                                             ));
                                                         }
                                                     });
                                                 }
                                             }
                                            },
                                    );
                                });
                                let referenced_vars = Self::collect_preset_referenced_variables(preset);
                                if !referenced_vars.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.add_space(4.0);
                                        ui.label(RichText::new(Self::tr_lang(language, "Active Variables:", "Active Variables:")).size(11.0).weak());
                                        let vars_map = crate::overlay::RUNTIME_VARIABLES.lock();
                                        for var_name in &referenced_vars {
                                            let val = vars_map.get(var_name).copied();
                                            let val_str = val.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
                                            let bg_color = if val.is_some() {
                                                Color32::from_rgba_premultiplied(0, 191, 255, 34)
                                            } else {
                                                Color32::from_rgba_premultiplied(54, 54, 54, 230)
                                            };
                                            let stroke_color = if val.is_some() {
                                                Color32::from_rgb(0, 191, 255)
                                            } else {
                                                Color32::from_rgb(150, 150, 150)
                                            };
                                            egui::Frame::none()
                                                .fill(bg_color)
                                                .stroke(egui::Stroke::new(1.0, stroke_color))
                                                .inner_margin(egui::Margin::symmetric(6, 2))
                                                .rounding(4.0)
                                                .show(ui, |ui| {
                                                    ui.label(
                                                        RichText::new(format!("{} = {}", var_name, val_str))
                                                            .size(11.0)
                                                            .strong()
                                                            .color(if val.is_some() { Color32::from_rgb(0, 191, 255) } else { Color32::from_rgb(245, 245, 245) })
                                                    );
                                                });
                                        }
                                    });
                                }
                                if !preset.collapsed {
                                    ui.push_id((group.id, preset.id, "preset-steps-container"), |ui| {
                                        ui.horizontal(|ui| {
                            ui.label(Self::tr_lang(language, "Mode", "Mode"));
                            egui::ComboBox::from_id_salt((group.id, preset.id, "trigger-mode"))
                                .width(108.0)
                                .selected_text(Self::macro_trigger_mode_label(preset.trigger_mode, language))
                                .show_ui(ui, |ui| {
                                    for mode in [
                                        MacroTriggerMode::Press,
                                        MacroTriggerMode::Hold,
                                        MacroTriggerMode::Release,
                                    ] {
                                        if ui
                                            .selectable_label(
                                                preset.trigger_mode == mode,
                                                Self::macro_trigger_mode_label(mode, language),
                                            )
                                            .clicked()
                                        {
                                            preset.trigger_mode = mode;
                                            live_sync = true;
                                        }
                                    }
                                });
                            if preset.trigger_mode == MacroTriggerMode::Press {
                                live_sync |= ui
                                    .checkbox(
                                        &mut preset.stop_on_retrigger_immediate,
                                        Self::tr_lang(language, "Stop on trigger again", "Stop on trigger again"),
                                    )
                                    .on_hover_text(
                                        Self::tr_lang(
                                            language,
                                            "Press the trigger again to stop this macro immediately, without waiting for a StopIfTriggerPressedAgain step.",
                                            "Press the trigger again to stop this macro immediately, without waiting for a StopIfTriggerPressedAgain step.",
                                        ),
                                    )
                                    .changed();
                            } else {
                                preset.stop_on_retrigger_immediate = false;
                            }
                            if preset.trigger_mode == MacroTriggerMode::Hold {
                                ui.add_space(8.0);
                                live_sync |= ui
                                    .checkbox(
                                        &mut preset.hold_stop_step_enabled,
                                        Self::tr_lang(
                                            language,
                                            "Run one action if hold stops early",
                                            "ChÃ¡ÂºÂ¡y mÃ¡Â»â„¢t action náº¿u hold dÃ¡Â»Â«ng sÃ¡Â»â€ºm",
                                        ),
                                    )
                                    .on_hover_text(
                                        Self::tr_lang(
                                            language,
                                            "If this hold macro is interrupted before it finishes all steps, run this extra action once on stop.",
                                            "NÃ¡ÂºÂ¿u macro hold nÃƒÂ y bÃ¡Â»â€¹ ngáº¯t trÆ°á»›c khi chÃ¡ÂºÂ¡y háº¿t cÃƒÂ¡c bÆ°á»›c, hÃƒÂ£y chÃ¡ÂºÂ¡y thÃƒÂªm action nÃƒÂ y mÃ¡Â»â„¢t lÃ¡ÂºÂ§n khi dÃ¡Â»Â«ng.",
                                        ),
                                    )
                                    .changed();
                            } else {
                                preset.hold_stop_step_enabled = false;
                            }
                            if preset.trigger_mode == MacroTriggerMode::Release {
                                ui.add_space(8.0);
                                live_sync |= ui
                                    .checkbox(
                                        &mut preset.release_requires_all_inputs_released,
                                        Self::tr_lang(
                                            language,
                                            "Wait for other keys to release before triggering",
                                            "Ã„ÂÃ¡Â»Â£i cÃƒÂ¡c phÃ­m khÃ¡c nháº£ ra rÃ¡Â»â€œi mÃ¡Â»â€ºi kÃƒÂ­ch hoáº¡t",
                                        ),
                                    )
                                    .on_hover_text(
                                        Self::tr_lang(
                                            language,
                                            "If enabled, releasing the trigger key or mouse button will not fire while any other key or mouse button is still held down.",
                                            "NÃ¡ÂºÂ¿u bÃ¡ÂºÂ­t, khi bÃ¡ÂºÂ¡n tháº£ phÃ­m kÃƒÂ­ch hoáº¡t ra, macro sÃ¡ÂºÂ½ chÃ†Â°a chÃ¡ÂºÂ¡y ngay náº¿u vÃ¡ÂºÂ«n cÃƒÂ²n cÃƒÂ¡c phÃ­m/nÃƒÂºt chuá»™t khÃ¡c Ä‘ang Ä‘Æ°á»£c giá»¯. NÃƒÂ³ sÃ¡ÂºÂ½ Ä‘á»Â£i cho Ã„â€˜Ã¡ÂºÂ¿n khi toÃƒÂ n bÃ¡Â»â„¢ cÃƒÂ¡c phÃ­m Ã„â€˜ÃƒÂ³ Ä‘Æ°á»£c nháº£ ra háº¿t rÃ¡Â»â€œi mÃ¡Â»â€ºi chÃƒÂ­nh thÃ¡Â»Â©c kÃƒÂ­ch hoáº¡t.",
                                        ),
                                    )
                                    .changed();
                            } else {
                                preset.release_requires_all_inputs_released = false;
                            }
                        });
                                    if preset.trigger_mode == MacroTriggerMode::Release {
                                        if preset.release_requires_all_inputs_released {
                                            ui.horizontal(|ui| {
                                                live_sync |= Self::render_key_list_chips(
                                                    ui,
                                                    language,
                                                    &mut preset.release_wait_key,
                                                    Self::tr_lang(language, "Not set", "Not set"),
                                                );
                                                let wait_capture_target =
                                                    CaptureRequest::MacroPresetReleaseWaitKey(
                                                        group.id, preset.id,
                                                    );
                                                if ui
                                                    .add_sized(
                                                        [64.0, 22.0],
                                                        Button::new(Self::capture_button_text(
                                                            language,
                                                            capture_target_snapshot.as_ref()
                                                                == Some(&wait_capture_target),
                                                        )),
                                                    )
                                                    .clicked()
                                                {
                                                    if capture_target_snapshot.as_ref()
                                                        == Some(&wait_capture_target)
                                                    {
                                                        cancel_active_capture = true;
                                                    } else {
                                                        next_capture_target =
                                                            Some(wait_capture_target);
                                                    }
                                                }
                                            });
                                        }
                                    }
                                    if preset.trigger_mode == MacroTriggerMode::Hold && preset.hold_stop_step_enabled {
                                        Frame::group(ui.style())
                                .inner_margin(egui::Margin::symmetric(6, 4))
                                .show(ui, |ui| {
                                        let mut clear_hold_stop_step = false;
                                        let step = &mut preset.hold_stop_step;
                                        let hover_preview_source_id = ui.make_persistent_id((
                                            group.id,
                                            preset.id,
                                            "hold-stop-step-hover-preview",
                                        ));
                                        let is_dark_theme = self.state.ui_theme == UiThemeMode::Dark;
                                        let hint_color = if is_dark_theme {
                                            Color32::from_rgba_unmultiplied(140, 140, 140, 150)
                                        } else {
                                            Color32::from_rgba_unmultiplied(100, 100, 100, 150)
                                        };
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(Self::tr_lang(language, "On Stop", "On Stop"));
                                            let hold_stop_combo = egui::ComboBox::from_id_salt((
                                                group.id,
                                                preset.id,
                                                "hold-stop-action",
                                            ))
                                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                                            .width(168.0)
                                            .selected_text(format!(
                                                "{} {}",
                                                Self::macro_action_icon(step.action),
                                                Self::macro_action_selected_label(step.action, language)
                                            ))
                                            .show_ui(ui, |ui| {
                                                live_sync |= ui.checkbox(&mut step.toggle_enabled_on_run, Self::tr_lang(
                                                    language,
                                                    "🔄 Toggle self enabled on run",
                                                    ""
                                                )).changed();
                                                ui.separator();
                                                let action_hover_id = ui.make_persistent_id((
                                                    group.id,
                                                    preset.id,
                                                    "hold-stop-action-hover",
                                                ));
                                                ui.ctx().data_mut(|data| {
                                                    data.insert_temp(action_hover_id, false);
                                                });
                                                egui::Grid::new((group.id, preset.id, "hold-stop-action-grid"))
                                                    .num_columns(8)
                                                    .spacing([6.0, 6.0])
                                                    .show(ui, |ui| {
                                                        for (index, action) in [
                                                            MacroAction::KeyPress,
                                                            MacroAction::KeyDown,
                                                            MacroAction::KeyUp,
                                                            MacroAction::TypeText,
                                                            MacroAction::ApplyWindowPreset,
                                                            MacroAction::FocusWindowPreset,
                                                            MacroAction::TriggerMacroPreset,
                                                            MacroAction::TriggerCommandPreset,
                                                            MacroAction::EnableCrosshairProfile,
                                                            MacroAction::DisableCrosshair,
                                                            MacroAction::EnablePinPreset,
                                                            MacroAction::DisablePin,
                                                            MacroAction::PlaySoundPreset,
                                                            MacroAction::PlayVideoPreset,
                                                            MacroAction::ApplyMouseSensitivityPreset,
                                                            MacroAction::LoopStart,
                                                            MacroAction::LoopEnd,
                                                            MacroAction::StopIfKeyPressed,
                                                            MacroAction::ShowHud,
                                                            MacroAction::HideHud,
                                                            MacroAction::LockKeys,
                                                            MacroAction::UnlockKeys,
                                                             MacroAction::EnableMacroPreset,
                                                             MacroAction::DisableMacroPreset,
                                                                 MacroAction::EnableStep,
                                                                 MacroAction::DisableStep,
                                                              MacroAction::SetVariable,
                                                        ]
                                                        .into_iter()
                                                        .enumerate()
                                                        {
                                                            Self::render_macro_action_option(
                                                                ui,
                                                                language,
                                                                &mut step.action,
                                                                action,
                                                                &mut live_sync,
                                                                action_hover_id,
                                                                false,
                                                            );
                                                            if (index + 1) % 8 == 0 {
                                                                ui.end_row();
                                                            }
                                                        }
                                                        Self::render_mouse_action_group_option(
                                                            ui,
                                                            language,
                                                            (group.id, preset.id, "hold-stop-mouse-group"),
                                                            &mut step.action,
                                                            &mut live_sync,
                                                            action_hover_id,
                                                        );
                                                        Self::render_image_search_action_group_option(
                                                            ui,
                                                            language,
                                                            (group.id, preset.id, "hold-stop-image-search-group"),
                                                            &mut step.action,
                                                            &mut live_sync,
                                                            action_hover_id,
                                                        );
                                                        Self::render_timer_action_group_option(
                                                             ui,
                                                             language,
                                                             (group.id, preset.id, "hold-stop-timer-group"),
                                                             &mut step.action,
                                                             &mut live_sync,
                                                             action_hover_id,
                                                         );
                                                        Self::render_if_action_group_option(
                                                            ui,
                                                            language,
                                                            (group.id, preset.id, "hold-stop-if-group"),
                                                            &mut step.action,
                                                            &mut live_sync,
                                                            action_hover_id,
                                                        );
                                });
                            });
                                            Self::show_instant_hover_tooltip(
                                                ui,
                                                &hold_stop_combo.response,
                                                Self::macro_action_tooltip(step.action, language),
                                            );
                                            if hold_stop_combo.response.hovered() {
                                                let hover_request = Self::build_hover_preview_request(
                                                    language,
                                                    hover_preview_source_id,
                                                    step,
                                                );
                                                let hover_anchor_pos = ui.ctx().pointer_hover_pos().unwrap_or(hold_stop_combo.response.rect.right_bottom());
                                                ui.ctx().data_mut(|data| {
                                                    data.insert_temp(
                                                        hover_preview_state_id,
                                                        hover_request
                                                            .as_ref()
                                                            .map(|request| (group.id, request.clone(), hover_anchor_pos)),
                                                    )
                                                });
                                                if let Some(hover_request) = hover_request {
                                                    hover_preview_request = Some((group.id, hover_request));
                                                } else {
                                                    hover_preview_request = None;
                                                }
                                            }
                                            let action_uses_key = Self::macro_action_uses_key(step.action);
                                            let action_supports_capture =
                                                Self::macro_action_supports_capture(step.action);
                                            if action_uses_key {
                                                if step.action == MacroAction::ApplyWindowPreset {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .window_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            Self::tr_lang(language, "Select window", "ChÃ¡Â»Ân cá»­a sá»•").to_owned()
                                                        });
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-window-preset"))
                                                        .width(160.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.window_presets {
                                                                if ui
                                                                    .selectable_label(selected_id == Some(preset_option.id), &preset_option.name)
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::FocusWindowPreset {
                                                    let selected_label = if step.key.trim().is_empty() {
                                                        Self::tr_lang(language, "Select window", "Chọn cửa sổ").to_owned()
                                                    } else {
                                                        Self::simplify_window_title(&step.key)
                                                    };
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-focus-window-preset"))
                                                        .width(160.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            ui.strong(Self::tr_lang(language, "Open Windows", "Cửa sổ đang mở"));
                                                            if self.open_windows.is_empty() {
                                                                ui.weak(Self::tr_lang(language, "No open windows found", "Không tìm thấy cửa sổ nào"));
                                                            } else {
                                                                for win in self.open_windows.iter().take(30) {
                                                                    let label = Self::simplify_window_title(win);
                                                                    if ui
                                                                        .selectable_label(step.key == *win, label)
                                                                        .on_hover_text(win)
                                                                        .clicked()
                                                                    {
                                                                        step.key = win.clone();
                                                                        live_sync = true;
                                                                    }
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::TriggerMacroPreset {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            group_preset_options
                                                                .iter()
                                                                .find(|(preset_id, _)| *preset_id == id)
                                                                .map(|(_, label)| label.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            Self::tr_lang(language, "Select macro", "Chá»n macro").to_owned()
                                                        });
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-trigger-macro"))
                                                        .width(160.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for (preset_option_id, preset_option_label) in &group_preset_options {
                                                                if *preset_option_id == preset.id {
                                                                    continue;
                                                                }
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(*preset_option_id),
                                                                        preset_option_label,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option_id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                        ui.add_space(4.0);
                                                        let cb_text = Self::tr_lang(language, "Wait for completion", "Đợi chạy xong");
                                                        if ui.checkbox(&mut step.wait_for_completion, cb_text).changed() {
                                                            live_sync = true;
                                                        }
                                                } else if step.action == MacroAction::TriggerCommandPreset {
                                                    let selected_id = step
                                                        .key
                                                        .trim()
                                                        .parse::<u32>()
                                                        .ok()
                                                        .or_else(|| {
                                                            self.state
                                                                .command_presets
                                                                .iter()
                                                                .find(|preset| preset.name.trim().eq_ignore_ascii_case(step.key.trim()))
                                                                .map(|preset| preset.id)
                                                        });
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .command_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            if step.key.trim().is_empty() {
                                                                Self::tr_lang(language, "Select command", "ChÃ¡Â»Ân cÃƒÂ¢u lÃ¡Â»â€¡nh")
                                                                .to_owned()
                                                            } else {
                                                                step.key.clone()
                                                            }
                                                        });
                                                    let custom_preset_combo = egui::ComboBox::from_id_salt((group.id, preset.id, "trigger-custom-preset-step"))
                                                        .width(160.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.command_presets {
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(preset_option.id),
                                                                        &preset_option.name,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                    let is_generating = self.command_ai_job.as_ref()
                                                         .map(|job| job.preset_id == 999999)
                                                         .unwrap_or(false)
                                                         && self.command_ai_step_target.as_ref()
                                                             .map(|target| target.0 == group.id && target.1 == preset.id && target.2.is_none())
                                                             .unwrap_or(false);
                                                     let (custom_draft_changed, custom_save_request, custom_save_and_open_ai_request, open_ai_preset_id) = Self::render_custom_preset_step_draft_popup(
                                                          ui,
                                                          &custom_preset_combo.response,
                                                          &custom_preset_combo.response,
                                                          step,
                                                          (group.id, preset.id, "hold-stop"),
                                                          None,
                                                          language,
                                                          &command_presets_snapshot,
                                                          is_generating,
                                                     );
                                                     live_sync |= custom_draft_changed;
                                                     if let Some((step_index, name, command, use_powershell)) = custom_save_request {
                                                         pending_custom_preset_save = Some((
                                                             group.id,
                                                             preset.id,
                                                             step_index,
                                                             name,
                                                             command,
                                                             use_powershell,
                                                         ));
                                                     }
                                                     if let Some((step_index, name, command, use_powershell, is_ad_hoc)) = custom_save_and_open_ai_request {
                                                         pending_custom_preset_save_and_open_ai = Some((
                                                             group.id,
                                                             preset.id,
                                                             step_index,
                                                             name,
                                                             command,
                                                             use_powershell,
                                                             is_ad_hoc,
                                                         ));
                                                     }
                                                     if let Some(preset_id) = open_ai_preset_id {
                                                         pending_open_ai_preset_id = Some(preset_id);
                                                     }
                                                } else if matches!(
                                                    step.action,
                                                    MacroAction::EnableMacroPreset
                                                        | MacroAction::DisableMacroPreset
                                                ) {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            group_preset_options
                                                                .iter()
                                                                .find(|(preset_id, _)| *preset_id == id)
                                                                .map(|(_, label)| label.clone())
                                                        })
                                                            .unwrap_or_else(|| {
                                                                Self::tr_lang(language, "Select macro", "Chá»n macro").to_owned()
                                                            });
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-macro-enable"))
                                                        .width(160.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for (preset_option_id, preset_option_label) in &group_preset_options {
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(*preset_option_id),
                                                                        preset_option_label,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option_id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if matches!(step.action, MacroAction::EnableStep | MacroAction::DisableStep) {
                                                    // Parse `preset_id|1,2,3` or `1,2,3` for legacy fallback
                                                    let (selected_preset_id, mut selected_steps) = {
                                                        let parts: Vec<&str> = step.key.split('|').collect();
                                                        if parts.len() == 2 {
                                                            let p_id = parts[0].trim().parse::<u32>().ok();
                                                            let s_list = parts[1].split(',').filter_map(|s| s.trim().parse::<u32>().ok()).collect::<Vec<u32>>();
                                                            (p_id, s_list)
                                                        } else {
                                                            let s_list = step.key.split(',').filter_map(|s| s.trim().parse::<u32>().ok()).collect::<Vec<u32>>();
                                                            (None, s_list)
                                                        }
                                                    };
                                                    let current_preset_id = selected_preset_id.unwrap_or(preset.id);
                                                    ui.horizontal(|ui| {
                                                        ui.spacing_mut().interact_size.y = 18.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        let preset_label = group_preset_options.iter()
                                                            .find(|(id, _)| *id == current_preset_id)
                                                            .map(|(_, label)| label.clone())
                                                            .unwrap_or_else(|| Self::tr_lang(language, "Select preset", "Chá»n preset").to_owned());
                                                        egui::ComboBox::from_id_salt((group.id, preset.id, 0, "step-preset-select"))
                                                            .width(100.0)
                                                            .selected_text(preset_label)
                                                            .show_ui(ui, |ui| {
                                                                for (preset_option_id, preset_option_label) in &group_preset_options {
                                                                    if ui.selectable_label(current_preset_id == *preset_option_id, preset_option_label).clicked() {
                                                                        if current_preset_id != *preset_option_id {
                                                                            step.key = format!("{}|", preset_option_id);
                                                                            live_sync = true;
                                                                        }
                                                                    }
                                                                }
                                                            });
                                                        let target_step_count = group_preset_step_counts.iter()
                                                            .find(|(id, _)| *id == current_preset_id)
                                                            .map(|(_, count)| *count)
                                                            .unwrap_or(0);
                                                        let original_len = selected_steps.len();
                                                        selected_steps.retain(|&x| x <= target_step_count);
                                                        if selected_steps.len() != original_len {
                                                            let steps_str = selected_steps.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(",");
                                                            step.key = format!("{}|{}", current_preset_id, steps_str);
                                                            live_sync = true;
                                                        }
                                                        let steps_label = if selected_steps.is_empty() {
                                                            Self::tr_lang(language, "Select steps", "Chá»n steps").to_owned()
                                                        } else {
                                                            selected_steps.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")
                                                        };
                                                        egui::ComboBox::from_id_salt((group.id, preset.id, 0, "step-multi-select"))
                                                            .width(100.0)
                                                            .selected_text(steps_label)
                                                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                                                            .show_ui(ui, |ui| {
                                                                for i in 1..=target_step_count {
                                                                    let mut is_selected = selected_steps.contains(&i);
                                                                    if ui.checkbox(&mut is_selected, format!("Step {}", i)).changed() {
                                                                        if is_selected {
                                                                            selected_steps.push(i);
                                                                        } else {
                                                                            selected_steps.retain(|x| *x != i);
                                                                        }
                                                                        selected_steps.sort_unstable();
                                                                        let steps_str = selected_steps.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(",");
                                                                        step.key = format!("{}|{}", current_preset_id, steps_str);
                                                                        live_sync = true;
                                                                    }
                                                                }
                                                            });
                                                    });
                                                } else if matches!(step.action, MacroAction::StartTimerPreset | MacroAction::PauseTimerPreset | MacroAction::StopTimerPreset) {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state.timer_presets.iter()
                                                                .find(|p| p.id == id)
                                                                .map(|p| p.name.clone())
                                                        })
                                                        .unwrap_or_else(|| Self::tr_lang(language, "Select timer", "Chá»n háº¹n giá»").to_owned());
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-timer-preset"))
                                                        .width(160.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for timer in &self.state.timer_presets {
                                                                if ui.selectable_label(selected_id == Some(timer.id), &timer.name).clicked() {
                                                                    step.key = timer.id.to_string();
                                                                    step.timer_preset_id = Some(timer.id);
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::EnableCrosshairProfile {
                                                    let selected_label = if step.key.trim().is_empty() {
                                                        Self::tr_lang(language, "Select crosshair", "Chá»n tÃ¢m").to_owned()
                                                    } else {
                                                        step.key.clone()
                                                    };
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-crosshair"))
                                                        .width(160.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for profile in &self.state.profiles {
                                                                if ui
                                                                    .selectable_label(step.key == profile.name, &profile.name)
                                                                    .clicked()
                                                                {
                                                                    step.key = profile.name.clone();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::EnablePinPreset {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .pin_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            Self::tr_lang(language, "Select pin", "Chá»n ghim").to_owned()
                                                        });
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-pin-preset"))
                                                        .width(160.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.pin_presets {
                                                                if ui
                                                                    .selectable_label(selected_id == Some(preset_option.id), &preset_option.name)
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::PlayMousePathPreset {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .mouse_path_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            Self::tr_lang(language, "Select path", "ChÃ¡Â»Ân Ã„â€˜Ã†Â°Ã¡Â»Âng chuá»™t").to_owned()
                                                        });
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-mouse-path"))
                                                        .width(160.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.mouse_path_presets {
                                                                if ui
                                                                    .selectable_label(selected_id == Some(preset_option.id), &preset_option.name)
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if matches!(
                                                    step.action,
                                                    MacroAction::StartVisionSearch
                                                         | MacroAction::ScanVisionOnce
                                                         | MacroAction::TriggerVisionMove
                                                         | MacroAction::StopVision
                                                ) {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            image_search_preset_options
                                                                .iter()
                                                                .find(|(preset_id, _)| *preset_id == id)
                                                                .map(|(_, label)| label.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            Self::tr_lang(
                                                                language,
                                                                "Select image search preset",
                                                                "Chá»n preset image search",
                                                            )
                                                            .to_owned()
                                                        });
                                                egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-image-search"))
                                                    .width(160.0)
                                                    .selected_text(selected_label)
                                                    .show_ui(ui, |ui| {
                                                        for (preset_option_id, preset_option_label) in &image_search_preset_options {
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(*preset_option_id),
                                                                        preset_option_label,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option_id.to_string();
                                                                    live_sync = true;
                                                                }
                                                        }
                                                    });
                                                     let is_pixel = selected_id.and_then(|id| {
                                                         self.state.vision_presets.iter().find(|p| p.id == id)
                                                     }).map(|p| p.is_pixel_counter).unwrap_or(false);
                                                     if step.action == MacroAction::ScanVisionOnce && is_pixel {
                                                         ui.add_space(4.0);
                                                         ui.horizontal(|ui| {
                                                             let mut variable_layouter = |ui: &egui::Ui, string: &dyn TextBuffer, wrap_width: f32| {
                                                                 let job = Self::variable_highlight_job(ui, string.as_str(), wrap_width);
                                                                 ui.fonts_mut(|fonts| fonts.layout_job(job))
                                                             };
                                                             let response = ui.add_sized(
                                                                 [100.0, 22.0],
                                                                 TextEdit::singleline(&mut step.if_variable_name)
                                                                     .layouter(&mut variable_layouter)
                                                                     .hint_text(RichText::new(Self::tr_lang(language, "set variable", "gÃ¡n biáº¿n")).color(hint_color).weak()),
                                                             );
                                                             Self::apply_vietnamese_input_if_changed(
                                                                 &response,
                                                                 self.state.vietnamese_input_enabled,
                                                                 self.state.vietnamese_input_mode,
                                                                 &mut step.if_variable_name,
                                                             );
                                                             live_sync |= response.changed();
                                                             Self::render_variable_suggestions_raw(ui, &response, &mut step.if_variable_name, &timer_names, language);
                                                         });
                                                     }
                                                } else if step.action == MacroAction::ApplyMouseSensitivityPreset {
                                                    live_sync |= ui.checkbox(&mut step.manual_mouse_sensitivity, Self::tr_lang(language, "Manual", "Nháº­p tay")).changed();
                                                    if step.manual_mouse_sensitivity {
                                                        ui.vertical(|ui| {
                                                            let mut variable_layouter = |ui: &egui::Ui, string: &dyn TextBuffer, wrap_width: f32| {
                                                                let job = Self::variable_highlight_job(ui, string.as_str(), wrap_width);
                                                                ui.fonts_mut(|fonts| fonts.layout_job(job))
                                                            };
                                                            let response = ui.add_sized(
                                                                [110.0, 22.0],
                                                                TextEdit::singleline(&mut step.key)
                                                                    .layouter(&mut variable_layouter)
                                                                    .hint_text(RichText::new(Self::tr_lang(language, "value/expr", "giÃƒÂ¡ trÃ¡Â»â€¹")).color(hint_color).weak()),
                                                            );
                                                            Self::apply_vietnamese_input_if_changed(
                                                                &response,
                                                                self.state.vietnamese_input_enabled,
                                                                self.state.vietnamese_input_mode,
                                                                &mut step.key,
                                                            );
                                                            live_sync |= response.changed();
                                                            let interpolated = crate::overlay::interpolate_variables(&step.key);
                                                            let evaluated = crate::overlay::evaluate_math_expression(&interpolated);
                                                            let clamped = evaluated.clamp(1, 20);
                                                            let tooltip_text = match language {
                                                                UiLanguage::Vietnamese => format!("Káº¿t quáº£: {} (giÃ¡Â»â€ºi hÃ¡ÂºÂ¡n: {} trong 1..20)", evaluated, clamped),
                                                                _ => format!("Evaluated: {} (clamped to: {} within 1..20)", evaluated, clamped),
                                                            };
                                                            let response = response.on_hover_text(tooltip_text);
                                                            Self::render_variable_suggestions(ui, &response, &mut step.key, &timer_names, language);
                                                        });
                                                    } else {
                                                        let selected_id = step.key.trim().parse::<u32>().ok();
                                                        let selected_label = selected_id
                                                            .and_then(|id| {
                                                                self.state
                                                                    .mouse_sensitivity_presets
                                                                    .iter()
                                                                    .find(|preset| preset.id == id)
                                                                    .map(|preset| preset.name.clone())
                                                            })
                                                            .unwrap_or_else(|| {
                                                                Self::tr_lang(language, "Select sens", "ChÃ¡Â»Ân Ä‘á»™ nhÃ¡ÂºÂ¡y")
                                                                .to_owned()
                                                            });
                                                        ui.push_id((group.id, preset.id, "mouse-sensitivity-preset-step"), |ui| {
                                                            egui::ComboBox::from_id_salt("mouse-sensitivity-preset-step-combo")
                                                                .width(110.0)
                                                                .selected_text(selected_label)
                                                                .show_ui(ui, |ui| {
                                                                    for preset_option in &self.state.mouse_sensitivity_presets {
                                                                        if ui
                                                                            .selectable_label(
                                                                                selected_id == Some(preset_option.id),
                                                                                &preset_option.name,
                                                                            )
                                                                            .clicked()
                                                                        {
                                                                            step.key = preset_option.id.to_string();
                                                                            live_sync = true;
                                                                        }
                                                                    }
                                                                });
                                                        });
                                                    }
                                                } else if step.action == MacroAction::EnableZoomPreset {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .zoom_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            Self::tr_lang(language, "Select zoom preset", "Select zoom preset").to_owned()
                                                        });
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-zoom"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.zoom_presets {
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(preset_option.id),
                                                                        &preset_option.name,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::PlaySoundPreset {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .audio_settings
                                                                .presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            Self::tr_lang(language, "Select sound", "Chá»n Ã¢m thanh").to_owned()
                                                        });
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-sound"))
                                                        .width(160.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.audio_settings.presets {
                                                                if ui
                                                                    .selectable_label(selected_id == Some(preset_option.id), &preset_option.name)
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::PlayVideoPreset {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .audio_settings
                                                                .video_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            Self::tr_lang(language, "Select video", "Chọn video").to_owned()
                                                        });
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-video"))
                                                        .width(160.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.audio_settings.video_presets {
                                                                if ui
                                                                    .selectable_label(selected_id == Some(preset_option.id), &preset_option.name)
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::UnlockKeys {
                                                    let key_id = ui.id().with(("hold-stop-unlockkeys-key",));
                                                    let response = Self::render_expandable_text_edit(
                                                        ui,
                                                        &mut step.key,
                                                        key_id,
                                                        160.0,
                                                        260.0,
                                                        22.0,
                                                        22.0,
                                                        "A,S,W,D",
                                                        false,
                                                    );
                                                    Self::apply_vietnamese_input_if_changed(
                                                        &response,
                                                        self.state.vietnamese_input_enabled,
                                                        self.state.vietnamese_input_mode,
                                                        &mut step.key,
                                                    );
                                                    live_sync |= response.changed();
                                                } else if step.action == MacroAction::LockKeys {
                                                    let key_id = ui.id().with(("hold-stop-lockkeys-key",));
                                                    let response = Self::render_expandable_text_edit(
                                                        ui,
                                                        &mut step.key,
                                                        key_id,
                                                        160.0,
                                                        260.0,
                                                        22.0,
                                                        22.0,
                                                        "A,S,W,D",
                                                        false,
                                                    );
                                                    Self::apply_vietnamese_input_if_changed(
                                                        &response,
                                                        self.state.vietnamese_input_enabled,
                                                        self.state.vietnamese_input_mode,
                                                        &mut step.key,
                                                    );
                                                    live_sync |= response.changed();
                                                    ui.add_space(4.0);
                                                    let unlock_resp = ui.checkbox(&mut step.unlock_on_exit, Self::tr_lang(language, "Unlock when macro ends", ""));
                                                    if unlock_resp.changed() {
                                                        live_sync = true;
                                                    }
                                                    if !step.unlock_on_exit {
                                                        let warn_color = Color32::from_rgb(255, 90, 0);
                                                        let response = ui.add(egui::Label::new(Self::material_icon_text(0xe002, 14.0).color(warn_color)).sense(egui::Sense::hover()));
                                                        if response.contains_pointer() {
                                                            egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), response.id.with("lockkeys-warning-tip"), |ui| {
                                                                ui.horizontal(|ui| {
                                                                    ui.label(Self::material_icon_text(0xe002, 14.0).color(warn_color));
                                                                    ui.label(RichText::new(Self::tr_lang(language, "STEP WARNING", "CẢNH BÁO BƯỚC")).strong().color(warn_color));
                                                                });
                                                                ui.label(Self::tr_lang(
                                                                    language,
                                                                    "Warning: Keeping keys locked after the macro ends can make your keyboard unresponsive until manually unlocked!",
                                                                    ""
                                                                ));
                                                            });
                                                        }
                                                    }
                                                } else if step.action == MacroAction::LoopStart {
                                                    let mut infinite = Self::loop_is_infinite(step);
                                                     if ui
                                                         .checkbox(
                                                             &mut infinite,
                                                             RichText::new(Self::tr_lang(
                                                                 language,
                                                                 "Infinite",
                                                                 "Infinite",
                                                             ))
                                                             .color(Color32::WHITE)
                                                             .strong(),
                                                         )
                                                         .changed()
                                                     {
                                                         step.key = if infinite {
                                                             "infinite".to_owned()
                                                         } else {
                                                             "1".to_owned()
                                                         };
                                                         live_sync = true;
                                                      }
                                                      if !infinite {
                                                          ui.vertical(|ui| {
                                                              let key_id = ui.id().with(("hold-stop-loop-count",));
                                                              let response = Self::render_variable_text_edit(
                                                                  ui,
                                                                  &mut step.key,
                                                                  key_id,
                                                                  96.0,
                                                                  180.0,
                                                                  22.0,
                                                                  22.0,
                                                                  &Self::tr_lang(language, "Loop count", "Số lần lặp"),
                                                                  false,
                                                              );
                                                              Self::apply_vietnamese_input_if_changed(
                                                                  &response,
                                                                  self.state.vietnamese_input_enabled,
                                                                  self.state.vietnamese_input_mode,
                                                                  &mut step.key,
                                                              );
                                                              live_sync |= response.changed();
                                                              Self::render_variable_suggestions(ui, &response, &mut step.key, &timer_names, language);
                                                          });
                                                      }
                                                } else if step.action == MacroAction::StopIfKeyPressed {
                                                     ui.scope(|ui| {
                                                         ui.spacing_mut().item_spacing.x = 4.0;
                                                         ui.spacing_mut().interact_size.y = 22.0;
                                                         ui.spacing_mut().button_padding.y = 0.0;
                                                         ui.allocate_ui_with_layout(
                                                             vec2(ui.available_width(), 22.0),
                                                             egui::Layout::top_down(egui::Align::Min),
                                                             |ui| {
                                                             ui.horizontal(|ui| {
                                                                 let current_mode = step.get_break_loop_mode().to_string();
                                                                 let mode_label = match current_mode.as_str() {
                                                                     "VarCompare" => Self::tr_lang(language, "Var compare", "So sánh biến"),
                                                                     "StopKey" => Self::tr_lang(language, "Stop key", "Nút đã nhấn"),
                                                                     _ => Self::tr_lang(language, "Break Loop", "Ngắt lặp"),
                                                                 };
                                                                 egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-loop-break-mode"))
                                                                     .width(100.0)
                                                                     .selected_text(mode_label)
                                                                     .show_ui(ui, |ui| {
                                                                         if ui.selectable_label(current_mode == "Immediate", Self::tr_lang(language, "Break Loop", "Ngắt lặp")).clicked() {
                                                                             step.break_loop_mode = "Immediate".to_string();
                                                                             step.break_loop_by_variable = false;
                                                                             live_sync = true;
                                                                         }
                                                                         if ui.selectable_label(current_mode == "VarCompare", Self::tr_lang(language, "Var compare", "So sánh biến")).clicked() {
                                                                             step.break_loop_mode = "VarCompare".to_string();
                                                                             step.break_loop_by_variable = true;
                                                                             live_sync = true;
                                                                         }
                                                                         if ui.selectable_label(current_mode == "StopKey", Self::tr_lang(language, "Stop key", "Nút đã nhấn")).clicked() {
                                                                             step.break_loop_mode = "StopKey".to_string();
                                                                             step.break_loop_by_variable = false;
                                                                             live_sync = true;
                                                                         }
                                                                     });

                                                                 let mode = step.get_break_loop_mode();
                                                                 if mode == "VarCompare" {
                                                                     let var_name_id = ui.id().with("hold-stop-loop-break-var-name");
                                                                     let response = Self::render_variable_text_edit(
                                                                         ui,
                                                                         &mut step.if_variable_name,
                                                                         var_name_id,
                                                                         76.0,
                                                                         140.0,
                                                                         22.0,
                                                                         22.0,
                                                                         Self::tr_lang(language, "variable", "biến"),
                                                                         false,
                                                                     );
                                                                     Self::apply_vietnamese_input_if_changed(
                                                                         &response,
                                                                         self.state.vietnamese_input_enabled,
                                                                         self.state.vietnamese_input_mode,
                                                                         &mut step.if_variable_name,
                                                                     );
                                                                     live_sync |= response.changed();
                                                                     Self::render_variable_suggestions(ui, &response, &mut step.if_variable_name, &timer_names, language);
                                                                     egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-loop-if-op"))
                                                                         .width(40.0)
                                                                         .selected_text(&step.if_operator)
                                                                         .show_ui(ui, |ui| {
                                                                             for op in &["==", ">", "<", ">=", "<=", "!="] {
                                                                                 if ui.selectable_label(step.if_operator == *op, *op).clicked() {
                                                                                     step.if_operator = op.to_string();
                                                                                     live_sync = true;
                                                                                 }
                                                                             }
                                                                         });
                                                                     let key_val_id = ui.id().with("hold-stop-loop-break-var-val");
                                                                     let response2 = Self::render_variable_text_edit(
                                                                         ui,
                                                                         &mut step.key,
                                                                         key_val_id,
                                                                         76.0,
                                                                         140.0,
                                                                         22.0,
                                                                         22.0,
                                                                         Self::tr_lang(language, "value/expr", "giá trị"),
                                                                         false,
                                                                     );
                                                                     Self::apply_vietnamese_input_if_changed(
                                                                         &response2,
                                                                         self.state.vietnamese_input_enabled,
                                                                         self.state.vietnamese_input_mode,
                                                                         &mut step.key,
                                                                     );
                                                                     live_sync |= response2.changed();
                                                                     Self::render_variable_suggestions(ui, &response2, &mut step.key, &timer_names, language);
                                                                     let var_name = step.if_variable_name.trim();
                                                                     if !var_name.is_empty() {
                                                                         let current_val = crate::overlay::RUNTIME_VARIABLES.lock().get(var_name).copied();
                                                                         let val_str = current_val.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
                                                                         ui.add_space(2.0);
                                                                         ui.label(
                                                                             RichText::new(format!("({})", val_str))
                                                                                 .size(10.0)
                                                                                 .color(Color32::from_rgb(0, 191, 255))
                                                                         ).on_hover_text(Self::tr_lang(language, "Current runtime value", "Giá trị chạy hiện tại"));
                                                                     }
                                                                     if ui.add_sized([24.0, 24.0], Button::new("+")).on_hover_text(Self::tr_lang(language, "Add condition", "Thêm điều kiện")).clicked() {
                                                                         step.extra_conditions.push(ExtraCondition::default());
                                                                         live_sync = true;
                                                                     }
                                                                 } else if mode == "StopKey" {
                                                                     let response = ui.add_sized(
                                                                         [160.0, 22.0],
                                                                         TextEdit::singleline(&mut step.key).hint_text(
                                                                             RichText::new(Self::tr_lang(
                                                                                 language,
                                                                                 "Stop key",
                                                                                 "Phím dừng vòng lặp",
                                                                             ))
                                                                             .color(hint_color)
                                                                             .italics(),
                                                                         ),
                                                                     );
                                                                     Self::apply_vietnamese_input_if_changed(
                                                                         &response,
                                                                         self.state.vietnamese_input_enabled,
                                                                         self.state.vietnamese_input_mode,
                                                                         &mut step.key,
                                                                     );
                                                                     live_sync |= response.changed();
                                                                     Self::render_variable_suggestions(ui, &response, &mut step.key, &timer_names, language);
                                                                 } else {
                                                                     ui.label(RichText::new(Self::tr_lang(language, "No input", "Không có đầu vào")).weak().italics());
                                                                 }
                                                             });
                                                             if step.get_break_loop_mode() == "VarCompare" {
                                                                 let mut remove_extra_idx = None;
                                                                 for (extra_idx, cond) in step.extra_conditions.iter_mut().enumerate() {
                                                                      ui.horizontal(|ui| {
                                                                          ui.add_space(100.0);
                                                                                                                                                  egui::ComboBox::from_id_salt((group.id, preset.id, extra_idx, "hold-stop-loop-extra-join"))
                                                                              .width(56.0)
                                                                              .selected_text(if cond.join_operator.eq_ignore_ascii_case("OR") { Self::tr_lang(language, "OR", "HO?C") } else { Self::tr_lang(language, "AND", "VÀ") })
                                                                              .show_ui(ui, |ui| {
                                                                                 for op in &["AND", "OR"] {
                                                                                     let label = if *op == "AND" {
                                                                                         Self::tr_lang(language, "AND", "VÀ")
                                                                                     } else {
                                                                                         Self::tr_lang(language, "OR", "HO?C")
                                                                                     };
                                                                                     if ui.selectable_label(cond.join_operator.eq_ignore_ascii_case(op), label).clicked() {
                                                                                         cond.join_operator = op.to_string();
                                                                                         live_sync = true;
                                                                                     }
                                                                                 }
                                                                             });
                                                                         let cond_var_id = ui.id().with((extra_idx, "hold-stop-loop-extra-var"));
                                                                         let response = Self::render_variable_text_edit(
                                                                             ui,
                                                                             &mut cond.variable_name,
                                                                             cond_var_id,
                                                                             76.0,
                                                                             140.0,
                                                                             22.0,
                                                                             22.0,
                                                                             Self::tr_lang(language, "variable", "biáº¿n"),
                                                                             false,
                                                                         );
                                                                         Self::apply_vietnamese_input_if_changed(
                                                                             &response,
                                                                             self.state.vietnamese_input_enabled,
                                                                             self.state.vietnamese_input_mode,
                                                                             &mut cond.variable_name,
                                                                         );
                                                                         live_sync |= response.changed();
                                                                         egui::ComboBox::from_id_salt((group.id, preset.id, extra_idx, "hold-stop-loop-extra-op"))
                                                                             .width(40.0)
                                                                             .selected_text(&cond.operator)
                                                                             .show_ui(ui, |ui| {
                                                                                 for op in &["==", ">", "<", ">=", "<=", "!="] {
                                                                                     if ui.selectable_label(cond.operator == *op, *op).clicked() {
                                                                                         cond.operator = op.to_string();
                                                                                         live_sync = true;
                                                                                     }
                                                                                 }
                                                                             });
                                                                          let cond_expr_id = ui.id().with((extra_idx, "hold-stop-loop-extra-expr"));
                                                                          let response2 = Self::render_variable_text_edit(
                                                                              ui,
                                                                              &mut cond.expression,
                                                                              cond_expr_id,
                                                                              76.0,
                                                                              160.0,
                                                                              22.0,
                                                                              22.0,
                                                                              Self::tr_lang(language, "value/expr", "giá trị/expr"),
                                                                              false,
                                                                          );
                                                                         Self::apply_vietnamese_input_if_changed(
                                                                             &response2,
                                                                             self.state.vietnamese_input_enabled,
                                                                             self.state.vietnamese_input_mode,
                                                                             &mut cond.expression,
                                                                         );
                                                                         live_sync |= response2.changed();
                                                                         let var_name = cond.variable_name.trim();
                                                                         if !var_name.is_empty() {
                                                                             let current_val = crate::overlay::RUNTIME_VARIABLES.lock().get(var_name).copied();
                                                                             let val_str = current_val.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
                                                                             ui.label(
                                                                                 RichText::new(format!("({})", val_str))
                                                                                     .size(10.0)
                                                                                     .color(Color32::from_rgb(0, 191, 255))
                                                                             );
                                                                         }
                                                                         if ui.add_sized([24.0, 24.0], Button::new("-")).on_hover_text(Self::tr_lang(language, "Remove condition", "Xóa điều kiện")).clicked() {
                                                                             remove_extra_idx = Some(extra_idx);
                                                                         }
                                                                     });
                                                                 }
                                                                 if let Some(remove_idx) = remove_extra_idx {
                                                                     step.extra_conditions.remove(remove_idx);
                                                                     live_sync = true;
                                                                 }
                                                             }
                                                         });
                                                     });
                                                } else if step.action == MacroAction::ShowHud {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .hud_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            if step.key.trim().is_empty() {
                                                                Self::tr_lang(
                                                                    language,
                                                                    "Select HUD",
                                                                    "Chá»n HUD",
                                                                )
                                                                .to_owned()
                                                            } else {
                                                                format!("CÃ…Â©: {}", step.key)
                                                            }
                                                        });
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.spacing_mut().interact_size.y = 22.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-toolbox-preset"))
                                                            .width(112.0)
                                                            .selected_text(selected_label)
                                                            .show_ui(ui, |ui| {
                                                                for toolbox_preset in &self.state.hud_presets {
                                                                    if ui
                                                                        .selectable_label(
                                                                            selected_id == Some(toolbox_preset.id),
                                                                            &toolbox_preset.name,
                                                                        )
                                                                        .clicked()
                                                                    {
                                                                        step.key = toolbox_preset.id.to_string();
                                                                        live_sync = true;
                                                                    }
                                                                }
                                                            });
                                                        let text_id = ui.id().with(("hold-stop-showhud-text-override",));
                                                        let response = Self::render_variable_text_edit(
                                                            ui,
                                                            &mut step.text_override,
                                                            text_id,
                                                            120.0,
                                                            240.0,
                                                            22.0,
                                                            22.0,
                                                            &Self::tr_lang(language, "Text override", "Ghi đè văn bản"),
                                                            false,
                                                        );
                                                        Self::apply_vietnamese_input_if_changed(
                                                            &response,
                                                            self.state.vietnamese_input_enabled,
                                                            self.state.vietnamese_input_mode,
                                                            &mut step.text_override,
                                                        );
                                                        live_sync |= response.changed();
                                                        Self::render_variable_suggestions(
                                                            ui,
                                                            &response,
                                                            &mut step.text_override,
                                                            &timer_names,
                                                            language,
                                                        );
                                                    });
                                                } else if step.action == MacroAction::TypeText {
                                                    ui.vertical(|ui| {
                                                        let response = Self::render_variable_text_edit(
                                                            ui,
                                                            &mut step.key,
                                                            ui.id().with("hold-stop-type-text-key"),
                                                            220.0,
                                                            360.0,
                                                            22.0,
                                                            44.0,
                                                            Self::tr_lang(language, "Text to type", "Văn bảnh cần gõ"),
                                                            true,
                                                        );
                                                        Self::apply_vietnamese_input_if_changed(
                                                            &response,
                                                            self.state.vietnamese_input_enabled,
                                                            self.state.vietnamese_input_mode,
                                                            &mut step.key,
                                                        );
                                                        live_sync |= response.changed();
                                                        Self::render_variable_suggestions(ui, &response, &mut step.key, &timer_names, language);
                                                    });
                                                } else if step.action == MacroAction::DisableCrosshair {
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.spacing_mut().interact_size.y = 22.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        ui.horizontal(|ui| {
                                                            let response = ui.checkbox(&mut step.lock_mouse_left, Self::tr_lang(language, "All", ""));
                                                            live_sync |= response.changed();
                                                            if !step.lock_mouse_left {
                                                                let selected_label = if step.key.trim().is_empty() {
                                                                    Self::tr_lang(language, "Select profile", "Chá»n profile").to_owned()
                                                                } else {
                                                                    step.key.clone()
                                                                };
                                                                egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-disable-crosshair"))
                                                                    .width(110.0)
                                                                    .selected_text(selected_label)
                                                                    .show_ui(ui, |ui| {
                                                                        for profile in &self.state.profiles {
                                                                            if ui
                                                                                .selectable_label(step.key == profile.name, &profile.name)
                                                                                .clicked()
                                                                            {
                                                                                step.key = profile.name.clone();
                                                                                live_sync = true;
                                                                            }
                                                                        }
                                                                    });
                                                            }
                                                        });
                                                    });
                                                } else if step.action == MacroAction::DisablePin {
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.spacing_mut().interact_size.y = 22.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        ui.horizontal(|ui| {
                                                            let response = ui.checkbox(&mut step.lock_mouse_left, Self::tr_lang(language, "All", ""));
                                                            live_sync |= response.changed();
                                                            if !step.lock_mouse_left {
                                                                let selected_id = step.key.trim().parse::<u32>().ok();
                                                                let selected_label = selected_id
                                                                    .and_then(|id| {
                                                                        self.state
                                                                            .pin_presets
                                                                            .iter()
                                                                            .find(|p| p.id == id)
                                                                            .map(|p| p.name.clone())
                                                                    })
                                                                    .unwrap_or_else(|| {
                                                                        Self::tr_lang(language, "Select pin", "Chá»n preset ghim").to_owned()
                                                                    });
                                                                egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-disable-pin"))
                                                                    .width(110.0)
                                                                    .selected_text(selected_label)
                                                                    .show_ui(ui, |ui| {
                                                                        for preset_option in &self.state.pin_presets {
                                                                            if ui
                                                                                .selectable_label(selected_id == Some(preset_option.id), &preset_option.name)
                                                                                .clicked()
                                                                            {
                                                                                step.key = preset_option.id.to_string();
                                                                                live_sync = true;
                                                                            }
                                                                        }
                                                                    });
                                                            }
                                                        });
                                                    });
                                                } else if matches!(step.action, MacroAction::DisableZoom | MacroAction::Else | MacroAction::IfEnd | MacroAction::HideHud | MacroAction::UnlockMouse) {
                                                    ui.add_sized(
                                                        [110.0, 22.0],
                                                        egui::Label::new(Self::tr_lang(language, "No input", "No input")),
                                                    );
                                                } else if step.action == MacroAction::LockMouse {
                                                    ui.horizontal(|ui| {
                                                        let unlock_resp = ui.checkbox(&mut step.unlock_on_exit, Self::tr_lang(language, "Unlock when macro ends", ""));
                                                        if unlock_resp.changed() {
                                                            live_sync = true;
                                                        }
                                                        if !step.unlock_on_exit {
                                                        let warn_color = Color32::from_rgb(255, 90, 0);
                                                        let response = ui.add(egui::Label::new(Self::material_icon_text(0xe002, 14.0).color(warn_color)).sense(egui::Sense::hover()));
                                                        if response.contains_pointer() {
                                                            egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), response.id.with("lockmouse-warning-tip"), |ui| {
                                                                ui.horizontal(|ui| {
                                                                    ui.label(Self::material_icon_text(0xe002, 14.0).color(warn_color));
                                                                    ui.label(RichText::new(Self::tr_lang(language, "STEP WARNING", "CẢNH BÁO BƯỚC")).strong().color(warn_color));
                                                                });
                                                                ui.label(Self::tr_lang(
                                                                    language,
                                                                    "Warning: Keeping mouse locked after the macro ends can make your mouse unresponsive until manually unlocked!",
                                                                    ""
                                                                ));
                                                            });
                                                        }
                                                    }
                                                    });
                                                } else if step.action == MacroAction::IfStart {
                                                     ui.scope(|ui| {
                                                         ui.spacing_mut().item_spacing.x = 4.0;
                                                         ui.spacing_mut().interact_size.y = 22.0;
                                                         ui.spacing_mut().button_padding.y = 0.0;
                                                         ui.allocate_ui_with_layout(
                                                             vec2(ui.available_width(), 22.0),
                                                             egui::Layout::top_down(egui::Align::Min),
                                                             |ui| {
                                                             ui.horizontal(|ui| {
                                                                   ui.add_sized(
                                                                       [56.0, 22.0],
                                                                       egui::Label::new(Self::tr_lang(language, "IF", "Náº¾U")),
                                                                   );
                                                                   if step.if_condition_type != IfConditionType::Variable {
                                                                       step.if_condition_type = IfConditionType::Variable;
                                                                       live_sync = true;
                                                                   }
                                                                   if step.if_condition_type == IfConditionType::Variable {
                                                                    let var_name_id = ui.id().with("hold-stop-if-var-name");
                                                                    let response = Self::render_variable_text_edit(
                                                                        ui,
                                                                        &mut step.if_variable_name,
                                                                        var_name_id,
                                                                        76.0,
                                                                        140.0,
                                                                        22.0,
                                                                        22.0,
                                                                        Self::tr_lang(language, "value/expr", "biến/expr"),
                                                                        false,
                                                                    );
                                                                    Self::apply_vietnamese_input_if_changed(
                                                                        &response,
                                                                        self.state.vietnamese_input_enabled,
                                                                        self.state.vietnamese_input_mode,
                                                                        &mut step.if_variable_name,
                                                                    );
                                                                    live_sync |= response.changed();
                                                                    Self::render_variable_suggestions(ui, &response, &mut step.if_variable_name, &timer_names, language);
                                                                    egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-if-op"))
                                                                        .width(40.0)
                                                                        .selected_text(&step.if_operator)
                                                                        .show_ui(ui, |ui| {
                                                                            for op in &["==", ">", "<", ">=", "<=", "!="] {
                                                                                if ui.selectable_label(step.if_operator == *op, *op).clicked() {
                                                                                    step.if_operator = op.to_string();
                                                                                    live_sync = true;
                                                                                }
                                                                            }
                                                                        });
                                                                    let var_val_id = ui.id().with("hold-stop-if-var-val");
                                                                    let response2 = Self::render_variable_text_edit(
                                                                        ui,
                                                                        &mut step.key,
                                                                        var_val_id,
                                                                        76.0,
                                                                        180.0,
                                                                        22.0,
                                                                        22.0,
                                                                        Self::tr_lang(language, "value/expr", "giá trị/expr"),
                                                                        false,
                                                                    );
                                                                    Self::apply_vietnamese_input_if_changed(
                                                                        &response2,
                                                                        self.state.vietnamese_input_enabled,
                                                                        self.state.vietnamese_input_mode,
                                                                        &mut step.key,
                                                                    );
                                                                    live_sync |= response2.changed();
                                                                    Self::render_variable_suggestions(ui, &response2, &mut step.key, &timer_names, language);
                                                                       let left_expr = step.if_variable_name.trim();
                                                                       if !left_expr.is_empty() {
                                                                           let left_val = crate::overlay::evaluate_math_expression(left_expr);
                                                                           ui.add_space(2.0);
                                                                           ui.label(
                                                                               RichText::new(format!("({})", left_val))
                                                                                   .size(10.0)
                                                                                   .color(Color32::from_rgb(0, 191, 255))
                                                                           ).on_hover_text(Self::tr_lang(language, "Evaluated left expression", "Gia tri bieu thuc ben trai"));
                                                                       }
                                                                   } else if step.if_condition_type == IfConditionType::PixelColor {
                                                                       ui.label("X:");
                                                                       let resp_x = ui.add(egui::DragValue::new(&mut step.x));
                                                                       live_sync |= resp_x.changed();
                                                                       ui.label("Y:");
                                                                       let resp_y = ui.add(egui::DragValue::new(&mut step.y));
                                                                       live_sync |= resp_y.changed();
                                                                       let resp_col = ui.add_sized(
                                                                           [64.0, 22.0],
                                                                           TextEdit::singleline(&mut step.if_target_color)
                                                                               .hint_text(RichText::new("#RRGGBB").color(hint_color).weak()),
                                                                       );
                                                                       live_sync |= resp_col.changed();
                                                                       ui.label(Self::tr_lang(language, "Tol:", "Sai sÃ¡Â»â€˜:"));
                                                                       let resp_tol = ui.add(egui::DragValue::new(&mut step.if_color_tolerance).range(0..=255));
                                                                       live_sync |= resp_tol.changed();
                                                                   } else if step.if_condition_type == IfConditionType::VisionMatch {
                                                                       let selected_id = step.if_vision_preset_id;
                                                                       let selected_label = selected_id
                                                                           .and_then(|id| {
                                                                               self.state.vision_presets.iter().find(|p| p.id == id).map(|p| p.name.clone())
                                                                           })
                                                                           .unwrap_or_else(|| Self::tr_lang(language, "Select preset", "Chá»n preset").to_owned());
                                                                       egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-if-vision-preset"))
                                                                           .width(146.0)
                                                                           .selected_text(selected_label)
                                                                           .show_ui(ui, |ui| {
                                                                               for vision_preset in &self.state.vision_presets {
                                                                                   if ui.selectable_label(selected_id == Some(vision_preset.id), &vision_preset.name).clicked() {
                                                                                       step.if_vision_preset_id = Some(vision_preset.id);
                                                                                       live_sync = true;
                                                                                   }
                                                                               }
                                                                           });
                                                                   } else if step.if_condition_type == IfConditionType::KeyHeld || step.if_condition_type == IfConditionType::KeyPressed {
                                                                       let resp_key = ui.add_sized(
                                                                           [80.0, 22.0],
                                                                           TextEdit::singleline(&mut step.if_key_held_name)
                                                                               .hint_text(RichText::new(Self::tr_lang(language, "Key", "PhÃ­m")).color(hint_color).weak()),
                                                                       );
                                                                       Self::apply_vietnamese_input_if_changed(
                                                                           &resp_key,
                                                                           self.state.vietnamese_input_enabled,
                                                                           self.state.vietnamese_input_mode,
                                                                           &mut step.if_key_held_name,
                                                                       );
                                                                       live_sync |= resp_key.changed();
                                                                   } else if step.if_condition_type == IfConditionType::MouseHeld {
                                                                       egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-if-mouse-button"))
                                                                           .width(90.0)
                                                                           .selected_text(&step.if_mouse_button)
                                                                           .show_ui(ui, |ui| {
                                                                               for btn in &["MouseLeft", "MouseRight", "MouseMiddle", "MouseX1", "MouseX2"] {
                                                                                   if ui.selectable_label(step.if_mouse_button == *btn, *btn).clicked() {
                                                                                       step.if_mouse_button = btn.to_string();
                                                                                       live_sync = true;
                                                                                   }
                                                                               }
                                                                           });
                                                                   } else if step.if_condition_type == IfConditionType::MouseScroll {
                                                                       egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-if-scroll-dir"))
                                                                           .width(70.0)
                                                                           .selected_text(&step.if_scroll_direction)
                                                                           .show_ui(ui, |ui| {
                                                                               for dir in &["Up", "Down"] {
                                                                                   let label = match *dir {
                                                                                       "Up" => Self::tr_lang(language, "Up", "LÃªn"),
                                                                                       _ => Self::tr_lang(language, "Down", "Xuá»‘ng"),
                                                                                   };
                                                                                   if ui.selectable_label(step.if_scroll_direction == *dir, label).clicked() {
                                                                                       step.if_scroll_direction = dir.to_string();
                                                                                       live_sync = true;
                                                                                   }
                                                                               }
                                                                           });
                                                                   } else if step.if_condition_type == IfConditionType::MousePosition {
                                                                       egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-if-mouse-axis"))
                                                                           .width(50.0)
                                                                           .selected_text(&step.if_mouse_axis)
                                                                           .show_ui(ui, |ui| {
                                                                               for axis in &["X", "Y"] {
                                                                                   if ui.selectable_label(step.if_mouse_axis == *axis, *axis).clicked() {
                                                                                       step.if_mouse_axis = axis.to_string();
                                                                                       live_sync = true;
                                                                                   }
                                                                               }
                                                                           });
                                                                       egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-if-mouse-pos-op"))
                                                                           .width(40.0)
                                                                           .selected_text(&step.if_operator)
                                                                           .show_ui(ui, |ui| {
                                                                               for op in &["==", ">", "<", ">=", "<=", "!="] {
                                                                                   if ui.selectable_label(step.if_operator == *op, *op).clicked() {
                                                                                       step.if_operator = op.to_string();
                                                                                       live_sync = true;
                                                                                   }
                                                                               }
                                                                           });
                                                                       let mut variable_layouter = |ui: &egui::Ui, string: &dyn TextBuffer, wrap_width: f32| {
                                                                           let job = Self::variable_highlight_job(ui, string.as_str(), wrap_width);
                                                                           ui.fonts_mut(|fonts| fonts.layout_job(job))
                                                                       };
                                                                       let response2 = ui.add_sized(
                                                                            [76.0, 22.0],
                                                                            TextEdit::singleline(&mut step.key)
                                                                                .layouter(&mut variable_layouter)
                                                                                .hint_text(RichText::new(Self::tr_lang(language, "value/expr", "giÃƒÂ¡ trÃ¡Â»â€¹/expr")).color(hint_color).weak()),
                                                                        );
                                                                        Self::apply_vietnamese_input_if_changed(
                                                                            &response2,
                                                                            self.state.vietnamese_input_enabled,
                                                                            self.state.vietnamese_input_mode,
                                                                            &mut step.key,
                                                                        );
                                                                        live_sync |= response2.changed();
                                                                        Self::render_variable_suggestions(ui, &response2, &mut step.key, &timer_names, language);
                                                                   } else if step.if_condition_type == IfConditionType::PresetRunning {
                                                                       let selected_id = step.if_running_preset_id;
                                                                       let selected_label = selected_id
                                                                           .and_then(|id| {
                                                                               all_presets.iter().find(|(pid, _)| *pid == id).map(|(_, name)| name.clone())
                                                                           })
                                                                           .unwrap_or_else(|| Self::tr_lang(language, "Select preset", "Chá»n preset").to_owned());
                                                                       egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-if-running-preset"))
                                                                           .width(120.0)
                                                                           .selected_text(selected_label)
                                                                           .show_ui(ui, |ui| {
                                                                               for (pid, pname) in &all_presets {
                                                                                   if ui.selectable_label(selected_id == Some(*pid), pname).clicked() {
                                                                                       step.if_running_preset_id = Some(*pid);
                                                                                       live_sync = true;
                                                                                   }
                                                                               }
                                                                           });
                                                                   } else if step.if_condition_type == IfConditionType::TimerRunning {
                                                                       let selected_id = step.timer_preset_id;
                                                                       let selected_label = selected_id
                                                                           .and_then(|id| {
                                                                               self.state.timer_presets.iter().find(|t| t.id == id).map(|t| t.name.clone())
                                                                           })
                                                                           .unwrap_or_else(|| Self::tr_lang(language, "Select timer", "Chá»n timer").to_owned());
                                                                       egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-if-timer-preset"))
                                                                           .width(120.0)
                                                                           .selected_text(selected_label)
                                                                           .show_ui(ui, |ui| {
                                                                               for timer in &self.state.timer_presets {
                                                                                   if ui.selectable_label(selected_id == Some(timer.id), &timer.name).clicked() {
                                                                                       step.timer_preset_id = Some(timer.id);
                                                                                       live_sync = true;
                                                                                   }
                                                                               }
                                                                           });
                                                                   }
                                                                     if ui.add_sized([24.0, 24.0], Button::new("+")).on_hover_text(Self::tr_lang(language, "Add condition", "Thêm điều kiện")).clicked() {
                                                                       step.extra_conditions.push(ExtraCondition::default());
                                                                       live_sync = true;
                                                                   }
                                                               });
                                                             let mut remove_extra_idx = None;
                                                             for (extra_idx, cond) in step.extra_conditions.iter_mut().enumerate() {
                                                                  ui.horizontal(|ui| {
                                                                          egui::ComboBox::from_id_salt((group.id, preset.id, extra_idx, "hold-stop-if-extra-join"))
                                                                              .width(56.0)
                                                                              .selected_text(if cond.join_operator.eq_ignore_ascii_case("OR") { Self::tr_lang(language, "OR", "HO?C") } else { Self::tr_lang(language, "AND", "VÀ") })
                                                                              .show_ui(ui, |ui| {
                                                                                 for op in &["AND", "OR"] {
                                                                                     let label = if *op == "AND" {
                                                                                         Self::tr_lang(language, "AND", "VÀ")
                                                                                     } else {
                                                                                         Self::tr_lang(language, "OR", "HO?C")
                                                                                     };
                                                                                     if ui.selectable_label(cond.join_operator.eq_ignore_ascii_case(op), label).clicked() {
                                                                                         cond.join_operator = op.to_string();
                                                                                         live_sync = true;
                                                                                     }
                                                                                 }
                                                                             });
                                                                         let cond_var_id = ui.id().with((extra_idx, "hold-stop-extra-if-var"));
                                                                         let response = Self::render_variable_text_edit(
                                                                             ui,
                                                                             &mut cond.variable_name,
                                                                             cond_var_id,
                                                                             76.0,
                                                                             140.0,
                                                                             22.0,
                                                                             22.0,
                                                                             Self::tr_lang(language, "value/expr", "giÃƒÂ¡ trÃ¡Â»â€¹/expr"),
                                                                             false,
                                                                         );
                                                                     Self::apply_vietnamese_input_if_changed(
                                                                         &response,
                                                                         self.state.vietnamese_input_enabled,
                                                                         self.state.vietnamese_input_mode,
                                                                         &mut cond.variable_name,
                                                                     );
                                                                     live_sync |= response.changed();
                                                                     egui::ComboBox::from_id_salt((group.id, preset.id, extra_idx, "hold-stop-extra-if-op"))
                                                                         .width(40.0)
                                                                         .selected_text(&cond.operator)
                                                                         .show_ui(ui, |ui| {
                                                                             for op in &["==", ">", "<", ">=", "<=", "!="] {
                                                                                 if ui.selectable_label(cond.operator == *op, *op).clicked() {
                                                                                     cond.operator = op.to_string();
                                                                                     live_sync = true;
                                                                                 }
                                                                             }
                                                                         });
                                                                      let cond_expr_id = ui.id().with((extra_idx, "hold-stop-extra-if-expr"));
                                                                      let response2 = Self::render_variable_text_edit(
                                                                          ui,
                                                                          &mut cond.expression,
                                                                          cond_expr_id,
                                                                          76.0,
                                                                          160.0,
                                                                          22.0,
                                                                          22.0,
                                                                          Self::tr_lang(language, "value/expr", "giá trị/expr"),
                                                                          false,
                                                                      );
                                                                     Self::apply_vietnamese_input_if_changed(
                                                                         &response2,
                                                                         self.state.vietnamese_input_enabled,
                                                                         self.state.vietnamese_input_mode,
                                                                         &mut cond.expression,
                                                                     );
                                                                     live_sync |= response2.changed();
                                                                     let left_expr = cond.variable_name.trim();
                                                                     if !left_expr.is_empty() {
                                                                         let left_val = crate::overlay::evaluate_math_expression(left_expr);
                                                                         ui.label(
                                                                             RichText::new(format!("({})", left_val))
                                                                                 .size(10.0)
                                                                                 .color(Color32::from_rgb(0, 191, 255))
                                                                         );
                                                                     }
                                                                         if ui.add_sized([24.0, 24.0], Button::new("-")).on_hover_text(Self::tr_lang(language, "Remove condition", "Xóa điều kiện")).clicked() {
                                                                         remove_extra_idx = Some(extra_idx);
                                                                     }
                                                                 });
                                                             }
                                                             if let Some(remove_idx) = remove_extra_idx {
                                                                 step.extra_conditions.remove(remove_idx);
                                                                 live_sync = true;
                                                             }
                                                         });
                                                     });
                                                                                                } else if step.action == MacroAction::SetVariable {
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.spacing_mut().interact_size.y = 22.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        ui.vertical(|ui| {
                                                            ui.horizontal(|ui| {
                                                                  let var_name_id = ui.id().with("hold-stop-set-var-name");
                                                                  let response = Self::render_variable_text_edit(
                                                                      ui,
                                                                      &mut step.if_variable_name,
                                                                      var_name_id,
                                                                      76.0,
                                                                      140.0,
                                                                      22.0,
                                                                      22.0,
                                                                      Self::tr_lang(language, "variable", "biến"),
                                                                      false,
                                                                  );
                                                                  Self::apply_vietnamese_input_if_changed(
                                                                      &response,
                                                                      self.state.vietnamese_input_enabled,
                                                                      self.state.vietnamese_input_mode,
                                                                      &mut step.if_variable_name,
                                                                  );
                                                                  live_sync |= response.changed();
                                                                  ui.label(" = ");
                                                                  let var_val_id = ui.id().with("hold-stop-set-var-val");
                                                                  let response2 = Self::render_variable_text_edit(
                                                                      ui,
                                                                      &mut step.key,
                                                                      var_val_id,
                                                                      76.0,
                                                                      180.0,
                                                                      22.0,
                                                                      22.0,
                                                                      Self::tr_lang(language, "value/expr", "giá trị"),
                                                                      false,
                                                                  );
                                                                  Self::apply_vietnamese_input_if_changed(
                                                                      &response2,
                                                                      self.state.vietnamese_input_enabled,
                                                                      self.state.vietnamese_input_mode,
                                                                      &mut step.key,
                                                                  );
                                                                  live_sync |= response2.changed();
                                                                Self::render_variable_suggestions_raw(ui, &response, &mut step.if_variable_name, &timer_names, language);
                                                                Self::render_variable_suggestions(ui, &response2, &mut step.key, &timer_names, language);
                                                                let var_name = step.if_variable_name.trim();
                                                                if !var_name.is_empty() {
                                                                    let current_val = crate::overlay::RUNTIME_VARIABLES.lock().get(var_name).copied();
                                                                    let val_str = current_val.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
                                                                    ui.add_space(2.0);
                                                                    ui.label(
                                                                        RichText::new(format!("({})", val_str))
                                                                            .size(10.0)
                                                                            .color(Color32::from_rgb(0, 191, 255))
                                                                    ).on_hover_text(Self::tr_lang(language, "Current runtime value", "Giá trị chạy hiện tại"));
                                                                }
                                                            });
                                                        });
                                                    });
                                                } else {
                                                    let id = ui.id().with("hold-stop-default-key");
                                                    let response = Self::render_expandable_text_edit(
                                                        ui,
                                                        &mut step.key,
                                                        id,
                                                        160.0,
                                                        240.0,
                                                        22.0,
                                                        22.0,
                                                        "...",
                                                        false,
                                                     );
                                                     live_sync |= response.changed();
                                                }
                                            } else {
                                                ui.add_sized([70.0, 22.0], egui::Label::new(""));
                                            }
                                            if Self::macro_action_uses_position(step.action) {
                                                live_sync |= ui
                                                    .add_sized([58.0, 22.0], DragValue::new(&mut step.x).range(-30000..=30000))
                                                    .changed();
                                                live_sync |= ui
                                                    .add_sized([58.0, 22.0], DragValue::new(&mut step.y).range(-30000..=30000))
                                                    .changed();
                                            } else if step.action == MacroAction::ShowHud {
                                                let mut temp_ms = if step.timed_override { step.duration_override_ms } else { 0 };
                                                let changed = ui.add_sized(
                                                    [98.0, 22.0],
                                                    DragValue::new(&mut temp_ms)
                                                        .range(0..=60_000)
                                                        .suffix(" ms"),
                                                ).on_hover_text(Self::tr_lang(
                                                    language,
                                                    "Display duration (0 ms = show until macro ends)",
                                                    "ThÃ¡Â»Âi gian hiÃ¡Â»Æ’n thÃ¡Â»â€¹ (0 ms = hiÃ¡Â»â€¡n Ã„â€˜Ã¡ÂºÂ¿n khi dÃ¡Â»Â«ng macro)",
                                                )).changed();
                                                if changed {
                                                    step.duration_override_ms = temp_ms;
                                                    step.timed_override = temp_ms > 0;
                                                    live_sync = true;
                                                }
                                            } else {
                                                ui.add_sized([24.0, 22.0], egui::Label::new(""));
                                                ui.add_sized([24.0, 22.0], egui::Label::new(""));
                                            }
                                            if action_supports_capture && !(step.action == MacroAction::StopIfKeyPressed && step.break_loop_by_variable) {
                                                let hold_stop_capture_target =
                                                    CaptureRequest::MacroPresetHoldStopInput(group.id, preset.id);
                                                let hold_stop_capture_active =
                                                    capture_target_snapshot.as_ref() == Some(&hold_stop_capture_target);
                                                let hold_stop_capture_width =
                                                    if hold_stop_capture_active { 92.0 } else { 28.0 };
                                                let hold_stop_capture_button = if hold_stop_capture_active {
                                                    Button::new(Self::capture_button_text(language, true))
                                                        .min_size(vec2(hold_stop_capture_width, 22.0))
                                                        .fill(Color32::from_rgb(88, 84, 44))
                                                } else {
                                                    Button::new(Self::material_icon_text(0xe312, 18.0))
                                                        .min_size(vec2(hold_stop_capture_width, 22.0))
                                                };
                                                if ui
                                                    .add_sized([hold_stop_capture_width, 22.0], hold_stop_capture_button)
                                                    .on_hover_text(Self::tr_lang(
                                                        language,
                                                        "Capture hold stop key",
                                                        "",
                                                    ))
                                                    .clicked()
                                                {
                                                    if hold_stop_capture_active {
                                                        cancel_active_capture = true;
                                                    } else {
                                                        next_capture_target = Some(hold_stop_capture_target);
                                                    }
                                                }
                                                // Dropdown right here for hold stop
                                                let hs_menu_response = ui.menu_button(Self::material_icon_text(0xe5d2, 14.0), |ui| {
                                                    ui.set_max_width(200.0);
                                                     ui.menu_button(Self::tr_lang(language, "Letters (A-Z)", "Chá»¯ cÃ¡i (A-Z)"), |ui| {
                                                         ui.set_max_width(120.0);
                                                         egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                                             for ch in b'A'..=b'Z' {
                                                                 let key_str = (ch as char).to_string();
                                                                 if ui.button(&key_str).clicked() {
                                                                     step.key = key_str;
                                                                     live_sync = true;
                                                                     ui.close_menu();
                                                                 }
                                                             }
                                                         });
                                                     });
                                                     ui.menu_button(Self::tr_lang(language, "Numbers & Symbols", "SÃ¡Â»â€˜ & KÃƒÂ­ tÃ¡Â»Â±"), |ui| {
                                                         ui.set_max_width(140.0);
                                                         egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                                             for num in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
                                                                 if ui.button(num).clicked() {
                                                                     step.key = num.to_string();
                                                                     live_sync = true;
                                                                     ui.close_menu();
                                                                 }
                                                             }
                                                             ui.separator();
                                                             for sym in [";", "=", ",", "-", ".", "/", "`", "[", "\\", "]", "'"] {
                                                                 if ui.button(sym).clicked() {
                                                                     step.key = sym.to_string();
                                                                     live_sync = true;
                                                                     ui.close_menu();
                                                                 }
                                                             }
                                                         });
                                                     });
                                                     ui.menu_button(Self::tr_lang(language, "Navigation", "Ã„ÂiÃ¡Â»Âu hÃ†Â°Ã¡Â»â€ºng & PhÃ­m táº¯t"), |ui| {
                                                         ui.set_max_width(160.0);
                                                         for key in ["Escape", "Enter", "Space", "Backspace", "Tab", "Insert", "Delete", "Home", "End", "PageUp", "PageDown", "Left", "Up", "Right", "Down", "PrintScreen", "Pause"] {
                                                             if ui.button(key).clicked() {
                                                                 step.key = key.to_string();
                                                                 live_sync = true;
                                                                 ui.close_menu();
                                                             }
                                                         }
                                                     });
                                                     ui.menu_button(Self::tr_lang(language, "Function (F1-F24)", "PhÃ­m chÃ¡Â»Â©c nÃ„Æ’ng"), |ui| {
                                                         ui.set_max_width(100.0);
                                                         egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                                             for num in 1..=24 {
                                                                 let key_str = format!("F{}", num);
                                                                 if ui.button(&key_str).clicked() {
                                                                     step.key = key_str;
                                                                     live_sync = true;
                                                                     ui.close_menu();
                                                                 }
                                                             }
                                                         });
                                                     });
                                                     ui.menu_button(Self::tr_lang(language, "Numpad", "BÃƒÂ n phÃ­m sÃ¡Â»â€˜ phÃ¡Â»Â¥"), |ui| {
                                                         ui.set_max_width(160.0);
                                                         for key in ["Numpad0", "Numpad1", "Numpad2", "Numpad3", "Numpad4", "Numpad5", "Numpad6", "Numpad7", "Numpad8", "Numpad9", "NumpadMultiply", "NumpadAdd", "NumpadSubtract", "NumpadDecimal", "NumpadDivide"] {
                                                             if ui.button(key).clicked() {
                                                                 step.key = key.to_string();
                                                                 live_sync = true;
                                                                 ui.close_menu();
                                                             }
                                                         }
                                                     });
                                                     ui.menu_button(Self::tr_lang(language, "Modifiers & Locks", "PhÃ­m khÃƒÂ³a & bá»• trÃ¡Â»Â£"), |ui| {
                                                         ui.set_max_width(150.0);
                                                         for key in ["Ctrl", "Alt", "Shift", "Win", "CapsLock", "NumLock", "ScrollLock", "Apps"] {
                                                             if ui.button(key).clicked() {
                                                                 step.key = key.to_string();
                                                                 live_sync = true;
                                                                 ui.close_menu();
                                                             }
                                                         }
                                                     });
                                                 });
                                                 hs_menu_response.response.on_hover_text(Self::tr_lang(
                                                     language,
                                                     "Manually select key",
                                                     "ChÃ¡Â»Ân phÃ­m thÃ¡Â»Â§ cÃƒÂ´ng"
                                                 ));
                                            } else {
                                                ui.add_sized([28.0, 22.0], egui::Label::new(""));
                                            }
                                            if ui.button(Self::tr_lang(language, "Clear", "Clear")).clicked() {
                                                clear_hold_stop_step = true;
                                            }
                                        });
                                        if clear_hold_stop_step {
                                            preset.hold_stop_step = MacroStep::default();
                                            live_sync = true;
                                        }
                                });
                                    }
                                    ui.scope(|ui| {
                            Frame::new()
                                .inner_margin(egui::Margin::symmetric(4, 2))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 2.0;
                                        let capture_target = CaptureRequest::MacroPresetRecordHotkey(group.id, preset.id);
                                        let has_rec_hotkey = preset.record_hotkey.is_some();
                                        let capture_active = self.capture_target.as_ref() == Some(&capture_target);
                                        let (rect, _) = ui.allocate_exact_size(egui::vec2(118.0, 20.0), egui::Sense::hover());
                                         let mut child_ui = ui.new_child(
                                             egui::UiBuilder::new()
                                                 .max_rect(rect)
                                                 .layout(egui::Layout::left_to_right(egui::Align::Center))
                                         );
                                         child_ui.spacing_mut().item_spacing.x = 2.0;
                                         if child_ui
                                             .add_sized([22.0, 20.0], Button::new(Self::material_icon_text(0xe145, 12.0)))
                                             .on_hover_text(Self::tr_lang(
                                                 language,
                                                 "Add step",
                                                 "ThÃªm mÃ¡Â»â„¢t bÆ°á»›c vÃƒÂ o Ã„â€˜Ã¡ÂºÂ§u preset nÃƒÂ y",
                                             ))
                                             .clicked()
                                         {
                                             preset.steps.insert(0, MacroStep::default());
                                             live_sync = true;
                                         }
                                         let is_recording_this = self.active_macro_record_preset_id == Some(preset.id);
                                         let record_icon = if is_recording_this { 0xe047 } else { 0xe061 }; // stop square or solid circle
                                         let mut dot_color = Color32::from_rgb(255, 60, 60);
                                         if is_recording_this {
                                             let ms = std::time::SystemTime::now()
                                                 .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                                 .unwrap_or_default()
                                                 .as_millis();
                                             if (ms / 500) % 2 == 0 {
                                                 dot_color = Color32::from_rgba_unmultiplied(255, 60, 60, 80);
                                             }
                                             child_ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                                         }
                                         let record_text = if is_recording_this {
                                             Self::tr_lang(language, "Stop", "")
                                         } else {
                                             Self::tr_lang(language, "Record", "")
                                         };
                                         let record_btn = Button::new(
                                             RichText::new(format!("{} {}", Self::material_icon_text(record_icon, 10.0).text(), record_text))
                                                 .color(dot_color)
                                                 .strong()
                                         );
                                         if child_ui.add_sized([70.0, 20.0], record_btn)
                                             .on_hover_text(Self::tr_lang(
                                                 language,
                                                 "Record your keyboard and mouse clicks globally to automatically generate macro steps",
                                                 "Ghi lÃ¡ÂºÂ¡i thao tÃƒÂ¡c phÃ­m vÃƒÂ  click chuá»™t toÃƒÂ n mÃƒÂ n hÃƒÂ¬nh Ä‘á»Æ’ tÃ¡Â»Â± Ä‘á»™ng tÃ¡ÂºÂ¡o bÆ°á»›c macro",
                                             ))
                                             .clicked()
                                         {
                                             let _ = self.overlay_tx.send(crate::overlay::OverlayCommand::ToggleMacroRecording(
                                                 group.id,
                                                 preset.id,
                                                 group.name.clone(),
                                             ));
                                         }
                                         // Keyboard Trigger Hotkey Capture UI
                                         let pulse = if capture_active {
                                             let capture_time = child_ui.ctx().input(|input| input.time) as f32;
                                             0.5 + 0.5 * (capture_time * 6.0).sin().abs()
                                         } else {
                                             0.0
                                         };
                                         let capture_fill = if capture_active {
                                             Color32::from_rgba_premultiplied(
                                                 (88.0 + pulse * 28.0) as u8,
                                                 (84.0 + pulse * 28.0) as u8,
                                                 (44.0 + pulse * 10.0) as u8,
                                                 255,
                                             )
                                         } else {
                                             child_ui.visuals().widgets.inactive.bg_fill
                                         };
                                          let kbd_btn_text = if capture_active {
                                              if let Some(pending) = self.capture_hotkey_combo_keys.as_ref() {
                                                  if !pending.is_empty() {
                                                      let preview = Self::hotkey_binding_from_combo_keys(pending.clone());
                                                      Self::format_binding_ui(language, Some(&preview))
                                                  } else {
                                                      Self::tr_lang(language, "capturing", "capturing").to_owned()
                                                  }
                                              } else {
                                                  Self::tr_lang(language, "capturing", "capturing").to_owned()
                                              }
                                          } else if let Some(binding) = &preset.record_hotkey {
                                              Self::format_binding_ui(language, Some(binding))
                                          } else {
                                              Self::material_icon_text(0xe312, 10.0).text().to_owned()
                                          };
                                          let text_color = if capture_active {
                                              Color32::WHITE
                                          } else if has_rec_hotkey {
                                              Color32::from_rgb(96, 232, 255)
                                          } else {
                                              child_ui.visuals().widgets.inactive.text_color()
                                          };
                                          let hover_text = if let Some(binding) = &preset.record_hotkey {
                                              let key_ui = Self::format_binding_ui(language, Some(binding));
                                              let fmt = Self::tr_lang(
                                                  language,
                                                  "Bound trigger key: {} (Click to clear)",
                                                  "PhÃ­m táº¯t Ã„â€˜ÃƒÂ£ gÃƒÂ¡n: {} (NhÃ¡ÂºÂ¥p Ä‘á»Æ’ xÃ³a)",
                                              );
                                              fmt.replace("{}", &key_ui)
                                          } else {
                                              Self::tr_lang(
                                                  language,
                                                  "Click to bind a keyboard key to start/stop macro recording dynamically",
                                                  "NhÃ¡ÂºÂ¥p Ä‘á»Æ’ gÃƒÂ¡n phÃ­m táº¯t báº¯t Ã„â€˜Ã¡ÂºÂ§u/dÃ¡Â»Â«ng ghi macro nhanh",
                                              ).to_string()
                                          };
                                          let clicked = child_ui.scope(|ui| {
                                              ui.spacing_mut().button_padding = egui::vec2(6.0, 0.0);
                                              let kbd_btn = Button::new(
                                                  RichText::new(kbd_btn_text)
                                                      .color(text_color)
                                                      .strong()
                                                      .size(10.0)
                                              )
                                              .fill(capture_fill)
                                              .min_size(egui::vec2(22.0, 20.0));
                                              ui.add_sized([22.0, 20.0], kbd_btn)
                                          }).inner.on_hover_text(hover_text).clicked();
                                          if clicked {
                                              if capture_active {
                                                  cancel_active_capture = true;
                                              } else if has_rec_hotkey {
                                                  preset.record_hotkey = None;
                                                  live_sync = true;
                                              } else {
                                                  next_capture_target = Some(capture_target.clone());
                                              }
                                          }
                                         let is_recording_this = self.active_macro_record_preset_id == Some(preset.id);
                                         let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, 20.0), egui::Sense::hover());
                                         let mut child_ui = ui.new_child(
                                             egui::UiBuilder::new()
                                                 .max_rect(rect)
                                                 .layout(egui::Layout::left_to_right(egui::Align::Center))
                                         );
                                         child_ui.spacing_mut().item_spacing.x = 2.0;
                                         if is_recording_this {
                                             let label_color = Color32::from_rgb(255, 96, 96);
                                             let is_even = (std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap_or_default().as_millis() / 500) % 2 == 0;
                                             let dot_color = if is_even { label_color } else { label_color.linear_multiply(0.3) };
                                             child_ui.add_sized([30.0, 20.0], egui::Label::new(
                                                 RichText::new("● REC")
                                                     .color(dot_color)
                                                     .size(9.0)
                                                     .strong()
                                             )).on_hover_text(Self::tr_lang(language, "Macro recording is active", ""));
                                         }
                                         ui.add_sized(
                                             [20.0, 18.0],
                                             egui::Label::new(RichText::new("#").strong())
                                                 .halign(egui::Align::Center),
                                         );
                                          let (rect, _) = ui.allocate_exact_size(egui::vec2(120.0, 18.0), egui::Sense::hover());
                                          let mut child_ui = ui.new_child(
                                              egui::UiBuilder::new()
                                                  .max_rect(rect)
                                                  .layout(egui::Layout::top_down(egui::Align::Center))
                                          );
                                          child_ui.horizontal(|ui| {
                                               ui.add_space(26.0);
                                               ui.label(RichText::new(Self::tr_lang(language, "Delay", "Delay")).strong());
                                           });
                                         ui.add_sized([148.0, 18.0], egui::Label::new(RichText::new(Self::tr_lang(language, "Action", "Action")).strong()));
                                         ui.add_sized([146.0, 18.0], egui::Label::new(""));
                                        let has_selected_steps = selected_steps_snapshot.iter().any(|(g_id, p_id, _)| *g_id == group.id && *p_id == preset.id);
                                         if has_selected_steps {
                                             let delete_btn = Button::new(Self::tr_lang(language, "Delete", "XÃ³a"))
                                                 .min_size(egui::vec2(64.0, 20.0));
                                             if ui
                                                 .add(delete_btn)
                                                 .on_hover_text(Self::tr_lang(language, "Delete selected steps", "XÃƒÂ³a cÃƒÂ¡c bÆ°á»›c Ã„â€˜ÃƒÂ£ chÃ¡Â»Ân"))
                                                 .clicked()
                                             {
                                                 delete_selected_steps = Some((group.id, preset.id));
                                             }
                                             let copy_btn = Button::new(Self::tr_lang(language, "Copy", "Copy"))
                                                 .min_size(egui::vec2(56.0, 20.0));
                                             if ui
                                                 .add(copy_btn)
                                                 .on_hover_text(Self::tr_lang(
                                                     language,
                                                     "Copy the selected steps in this preset.",
                                                     "Copy selected steps in this preset.",
                                                 ))
                                                 .clicked()
                                             {
                                                 copy_selected_steps = Some((group.id, preset.id));
                                             }
                                         }
                                        if has_rec_hotkey && !capture_active {
                                            let clear_btn = Button::new(RichText::new(Self::material_icon_text(0xe14c, 10.0).text()).color(Color32::LIGHT_RED));
                                            if ui.add_sized([20.0, 20.0], clear_btn)
                                                .on_hover_text(Self::tr_lang(language, "Clear hotkey", "XÃƒÂ³a phÃ­m táº¯t"))
                                                .clicked()
                                            {
                                                preset.record_hotkey = None;
                                                live_sync = true;
                                            }
                                        }
                                        });
                                    });
                            let steps_len = preset.steps.len();
                            let has_stop_vision = preset.steps.iter().any(|s| s.action == MacroAction::StopVision && s.enabled);
                            let drag_payload = egui::DragAndDrop::payload::<MacroStepDragPayload>(ui.ctx())
                                .filter(|payload| payload.group_id == group.id && payload.preset_id == preset.id);
                            let pointer_y = ui.ctx().pointer_interact_pos().map(|pointer| pointer.y);
                            let mut preview_drop_index = steps_len;
                            let mut preview_drawn = false;
                            let paint_drop_preview = |ui: &mut egui::Ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    vec2(ui.available_width(), 24.0),
                                    Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    rect.shrink2(vec2(4.0, 3.0)),
                                    5.0,
                                    Color32::from_rgba_premultiplied(124, 240, 164, 96),
                                );
                                ui.painter().rect_stroke(
                                    rect.shrink2(vec2(4.0, 3.0)),
                                    5.0,
                                    egui::Stroke::new(2.0, Color32::from_rgb(124, 240, 164)),
                                    egui::StrokeKind::Outside,
                                );
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "Drop here",
                                    egui::TextStyle::Body.resolve(ui.style()),
                                    Color32::from_rgb(22, 66, 34),
                                );
                            };
                            let mut step_rects = vec![Rect::ZERO; steps_len];
                            for step_index in 0..steps_len {
                                if drag_payload.is_some()
                                    && !preview_drawn
                                    && pointer_y.is_some_and(|pointer_y| {
                                        pointer_y <= ui.cursor().min.y + 12.0
                                    })
                                {
                                    preview_drop_index = step_index;
                                    preview_drawn = true;
                                    paint_drop_preview(ui);
                                }
                                let has_step_break_loop_warning = {
                                    let current_step = &preset.steps[step_index];
                                    current_step.action == MacroAction::StopIfKeyPressed
                                        && current_step.enabled
                                        && !{
                                            let mut depth = 0;
                                            let mut inside = false;
                                            for (idx, s) in preset.steps.iter().enumerate() {
                                                if idx == step_index {
                                                    if depth > 0 {
                                                        inside = true;
                                                    }
                                                    break;
                                                }
                                                if s.enabled {
                                                    if s.action == MacroAction::LoopStart {
                                                        depth += 1;
                                                    } else if s.action == MacroAction::LoopEnd {
                                                        if depth > 0 {
                                                            depth -= 1;
                                                        }
                                                    }
                                                }
                                            }
                                            inside
                                        }
                                };
                                let is_step_executing = crate::overlay::ACTIVE_MACRO_STEPS.lock()
                                    .get(&preset.id)
                                    .map(|set| set.contains(&step_index))
                                    .unwrap_or(false);
                                let step_ref = &preset.steps[step_index];
                                let is_vision_active = step_ref.action == MacroAction::StartVisionSearch && {
                                    crate::overlay::is_vision_following_active_by_spec(&step_ref.key)
                                };
                                let is_timer_active = step_ref.action == MacroAction::StartTimerPreset && {
                                    let t_id = step_ref.timer_preset_id.or_else(|| step_ref.key.trim().parse::<u32>().ok());
                                    crate::overlay::is_timer_preset_active(t_id)
                                };
                                let is_loop_end_active = step_ref.action == MacroAction::LoopEnd && {
                                    let mut matching_start_idx = None;
                                    let mut depth = 0usize;
                                    for idx in (0..=step_index).rev() {
                                        let s = &preset.steps[idx];
                                        match s.action {
                                            MacroAction::LoopEnd => depth += 1,
                                            MacroAction::LoopStart => {
                                                depth = depth.saturating_sub(1);
                                                if depth == 0 {
                                                    matching_start_idx = Some(idx);
                                                    break;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    if let Some(start_idx) = matching_start_idx {
                                        crate::overlay::ACTIVE_MACRO_STEPS.lock()
                                            .get(&preset.id)
                                            .map(|set| set.contains(&start_idx))
                                            .unwrap_or(false)
                                    } else {
                                        false
                                    }
                                };
                                let is_active = is_step_executing || is_vision_active || is_timer_active || is_loop_end_active;
                                let step = &mut preset.steps[step_index];
                                let hover_preview_source_id = ui.make_persistent_id((
                                    group.id,
                                    preset.id,
                                    step_index,
                                    "macro-step-hover-preview",
                                ));
                                let is_selected = selected_steps_snapshot
                                    .contains(&(group.id, preset.id, step_index));
                                let drag_indices = if is_selected {
                                    let mut indices = selected_steps_snapshot
                                        .iter()
                                        .filter_map(|(selected_group, selected_preset, selected_index)| {
                                            (*selected_group == group.id
                                                && *selected_preset == preset.id)
                                                .then_some(*selected_index)
                                        })
                                        .collect::<Vec<_>>();
                                    indices.sort_unstable();
                                    if indices.is_empty() {
                                        vec![step_index]
                                    } else {
                                        indices
                                    }
                                } else {
                                    vec![step_index]
                                };
                                let mut row_fill = if is_selected {
                                    if step.enabled {
                                        Color32::from_rgba_premultiplied(88, 148, 220, 130)
                                    } else {
                                        Color32::from_rgba_premultiplied(68, 118, 180, 130)
                                    }
                                } else {
                                    ui.visuals().faint_bg_color
                                };
                                if !step.enabled && !is_selected {
                                    row_fill = Color32::from_rgba_unmultiplied(62, 62, 62, 220);
                                }
                                let has_infinite_loop_warning = self.state.macro_infinite_loop_warning_enabled
                                    && preset.enabled
                                    && (
                                        (preset.trigger_mode == MacroTriggerMode::Press && !preset.stop_on_retrigger_immediate)
                                        || preset.trigger_mode == MacroTriggerMode::Release
                                    )
                                    && step.action == MacroAction::LoopStart
                                    && step.is_infinite_loop();
                                let has_step_vision_leak = preset.enabled
                                    && (preset.trigger_mode == MacroTriggerMode::Press || preset.trigger_mode == MacroTriggerMode::Release)
                                    && step.action == MacroAction::StartVisionSearch
                                    && step.enabled
                                    && !has_stop_vision;
                                if has_infinite_loop_warning || has_step_vision_leak {
                                    row_fill = Color32::from_rgba_unmultiplied(255, 90, 0, 25);
                                } else if has_step_break_loop_warning {
                                    row_fill = Color32::from_rgba_unmultiplied(255, 200, 0, 15);
                                }
                                if is_active {
                                    row_fill = Color32::from_rgba_unmultiplied(0, 255, 170, 35);
                                }
                                let drag_payload = MacroStepDragPayload {
                                    group_id: group.id,
                                    preset_id: preset.id,
                                    indices: drag_indices,
                                };
                                let border_stroke = if is_active {
                                    egui::Stroke::new(1.5, Color32::from_rgb(0, 255, 170))
                                } else {
                                    egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
                                };
                                let row_response = Frame::group(ui.style())
                                    .fill(row_fill)
                                    .stroke(border_stroke)
                                    .inner_margin(egui::Margin::symmetric(4, 2))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 2.0;
                                            let is_dark_theme = self.state.ui_theme == UiThemeMode::Dark;
                                            let hint_color = if is_dark_theme {
                                                Color32::from_rgba_unmultiplied(140, 140, 140, 150)
                                            } else {
                                                Color32::from_rgba_unmultiplied(100, 100, 100, 150)
                                            };
                                            let (rect, _) = ui.allocate_exact_size(egui::vec2(118.0, 20.0), egui::Sense::hover());
                                            let mut child_ui = ui.new_child(
                                                egui::UiBuilder::new()
                                                    .max_rect(rect)
                                                    .layout(egui::Layout::left_to_right(egui::Align::Center))
                                            );
                                            child_ui.spacing_mut().item_spacing.x = 2.0;
                                            if child_ui
                                                .add_sized([22.0, 20.0], Button::new(Self::material_icon_text(0xe145, 12.0)))
                                                .on_hover_text(Self::tr_lang(language, "Add a new step below this one", "ThÃªm mÃ¡Â»â„¢t bÆ°á»›c mÃ¡Â»â€ºi phÃƒÂ­a dÃ†Â°Ã¡Â»â€ºi"))
                                                .clicked()
                                            {
                                                insert_step_after = Some((preset.id, step_index));
                                            }
                                            let select_icon = if is_selected {
                                                Self::material_icon_text(0xe5ca, 12.0).color(Color32::from_rgb(96, 232, 255))
                                            } else {
                                                RichText::new("")
                                            };
                                            if child_ui
                                                .add_sized(
                                                    [22.0, 20.0],
                                                    Button::new(select_icon),
                                                )
                                                .on_hover_text(Self::tr_lang(language, "Select step", "ChÃ¡Â»Ân bÆ°á»›c nÃƒÂ y"))
                                                .clicked()
                                            {
                                                pending_step_selection = Some((
                                                    group.id,
                                                    preset.id,
                                                    step_index,
                                                    child_ui.input(|input| input.modifiers.ctrl),
                                                    child_ui.input(|input| input.modifiers.shift),
                                                ));
                                            }
                                            child_ui.scope(|ui| {
                                                ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
                                                ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                                                ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                                let hover_bg = if is_dark_theme {
                                                    Color32::from_rgba_unmultiplied(255, 255, 255, 20)
                                                } else {
                                                    Color32::from_rgba_unmultiplied(0, 0, 0, 15)
                                                };
                                                let hover_stroke = if is_dark_theme {
                                                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 40))
                                                } else {
                                                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 30))
                                                };
                                                let active_bg = if is_dark_theme {
                                                    Color32::from_rgba_unmultiplied(255, 255, 255, 35)
                                                } else {
                                                    Color32::from_rgba_unmultiplied(0, 0, 0, 25)
                                                };
                                                let active_stroke = if is_dark_theme {
                                                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 60))
                                                } else {
                                                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 45))
                                                };
                                                ui.visuals_mut().widgets.hovered.bg_fill = hover_bg;
                                                ui.visuals_mut().widgets.hovered.bg_stroke = hover_stroke;
                                                ui.visuals_mut().widgets.active.bg_fill = active_bg;
                                                ui.visuals_mut().widgets.active.bg_stroke = active_stroke;
                                                let (toggle_changed, new_enabled) = ui.scope(|ui| {
                                                    let icon = if step.enabled { 0xe5ca } else { 0xe835 };
                                                    let fill = if step.enabled {
                                                        Color32::from_rgba_premultiplied(72, 156, 116, 120)
                                                    } else {
                                                        ui.visuals().faint_bg_color
                                                    };
                                                    let stroke = if step.enabled {
                                                        Color32::from_rgb(126, 224, 182)
                                                    } else {
                                                        ui.visuals().widgets.noninteractive.bg_stroke.color
                                                    };
                                                    let resp = ui.add_sized(
                                                        [22.0, 20.0],
                                                        Button::new(Self::material_icon_text(icon, 12.0))
                                                            .fill(fill)
                                                            .stroke(egui::Stroke::new(1.0, stroke)),
                                                    ).on_hover_text(Self::tr_lang(language, "Toggle step enabled", "Bật/Tắt bước này"));
                                                    (resp.clicked(), !step.enabled)
                                                }).inner;
                                                if toggle_changed {
                                                    step.enabled = new_enabled;
                                                    live_sync = true;
                                                }
                                                if ui
                                                    .add_sized(
                                                        [22.0, 20.0],
                                                        Button::new(Self::material_icon_text(0xe872, 16.0)),
                                                    )
                                                    .on_hover_text(Self::tr_lang(
                                                        language,
                                                        "Remove this step",
                                                        "XÃƒÂ³a bÆ°á»›c nÃƒÂ y",
                                                    ))
                                                    .clicked()
                                                {
                                                    remove_step = Some((preset.id, step_index));
                                                }
                                                let drag_handle = ui
                                                    .add_sized(
                                                        [22.0, 20.0],
                                                        Button::new(Self::material_icon_text(0xe25d, 16.0))
                                                            .sense(Sense::drag()),
                                                    )
                                                    .on_hover_cursor(egui::CursorIcon::Grab);
                                                drag_handle.dnd_set_drag_payload(drag_payload.clone());
                                            });
                                            let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, 20.0), egui::Sense::hover());
                                            let mut child_ui = ui.new_child(
                                                egui::UiBuilder::new()
                                                    .max_rect(rect)
                                                    .layout(egui::Layout::left_to_right(egui::Align::Center))
                                            );
                                            child_ui.spacing_mut().item_spacing.x = 2.0;
                                            if has_infinite_loop_warning || has_step_vision_leak || has_step_break_loop_warning {
                                                let warn_color = if has_infinite_loop_warning || has_step_vision_leak {
                                                    Color32::from_rgb(255, 90, 0)
                                                } else {
                                                    Color32::from_rgb(255, 200, 0)
                                                };
                                                let response = child_ui.add_sized([20.0, 20.0], egui::Button::new(
                                                    Self::material_icon_text(0xe002, 16.0).color(warn_color)
                                                ).frame(false));
                                                if response.contains_pointer() {
                                                    egui::show_tooltip_at_pointer(child_ui.ctx(), child_ui.layer_id(), response.id.with("step-tip"), |ui| {
                                                        ui.horizontal(|ui| {
                                                            ui.label(Self::material_icon_text(0xe002, 14.0).color(warn_color));
                                                            ui.label(RichText::new(Self::tr_lang(language, "STEP WARNING", "CÃ¡ÂºÂ¢NH BÃƒÂO BÃ†Â¯Ã¡Â»Å¡C")).strong().color(warn_color));
                                                        });
                                                        if has_infinite_loop_warning {
                                                            ui.label(Self::tr_lang(
                                                                language,
                                                                "This step starts an infinite loop without an end point. The macro will run forever until you manually stop it.",
                                                                "BÃ†Â°Ã¡Â»â€ºc nÃƒÂ y khá»Ÿi Ã„â€˜Ã¡ÂºÂ§u mÃ¡Â»â„¢t vÃƒÂ²ng lÃ¡ÂºÂ·p vÃƒÂ´ tÃ¡ÂºÂ­n mÃƒÂ  khÃƒÂ´ng cÃƒÂ³ Ã„â€˜iÃ¡Â»Æ’m dÃ¡Â»Â«ng, macro sÃ¡ÂºÂ½ chÃ¡ÂºÂ¡y mÃ£i mÃ£i cho Ã„â€˜Ã¡ÂºÂ¿n khi bÃ¡ÂºÂ¡n chÃ¡Â»Â§ Ä‘á»™ng bÃ¡ÂºÂ¥m dÃ¡Â»Â«ng."
                                                            ));
                                                        }
                                                        if has_step_vision_leak {
                                                            ui.label(Self::tr_lang(
                                                                language,
                                                                "This step starts image search under Press/Release trigger, but there is no 'StopImageSearch' action in this macro! This could lead to a persistent background CPU thread. Add a 'StopImageSearch' step or change trigger to 'Hold'.",
                                                                "BÃ†Â°Ã¡Â»â€ºc nÃƒÂ y báº¯t Ã„â€˜Ã¡ÂºÂ§u tÃƒÂ¬m áº£nhh (chÃ¡ÂºÂ¿ Ä‘á»™ Nháº¥n/Tháº£) nhÃ†Â°ng macro khÃƒÂ´ng cÃƒÂ³ bÆ°á»›c dÃ¡Â»Â«ng tÃƒÂ¬m áº£nhh! Ã„ÂiÃ¡Â»Âu nÃƒÂ y cÃƒÂ³ thá»ƒ gÃƒÂ¢y chÃ¡ÂºÂ¡y ngÃ¡ÂºÂ§m hao CPU. HÃƒÂ£y thÃƒÂªm bÆ°á»›c dÃ¡Â»Â«ng tÃƒÂ¬m áº£nhh hoáº·c Ã„â€˜á»•i trigger sang Giá»¯ (Hold)."
                                                            ));
                                                        }
                                                        if has_step_break_loop_warning {
                                                            ui.label(Self::tr_lang(
                                                                language,
                                                                "This step breaks a loop, but it is not placed inside any Loop Start / Loop End block! It will have no effect.",
                                                                "BÃ†Â°Ã¡Â»â€ºc nÃƒÂ y thoÃƒÂ¡t vÃƒÂ²ng lÃ¡ÂºÂ·p, nhÃ†Â°ng nÃƒÂ³ hiÃ¡Â»â€¡n khÃƒÂ´ng náº±m trong cÃ¡ÂºÂ·p khÃ¡Â»â€˜i LÃ¡ÂºÂ·p (Loop Start) / Háº¿t lÃ¡ÂºÂ·p (Loop End) nÃƒÂ o! NÃƒÂ³ sÃ¡ÂºÂ½ khÃƒÂ´ng cÃƒÂ³ tÃƒÂ¡c dá»¥ng."
                                                            ));
                                                        }
                                                    });
                                                }
                                            }
                                            if is_active {
                                                child_ui.add_sized([8.0, 20.0], egui::Label::new(
                                                    RichText::new("\u{25CF} ")
                                                        .color(Color32::from_rgb(0, 255, 170))
                                                        .size(12.0)
                                                ))
                                                .on_hover_text(Self::tr_lang(language, "Step is running/active", "BÃ†Â°Ã¡Â»â€ºc nÃƒÂ y Ä‘ang chÃ¡ÂºÂ¡y/hoáº¡t Ä‘á»™ng"));
                                            } else {
                                                child_ui.add_sized([8.0, 20.0], egui::Label::new(""));
                                            }
                                            let step_num_text = format!("{}", step_index + 1);
                                            let label_width = if has_infinite_loop_warning || has_step_vision_leak || has_step_break_loop_warning { 20.0 } else { 20.0 };
                                            ui.add_sized(
                                                [label_width, 18.0],
                                                egui::Label::new(
                                                    if is_active {
                                                        RichText::new(step_num_text)
                                                            .monospace()
                                                            .color(Color32::from_rgb(0, 255, 170))
                                                            .strong()
                                                    } else {
                                                        RichText::new(step_num_text).monospace()
                                                    },
                                                )
                                                .halign(egui::Align::Center),
                                            );
                                            if step.delay_expr.is_empty() && step.delay_ms > 0 {
                                                step.delay_expr = step.delay_ms.to_string();
                                            }
                                            let (rect, _) = ui.allocate_exact_size(egui::vec2(120.0, 18.0), egui::Sense::hover());
                                            let mut child_ui = ui.new_child(
                                                egui::UiBuilder::new()
                                                    .max_rect(rect)
                                                    .layout(egui::Layout::left_to_right(egui::Align::Center))
                                            );
                                            child_ui.spacing_mut().item_spacing.x = 0.0;
                                            child_ui.spacing_mut().button_padding = egui::vec2(2.0, 0.0);
                                            child_ui.spacing_mut().interact_size.y = 18.0;
                                            child_ui.spacing_mut().interact_size.x = 36.0;
                                            let left_rounding = egui::CornerRadius { nw: 4, ne: 0, se: 0, sw: 4 };
                                            let right_rounding = egui::CornerRadius { nw: 0, ne: 4, se: 4, sw: 0 };
                                            child_ui.visuals_mut().widgets.inactive.corner_radius = left_rounding;
                                            child_ui.visuals_mut().widgets.hovered.corner_radius = left_rounding;
                                            child_ui.visuals_mut().widgets.active.corner_radius = left_rounding;
                                            child_ui.visuals_mut().widgets.open.corner_radius = left_rounding;
                                            child_ui.visuals_mut().widgets.noninteractive.corner_radius = left_rounding;
                                            let edit_id = child_ui.make_persistent_id((group.id, preset.id, step_index, "delay-edit-state"));
                                            let is_editing = child_ui.memory(|mem| mem.data.get_temp::<bool>(edit_id).unwrap_or(false));
                                            if is_editing {
                                                let delay_id = child_ui.id().with((step_index, "delay"));
                                                let response = Self::render_variable_text_edit(
                                                    &mut child_ui,
                                                    &mut step.delay_expr,
                                                    delay_id,
                                                    78.0,
                                                    130.0,
                                                    18.0,
                                                    18.0,
                                                    "0",
                                                    false,
                                                );
                                                Self::apply_vietnamese_input_if_changed(
                                                    &response,
                                                    self.state.vietnamese_input_enabled,
                                                    self.state.vietnamese_input_mode,
                                                    &mut step.delay_expr,
                                                );
                                                let just_started_id = edit_id.with("just_started");
                                                let just_started = child_ui.memory(|mem| mem.data.get_temp::<bool>(just_started_id).unwrap_or(false));
                                                if just_started {
                                                    response.request_focus();
                                                    child_ui.memory_mut(|mem| mem.data.insert_temp(just_started_id, false));
                                                }
                                                if response.changed() {
                                                    if let Ok(val) = step.delay_expr.trim().parse::<u64>() {
                                                        step.delay_ms = val;
                                                    } else {
                                                        step.delay_ms = 0;
                                                    }
                                                    live_sync = true;
                                                }
                                                if response.lost_focus() || child_ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                                    child_ui.memory_mut(|mem| mem.data.insert_temp(edit_id, false));
                                                }
                                            } else {
                                                let display_text = if step.delay_expr.is_empty() {
                                                    "0".to_string()
                                                } else {
                                                    step.delay_expr.clone()
                                                };
                                                let response = child_ui.add_sized(
                                                    [78.0, 18.0],
                                                    egui::Button::new(display_text)
                                                        .wrap_mode(egui::TextWrapMode::Truncate)
                                                        .sense(egui::Sense::click_and_drag()),
                                                )
                                                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                                                let has_dragged_id = edit_id.with("has-dragged");
                                                if response.dragged() {
                                                    child_ui.memory_mut(|mem| mem.data.insert_temp(has_dragged_id, true));
                                                    let accum_id = edit_id.with("drag-accum");
                                                    let mut accum = child_ui.memory(|mem| mem.data.get_temp::<f32>(accum_id).unwrap_or(0.0));
                                                    accum += response.drag_delta().x;
                                                    let step_size = if child_ui.input(|i| i.modifiers.shift) {
                                                        10.0
                                                    } else {
                                                        1.0
                                                    };
                                                    let pixels_per_unit = 2.0;
                                                    let delta_units = (accum / pixels_per_unit).trunc();
                                                    if delta_units != 0.0 {
                                                        accum -= delta_units * pixels_per_unit;
                                                        let delta_int = (delta_units * step_size).round() as i32;
                                                        if delta_int != 0 {
                                                            step.delay_expr = Self::adjust_expression_by_delta(&step.delay_expr, delta_int);
                                                            if let Ok(val) = step.delay_expr.trim().parse::<u64>() {
                                                                step.delay_ms = val;
                                                            } else {
                                                                step.delay_ms = 0;
                                                            }
                                                            live_sync = true;
                                                        }
                                                    }
                                                    child_ui.memory_mut(|mem| mem.data.insert_temp(accum_id, accum));
                                                } else {
                                                    if !child_ui.input(|i| i.pointer.any_down()) {
                                                        let accum_id = edit_id.with("drag-accum");
                                                        child_ui.memory_mut(|mem| {
                                                            mem.data.insert_temp(has_dragged_id, false);
                                                            mem.data.insert_temp(accum_id, 0.0);
                                                        });
                                                    }
                                                }
                                                if response.clicked() {
                                                    let has_dragged = child_ui.memory(|mem| mem.data.get_temp::<bool>(has_dragged_id).unwrap_or(false));
                                                    if !has_dragged {
                                                        child_ui.memory_mut(|mem| {
                                                            mem.data.insert_temp(edit_id, true);
                                                            mem.data.insert_temp(edit_id.with("just_started"), true);
                                                        });
                                                    }
                                                }
                                            }
                                            child_ui.visuals_mut().widgets.inactive.corner_radius = right_rounding;
                                            child_ui.visuals_mut().widgets.hovered.corner_radius = right_rounding;
                                            child_ui.visuals_mut().widgets.active.corner_radius = right_rounding;
                                            child_ui.visuals_mut().widgets.open.corner_radius = right_rounding;
                                            child_ui.visuals_mut().widgets.noninteractive.corner_radius = right_rounding;
                                            let unit_text = if step.wait_time_unit.is_empty() { "ms" } else { &step.wait_time_unit };
                                            let popup_rounding = right_rounding;
                                            let popup_style_modifier = egui::style::StyleModifier::new(move |style| {
                                                style.visuals.widgets.noninteractive.corner_radius = popup_rounding;
                                            });
                                            egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "delay-unit"))
                                                .width(42.0)
                                                .selected_text(unit_text)
                                                .popup_style(popup_style_modifier)
                                                .show_ui(&mut child_ui, |ui| {
                                                    for unit in &["ms", "s", "m", "h"] {
                                                        let label = *unit;
                                                        let val = if label == "ms" { "" } else { label };
                                                        if ui.selectable_label(step.wait_time_unit == val, label).clicked() {
                                                            step.wait_time_unit = val.to_string();
                                                            live_sync = true;
                                                        }
                                                    }
                                                });
                                            let action_combo = egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "action"))
                                                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                                                .width(148.0)
                                                .selected_text(format!(
                                                    "{} {}",
                                                    Self::macro_action_icon(step.action),
                                                    Self::macro_action_selected_label(step.action, language)
                                                ))
                                                .show_ui(ui, |ui| {
                                                    live_sync |= ui.checkbox(&mut step.toggle_enabled_on_run, Self::tr_lang(
                                                        language,
                                                        "🔄 Toggle self enabled on run",
                                                        ""
                                                    )).changed();
                                                    ui.separator();
                                                    let action_hover_id = ui.make_persistent_id((
                                                        group.id,
                                                        preset.id,
                                                        step_index,
                                                        "action-hover",
                                                    ));
                                                    ui.ctx().data_mut(|data| {
                                                        data.insert_temp(action_hover_id, false);
                                                    });
                                                    egui::Grid::new((group.id, preset.id, step_index, "action-grid"))
                                                        .num_columns(8)
                                                        .spacing([6.0, 6.0])
                                                        .show(ui, |ui| {
                                                        for (index, action) in [
                                                                MacroAction::KeyPress,
                                                                MacroAction::KeyDown,
                                                                MacroAction::KeyUp,
                                                                MacroAction::TypeText,
                                                                MacroAction::ApplyWindowPreset,
                                                                MacroAction::FocusWindowPreset,
                                                                MacroAction::TriggerMacroPreset,
                                                                MacroAction::TriggerCommandPreset,
                                                                MacroAction::EnableCrosshairProfile,
                                                                MacroAction::DisableCrosshair,
                                                                MacroAction::EnablePinPreset,
                                                                MacroAction::DisablePin,
                                                                MacroAction::PlaySoundPreset,
                                                                MacroAction::PlayVideoPreset,
                                                                MacroAction::ApplyMouseSensitivityPreset,
                                                                MacroAction::LoopStart,
                                                                MacroAction::LoopEnd,
                                                                MacroAction::StopIfKeyPressed,
                                                            MacroAction::ShowHud,
                                                                MacroAction::HideHud,
                                                                MacroAction::LockKeys,
                                                                MacroAction::UnlockKeys,
                                                                 MacroAction::EnableMacroPreset,
                                                                 MacroAction::DisableMacroPreset,
                                                                 MacroAction::EnableStep,
                                                                 MacroAction::DisableStep,
                                                                 MacroAction::SetVariable,
                                                            ]
                                                            .into_iter()
                                                            .enumerate()
                                                            {
                                                                Self::render_macro_action_option(
                                                                    ui,
                                                                    language,
                                                                    &mut step.action,
                                                                    action,
                                                                    &mut live_sync,
                                                                    action_hover_id,
                                                                    false,
                                                                );
                                                                if (index + 1) % 8 == 0 {
                                                                    ui.end_row();
                                                                }
                                                            }
                                                            Self::render_mouse_action_group_option(
                                                                ui,
                                                                language,
                                                                (group.id, preset.id, step_index, "mouse-group"),
                                                                &mut step.action,
                                                                &mut live_sync,
                                                                action_hover_id,
                                                            );
                                                            Self::render_image_search_action_group_option(
                                                                ui,
                                                                language,
                                                                (group.id, preset.id, step_index, "image-search-group"),
                                                                &mut step.action,
                                                                &mut live_sync,
                                                                action_hover_id,
                                                            );
                                                            Self::render_timer_action_group_option(
                                                                ui,
                                                                language,
                                                                (group.id, preset.id, step_index, "timer-group"),
                                                                &mut step.action,
                                                                &mut live_sync,
                                                                action_hover_id,
                                                            );
                                                            Self::render_if_action_group_option(
                                                                ui,
                                                                language,
                                                                (group.id, preset.id, step_index, "if-group"),
                                                                &mut step.action,
                                                                &mut live_sync,
                                                                action_hover_id,
                                                            );
                                                        });
                                                });
                                            Self::show_instant_hover_tooltip(
                                                ui,
                                                &action_combo.response,
                                                Self::macro_action_tooltip(step.action, language),
                                            );
                                            let action_uses_key = Self::macro_action_uses_key(step.action);
                                            let action_supports_capture =
                                                Self::macro_action_supports_capture(step.action);
                                            if action_uses_key {
                                                 if step.action == MacroAction::ApplyWindowPreset {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .window_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| Self::tr_lang(language, "Select window", "ChÃ¡Â»Ân cá»­a sá»•").to_owned());
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "window-preset-step"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.window_presets {
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(preset_option.id),
                                                                        &preset_option.name,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::FocusWindowPreset {
                                                    let selected_label = if step.key.trim().is_empty() {
                                                        Self::tr_lang(language, "Select window", "Chọn cửa sổ").to_owned()
                                                    } else {
                                                        Self::simplify_window_title(&step.key)
                                                    };
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "focus-window-preset-step"))
                                                        .width(160.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            ui.strong(Self::tr_lang(language, "Open Windows", "Cửa sổ đang mở"));
                                                            if self.open_windows.is_empty() {
                                                                ui.weak(Self::tr_lang(language, "No open windows found", "Không tìm thấy cửa sổ nào"));
                                                            } else {
                                                                for win in self.open_windows.iter().take(30) {
                                                                    let label = Self::simplify_window_title(win);
                                                                    if ui
                                                                        .selectable_label(step.key == *win, label)
                                                                        .on_hover_text(win)
                                                                        .clicked()
                                                                    {
                                                                        step.key = win.clone();
                                                                        live_sync = true;
                                                                    }
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::TriggerMacroPreset {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            group_preset_options
                                                                .iter()
                                                                .find(|(preset_id, _)| *preset_id == id)
                                                                .map(|(_, label)| label.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            Self::tr_lang(language, "Select macro", "Chá»n macro").to_owned()
                                                        });
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "trigger-macro-preset-step"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for (preset_option_id, preset_option_label) in &group_preset_options {
                                                                if *preset_option_id == preset.id {
                                                                    continue;
                                                                }
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(*preset_option_id),
                                                                        preset_option_label,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option_id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                        ui.add_space(4.0);
                                                        let cb_text = Self::tr_lang(language, "Wait for completion", "Đợi chạy xong");
                                                        if ui.checkbox(&mut step.wait_for_completion, cb_text).changed() {
                                                            live_sync = true;
                                                        }
                                                } else if step.action == MacroAction::TriggerCommandPreset {
                                                    let selected_id = step
                                                        .key
                                                        .trim()
                                                        .parse::<u32>()
                                                        .ok()
                                                        .or_else(|| {
                                                            self.state
                                                                .command_presets
                                                                .iter()
                                                                .find(|preset| preset.name.trim().eq_ignore_ascii_case(step.key.trim()))
                                                                .map(|preset| preset.id)
                                                        });
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .command_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            if step.key.trim().is_empty() {
                                                                Self::tr_lang(language, "Select command", "ChÃ¡Â»Ân cÃƒÂ¢u lÃ¡Â»â€¡nh")
                                                                .to_owned()
                                                            } else {
                                                                step.key.clone()
                                                            }
                                                        });
                                                    let custom_preset_combo = egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "trigger-custom-preset-step"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.command_presets {
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(preset_option.id),
                                                                        &preset_option.name,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                    let is_generating = self.command_ai_job.as_ref()
                                                         .map(|job| job.preset_id == 999999)
                                                         .unwrap_or(false)
                                                         && self.command_ai_step_target.as_ref()
                                                             .map(|target| target.0 == group.id && target.1 == preset.id && target.2 == Some(step_index))
                                                             .unwrap_or(false);
                                                     let (custom_draft_changed, custom_save_request, custom_save_and_open_ai_request, open_ai_preset_id) = Self::render_custom_preset_step_draft_popup(
                                                          ui,
                                                          &custom_preset_combo.response,
                                                          &custom_preset_combo.response,
                                                          step,
                                                          (group.id, preset.id, step_index),
                                                          Some(step_index),
                                                          language,
                                                          &command_presets_snapshot,
                                                          is_generating,
                                                     );
                                                     live_sync |= custom_draft_changed;
                                                     if let Some((save_step_index, name, command, use_powershell)) = custom_save_request {
                                                         pending_custom_preset_save = Some((
                                                             group.id,
                                                             preset.id,
                                                             save_step_index,
                                                             name,
                                                             command,
                                                             use_powershell,
                                                         ));
                                                     }
                                                     if let Some((save_step_index, name, command, use_powershell, is_ad_hoc)) = custom_save_and_open_ai_request {
                                                         pending_custom_preset_save_and_open_ai = Some((
                                                             group.id,
                                                             preset.id,
                                                             save_step_index,
                                                             name,
                                                             command,
                                                             use_powershell,
                                                             is_ad_hoc,
                                                         ));
                                                     }
                                                     if let Some(preset_id) = open_ai_preset_id {
                                                         pending_open_ai_preset_id = Some(preset_id);
                                                     }
                                                } else if matches!(
                                                    step.action,
                                                    MacroAction::EnableMacroPreset
                                                        | MacroAction::DisableMacroPreset
                                                ) {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            group_preset_options
                                                                .iter()
                                                                .find(|(preset_id, _)| *preset_id == id)
                                                                .map(|(_, label)| label.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            Self::tr_lang(language, "Select macro", "Chá»n macro").to_owned()
                                                        });
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "macro-enable-preset-step"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for (preset_option_id, preset_option_label) in &group_preset_options {
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(*preset_option_id),
                                                                        preset_option_label,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option_id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if matches!(step.action, MacroAction::EnableStep | MacroAction::DisableStep) {
                                                    // Parse `preset_id|1,2,3` or `1,2,3` for legacy fallback
                                                    let (selected_preset_id, mut selected_steps) = {
                                                        let parts: Vec<&str> = step.key.split('|').collect();
                                                        if parts.len() == 2 {
                                                            let p_id = parts[0].trim().parse::<u32>().ok();
                                                            let s_list = parts[1].split(',').filter_map(|s| s.trim().parse::<u32>().ok()).collect::<Vec<u32>>();
                                                            (p_id, s_list)
                                                        } else {
                                                            let s_list = step.key.split(',').filter_map(|s| s.trim().parse::<u32>().ok()).collect::<Vec<u32>>();
                                                            (None, s_list)
                                                        }
                                                    };
                                                    let current_preset_id = selected_preset_id.unwrap_or(preset.id);
                                                    ui.horizontal(|ui| {
                                                        ui.spacing_mut().interact_size.y = 18.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        let preset_label = group_preset_options.iter()
                                                            .find(|(id, _)| *id == current_preset_id)
                                                            .map(|(_, label)| label.clone())
                                                            .unwrap_or_else(|| Self::tr_lang(language, "Select preset", "Chá»n preset").to_owned());
                                                        egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "step-preset-select"))
                                                            .width(146.0)
                                                            .selected_text(preset_label)
                                                            .show_ui(ui, |ui| {
                                                                for (preset_option_id, preset_option_label) in &group_preset_options {
                                                                    if ui.selectable_label(current_preset_id == *preset_option_id, preset_option_label).clicked() {
                                                                        if current_preset_id != *preset_option_id {
                                                                            step.key = format!("{}|", preset_option_id);
                                                                            live_sync = true;
                                                                        }
                                                                    }
                                                                }
                                                            });
                                                        let target_step_count = group_preset_step_counts.iter()
                                                            .find(|(id, _)| *id == current_preset_id)
                                                            .map(|(_, count)| *count)
                                                            .unwrap_or(0);
                                                        let original_len = selected_steps.len();
                                                        selected_steps.retain(|&x| x <= target_step_count);
                                                        if selected_steps.len() != original_len {
                                                            let steps_str = selected_steps.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(",");
                                                            step.key = format!("{}|{}", current_preset_id, steps_str);
                                                            live_sync = true;
                                                        }
                                                        let steps_label = if selected_steps.is_empty() {
                                                            Self::tr_lang(language, "Select steps", "Chá»n steps").to_owned()
                                                        } else {
                                                            selected_steps.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")
                                                        };
                                                        egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "step-multi-select"))
                                                            .width(100.0)
                                                            .selected_text(steps_label)
                                                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                                                            .show_ui(ui, |ui| {
                                                                for i in 1..=target_step_count {
                                                                    let mut is_selected = selected_steps.contains(&i);
                                                                    if ui.checkbox(&mut is_selected, format!("Step {}", i)).changed() {
                                                                        if is_selected {
                                                                            selected_steps.push(i);
                                                                        } else {
                                                                            selected_steps.retain(|x| *x != i);
                                                                        }
                                                                        selected_steps.sort_unstable();
                                                                        let steps_str = selected_steps.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(",");
                                                                        step.key = format!("{}|{}", current_preset_id, steps_str);
                                                                        live_sync = true;
                                                                    }
                                                                }
                                                            });
                                                    });
                                                } else if matches!(step.action, MacroAction::StartTimerPreset | MacroAction::PauseTimerPreset | MacroAction::StopTimerPreset) {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state.timer_presets.iter()
                                                                .find(|p| p.id == id)
                                                                .map(|p| p.name.clone())
                                                        })
                                                        .unwrap_or_else(|| Self::tr_lang(language, "Select timer", "Chá»n háº¹n giá»").to_owned());
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "step-timer-preset-select"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for timer in &self.state.timer_presets {
                                                                if ui.selectable_label(selected_id == Some(timer.id), &timer.name).clicked() {
                                                                    step.key = timer.id.to_string();
                                                                    step.timer_preset_id = Some(timer.id);
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::EnableCrosshairProfile {
                                                    let selected_label = if step.key.trim().is_empty() {
                                                        Self::tr_lang(language, "Select crosshair", "Chá»n tÃ¢m").to_owned()
                                                    } else {
                                                        step.key.clone()
                                                    };
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "crosshair-profile-step"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for profile in &self.state.profiles {
                                                                if ui
                                                                    .selectable_label(step.key == profile.name, &profile.name)
                                                                    .clicked()
                                                                {
                                                                    step.key = profile.name.clone();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::EnablePinPreset {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .pin_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| Self::tr_lang(language, "Select pin", "Chá»n ghim").to_owned());
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "pin-preset-step"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.pin_presets {
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(preset_option.id),
                                                                        &preset_option.name,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::PlayMousePathPreset {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .mouse_path_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| Self::tr_lang(language, "Select path", "ChÃ¡Â»Ân Ã„â€˜Ã†Â°Ã¡Â»Âng chuá»™t").to_owned());
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "mouse-path-preset-step"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.mouse_path_presets {
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(preset_option.id),
                                                                        &preset_option.name,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if matches!(
                                                    step.action,
                                                    MacroAction::StartVisionSearch
                                                         | MacroAction::ScanVisionOnce
                                                         | MacroAction::TriggerVisionMove
                                                         | MacroAction::StopVision
                                                ) {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            image_search_preset_options
                                                                .iter()
                                                                .find(|(preset_id, _)| *preset_id == id)
                                                                .map(|(_, label)| label.clone())
                                                        })
                                                        .unwrap_or_else(|| "Select image".to_owned());
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "image-search-preset-step"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for (preset_option_id, preset_option_label) in &image_search_preset_options {
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(*preset_option_id),
                                                                        preset_option_label,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option_id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                     let is_pixel = selected_id.and_then(|id| {
                                                         self.state.vision_presets.iter().find(|p| p.id == id)
                                                     }).map(|p| p.is_pixel_counter).unwrap_or(false);
                                                     if step.action == MacroAction::ScanVisionOnce && is_pixel {
                                                         ui.add_space(4.0);
                                                         ui.horizontal(|ui| {
                                                             let response = ui.add_sized(
                                                                 [100.0, 22.0],
                                                                 TextEdit::singleline(&mut step.if_variable_name)
                                                                     .hint_text(RichText::new(Self::tr_lang(language, "set variable", "gÃ¡n biáº¿n")).color(hint_color).weak()),
                                                             );
                                                             Self::apply_vietnamese_input_if_changed(
                                                                 &response,
                                                                 self.state.vietnamese_input_enabled,
                                                                 self.state.vietnamese_input_mode,
                                                                 &mut step.if_variable_name,
                                                             );
                                                             live_sync |= response.changed();
                                                             Self::render_variable_suggestions_raw(ui, &response, &mut step.if_variable_name, &timer_names, language);
                                                         });
                                                     }
                                                    if step.action == MacroAction::TriggerVisionMove {
                                                        ui.add_space(4.0);
                                                        ui.horizontal(|ui| {
                                                            live_sync |= ui
                                                                .checkbox(
                                                                    &mut step.vision_move_cursor_on_match,
                                                                    Self::tr_lang(language, "Move", "Move"),
                                                                )
                                                                .on_hover_text(Self::tr_lang(
                                                                    language,
                                                                    "Move the cursor to the matched image before continuing.",
                                                                    "Di chuyÃ¡Â»Æ’n chuá»™t tÃ¡Â»â€ºi áº£nhh tÃƒÂ¬m thÃ¡ÂºÂ¥y rÃ¡Â»â€œi mÃ¡Â»â€ºi tiÃ¡ÂºÂ¿p tÃ¡Â»Â¥c.",
                                                                ))
                                                                .changed();
                                                            live_sync |= ui
                                                                .checkbox(
                                                                    &mut step.vision_wait_until_found,
                                                                    Self::tr_lang(language, "Wait", "Wait"),
                                                                )
                                                                .on_hover_text(Self::tr_lang(
                                                                    language,
                                                                    "Keep scanning until the image is found.",
                                                                    "TiÃ¡ÂºÂ¿p tÃ¡Â»Â¥c dÃƒÂ² cho tÃ¡Â»â€ºi khi thÃ¡ÂºÂ¥y áº£nhh.",
                                                                ))
                                                                .changed();
                                                            let mut trigger_macro_enabled = step.vision_trigger_macro_enabled;
                                                            if ui
                                                                .checkbox(
                                                                    &mut trigger_macro_enabled,
                                                                    Self::tr_lang(language, "Macro", "Macro"),
                                                                )
                                                                .on_hover_text(Self::tr_lang(
                                                                    language,
                                                                    "Trigger another macro preset from the same macro group.",
                                                                    "KÃƒÂ­ch hoáº¡t mÃ¡Â»â„¢t preset macro khÃ¡c trong cÃƒÂ¹ng group.",
                                                                ))
                                                                .changed()
                                                            {
                                                                step.vision_trigger_macro_enabled = trigger_macro_enabled;
                                                                if trigger_macro_enabled {
                                                                    if step
                                                                        .vision_trigger_macro_preset_id
                                                                        .is_none()
                                                                    {
                                                                        step.vision_trigger_macro_preset_id = group_preset_options
                                                                            .iter()
                                                                            .find(|(preset_option_id, _)| *preset_option_id != preset.id)
                                                                            .map(|(preset_option_id, _)| *preset_option_id);
                                                                    }
                                                                }
                                                                live_sync = true;
                                                            }
                                                            if step.vision_trigger_macro_enabled {
                                                                let selected_id = step.vision_trigger_macro_preset_id;
                                                                let selected_label = group_preset_options
                                                                    .iter()
                                                                    .find(|(preset_option_id, _)| Some(*preset_option_id) == selected_id)
                                                                    .map(|(_, label)| label.clone())
                                                                    .unwrap_or_else(|| "Select macro".to_owned());
                                                                egui::ComboBox::from_id_salt((
                                                                    group.id,
                                                                    preset.id,
                                                                    step_index,
                                                                    "image-search-trigger-macro-preset",
                                                                    ))
                                                                .width(146.0)
                                                                .selected_text(selected_label)
                                                                .show_ui(ui, |ui| {
                                                                    for (preset_option_id, preset_option_label) in &group_preset_options {
                                                                        if *preset_option_id == preset.id {
                                                                            continue;
                                                                        }
                                                                        if ui
                                                                            .selectable_label(
                                                                                selected_id == Some(*preset_option_id),
                                                                                preset_option_label,
                                                                            )
                                                                            .clicked()
                                                                        {
                                                                            step.vision_trigger_macro_preset_id =
                                                                                Some(*preset_option_id);
                                                                            live_sync = true;
                                                                }
                                                            }
                                                        });
                                                    }
                                                        });
                                                    }
                                                } else if step.action == MacroAction::EnableZoomPreset {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .zoom_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| "Select zoom".to_owned());
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "zoom-preset-step"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.zoom_presets {
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(preset_option.id),
                                                                        &preset_option.name,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::PlaySoundPreset {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .audio_settings
                                                                .presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| Self::tr_lang(language, "Select sound", "Chá»n Ã¢m thanh").to_owned());
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "sound-preset-step"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.audio_settings.presets {
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(preset_option.id),
                                                                        &preset_option.name,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::PlayVideoPreset {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .audio_settings
                                                                .video_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| Self::tr_lang(language, "Select video", "Chọn video").to_owned());
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "video-preset-step"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.audio_settings.video_presets {
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(preset_option.id),
                                                                        &preset_option.name,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                } else if step.action == MacroAction::ApplyMouseSensitivityPreset {
                                                    live_sync |= ui.checkbox(&mut step.manual_mouse_sensitivity, Self::tr_lang(language, "Manual", "Nháº­p tay")).changed();
                                                    if step.manual_mouse_sensitivity {
                                                        ui.vertical(|ui| {
                                                            let response = ui.add_sized(
                                                                [96.0, 18.0],
                                                                TextEdit::singleline(&mut step.key)
                                                                    .hint_text(RichText::new(Self::tr_lang(language, "value/expr", "giÃƒÂ¡ trÃ¡Â»â€¹")).color(hint_color).weak()),
                                                            );
                                                            Self::apply_vietnamese_input_if_changed(
                                                                &response,
                                                                self.state.vietnamese_input_enabled,
                                                                self.state.vietnamese_input_mode,
                                                                &mut step.key,
                                                            );
                                                            live_sync |= response.changed();
                                                            let interpolated = crate::overlay::interpolate_variables(&step.key);
                                                            let evaluated = crate::overlay::evaluate_math_expression(&interpolated);
                                                            let clamped = evaluated.clamp(1, 20);
                                                            let tooltip_text = match language {
                                                                UiLanguage::Vietnamese => format!("Káº¿t quáº£: {} (giÃ¡Â»â€ºi hÃ¡ÂºÂ¡n: {} trong 1..20)", evaluated, clamped),
                                                                _ => format!("Evaluated: {} (clamped to: {} within 1..20)", evaluated, clamped),
                                                            };
                                                            let response = response.on_hover_text(tooltip_text);
                                                            Self::render_variable_suggestions(ui, &response, &mut step.key, &timer_names, language);
                                                        });
                                                    } else {
                                                        let selected_id = step.key.trim().parse::<u32>().ok();
                                                        let selected_label = selected_id
                                                            .and_then(|id| {
                                                                self.state
                                                                    .mouse_sensitivity_presets
                                                                    .iter()
                                                                    .find(|preset| preset.id == id)
                                                                    .map(|preset| preset.name.clone())
                                                            })
                                                            .unwrap_or_else(|| {
                                                                Self::tr_lang(language, "Select sens", "ChÃ¡Â»Ân Ä‘á»™ nhÃ¡ÂºÂ¡y")
                                                                .to_owned()
                                                            });
                                                        egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "mouse-sensitivity-preset-step"))
                                                            .width(96.0)
                                                            .selected_text(selected_label)
                                                            .show_ui(ui, |ui| {
                                                                for preset_option in &self.state.mouse_sensitivity_presets {
                                                                    if ui
                                                                        .selectable_label(
                                                                            selected_id == Some(preset_option.id),
                                                                            &preset_option.name,
                                                                        )
                                                                        .clicked()
                                                                    {
                                                                        step.key = preset_option.id.to_string();
                                                                        live_sync = true;
                                                                    }
                                                                }
                                                            });
                                                    }
                                                } else if step.action == MacroAction::UnlockKeys {
                                                    let key_id = ui.id().with((step_index, "unlockkeys-key"));
                                                    let response = Self::render_expandable_text_edit(
                                                        ui,
                                                        &mut step.key,
                                                        key_id,
                                                        146.0,
                                                        260.0,
                                                        18.0,
                                                        18.0,
                                                        &Self::tr_lang(language, "A,S,W,D", "A,S,W,D"),
                                                        false,
                                                    );
                                                    Self::apply_vietnamese_input_if_changed(
                                                        &response,
                                                        self.state.vietnamese_input_enabled,
                                                        self.state.vietnamese_input_mode,
                                                        &mut step.key,
                                                     );
                                                     live_sync |= response.changed();
                                                } else if step.action == MacroAction::LockKeys {
                                                    let key_id = ui.id().with((step_index, "lockkeys-key"));
                                                    let response = Self::render_expandable_text_edit(
                                                        ui,
                                                        &mut step.key,
                                                        key_id,
                                                        146.0,
                                                        260.0,
                                                        18.0,
                                                        18.0,
                                                        &Self::tr_lang(language, "A,S,W,D", "A,S,W,D"),
                                                        false,
                                                    );
                                                    Self::apply_vietnamese_input_if_changed(
                                                        &response,
                                                        self.state.vietnamese_input_enabled,
                                                        self.state.vietnamese_input_mode,
                                                        &mut step.key,
                                                     );
                                                     live_sync |= response.changed();
                                                    ui.add_space(4.0);
                                                    let unlock_resp = ui.checkbox(&mut step.unlock_on_exit, Self::tr_lang(language, "Unlock when macro ends", ""));
                                                    if unlock_resp.changed() {
                                                        live_sync = true;
                                                    }
                                                    if !step.unlock_on_exit {
                                                        let warn_color = Color32::from_rgb(255, 90, 0);
                                                        let response = ui.add(egui::Label::new(Self::material_icon_text(0xe002, 14.0).color(warn_color)).sense(egui::Sense::hover()));
                                                        if response.contains_pointer() {
                                                            egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), response.id.with("lockkeys-compact-warning-tip"), |ui| {
                                                                ui.horizontal(|ui| {
                                                                    ui.label(Self::material_icon_text(0xe002, 14.0).color(warn_color));
                                                                    ui.label(RichText::new(Self::tr_lang(language, "STEP WARNING", "CẢNH BÁO BƯỚC")).strong().color(warn_color));
                                                                });
                                                                ui.label(Self::tr_lang(
                                                                    language,
                                                                    "Warning: Keeping keys locked after the macro ends can make your keyboard unresponsive until manually unlocked!",
                                                                    ""
                                                                ));
                                                            });
                                                        }
                                                    }
                                                 } else if step.action == MacroAction::LoopStart {
                                                     let mut infinite = Self::loop_is_infinite(step);
                                                     if ui
                                                         .checkbox(
                                                             &mut infinite,
                                                             RichText::new(Self::tr_lang(
                                                                 language,
                                                                 "Infinite",
                                                                 "Infinite",
                                                             ))
                                                             .color(Color32::WHITE),
                                                         )
                                                         .changed()
                                                     {
                                                         step.key = if infinite {
                                                             "infinite".to_owned()
                                                         } else {
                                                             "1".to_owned()
                                                         };
                                                         live_sync = true;
                                                      }
                                                      if !infinite {
                                                          ui.vertical(|ui| {
                                                              let key_id = ui.id().with((step_index, "loop-count"));
                                                              let response = Self::render_variable_text_edit(
                                                                  ui,
                                                                  &mut step.key,
                                                                  key_id,
                                                                  80.0,
                                                                  180.0,
                                                                  18.0,
                                                                  18.0,
                                                                  &Self::tr_lang(language, "Loop count", "Số lần lặp"),
                                                                  false,
                                                              );
                                                              Self::apply_vietnamese_input_if_changed(
                                                                  &response,
                                                                  self.state.vietnamese_input_enabled,
                                                                  self.state.vietnamese_input_mode,
                                                                  &mut step.key,
                                                              );
                                                              live_sync |= response.changed();
                                                              Self::render_variable_suggestions(ui, &response, &mut step.key, &timer_names, language);
                                                          });
                                                      }
                                                } else if step.action == MacroAction::StopIfKeyPressed {
                                                     ui.scope(|ui| {
                                                         ui.spacing_mut().item_spacing.x = 4.0;
                                                         ui.spacing_mut().interact_size.y = 18.0;
                                                         ui.spacing_mut().button_padding.y = 0.0;
                                                         ui.allocate_ui_with_layout(
                                                             vec2(ui.available_width(), 18.0),
                                                             egui::Layout::top_down(egui::Align::Min),
                                                             |ui| {
                                                             ui.horizontal(|ui| {
                                                                 let current_mode = step.get_break_loop_mode().to_string();
                                                                 let mode_label = match current_mode.as_str() {
                                                                     "VarCompare" => Self::tr_lang(language, "Var compare", "So sánh biến"),
                                                                     "StopKey" => Self::tr_lang(language, "Stop key", "Nút đã nhấn"),
                                                                     _ => Self::tr_lang(language, "Break Loop", "Ngắt lặp"),
                                                                 };
                                                                 egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "stop-loop-break-mode"))
                                                                     .width(100.0)
                                                                     .selected_text(mode_label)
                                                                     .show_ui(ui, |ui| {
                                                                         if ui.selectable_label(current_mode == "Immediate", Self::tr_lang(language, "Break Loop", "Ngắt lặp")).clicked() {
                                                                             step.break_loop_mode = "Immediate".to_string();
                                                                             step.break_loop_by_variable = false;
                                                                             live_sync = true;
                                                                         }
                                                                         if ui.selectable_label(current_mode == "VarCompare", Self::tr_lang(language, "Var compare", "So sánh biến")).clicked() {
                                                                             step.break_loop_mode = "VarCompare".to_string();
                                                                             step.break_loop_by_variable = true;
                                                                             live_sync = true;
                                                                         }
                                                                         if ui.selectable_label(current_mode == "StopKey", Self::tr_lang(language, "Stop key", "Nút đã nhấn")).clicked() {
                                                                             step.break_loop_mode = "StopKey".to_string();
                                                                             step.break_loop_by_variable = false;
                                                                             live_sync = true;
                                                                         }
                                                                     });

                                                                 let mode = step.get_break_loop_mode();
                                                                 if mode == "VarCompare" {
                                                                     let var_name_id = ui.id().with((step_index, "loop-break-var-name"));
                                                                     let response = Self::render_variable_text_edit(
                                                                         ui,
                                                                         &mut step.if_variable_name,
                                                                         var_name_id,
                                                                         64.0,
                                                                         140.0,
                                                                         18.0,
                                                                         18.0,
                                                                         Self::tr_lang(language, "variable", "biến"),
                                                                         false,
                                                                     );
                                                                     Self::apply_vietnamese_input_if_changed(
                                                                         &response,
                                                                         self.state.vietnamese_input_enabled,
                                                                         self.state.vietnamese_input_mode,
                                                                         &mut step.if_variable_name,
                                                                     );
                                                                     live_sync |= response.changed();
                                                                     Self::render_variable_suggestions(ui, &response, &mut step.if_variable_name, &timer_names, language);
                                                                     egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "stop-loop-if-op"))
                                                                         .width(40.0)
                                                                         .selected_text(&step.if_operator)
                                                                         .show_ui(ui, |ui| {
                                                                             for op in &["==", ">", "<", ">=", "<=", "!="] {
                                                                                 if ui.selectable_label(step.if_operator == *op, *op).clicked() {
                                                                                     step.if_operator = op.to_string();
                                                                                     live_sync = true;
                                                                                 }
                                                                             }
                                                                         });
                                                                     let var_val_id = ui.id().with((step_index, "loop-break-var-val"));
                                                                     let response2 = Self::render_variable_text_edit(
                                                                         ui,
                                                                         &mut step.key,
                                                                         var_val_id,
                                                                         76.0,
                                                                         180.0,
                                                                         18.0,
                                                                         18.0,
                                                                         Self::tr_lang(language, "value/expr", "giá trị"),
                                                                         false,
                                                                     );
                                                                     Self::apply_vietnamese_input_if_changed(
                                                                         &response2,
                                                                         self.state.vietnamese_input_enabled,
                                                                         self.state.vietnamese_input_mode,
                                                                         &mut step.key,
                                                                     );
                                                                     live_sync |= response2.changed();
                                                                     Self::render_variable_suggestions(ui, &response2, &mut step.key, &timer_names, language);
                                                                     let var_name = step.if_variable_name.trim();
                                                                     if !var_name.is_empty() {
                                                                         let current_val = crate::overlay::RUNTIME_VARIABLES.lock().get(var_name).copied();
                                                                         let val_str = current_val.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
                                                                         ui.add_space(2.0);
                                                                         ui.label(
                                                                             RichText::new(format!("({})", val_str))
                                                                                 .size(10.0)
                                                                                 .color(Color32::from_rgb(0, 191, 255))
                                                                         ).on_hover_text(Self::tr_lang(language, "Current runtime value", "Giá trị chạy hiện tại"));
                                                                     }
                                                                     if ui.add_sized([24.0, 24.0], Button::new("+")).on_hover_text(Self::tr_lang(language, "Add condition", "Thêm điều kiện")).clicked() {
                                                                         step.extra_conditions.push(ExtraCondition::default());
                                                                         live_sync = true;
                                                                     }
                                                                 } else if mode == "StopKey" {
                                                                     let response = ui.add_sized(
                                                                         [146.0, 18.0],
                                                                         TextEdit::singleline(&mut step.key)
                                                                             .hint_text(RichText::new(Self::tr_lang(language, "Stop key", "Stop key")).color(hint_color).italics()),
                                                                     );
                                                                     Self::apply_vietnamese_input_if_changed(
                                                                         &response,
                                                                         self.state.vietnamese_input_enabled,
                                                                         self.state.vietnamese_input_mode,
                                                                         &mut step.key,
                                                                     );
                                                                     live_sync |= response.changed();
                                                                     Self::render_variable_suggestions(ui, &response, &mut step.key, &timer_names, language);
                                                                 } else {
                                                                     ui.label(RichText::new(Self::tr_lang(language, "No input", "Không có đầu vào")).weak().italics());
                                                                 }
                                                             });
                                                             if step.get_break_loop_mode() == "VarCompare" {
                                                                 let mut remove_extra_idx = None;
                                                                 for (extra_idx, cond) in step.extra_conditions.iter_mut().enumerate() {
                                                                      ui.horizontal(|ui| {
                                                                          ui.add_space(100.0);
                                                                                                                                                  egui::ComboBox::from_id_salt((group.id, preset.id, extra_idx, "stop-loop-extra-join"))
                                                                              .width(56.0)
                                                                              .selected_text(if cond.join_operator.eq_ignore_ascii_case("OR") { Self::tr_lang(language, "OR", "HO?C") } else { Self::tr_lang(language, "AND", "VÀ") })
                                                                              .show_ui(ui, |ui| {
                                                                                 for op in &["AND", "OR"] {
                                                                                     let label = if *op == "AND" {
                                                                                         Self::tr_lang(language, "AND", "VÀ")
                                                                                     } else {
                                                                                         Self::tr_lang(language, "OR", "HO?C")
                                                                                     };
                                                                                     if ui.selectable_label(cond.join_operator.eq_ignore_ascii_case(op), label).clicked() {
                                                                                         cond.join_operator = op.to_string();
                                                                                         live_sync = true;
                                                                                     }
                                                                                 }
                                                                             });
                                                                        let cond_var_id = ui.id().with((step_index, extra_idx, "extra-stop-var"));
                                                                        let response = Self::render_variable_text_edit(
                                                                            ui,
                                                                            &mut cond.variable_name,
                                                                            cond_var_id,
                                                                            64.0,
                                                                            140.0,
                                                                            18.0,
                                                                            18.0,
                                                                            Self::tr_lang(language, "variable", "biáº¿n"),
                                                                            false,
                                                                        );
                                                                         Self::apply_vietnamese_input_if_changed(
                                                                             &response,
                                                                             self.state.vietnamese_input_enabled,
                                                                             self.state.vietnamese_input_mode,
                                                                             &mut cond.variable_name,
                                                                         );
                                                                         live_sync |= response.changed();
                                                                         egui::ComboBox::from_id_salt((group.id, preset.id, step_index, extra_idx, "extra-stop-op"))
                                                                             .width(40.0)
                                                                             .selected_text(&cond.operator)
                                                                             .show_ui(ui, |ui| {
                                                                                 for op in &["==", ">", "<", ">=", "<=", "!="] {
                                                                                     if ui.selectable_label(cond.operator == *op, *op).clicked() {
                                                                                         cond.operator = op.to_string();
                                                                                         live_sync = true;
                                                                                     }
                                                                                 }
                                                                             });
                                                                          let cond_expr_id = ui.id().with((step_index, extra_idx, "extra-stop-expr"));
                                                                          let response2 = Self::render_variable_text_edit(
                                                                              ui,
                                                                              &mut cond.expression,
                                                                              cond_expr_id,
                                                                              64.0,
                                                                              160.0,
                                                                              18.0,
                                                                              18.0,
                                                                              Self::tr_lang(language, "value/expr", "giá trị/expr"),
                                                                              false,
                                                                          );
                                                                         Self::apply_vietnamese_input_if_changed(
                                                                             &response2,
                                                                             self.state.vietnamese_input_enabled,
                                                                             self.state.vietnamese_input_mode,
                                                                             &mut cond.expression,
                                                                         );
                                                                         live_sync |= response2.changed();
                                                                         let var_name = cond.variable_name.trim();
                                                                         if !var_name.is_empty() {
                                                                             let current_val = crate::overlay::RUNTIME_VARIABLES.lock().get(var_name).copied();
                                                                             let val_str = current_val.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
                                                                             ui.label(
                                                                                 RichText::new(format!("({})", val_str))
                                                                                     .size(10.0)
                                                                                     .color(Color32::from_rgb(0, 191, 255))
                                                                             );
                                                                         }
                                                                         if ui.add_sized([24.0, 24.0], Button::new("-")).on_hover_text(Self::tr_lang(language, "Remove condition", "Xóa điều kiện")).clicked() {
                                                                             remove_extra_idx = Some(extra_idx);
                                                                         }
                                                                     });
                                                                 }
                                                                 if let Some(remove_idx) = remove_extra_idx {
                                                                     step.extra_conditions.remove(remove_idx);
                                                                     live_sync = true;
                                                                 }
                                                             }
                                                         });
                                                     });
                                                } else if step.action == MacroAction::ShowHud {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .hud_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            if step.key.trim().is_empty() {
                                                                Self::tr_lang(
                                                                    language,
                                                                    "Select HUD",
                                                                    "Chá»n HUD",
                                                                )
                                                                .to_owned()
                                                            } else {
                                                                match language {
                                                                    UiLanguage::Vietnamese => format!("CÃ…Â©: {}", step.key),
                                                                    _ => format!("Legacy: {}", step.key),
                                                                }
                                                            }
                                                        });
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.spacing_mut().interact_size.y = 18.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "toolbox-preset-step"))
                                                            .width(146.0)
                                                            .selected_text(selected_label)
                                                            .show_ui(ui, |ui| {
                                                                for toolbox_preset in &self.state.hud_presets {
                                                                    if ui
                                                                        .selectable_label(
                                                                            selected_id == Some(toolbox_preset.id),
                                                                            &toolbox_preset.name,
                                                                        )
                                                                        .clicked()
                                                                    {
                                                                        step.key = toolbox_preset.id.to_string();
                                                                        live_sync = true;
                                                                    }
                                                                }
                                                            });
                                                        let text_id = ui.id().with((step_index, "showhud-text-override"));
                                                        let response = Self::render_variable_text_edit(
                                                            ui,
                                                            &mut step.text_override,
                                                            text_id,
                                                            122.0,
                                                            240.0,
                                                            18.0,
                                                            18.0,
                                                            &Self::tr_lang(language, "Text override", "Ghi đè văn bản"),
                                                            false,
                                                        );
                                                         Self::apply_vietnamese_input_if_changed(
                                                             &response,
                                                             self.state.vietnamese_input_enabled,
                                                             self.state.vietnamese_input_mode,
                                                             &mut step.text_override,
                                                         );
                                                         live_sync |= response.changed();
                                                         Self::render_variable_suggestions(
                                                             ui,
                                                             &response,
                                                             &mut step.text_override,
                                                             &timer_names,
                                                             language,
                                                         );
                                                     });
                                                } else if step.action == MacroAction::TypeText {
                                                     ui.vertical(|ui| {
                                                         let response = Self::render_variable_text_edit(
                                                             ui,
                                                             &mut step.key,
                                                             ui.id().with((step_index, "type-text-key")),
                                                             146.0,
                                                             260.0,
                                                             18.0,
                                                             36.0,
                                                             Self::tr_lang(language, "Text to type", "Văn bảnh cần gõ"),
                                                             true,
                                                         );
                                                         Self::apply_vietnamese_input_if_changed(
                                                             &response,
                                                             self.state.vietnamese_input_enabled,
                                                             self.state.vietnamese_input_mode,
                                                             &mut step.key,
                                                         );
                                                         live_sync |= response.changed();
                                                         Self::render_variable_suggestions(ui, &response, &mut step.key, &timer_names, language);
                                                     });
                                                } else if step.action == MacroAction::DisableCrosshair {
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.spacing_mut().interact_size.y = 18.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        ui.horizontal(|ui| {
                                                            let response = ui.checkbox(&mut step.lock_mouse_left, Self::tr_lang(language, "All", ""));
                                                            live_sync |= response.changed();
                                                            if !step.lock_mouse_left {
                                                                let selected_label = if step.key.trim().is_empty() {
                                                                    Self::tr_lang(language, "Select profile", "Chá»n profile").to_owned()
                                                                } else {
                                                                    step.key.clone()
                                                                };
                                                                egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "main-disable-crosshair"))
                                                                    .width(96.0)
                                                                    .selected_text(selected_label)
                                                                    .show_ui(ui, |ui| {
                                                                        for profile in &self.state.profiles {
                                                                            if ui
                                                                                .selectable_label(step.key == profile.name, &profile.name)
                                                                                .clicked()
                                                                            {
                                                                                step.key = profile.name.clone();
                                                                                live_sync = true;
                                                                            }
                                                                        }
                                                                    });
                                                            }
                                                        });
                                                    });
                                                } else if step.action == MacroAction::DisablePin {
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.spacing_mut().interact_size.y = 18.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        ui.horizontal(|ui| {
                                                            let response = ui.checkbox(&mut step.lock_mouse_left, Self::tr_lang(language, "All", ""));
                                                            live_sync |= response.changed();
                                                            if !step.lock_mouse_left {
                                                                let selected_id = step.key.trim().parse::<u32>().ok();
                                                                let selected_label = selected_id
                                                                    .and_then(|id| {
                                                                        self.state
                                                                            .pin_presets
                                                                            .iter()
                                                                            .find(|p| p.id == id)
                                                                            .map(|p| p.name.clone())
                                                                    })
                                                                    .unwrap_or_else(|| {
                                                                        Self::tr_lang(language, "Select pin", "Chá»n preset ghim").to_owned()
                                                                    });
                                                                egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "main-disable-pin"))
                                                                    .width(96.0)
                                                                    .selected_text(selected_label)
                                                                    .show_ui(ui, |ui| {
                                                                        for preset_option in &self.state.pin_presets {
                                                                            if ui
                                                                                .selectable_label(selected_id == Some(preset_option.id), &preset_option.name)
                                                                                .clicked()
                                                                            {
                                                                                step.key = preset_option.id.to_string();
                                                                                live_sync = true;
                                                                            }
                                                                        }
                                                                    });
                                                            }
                                                        });
                                                    });
                                                } else if matches!(step.action, MacroAction::DisableZoom | MacroAction::Else | MacroAction::IfEnd | MacroAction::HideHud | MacroAction::UnlockMouse) {
                                                    ui.add_sized(
                                                        [146.0, 18.0],
                                                        egui::Label::new(Self::tr_lang(language, "No input", "No input")),
                                                    );
                                                } else if step.action == MacroAction::LockMouse {
                                                    ui.horizontal(|ui| {
                                                        let unlock_resp = ui.checkbox(&mut step.unlock_on_exit, Self::tr_lang(language, "Unlock when macro ends", ""));
                                                        if unlock_resp.changed() {
                                                            live_sync = true;
                                                        }
                                                        if !step.unlock_on_exit {
                                                        let warn_color = Color32::from_rgb(255, 90, 0);
                                                        let response = ui.add(egui::Label::new(Self::material_icon_text(0xe002, 14.0).color(warn_color)).sense(egui::Sense::hover()));
                                                        if response.contains_pointer() {
                                                            egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), response.id.with("lockmouse-compact-warning-tip"), |ui| {
                                                                ui.horizontal(|ui| {
                                                                    ui.label(Self::material_icon_text(0xe002, 14.0).color(warn_color));
                                                                    ui.label(RichText::new(Self::tr_lang(language, "STEP WARNING", "CẢNH BÁO BƯỚC")).strong().color(warn_color));
                                                                });
                                                                ui.label(Self::tr_lang(
                                                                    language,
                                                                    "Warning: Keeping mouse locked after the macro ends can make your mouse unresponsive until manually unlocked!",
                                                                    ""
                                                                ));
                                                            });
                                                        }
                                                    }
                                                    });
                                                } else if step.action == MacroAction::IfStart {
                                                     ui.scope(|ui| {
                                                         ui.spacing_mut().item_spacing.x = 4.0;
                                                         ui.spacing_mut().interact_size.y = 22.0;
                                                         ui.spacing_mut().button_padding.y = 0.0;
                                                         ui.allocate_ui_with_layout(
                                                             vec2(ui.available_width(), 22.0),
                                                             egui::Layout::top_down(egui::Align::Min),
                                                             |ui| {
                                                             ui.horizontal(|ui| {
                                                                   ui.add_sized(
                                                                       [56.0, 22.0],
                                                                       egui::Label::new(Self::tr_lang(language, "IF", "Náº¾U")),
                                                                   );
                                                                   if step.if_condition_type != IfConditionType::Variable {
                                                                       step.if_condition_type = IfConditionType::Variable;
                                                                       live_sync = true;
                                                                   }
                                                                   if step.if_condition_type == IfConditionType::Variable {
                                                                    let var_name_id = ui.id().with((step_index, "regular-if-var-name"));
                                                                    let response = Self::render_variable_text_edit(
                                                                        ui,
                                                                        &mut step.if_variable_name,
                                                                        var_name_id,
                                                                        76.0,
                                                                        140.0,
                                                                        22.0,
                                                                        22.0,
                                                                        Self::tr_lang(language, "value/expr", "biến/expr"),
                                                                        false,
                                                                    );
                                                                    Self::apply_vietnamese_input_if_changed(
                                                                        &response,
                                                                        self.state.vietnamese_input_enabled,
                                                                        self.state.vietnamese_input_mode,
                                                                        &mut step.if_variable_name,
                                                                    );
                                                                    live_sync |= response.changed();
                                                                    Self::render_variable_suggestions(ui, &response, &mut step.if_variable_name, &timer_names, language);
                                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "if-op"))
                                                                        .width(40.0)
                                                                        .selected_text(&step.if_operator)
                                                                        .show_ui(ui, |ui| {
                                                                            for op in &["==", ">", "<", ">=", "<=", "!="] {
                                                                                if ui.selectable_label(step.if_operator == *op, *op).clicked() {
                                                                                    step.if_operator = op.to_string();
                                                                                    live_sync = true;
                                                                                }
                                                                            }
                                                                        });
                                                                    let var_val_id = ui.id().with((step_index, "regular-if-var-val"));
                                                                    let response2 = Self::render_variable_text_edit(
                                                                        ui,
                                                                        &mut step.key,
                                                                        var_val_id,
                                                                        76.0,
                                                                        180.0,
                                                                        22.0,
                                                                        22.0,
                                                                        Self::tr_lang(language, "value/expr", "giá trị/expr"),
                                                                        false,
                                                                    );
                                                                    Self::apply_vietnamese_input_if_changed(
                                                                        &response2,
                                                                        self.state.vietnamese_input_enabled,
                                                                        self.state.vietnamese_input_mode,
                                                                        &mut step.key,
                                                                    );
                                                                    live_sync |= response2.changed();
                                                                    Self::render_variable_suggestions(ui, &response2, &mut step.key, &timer_names, language);
                                                                       let left_expr = step.if_variable_name.trim();
                                                                       if !left_expr.is_empty() {
                                                                           let left_val = crate::overlay::evaluate_math_expression(left_expr);
                                                                           ui.add_space(2.0);
                                                                           ui.label(
                                                                               RichText::new(format!("({})", left_val))
                                                                                   .size(10.0)
                                                                                   .color(Color32::from_rgb(0, 191, 255))
                                                                           ).on_hover_text(Self::tr_lang(language, "Evaluated left expression", "Gia tri bieu thuc ben trai"));
                                                                       }
                                                                   } else if step.if_condition_type == IfConditionType::PixelColor {
                                                                       ui.label("X:");
                                                                       let resp_x = ui.add(egui::DragValue::new(&mut step.x));
                                                                       live_sync |= resp_x.changed();
                                                                       ui.label("Y:");
                                                                       let resp_y = ui.add(egui::DragValue::new(&mut step.y));
                                                                       live_sync |= resp_y.changed();
                                                                       let resp_col = ui.add_sized(
                                                                           [64.0, 22.0],
                                                                           TextEdit::singleline(&mut step.if_target_color)
                                                                               .hint_text(RichText::new("#RRGGBB").color(hint_color).weak()),
                                                                       );
                                                                       live_sync |= resp_col.changed();
                                                                       ui.label(Self::tr_lang(language, "Tol:", "Sai sÃ¡Â»â€˜:"));
                                                                       let resp_tol = ui.add(egui::DragValue::new(&mut step.if_color_tolerance).range(0..=255));
                                                                       live_sync |= resp_tol.changed();
                                                                   } else if step.if_condition_type == IfConditionType::VisionMatch {
                                                                       let selected_id = step.if_vision_preset_id;
                                                                       let selected_label = selected_id
                                                                           .and_then(|id| {
                                                                               self.state.vision_presets.iter().find(|p| p.id == id).map(|p| p.name.clone())
                                                                           })
                                                                           .unwrap_or_else(|| Self::tr_lang(language, "Select preset", "Chá»n preset").to_owned());
                                                                       egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "if-vision-preset"))
                                                                           .width(146.0)
                                                                           .selected_text(selected_label)
                                                                           .show_ui(ui, |ui| {
                                                                               for vision_preset in &self.state.vision_presets {
                                                                                   if ui.selectable_label(selected_id == Some(vision_preset.id), &vision_preset.name).clicked() {
                                                                                       step.if_vision_preset_id = Some(vision_preset.id);
                                                                                       live_sync = true;
                                                                                   }
                                                                               }
                                                                           });
                                                                   } else if step.if_condition_type == IfConditionType::KeyHeld || step.if_condition_type == IfConditionType::KeyPressed {
                                                                       let resp_key = ui.add_sized(
                                                                           [80.0, 22.0],
                                                                           TextEdit::singleline(&mut step.if_key_held_name)
                                                                               .hint_text(RichText::new(Self::tr_lang(language, "Key", "PhÃ­m")).color(hint_color).weak()),
                                                                       );
                                                                       Self::apply_vietnamese_input_if_changed(
                                                                           &resp_key,
                                                                           self.state.vietnamese_input_enabled,
                                                                           self.state.vietnamese_input_mode,
                                                                           &mut step.if_key_held_name,
                                                                       );
                                                                       live_sync |= resp_key.changed();
                                                                   } else if step.if_condition_type == IfConditionType::MouseHeld {
                                                                       egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "if-mouse-button"))
                                                                           .width(90.0)
                                                                           .selected_text(&step.if_mouse_button)
                                                                           .show_ui(ui, |ui| {
                                                                               for btn in &["MouseLeft", "MouseRight", "MouseMiddle", "MouseX1", "MouseX2"] {
                                                                                   if ui.selectable_label(step.if_mouse_button == *btn, *btn).clicked() {
                                                                                       step.if_mouse_button = btn.to_string();
                                                                                       live_sync = true;
                                                                                   }
                                                                               }
                                                                           });
                                                                   } else if step.if_condition_type == IfConditionType::MouseScroll {
                                                                       egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "if-scroll-dir"))
                                                                           .width(70.0)
                                                                           .selected_text(&step.if_scroll_direction)
                                                                           .show_ui(ui, |ui| {
                                                                               for dir in &["Up", "Down"] {
                                                                                   let label = match *dir {
                                                                                       "Up" => Self::tr_lang(language, "Up", "LÃªn"),
                                                                                       _ => Self::tr_lang(language, "Down", "Xuá»‘ng"),
                                                                                   };
                                                                                   if ui.selectable_label(step.if_scroll_direction == *dir, label).clicked() {
                                                                                       step.if_scroll_direction = dir.to_string();
                                                                                       live_sync = true;
                                                                                   }
                                                                               }
                                                                           });
                                                                   } else if step.if_condition_type == IfConditionType::MousePosition {
                                                                       egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "if-mouse-axis"))
                                                                           .width(50.0)
                                                                           .selected_text(&step.if_mouse_axis)
                                                                           .show_ui(ui, |ui| {
                                                                               for axis in &["X", "Y"] {
                                                                                   if ui.selectable_label(step.if_mouse_axis == *axis, *axis).clicked() {
                                                                                       step.if_mouse_axis = axis.to_string();
                                                                                       live_sync = true;
                                                                                   }
                                                                               }
                                                                           });
                                                                       egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "if-mouse-pos-op"))
                                                                           .width(40.0)
                                                                           .selected_text(&step.if_operator)
                                                                           .show_ui(ui, |ui| {
                                                                               for op in &["==", ">", "<", ">=", "<=", "!="] {
                                                                                   if ui.selectable_label(step.if_operator == *op, *op).clicked() {
                                                                                       step.if_operator = op.to_string();
                                                                                       live_sync = true;
                                                                                   }
                                                                               }
                                                                           });
                                                                       let response2 = ui.add_sized(
                                                                            [76.0, 22.0],
                                                                            TextEdit::singleline(&mut step.key)
                                                                                .hint_text(RichText::new(Self::tr_lang(language, "value/expr", "giÃƒÂ¡ trÃ¡Â»â€¹/expr")).color(hint_color).weak()),
                                                                        );
                                                                        Self::apply_vietnamese_input_if_changed(
                                                                            &response2,
                                                                            self.state.vietnamese_input_enabled,
                                                                            self.state.vietnamese_input_mode,
                                                                            &mut step.key,
                                                                        );
                                                                        live_sync |= response2.changed();
                                                                        Self::render_variable_suggestions(ui, &response2, &mut step.key, &timer_names, language);
                                                                   } else if step.if_condition_type == IfConditionType::PresetRunning {
                                                                       let selected_id = step.if_running_preset_id;
                                                                       let selected_label = selected_id
                                                                           .and_then(|id| {
                                                                               all_presets.iter().find(|(pid, _)| *pid == id).map(|(_, name)| name.clone())
                                                                           })
                                                                           .unwrap_or_else(|| Self::tr_lang(language, "Select preset", "Chá»n preset").to_owned());
                                                                       egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "if-running-preset"))
                                                                           .width(120.0)
                                                                           .selected_text(selected_label)
                                                                           .show_ui(ui, |ui| {
                                                                               for (pid, pname) in &all_presets {
                                                                                   if ui.selectable_label(selected_id == Some(*pid), pname).clicked() {
                                                                                       step.if_running_preset_id = Some(*pid);
                                                                                       live_sync = true;
                                                                                   }
                                                                               }
                                                                           });
                                                                   } else if step.if_condition_type == IfConditionType::TimerRunning {
                                                                       let selected_id = step.timer_preset_id;
                                                                       let selected_label = selected_id
                                                                           .and_then(|id| {
                                                                               self.state.timer_presets.iter().find(|t| t.id == id).map(|t| t.name.clone())
                                                                           })
                                                                           .unwrap_or_else(|| Self::tr_lang(language, "Select timer", "Chá»n timer").to_owned());
                                                                       egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "if-timer-preset"))
                                                                           .width(120.0)
                                                                           .selected_text(selected_label)
                                                                           .show_ui(ui, |ui| {
                                                                               for timer in &self.state.timer_presets {
                                                                                   if ui.selectable_label(selected_id == Some(timer.id), &timer.name).clicked() {
                                                                                       step.timer_preset_id = Some(timer.id);
                                                                                       live_sync = true;
                                                                                   }
                                                                               }
                                                                           });
                                                                   }
                                                                     if ui.add_sized([24.0, 24.0], Button::new("+")).on_hover_text(Self::tr_lang(language, "Add condition", "Thêm điều kiện")).clicked() {
                                                                       step.extra_conditions.push(ExtraCondition::default());
                                                                       live_sync = true;
                                                                   }
                                                               });
                                                             let mut remove_extra_idx = None;
                                                             for (extra_idx, cond) in step.extra_conditions.iter_mut().enumerate() {
                                                                  ui.horizontal(|ui| {
                                                                          egui::ComboBox::from_id_salt((group.id, preset.id, extra_idx, "if-extra-join"))
                                                                              .width(56.0)
                                                                              .selected_text(if cond.join_operator.eq_ignore_ascii_case("OR") { Self::tr_lang(language, "OR", "HO?C") } else { Self::tr_lang(language, "AND", "VÀ") })
                                                                              .show_ui(ui, |ui| {
                                                                                 for op in &["AND", "OR"] {
                                                                                     let label = if *op == "AND" {
                                                                                         Self::tr_lang(language, "AND", "VÀ")
                                                                                     } else {
                                                                                         Self::tr_lang(language, "OR", "HO?C")
                                                                                     };
                                                                                     if ui.selectable_label(cond.join_operator.eq_ignore_ascii_case(op), label).clicked() {
                                                                                         cond.join_operator = op.to_string();
                                                                                         live_sync = true;
                                                                                     }
                                                                                 }
                                                                             });
                                                                         let cond_var_id = ui.id().with((step_index, extra_idx, "extra-if-var"));
                                                                         let response = Self::render_variable_text_edit(
                                                                             ui,
                                                                             &mut cond.variable_name,
                                                                             cond_var_id,
                                                                             76.0,
                                                                             140.0,
                                                                             22.0,
                                                                             22.0,
                                                                             Self::tr_lang(language, "value/expr", "giÃƒÂ¡ trÃ¡Â»â€¹/expr"),
                                                                             false,
                                                                         );
                                                                     Self::apply_vietnamese_input_if_changed(
                                                                         &response,
                                                                         self.state.vietnamese_input_enabled,
                                                                         self.state.vietnamese_input_mode,
                                                                         &mut cond.variable_name,
                                                                     );
                                                                     live_sync |= response.changed();
                                                                     egui::ComboBox::from_id_salt((group.id, preset.id, step_index, extra_idx, "extra-if-op"))
                                                                         .width(40.0)
                                                                         .selected_text(&cond.operator)
                                                                         .show_ui(ui, |ui| {
                                                                             for op in &["==", ">", "<", ">=", "<=", "!="] {
                                                                                 if ui.selectable_label(cond.operator == *op, *op).clicked() {
                                                                                     cond.operator = op.to_string();
                                                                                     live_sync = true;
                                                                                 }
                                                                             }
                                                                         });
                                                                      let cond_expr_id = ui.id().with((step_index, extra_idx, "extra-if-expr"));
                                                                      let response2 = Self::render_variable_text_edit(
                                                                          ui,
                                                                          &mut cond.expression,
                                                                          cond_expr_id,
                                                                          76.0,
                                                                          160.0,
                                                                          22.0,
                                                                          22.0,
                                                                          Self::tr_lang(language, "value/expr", "giá trị/expr"),
                                                                          false,
                                                                      );
                                                                     Self::apply_vietnamese_input_if_changed(
                                                                         &response2,
                                                                         self.state.vietnamese_input_enabled,
                                                                         self.state.vietnamese_input_mode,
                                                                         &mut cond.expression,
                                                                     );
                                                                     live_sync |= response2.changed();
                                                                     let left_expr = cond.variable_name.trim();
                                                                     if !left_expr.is_empty() {
                                                                         let left_val = crate::overlay::evaluate_math_expression(left_expr);
                                                                         ui.label(
                                                                             RichText::new(format!("({})", left_val))
                                                                                 .size(10.0)
                                                                                 .color(Color32::from_rgb(0, 191, 255))
                                                                         );
                                                                     }
                                                                         if ui.add_sized([24.0, 24.0], Button::new("-")).on_hover_text(Self::tr_lang(language, "Remove condition", "Xóa điều kiện")).clicked() {
                                                                         remove_extra_idx = Some(extra_idx);
                                                                     }
                                                                 });
                                                             }
                                                             if let Some(remove_idx) = remove_extra_idx {
                                                                 step.extra_conditions.remove(remove_idx);
                                                                 live_sync = true;
                                                             }
                                                         });
                                                     });
                                                                                                 } else if step.action == MacroAction::SetVariable {
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.spacing_mut().interact_size.y = 22.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        ui.vertical(|ui| {
                                                            ui.horizontal(|ui| {
                                                                  let var_name_id = ui.id().with((step_index, "regular-set-var-name"));
                                                                  let response = Self::render_variable_text_edit(
                                                                      ui,
                                                                      &mut step.if_variable_name,
                                                                      var_name_id,
                                                                      76.0,
                                                                      140.0,
                                                                      22.0,
                                                                      22.0,
                                                                      Self::tr_lang(language, "variable", "biến"),
                                                                      false,
                                                                  );
                                                                  Self::apply_vietnamese_input_if_changed(
                                                                      &response,
                                                                      self.state.vietnamese_input_enabled,
                                                                      self.state.vietnamese_input_mode,
                                                                      &mut step.if_variable_name,
                                                                  );
                                                                  live_sync |= response.changed();
                                                                  ui.label(" = ");
                                                                  let var_val_id = ui.id().with((step_index, "regular-set-var-val"));
                                                                  let response2 = Self::render_variable_text_edit(
                                                                      ui,
                                                                      &mut step.key,
                                                                      var_val_id,
                                                                      76.0,
                                                                      180.0,
                                                                      22.0,
                                                                      22.0,
                                                                      Self::tr_lang(language, "value/expr", "giá trị"),
                                                                      false,
                                                                  );
                                                                  Self::apply_vietnamese_input_if_changed(
                                                                      &response2,
                                                                      self.state.vietnamese_input_enabled,
                                                                      self.state.vietnamese_input_mode,
                                                                      &mut step.key,
                                                                  );
                                                                  live_sync |= response2.changed();
                                                                Self::render_variable_suggestions_raw(ui, &response, &mut step.if_variable_name, &timer_names, language);
                                                                Self::render_variable_suggestions(ui, &response2, &mut step.key, &timer_names, language);
                                                                let var_name = step.if_variable_name.trim();
                                                                if !var_name.is_empty() {
                                                                    let current_val = crate::overlay::RUNTIME_VARIABLES.lock().get(var_name).copied();
                                                                    let val_str = current_val.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
                                                                    ui.add_space(2.0);
                                                                    ui.label(
                                                                        RichText::new(format!("({})", val_str))
                                                                            .size(10.0)
                                                                            .color(Color32::from_rgb(0, 191, 255))
                                                                    ).on_hover_text(Self::tr_lang(language, "Current runtime value", "Giá trị chạy hiện tại"));
                                                                }
                                                            });
                                                        });
                                                    });
                                                } else if matches!(step.action, MacroAction::StartVisionSearch
                                                         | MacroAction::ScanVisionOnce
                                                         | MacroAction::TriggerVisionMove
                                                         | MacroAction::StopVision | MacroAction::StopVisionWait) {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .vision_presets
                                                                .iter()
                                                                .find(|p| p.id == id)
                                                                .map(|p| p.name.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            if step.key.trim().is_empty() {
                                                                Self::tr_lang(
                                                                    language,
                                                                    "Select vision",
                                                                    "ChÃ¡Â»Ân hiÃ¡Â»Æ’n thÃ¡Â»â€¹",
                                                                )
                                                                .to_owned()
                                                            } else {
                                                                format!("ID: {}", step.key)
                                                            }
                                                        });
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "vision-preset-step"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for vision_preset in &self.state.vision_presets {
                                                                if ui
                                                                    .selectable_label(
                                                                        selected_id == Some(vision_preset.id),
                                                                        &vision_preset.name,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    step.key = vision_preset.id.to_string();
                                                                    live_sync = true;
                                                                }
                                                            }
                                                        });
                                                     let selected_preset = selected_id.and_then(|id| {
                                                         self.state.vision_presets.iter().find(|p| p.id == id)
                                                     });
                                                     if step.action == MacroAction::ScanVisionOnce && selected_preset.map(|p| p.is_pixel_counter).unwrap_or(false) {
                                                         ui.add_space(4.0);
                                                         ui.horizontal(|ui| {
                                                             let response = ui.add_sized(
                                                                 [100.0, 22.0],
                                                                 TextEdit::singleline(&mut step.if_variable_name)
                                                                     .hint_text(RichText::new(Self::tr_lang(language, "set variable", "gÃ¡n biáº¿n")).color(hint_color).weak()),
                                                             );
                                                             Self::apply_vietnamese_input_if_changed(
                                                                 &response,
                                                                 self.state.vietnamese_input_enabled,
                                                                 self.state.vietnamese_input_mode,
                                                                 &mut step.if_variable_name,
                                                             );
                                                             live_sync |= response.changed();
                                                             Self::render_variable_suggestions_raw(ui, &response, &mut step.if_variable_name, &timer_names, language);
                                                         });
                                                     }
                                                } else {
                                                    let step_capture_target = CaptureRequest::MacroStepInput {
                                                        group_id: group.id,
                                                        preset_id: preset.id,
                                                        step_index,
                                                    };
                                                    let step_capture_active =
                                                        capture_target_snapshot.as_ref() == Some(&step_capture_target);
                                                    let mut display_key = if step_capture_active {
                                                        Self::tr_lang(
                                                            language,
                                                            "Capturing...",
                                                            "Đang lấy phím...",
                                                        ).to_owned()
                                                    } else {
                                                        step.key.clone()
                                                    };
                                                                          let response = if step_capture_active {
                                                        let text_edit = TextEdit::singleline(&mut display_key).hint_text(Self::tr_lang(
                                                            language,
                                                            "Capturing...",
                                                            "Đang lấy phím...",
                                                        ));
                                                        ui.add_sized([146.0, 18.0], text_edit)
                                                    } else {
                                                        let key_id = ui.id().with((step_index, "regular-default-key"));
                                                        Self::render_expandable_text_edit(
                                                            ui,
                                                            &mut display_key,
                                                            key_id,
                                                            146.0,
                                                            240.0,
                                                            18.0,
                                                            18.0,
                                                            "...",
                                                            false,
                                                        )
                                                    };
                                                    if !step_capture_active {
                                                        Self::apply_vietnamese_input_if_changed(
                                                            &response,
                                                            self.state.vietnamese_input_enabled,
                                                            self.state.vietnamese_input_mode,
                                                            &mut display_key,
                                                        );
                                                        if response.changed() || step.key != display_key {
                                                            step.key = display_key;
                                                            live_sync = true;
                                                        }
                                                    }
                                                }
                                            } else {
                                                ui.add_sized([146.0, 18.0], egui::Label::new("-"));
                                            }
                                            let action_uses_position =
                                                Self::macro_action_uses_position(step.action);
                                            if action_uses_position {
                                                live_sync |= ui
                                                    .add_sized(
                                                        [48.0, 18.0],
                                                        DragValue::new(&mut step.x).range(-30000..=30000),
                                                    )
                                                    .changed();
                                                live_sync |= ui
                                                    .add_sized(
                                                        [48.0, 18.0],
                                                        DragValue::new(&mut step.y).range(-30000..=30000),
                                                    )
                                                    .changed();
                                                if step.action == MacroAction::MouseMoveAbsolute {
                                                    let capture_target = MouseMoveAbsoluteCaptureTarget {
                                                        group_id: Some(group.id),
                                                        preset_id: preset.id,
                                                        step_index,
                                                    };
                                                    let capture_active = self
                                                        .mouse_move_absolute_capture_target
                                                        == Some(capture_target);
                                                    if ui
                                                        .add_sized(
                                                            [62.0, 18.0],
                                                            Button::new(Self::pick_point_button_text(
                                                                language,
                                                                capture_active,
                                                            )),
                                                        )
                                                        .on_hover_text(Self::tr_lang(
                                                            language,
                                                            "Minimize the app and click anywhere on screen to capture screen X/Y.",
                                                            "Thu nhÃ¡Â»Â app rÃ¡Â»â€œi bÃ¡ÂºÂ¥m vÃƒÂ o báº¥t ká»³ vÃ¡Â»â€¹ trÃƒÂ­ nÃƒÂ o trÃƒÂªn mÃƒÂ n hÃƒÂ¬nh Ä‘á»Æ’ lÃ¡ÂºÂ¥y X/Y.",
                                                        ))
                                                    .clicked()
                                                    {
                                                        if capture_active {
                                                            cancel_mouse_move_absolute_capture = true;
                                                        } else {
                                                            begin_mouse_move_absolute_capture_target =
                                                                Some(capture_target);
                                                        }
                                                    }
                                                }
                                            } else if step.action == MacroAction::PlayMousePathPreset {
                                                live_sync |= ui
                                                    .checkbox(&mut step.smooth_mouse_path, "S")
                                                    .on_hover_text(Self::tr_lang(
                                                        language,
                                                        "Constant speed",
                                                        "Di chuyÃ¡Â»Æ’n chuá»™t vÃ¡Â»â€ºi tÃ¡Â»â€˜c Ä‘á»™ Ä‘á»Âu",
                                                    ))
                                                    .changed();
                                                live_sync |= ui
                                                    .add_sized(
                                                        [48.0, 18.0],
                                                        DragValue::new(&mut step.mouse_speed_percent)
                                                            .range(10..=1000)
                                                            .suffix("%"),
                                                    )
                                                    .changed();
                                            } else if step.action == MacroAction::ShowHud {
                                                let mut temp_ms = if step.timed_override { step.duration_override_ms } else { 0 };
                                                let changed = ui.add_sized(
                                                    [96.0, 18.0],
                                                    DragValue::new(&mut temp_ms)
                                                        .range(0..=60_000)
                                                        .suffix(" ms"),
                                                ).on_hover_text(Self::tr_lang(
                                                    language,
                                                    "Display duration (0 ms = show until macro ends)",
                                                    "ThÃ¡Â»Âi gian hiÃ¡Â»Æ’n thÃ¡Â»â€¹ (0 ms = hiÃ¡Â»â€¡n Ã„â€˜Ã¡ÂºÂ¿n khi dÃ¡Â»Â«ng macro)",
                                                )).changed();
                                                if changed {
                                                    step.duration_override_ms = temp_ms;
                                                    step.timed_override = temp_ms > 0;
                                                    live_sync = true;
                                                }
                                            } else if action_supports_capture && !(step.action == MacroAction::StopIfKeyPressed && step.break_loop_by_variable) {
                                                let step_capture_target = CaptureRequest::MacroStepInput {
                                                    group_id: group.id,
                                                    preset_id: preset.id,
                                                    step_index,
                                                };
                                                let step_capture_active =
                                                    capture_target_snapshot.as_ref() == Some(&step_capture_target);
                                                let step_capture_width = if step_capture_active { 84.0 } else { 22.0 };
                                                let step_capture_button = if step_capture_active {
                                                    Button::new(Self::capture_button_text(language, true))
                                                        .fill(Color32::from_rgb(88, 84, 44))
                                                } else {
                                                    Button::new(Self::material_icon_text(0xe312, 12.0))
                                                };
                                                if ui
                                                    .add_sized([step_capture_width, 18.0], step_capture_button)
                                                    .on_hover_text(Self::tr_lang(
                                                        language,
                                                        "Capture input",
                                                        "",
                                                    ))
                                                    .clicked()
                                                {
                                                    if step_capture_active {
                                                        cancel_active_capture = true;
                                                    } else {
                                                        next_capture_target = Some(step_capture_target);
                                                    }
                                                }
                                                // Dropdown right here (equal size: 22.0 wide, 18.0 high)
                                                let menu_response = ui.menu_button(Self::material_icon_text(0xe5d2, 12.0), |ui| {
                                                    ui.set_max_width(200.0);
                                                    ui.menu_button(Self::tr_lang(language, "Letters (A-Z)", "Chá»¯ cÃ¡i (A-Z)"), |ui| {
                                                        ui.set_max_width(120.0);
                                                        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                                            for ch in b'A'..=b'Z' {
                                                                let key_str = (ch as char).to_string();
                                                                if ui.button(&key_str).clicked() {
                                                                    step.key = key_str;
                                                                    live_sync = true;
                                                                    ui.close_menu();
                                                                }
                                                            }
                                                        });
                                                    });
                                                    ui.menu_button(Self::tr_lang(language, "Numbers & Symbols", "SÃ¡Â»â€˜ & KÃƒÂ­ tÃ¡Â»Â±"), |ui| {
                                                        ui.set_max_width(140.0);
                                                        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                                            for num in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
                                                                if ui.button(num).clicked() {
                                                                    step.key = num.to_string();
                                                                    live_sync = true;
                                                                    ui.close_menu();
                                                                }
                                                            }
                                                            ui.separator();
                                                            for sym in [";", "=", ",", "-", ".", "/", "`", "[", "\\", "]", "'"] {
                                                                if ui.button(sym).clicked() {
                                                                    step.key = sym.to_string();
                                                                    live_sync = true;
                                                                    ui.close_menu();
                                                                }
                                                            }
                                                        });
                                                    });
                                                    ui.menu_button(Self::tr_lang(language, "Navigation", "Ã„ÂiÃ¡Â»Âu hÃ†Â°Ã¡Â»â€ºng & PhÃ­m táº¯t"), |ui| {
                                                        ui.set_max_width(160.0);
                                                        for key in ["Escape", "Enter", "Space", "Backspace", "Tab", "Insert", "Delete", "Home", "End", "PageUp", "PageDown", "Left", "Up", "Right", "Down", "PrintScreen", "Pause"] {
                                                            if ui.button(key).clicked() {
                                                                step.key = key.to_string();
                                                                live_sync = true;
                                                                ui.close_menu();
                                                            }
                                                        }
                                                    });
                                                    ui.menu_button(Self::tr_lang(language, "Function (F1-F24)", "PhÃ­m chÃ¡Â»Â©c nÃ„Æ’ng"), |ui| {
                                                        ui.set_max_width(100.0);
                                                        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                                            for num in 1..=24 {
                                                                let key_str = format!("F{}", num);
                                                                if ui.button(&key_str).clicked() {
                                                                    step.key = key_str;
                                                                    live_sync = true;
                                                                    ui.close_menu();
                                                                }
                                                            }
                                                        });
                                                    });
                                                    ui.menu_button(Self::tr_lang(language, "Numpad", "BÃƒÂ n phÃ­m sÃ¡Â»â€˜ phÃ¡Â»Â¥"), |ui| {
                                                        ui.set_max_width(160.0);
                                                        for key in ["Numpad0", "Numpad1", "Numpad2", "Numpad3", "Numpad4", "Numpad5", "Numpad6", "Numpad7", "Numpad8", "Numpad9", "NumpadMultiply", "NumpadAdd", "NumpadSubtract", "NumpadDecimal", "NumpadDivide"] {
                                                            if ui.button(key).clicked() {
                                                                step.key = key.to_string();
                                                                live_sync = true;
                                                                ui.close_menu();
                                                            }
                                                        }
                                                    });
                                                    ui.menu_button(Self::tr_lang(language, "Modifiers & Locks", "PhÃ­m khÃƒÂ³a & bá»• trÃ¡Â»Â£"), |ui| {
                                                        ui.set_max_width(150.0);
                                                        for key in ["Ctrl", "Alt", "Shift", "Win", "CapsLock", "NumLock", "ScrollLock", "Apps"] {
                                                            if ui.button(key).clicked() {
                                                                step.key = key.to_string();
                                                                live_sync = true;
                                                                ui.close_menu();
                                                            }
                                                        }
                                                    });
                                                });
                                                menu_response.response.on_hover_text(Self::tr_lang(
                                                    language,
                                                    "Manually select key",
                                                    "ChÃ¡Â»Ân phÃ­m thÃ¡Â»Â§ cÃƒÂ´ng"
                                                ));
                                                // Trailing spacers placed after buttons to align columns with other rows having X/Y coords
                                                ui.add_sized([48.0, 18.0], egui::Label::new(""));
                                                ui.add_sized([48.0, 18.0], egui::Label::new(""));
                                            } else {
                                                ui.add_sized([28.0, 18.0], egui::Label::new(""));
                                            }
                                            let is_dark_theme = self.state.ui_theme == UiThemeMode::Dark;
                                            let paste_button_width = 56.0;
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui
                                                    .add_enabled(
                                                        !self.macro_step_clipboard.is_empty(),
                                                        Button::new(Self::tr_lang(language, "Paste", "Paste"))
                                                            .min_size(vec2(paste_button_width, 18.0)),
                                                    )
                                                    .on_hover_text(Self::tr_lang(
                                                        language,
                                                        "Paste the copied steps below this step.",
                                                        "Paste copied steps below this step.",
                                                    ))
                                                    .clicked()
                                                {
                                                    paste_step_after = Some((group.id, preset.id, step_index));
                                                }
                                                if ui
                                                     .add(
                                                         Button::new(Self::tr_lang(language, "Copy", "Copy"))
                                                             .min_size(vec2(paste_button_width, 18.0)),
                                                      )
                                                      .on_hover_text(Self::tr_lang(
                                                          language,
                                                          "Copy this step.",
                                                          "Copy step nÃƒÂ y.",
                                                      ))
                                                      .clicked()
                                                  {
                                                      copy_single_step = Some((group.id, preset.id, step_index));
                                                  }
                                                  if self.show_share_buttons {
                                                      if ui
                                                      .add(
                                                          Button::new(Self::tr_lang(language, "Exp", "Exp"))
                                                              .min_size(vec2(32.0, 18.0)),
                                                      )
                                                      .on_hover_text(Self::tr_lang(
                                                          language,
                                                          "Copy step code to clipboard.",
                                                          "Sao chÃƒÂ©p mÃ£ step vÃƒÂ o clipboard.",
                                                      ))
                                                      .clicked()
                                                  {
                                                      export_step = Some((preset.id, step_index));
                                                  }
                                                  if Self::is_copy_feedback_active(
                                                      self.macro_step_export_feedback_until,
                                                  ) {
                                                      ui.add_sized(
                                                          [62.0, 18.0],
                                                          egui::Label::new(
                                                              RichText::new(Self::tr_lang(
                                                                  language,
                                                                  "Copied",
                                                                  "Copied",
                                                              ))
                                                              .color(Color32::from_rgb(126, 224, 182))
                                                              .strong(),
                                                          ),
                                                      );
                                                  }
                                                  if ui
                                                      .add(
                                                          Button::new(Self::tr_lang(language, "Imp", "Imp"))
                                                              .min_size(vec2(32.0, 18.0)),
                                                      )
                                                      .on_hover_text(Self::tr_lang(
                                                          language,
                                                          "Import step from clipboard below this step.",
                                                          "Nháº­p step tÃ¡Â»Â« clipboard náº±m dÃ†Â°Ã¡Â»â€ºi step nÃƒÂ y.",
                                                      ))
                                                      .clicked()
                                                  {
                                                      import_step_to = Some((group.id, preset.id, Some(step_index)));
                                                  }
                                                  }
                                                if step.toggle_enabled_on_run {
                                                    ui.add_space(4.0);
                                                    ui.scope(|ui| {
                                                        ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
                                                        ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                                                        ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                                        let hover_bg = if is_dark_theme {
                                                            Color32::from_rgba_unmultiplied(255, 255, 255, 20)
                                                        } else {
                                                            Color32::from_rgba_unmultiplied(0, 0, 0, 15)
                                                        };
                                                        let hover_stroke = if is_dark_theme {
                                                            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 40))
                                                        } else {
                                                            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 30))
                                                        };
                                                        let active_bg = if is_dark_theme {
                                                            Color32::from_rgba_unmultiplied(255, 255, 255, 35)
                                                        } else {
                                                            Color32::from_rgba_unmultiplied(0, 0, 0, 25)
                                                        };
                                                        let active_stroke = if is_dark_theme {
                                                            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 60))
                                                        } else {
                                                            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 45))
                                                        };
                                                        ui.visuals_mut().widgets.hovered.bg_fill = hover_bg;
                                                        ui.visuals_mut().widgets.hovered.bg_stroke = hover_stroke;
                                                        ui.visuals_mut().widgets.active.bg_fill = active_bg;
                                                        ui.visuals_mut().widgets.active.bg_stroke = active_stroke;
                                                        let toggle_icon_color = Color32::from_rgb(0, 220, 255);
                                                        let toggle_icon = Self::material_icon_text(0xe040, 16.0).color(toggle_icon_color);
                                                        if ui
                                                            .add_sized([22.0, 20.0], Button::new(toggle_icon))
                                                            .on_hover_text(Self::tr_lang(
                                                                language,
                                                                "Toggle self enabled on run (run-loop/refresh state)",
                                                                "TÃ¡Â»Â± Ä‘á»™ng bÃ¡ÂºÂ­t/táº¯t bÆ°á»›c khi chÃ¡ÂºÂ¡y (trÃ¡ÂºÂ¡ng thÃƒÂ¡i chÃ¡ÂºÂ¡y lÃ¡ÂºÂ·p/chÃ¡ÂºÂ¡y tiÃ¡ÂºÂ¿p)"
                                                            ))
                                                            .clicked()
                                                        {
                                                            step.toggle_enabled_on_run = !step.toggle_enabled_on_run;
                                                            live_sync = true;
                                                        }
                                                    });
                                                }
                                            });
                                        });
                                    })
                                    .response;
                                if is_active {
                                    let is_focused = ui.ctx().input(|i| i.viewport().focused == Some(true));
                                    if is_focused {
                                        ui.ctx().request_repaint();
                                    }
                                    let rect = row_response.rect;
                                    let speed = 500.0;
                                    let t = ui.ctx().input(|i| i.time);
                                    let w = rect.width();
                                    let h = rect.height();
                                    let perimeter = 2.0 * (w + h);
                                    let d = (t * speed) % perimeter as f64;
                                    let get_point_on_rect = |dist: f64| -> egui::Pos2 {
                                        let mut dist = (dist % perimeter as f64) as f32;
                                        if dist < 0.0 {
                                            dist += perimeter;
                                        }
                                        if dist < w {
                                            egui::pos2(rect.min.x + dist, rect.min.y)
                                        } else if dist < w + h {
                                            egui::pos2(rect.max.x, rect.min.y + (dist - w))
                                        } else if dist < 2.0 * w + h {
                                            egui::pos2(rect.max.x - (dist - (w + h)), rect.max.y)
                                        } else {
                                            egui::pos2(rect.min.x, rect.max.y - (dist - (2.0 * w + h)))
                                        }
                                    };
                                    let dot_pos = get_point_on_rect(d);
                                    ui.painter().circle_filled(dot_pos, 2.5, Color32::from_rgb(0, 255, 170));
                                    let tail_pos1 = get_point_on_rect(d - 8.0);
                                    ui.painter().circle_filled(tail_pos1, 2.0, Color32::from_rgba_unmultiplied(0, 255, 170, 180));
                                    let tail_pos2 = get_point_on_rect(d - 16.0);
                                    ui.painter().circle_filled(tail_pos2, 1.5, Color32::from_rgba_unmultiplied(0, 255, 170, 110));
                                    let tail_pos3 = get_point_on_rect(d - 24.0);
                                    ui.painter().circle_filled(tail_pos3, 1.0, Color32::from_rgba_unmultiplied(0, 255, 170, 50));
                                }
                                if row_response.secondary_clicked() {
                                    remove_step = Some((preset.id, step_index));
                                }
                                if row_response.hovered() {
                                    let hover_request = Self::build_hover_preview_request(
                                        language,
                                        hover_preview_source_id,
                                        step,
                                    );
                                    let hover_anchor_pos = ui.ctx().pointer_hover_pos().unwrap_or(row_response.rect.right_bottom());
                                    ui.ctx().data_mut(|data| {
                                        data.insert_temp(
                                            hover_preview_state_id,
                                            hover_request
                                                .as_ref()
                                                .map(|request| (group.id, request.clone(), hover_anchor_pos)),
                                        )
                                    });
                                    if let Some(hover_request) = hover_request {
                                        if matches!(
                                            &hover_request,
                                            HoverPreviewRequest::MacroPreset { .. }
                                                | HoverPreviewRequest::StepToggle { .. }
                                        ) {
                                            let scroll_y = ui.ctx().input(|i| i.raw_scroll_delta.y);
                                            if scroll_y.abs() > 0.0 {
                                                let source_id = match &hover_request {
                                                    HoverPreviewRequest::MacroPreset { source_id, .. }
                                                    | HoverPreviewRequest::StepToggle { source_id, .. } => *source_id,
                                                    _ => hover_preview_source_id,
                                                };
                                                let offset_id = source_id.with("macro-hover-preview-offset");
                                                let line_count = match &hover_request {
                                                    HoverPreviewRequest::MacroPreset { preset_id, .. }
                                                    | HoverPreviewRequest::StepToggle { preset_id, .. } => {
                                                        all_preset_step_counts
                                                            .iter()
                                                            .find(|(id, _)| *id == *preset_id)
                                                            .map(|(_, count)| *count)
                                                            .unwrap_or(0)
                                                    }
                                                    _ => 0,
                                                };
                                                let visible_lines = 5i32;
                                                let max_offset = (line_count - visible_lines).max(0);
                                                let mut offset = ui
                                                    .ctx()
                                                    .data(|data| data.get_temp::<i32>(offset_id))
                                                    .unwrap_or(0);
                                                let delta = if scroll_y > 0.0 { -1 } else { 1 };
                                                offset = (offset + delta).clamp(0, max_offset);
                                                ui.ctx().data_mut(|data| data.insert_temp(offset_id, offset));
                                                ui.ctx().request_repaint();
                                            }
                                        }
                                        hover_preview_request = Some((group.id, hover_request));
                                    } else {
                                        hover_preview_request = None;
                                    }
                                }
                                step_rects[step_index] = row_response.rect;
                            }
                            if drag_payload.is_some() && !preview_drawn {
                                preview_drop_index = steps_len;
                                paint_drop_preview(ui);
                            }
                            // Dynamic hover highlight for Loop and If blocks (Gá»£i Ã½ 2)
                            let hover_pos = ui.ctx().pointer_interact_pos();
                            if let Some(pos) = hover_pos {
                                struct BlockRange {
                                    start_idx: usize,
                                    end_idx: usize,
                                }
                                let mut blocks = Vec::new();
                                let mut loop_stack = Vec::new();
                                let mut if_stack = Vec::new();
                                for (idx, s) in preset.steps.iter().enumerate() {
                                    if s.enabled {
                                        if s.action == MacroAction::LoopStart {
                                            loop_stack.push(idx);
                                        } else if s.action == MacroAction::LoopEnd {
                                            if let Some(start_idx) = loop_stack.pop() {
                                                blocks.push(BlockRange {
                                                    start_idx,
                                                    end_idx: idx,
                                                });
                                            }
                                        } else if s.action == MacroAction::IfStart {
                                            if_stack.push(idx);
                                        } else if s.action == MacroAction::IfEnd {
                                            if let Some(start_idx) = if_stack.pop() {
                                                blocks.push(BlockRange {
                                                    start_idx,
                                                    end_idx: idx,
                                                });
                                            }
                                        }
                                    }
                                }
                                let mut active_block: Option<&BlockRange> = None;
                                for block in &blocks {
                                    if block.start_idx < step_rects.len() && block.end_idx < step_rects.len() {
                                        let start_rect = step_rects[block.start_idx];
                                        let end_rect = step_rects[block.end_idx];
                                        if start_rect != Rect::ZERO && end_rect != Rect::ZERO {
                                            let union_rect = start_rect.union(end_rect);
                                            // KiÃ¡Â»Æ’m tra xem chuá»™t cÃƒÂ³ náº±m trong union_rect bao gÃ¡Â»â€œm cáº£ khoáº£nhg há»Ÿ dÃ¡Â»Âc khÃƒÂ´ng
                                            if union_rect.contains(pos) {
                                                match active_block {
                                                    None => active_block = Some(block),
                                                    Some(current) => {
                                                        if (block.end_idx - block.start_idx) < (current.end_idx - current.start_idx) {
                                                            active_block = Some(block);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Some(block) = active_block {
                                    let start_rect = step_rects[block.start_idx];
                                    let end_rect = step_rects[block.end_idx];
                                    let union_rect = start_rect.union(end_rect).expand(3.0);
                                    ui.painter().rect_stroke(
                                        union_rect,
                                        6.0,
                                        egui::Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 220)),
                                        egui::StrokeKind::Outside,
                                    );
                                    ui.ctx().request_repaint(); // ensure active repaint during hover
                                }
                            }
                            if let Some(payload) = drag_payload
                                && ui.input(|input| input.pointer.any_released())
                            {
                                move_step_to = Some((
                                    payload.preset_id,
                                    payload.indices.clone(),
                                    preview_drop_index,
                                ));
                            }
                        });
                                    });
                                    ui.add_space(4.0);
                                }
                            });
                        }
                        if let Some((preset_id, step_index)) = insert_step_after {
                            if let Some(target_preset) = group
                                .presets
                                .iter_mut()
                                .find(|preset| preset.id == preset_id)
                            {
                                let insert_at = (step_index + 1).min(target_preset.steps.len());
                                target_preset.steps.insert(insert_at, MacroStep::default());
                                live_sync = true;
                                clear_step_selection = Some((group.id, preset_id));
                            }
                        }
                        if let Some((preset_id, dragged_indices, to_index)) = move_step_to {
                            if let Some(target_preset) = group
                                .presets
                                .iter_mut()
                                .find(|preset| preset.id == preset_id)
                            {
                                let mut indices = dragged_indices
                                    .into_iter()
                                    .filter(|index| *index < target_preset.steps.len())
                                    .collect::<Vec<_>>();
                                indices.sort_unstable();
                                indices.dedup();
                                if !indices.is_empty() {
                                    let mut moved_steps = Vec::with_capacity(indices.len());
                                    for index in indices.iter().rev().copied() {
                                        moved_steps.push(target_preset.steps.remove(index));
                                    }
                                    moved_steps.reverse();
                                    let removed_before_target =
                                        indices.iter().filter(|index| **index < to_index).count();
                                    let insert_at = to_index
                                        .saturating_sub(removed_before_target)
                                        .min(target_preset.steps.len());
                                    for (offset, step) in moved_steps.into_iter().enumerate() {
                                        target_preset.steps.insert(insert_at + offset, step);
                                    }
                                    live_sync = true;
                                }
                            }
                        }
                        if let Some((preset_id, step_index)) = remove_step {
                            if let Some(preset) = group
                                .presets
                                .iter_mut()
                                .find(|preset| preset.id == preset_id)
                                && step_index < preset.steps.len()
                            {
                                preset.steps.remove(step_index);
                                live_sync = true;
                                clear_step_selection = Some((group.id, preset_id));
                            }
                        }
                        if let Some(preset_id) = remove_preset {
                            group.presets.retain(|preset| preset.id != preset_id);
                            live_sync = true;
                            clear_step_selection = Some((group.id, preset_id));
                        }
                            });
                        });
                    });
                    if let Some((group_id, preset_id, step_index, name, command, use_powershell)) =
                        pending_custom_preset_save.take()
                        && let Some(saved_id) = self.upsert_custom_preset_from_step_draft_values(
                            name,
                            command,
                            use_powershell,
                        )
                    {
                        live_sync = true;
                        if let Some(step_index) = step_index {
                            if let Some(group) = self
                                .state
                                .macro_groups
                                .iter_mut()
                                .find(|group| group.id == group_id)
                            {
                                if let Some(preset) = group
                                    .presets
                                    .iter_mut()
                                    .find(|preset| preset.id == preset_id)
                                {
                                    if let Some(step) = preset.steps.get_mut(step_index) {
                                        step.key = saved_id.to_string();
                                        step.command_preset_use_powershell = false;
                                    }
                                }
                            }
                        } else {
                            if let Some(group) = self
                                .state
                                .macro_groups
                                .iter_mut()
                                .find(|group| group.id == group_id)
                            {
                                if let Some(preset) = group
                                    .presets
                                    .iter_mut()
                                    .find(|preset| preset.id == preset_id)
                                {
                                    preset.hold_stop_step.key = saved_id.to_string();
                                    preset.hold_stop_step.command_preset_use_powershell = false;
                                }
                            }
                        }
                    }
                    if let Some((group_id, preset_id, step_index, name, command, use_powershell, is_ad_hoc)) =
                        pending_custom_preset_save_and_open_ai.take()
                    {
                        if is_ad_hoc {
                            self.command_ai_step_target = Some((group_id, preset_id, step_index));
                            self.state.command_presets.retain(|preset| preset.id != 999999);
                            let mut temp_preset = CommandPreset::new(999999);
                            temp_preset.name = "Step Custom Command".to_owned();
                            temp_preset.command = command;
                            temp_preset.use_powershell = use_powershell;
                            temp_preset.collapsed = true;
                            self.state.command_presets.push(temp_preset);
                            self.open_command_ai_dialog_for_preset(999999);
                        } else if let Some(saved_id) = self.upsert_custom_preset_from_step_draft_values(
                            name,
                            command,
                            use_powershell,
                        ) {
                            live_sync = true;
                            if let Some(step_index) = step_index {
                                if let Some(group) = self
                                    .state
                                    .macro_groups
                                    .iter_mut()
                                    .find(|group| group.id == group_id)
                                {
                                    if let Some(preset) = group
                                        .presets
                                        .iter_mut()
                                        .find(|preset| preset.id == preset_id)
                                    {
                                        if let Some(step) = preset.steps.get_mut(step_index) {
                                            step.key = saved_id.to_string();
                                            step.command_preset_command = "".to_owned();
                                            step.command_preset_use_powershell = false;
                                        }
                                    }
                                }
                            } else {
                                if let Some(group) = self
                                    .state
                                    .macro_groups
                                    .iter_mut()
                                    .find(|group| group.id == group_id)
                                {
                                    if let Some(preset) = group
                                        .presets
                                        .iter_mut()
                                        .find(|preset| preset.id == preset_id)
                                    {
                                        preset.hold_stop_step.key = saved_id.to_string();
                                        preset.hold_stop_step.command_preset_command = "".to_owned();
                                        preset.hold_stop_step.command_preset_use_powershell = false;
                                    }
                                }
                            }
                            self.open_command_ai_dialog_for_preset(saved_id);
                        }
                    }
                    if let Some(preset_id) = pending_open_ai_preset_id.take() {
                        self.open_command_ai_dialog_for_preset(preset_id);
                    }
                    if cancel_active_capture {
                        self.cancel_capture();
                    }
                    if cancel_mouse_move_absolute_capture {
                        self.cancel_mouse_move_absolute_capture(ui.ctx());
                    }
                    if let Some(target) = begin_mouse_move_absolute_capture_target {
                        self.begin_mouse_move_absolute_capture(ui.ctx(), target);
                    }
                    if let Some(target) = next_capture_target {
                        self.begin_capture(target, "Capturing macro input.".to_owned());
                    }
                    if let Some((group_id, preset_id)) = copy_selected_steps {
                        self.copy_selected_macro_steps_for_preset(group_id, preset_id);
                    }
                    if let Some((group_id, preset_id, step_index)) = copy_single_step {
                        if let Some(group) = self.state.macro_groups.iter().find(|g| g.id == group_id) {
                            if let Some(preset) = group.presets.iter().find(|p| p.id == preset_id) {
                                if let Some(step) = preset.steps.get(step_index) {
                                    self.macro_step_clipboard = vec![step.clone()];
                                    self.status = format!("Copied 1 step.");
                                }
                            }
                        }
                    }
                    if let Some((group_id, preset_id)) = delete_selected_steps {
                        self.remove_selected_macro_steps_for_preset(group_id, preset_id);
                        live_sync = true;
                    }
                    if let Some((group_id, preset_id, step_index)) = paste_step_after
                        && let Some(selection) =
                            self.paste_macro_steps_after(group_id, preset_id, step_index)
                    {
                        clear_step_selection = Some((group_id, preset_id));
                        selection_after_paste = Some((group_id, preset_id, selection));
                        live_sync = true;
                    }
                    if let Some((group_id, preset_id, step_index, ctrl, shift)) =
                        pending_step_selection
                    {
                        if shift {
                            let num_steps = self.state.macro_groups
                                .iter()
                                .find(|g| g.id == group_id)
                                .and_then(|g| g.presets.iter().find(|p| p.id == preset_id))
                                .map(|p| p.steps.len())
                                .unwrap_or(0);
                            if let Some((anchor_group, anchor_preset, anchor_index)) = self.last_selected_macro_step
                                && anchor_group == group_id
                                && anchor_preset == preset_id
                                && anchor_index < num_steps
                                && step_index < num_steps
                            {
                                let start = std::cmp::min(anchor_index, step_index);
                                let end = std::cmp::max(anchor_index, step_index);
                                if !ctrl {
                                    self.clear_macro_step_selection_for_preset(group_id, preset_id);
                                }
                                for i in start..=end {
                                    self.selected_macro_steps.insert((group_id, preset_id, i));
                                }
                            } else {
                                self.clear_macro_step_selection_for_preset(group_id, preset_id);
                                self.selected_macro_steps.insert((group_id, preset_id, step_index));
                                self.last_selected_macro_step = Some((group_id, preset_id, step_index));
                            }
                        } else {
                            let currently_selected = self
                                .selected_macro_steps
                                .contains(&(group_id, preset_id, step_index));
                            let selected_count_in_preset = self
                                .selected_macro_steps
                                .iter()
                                .filter(|(selected_group, selected_preset, _)| {
                                    *selected_group == group_id && *selected_preset == preset_id
                                })
                                .count();
                            self.select_macro_step(
                                group_id,
                                preset_id,
                                step_index,
                                ctrl,
                                currently_selected,
                                selected_count_in_preset,
                            );
                            self.last_selected_macro_step = Some((group_id, preset_id, step_index));
                        }
                    }
                    if !ui.input(|input| input.pointer.primary_down()) {
                        self.macro_drag_select_anchor = None;
                    }
                    if let Some((group_id, preset_id)) = clear_step_selection {
                        self.clear_macro_step_selection_for_preset(group_id, preset_id);
                    }
                    if let Some((group_id, preset_id, pasted_indices)) = selection_after_paste {
                        self.clear_macro_step_selection_for_preset(group_id, preset_id);
                        for pasted_index in pasted_indices {
                            self.selected_macro_steps
                                .insert((group_id, preset_id, pasted_index));
                        }
                    }
                    if let Some((preset_id, step_index)) = export_step {
                        let step_opt = self.state.macro_groups.iter().flat_map(|g| &g.presets).find(|p| p.id == preset_id).and_then(|p| p.steps.get(step_index)).cloned();
                        if let Some(step) = step_opt {
                            self.export_macro_step(&step);
                        }
                    }
                    if let Some((group_id, preset_id, insert_after_index)) = import_step_to {
                        self.import_macro_step_from_clipboard(group_id, preset_id, insert_after_index);
                    }
                    if let Some(preset_id) = export_preset {
                        let preset_opt = self.state.macro_groups.iter().flat_map(|g| &g.presets).find(|p| p.id == preset_id).cloned();
                        if let Some(preset) = preset_opt {
                            self.export_macro_preset(&preset);
                        }
                    }
                    if let Some((group_id, insert_after_preset_id)) = import_preset_to_group {
                        self.import_macro_preset_from_clipboard(group_id, insert_after_preset_id);
                    }
                    if let Some(group_id) = export_group {
                        let group_opt = self.state.macro_groups.iter().find(|g| g.id == group_id).cloned();
                        if let Some(group) = group_opt {
                            self.export_macro_group(&group);
                        }
                    }
                    if let Some(group_id) = import_group_after {
                        self.import_macro_group_from_clipboard(None, Some(group_id));
                    }
                }
            }
        }
    }
        if let Some(group_id) = add_preset_to_group {
            self.add_macro_preset_to_group(group_id);
            self.persist();
        }
        if let Some(group_id) = paste_preset_to_group
            && let Some(source_preset) = self.macro_preset_clipboard.clone()
        {
            let copied_preset = self.clone_macro_preset_with_new_id(&source_preset);
            if let Some(group) = self
                .state
                .macro_groups
                .iter_mut()
                .find(|group| group.id == group_id)
            {
                group.presets.push(copied_preset);
                self.persist_macro_presets();
            }
        }
        if ui
            .memory(|mem| mem.data.get_temp::<bool>(egui::Id::new("macro_variable_suggestion_committed")))
            .unwrap_or(false)
        {
            live_sync = true;
        }
        if live_sync {
            self.persist_macro_presets();
        }
        if let Some(folder_id) = release_folder_id {
            self.state
                .macro_folders
                .retain(|folder| folder.id != folder_id);
            for group in &mut self.state.macro_groups {
                if group.folder_id == Some(folder_id) {
                    group.folder_id = None;
                }
            }
            self.confirm_release_folder_id = None;
            if self.active_macro_folder_view == Some(folder_id) {
                self.set_active_macro_folder_view(None);
            }
            self.persist_macro_presets();
        }
        if let Some(folder_id) = delete_folder_id {
            let should_confirm = self
                .state
                .macro_groups
                .iter()
                .any(|group| group.folder_id == Some(folder_id))
                && self.confirm_delete_folder_id != Some(folder_id);
            if should_confirm {
                self.confirm_delete_folder_id = Some(folder_id);
            } else {
                self.state
                    .macro_groups
                    .retain(|group| group.folder_id != Some(folder_id));
                self.state
                    .macro_folders
                    .retain(|folder| folder.id != folder_id);
                self.confirm_delete_folder_id = None;
                self.confirm_release_folder_id = None;
                if self.active_macro_folder_view == Some(folder_id) {
                    self.set_active_macro_folder_view(None);
                }
                self.persist_macro_presets();
            }
        }
        if let Some(folder_id) = enter_folder_id {
            self.set_active_macro_folder_view(Some(folder_id));
        }
        if let Some(id) = remove_group {
            let should_confirm = self.confirm_delete_macro_group_id != Some(id);
            if should_confirm {
                self.confirm_delete_macro_group_id = Some(id);
            } else {
                self.state.macro_groups.retain(|group| group.id != id);
                self.selected_macro_groups.remove(&id);
                self.macro_group_clipboard
                    .retain(|group_id| *group_id != id);
                self.confirm_delete_macro_group_id = None;
                self.persist_macro_presets();
            }
        }
        if let Some((folder_id, name)) = renamed_folder {
            if let Some(folder) = self
                .state
                .macro_folders
                .iter_mut()
                .find(|folder| folder.id == folder_id)
            {
                folder.name = name;
                self.persist();
            }
        }
        if let Some(folder_id) = toggle_collapsed_folder_id {
            if let Some(folder) = self
                .state
                .macro_folders
                .iter_mut()
                .find(|folder| folder.id == folder_id)
            {
                folder.collapsed = !folder.collapsed;
                self.persist();
            }
        }
        if let Some(folder_id) = add_group_to_folder_id {
            self.add_macro_group_to_folder(folder_id);
            if let Some(folder) = self
                .state
                .macro_folders
                .iter_mut()
                .find(|folder| folder.id == folder_id)
            {
                folder.collapsed = false;
            }
            self.persist();
        }
        let active_hover_preview_state = ui
            .ctx()
            .data(|data| data.get_temp::<Option<(u32, HoverPreviewRequest, egui::Pos2)>>(hover_preview_state_id))
            .flatten();
        if let Some((group_id, hover_request, anchor_pos)) = active_hover_preview_state.clone() {
            hover_preview = self.resolve_hover_preview_request(group_id, hover_request);
            if let Some(preview) = hover_preview.as_ref() {
                let popup_hovered =
                    Self::render_hover_preview_popup(ui.ctx(), language, Some(preview), anchor_pos);
                if !popup_hovered && hover_preview_request.is_none() {
                    ui.ctx()
                        .data_mut(|data| {
                            data.insert_temp(
                                hover_preview_state_id,
                                None::<Option<(u32, HoverPreviewRequest, egui::Pos2)>>,
                            )
                        });
                }
            }
        } else {
            ui.ctx()
                .data_mut(|data| {
                    data.insert_temp(
                        hover_preview_state_id,
                        None::<Option<(u32, HoverPreviewRequest, egui::Pos2)>>,
                    )
                });
        }
        if !pending_macro_group_scroll_consumed {
            self.pending_macro_group_scroll_target = pending_macro_group_scroll_target;
        }
        });
    }
    fn collect_all_macro_referenced_variables(&self) -> Vec<String> {
        let mut vars = std::collections::HashSet::new();
        for group in &self.state.macro_groups {
            for preset in &group.presets {
                for step in &preset.steps {
                    Self::collect_vars_from_step(step, &mut vars);
                }
                if preset.hold_stop_step_enabled {
                    Self::collect_vars_from_step(&preset.hold_stop_step, &mut vars);
                }
            }
        }
        let mut list: Vec<String> = vars.into_iter().collect();
        list.sort();
        list
    }
    fn collect_vars_from_step(step: &MacroStep, vars: &mut std::collections::HashSet<String>) {
        if step.action == MacroAction::SetVariable {
            let name = step.key.trim();
            if !name.is_empty() {
                vars.insert(name.to_string());
            }
        }
        if !step.if_variable_name.trim().is_empty() {
            vars.insert(step.if_variable_name.trim().to_string());
        }
        for cond in &step.extra_conditions {
            let name = cond.variable_name.trim();
            if !name.is_empty() {
                vars.insert(name.to_string());
            }
            Self::extract_vars_from_expression(&cond.expression, vars);
        }
        Self::extract_braced_vars(&step.delay_expr, vars);
        Self::extract_braced_vars(&step.text_override, vars);
        Self::extract_braced_vars(&step.command_preset_command, vars);
    }
    fn extract_braced_vars(text: &str, vars: &mut std::collections::HashSet<String>) {
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '{' {
                let mut name = String::new();
                let mut found = false;
                while let Some(&next_c) = chars.peek() {
                    if next_c == '}' {
                        chars.next();
                        found = true;
                        break;
                    } else if next_c == '{' {
                        break;
                    } else {
                        name.push(chars.next().unwrap());
                    }
                }
                if found {
                    Self::extract_vars_from_expression(&name, vars);
                }
            }
        }
    }
    fn extract_vars_from_expression(expr: &str, vars: &mut std::collections::HashSet<String>) {
        let mut current_var = String::new();
        for c in expr.chars() {
            if c.is_alphanumeric() || c == '_' {
                current_var.push(c);
            } else {
                let trimmed = current_var.trim();
                if !trimmed.is_empty() && !trimmed.chars().next().unwrap().is_ascii_digit() {
                    vars.insert(trimmed.to_string());
                }
                current_var.clear();
            }
        }
        let trimmed = current_var.trim();
        if !trimmed.is_empty() && !trimmed.chars().next().unwrap().is_ascii_digit() {
            vars.insert(trimmed.to_string());
        }
    }
    pub(crate) fn render_variable_inspector(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        ui.vertical(|ui| {
            ui.add_space(4.0);
            // Grid for global constants
            if !self.state.global_constants.is_empty() {
                egui::ScrollArea::vertical()
                    .id_salt("global_constants_scroll")
                    .max_height(100.0)
                    .show(ui, |ui| {
                        egui::Grid::new("global_constants_grid")
                            .num_columns(3)
                            .spacing([8.0, 6.0])
                            .striped(true)
                            .show(ui, |ui| {
                                let mut to_remove_idx = None;
                                let mut to_update = None;
                                for (idx, (name, val)) in
                                    self.state.global_constants.iter().enumerate()
                                {
                                    ui.label(
                                        RichText::new(name)
                                            .monospace()
                                            .color(Color32::from_rgb(0, 180, 216)),
                                    );
                                    let id_editing = ui.id().with(("var-edit", name));
                                    let mut val_str = ui.memory(|mem| mem.data.get_temp::<String>(id_editing))
                                        .unwrap_or_else(|| val.to_string());
                                    let response = ui.add(
                                        egui::TextEdit::singleline(&mut val_str)
                                            .desired_width(70.0)
                                            .font(egui::FontId::monospace(14.0)),
                                    );
                                    if response.changed() {
                                        ui.memory_mut(|mem| mem.data.insert_temp(id_editing, val_str.clone()));
                                    }
                                    if response.lost_focus() || response.clicked_elsewhere() {
                                        if let Ok(new_val) = val_str.trim().parse::<i32>() {
                                            to_update = Some((name.clone(), new_val));
                                        }
                                        ui.memory_mut(|mem| mem.data.remove_temp::<String>(id_editing));
                                    }
                                    if ui
                                        .button(Self::material_icon_text(0xe872, 14.0)) // trash
                                        .on_hover_text(Self::tr_lang(language, "Delete", "Xóa"))
                                        .clicked()
                                    {
                                        to_remove_idx = Some(idx);
                                    }
                                    ui.end_row();
                                }
                                if let Some(idx) = to_remove_idx {
                                    let (removed_name, _) = self.state.global_constants.remove(idx);
                                    let mut vars = crate::overlay::RUNTIME_VARIABLES.lock();
                                    vars.remove(&removed_name);
                                    self.persist();
                                } else if let Some((name_to_up, new_val)) = to_update {
                                    if let Some(pos) = self
                                        .state
                                        .global_constants
                                        .iter()
                                        .position(|(n, _)| n == &name_to_up)
                                    {
                                        self.state.global_constants[pos].1 = new_val;
                                        let mut vars = crate::overlay::RUNTIME_VARIABLES.lock();
                                        vars.insert(name_to_up, new_val);
                                        self.persist();
                                    }
                                }
                            });
                    });
            }
            // Quick add global constant
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let id_const_name = ui.id().with("new_const_name");
                let id_const_val = ui.id().with("new_const_val");
                let mut name_buf = ui.memory(|mem| {
                    mem.data
                        .get_temp::<String>(id_const_name)
                        .unwrap_or_default()
                });
                let mut val_buf = ui.memory(|mem| {
                    mem.data
                        .get_temp::<String>(id_const_val)
                        .unwrap_or_default()
                });
                let is_dark_theme = self.state.ui_theme == UiThemeMode::Dark;
                let hint_color = if is_dark_theme {
                    Color32::from_rgba_premultiplied(140, 140, 140, 150)
                } else {
                    Color32::from_rgba_premultiplied(100, 100, 100, 150)
                };
                ui.add_sized(
                    [100.0, 20.0],
                    egui::TextEdit::singleline(&mut name_buf).hint_text(
                        RichText::new(Self::tr_lang(language, "CONST_NAME", "CONST_NAME"))
                            .color(hint_color)
                            .weak(),
                    ),
                );
                ui.label("=");
                ui.add_sized(
                    [70.0, 20.0],
                    egui::TextEdit::singleline(&mut val_buf).hint_text(
                        RichText::new(Self::tr_lang(language, "Value", "Value"))
                            .color(hint_color)
                            .weak(),
                    ),
                );
                if ui.button(Self::tr_lang(language, "Add", "Add")).clicked() {
                    let name_trimmed = name_buf.trim().to_uppercase();
                    if !name_trimmed.is_empty() {
                        let parsed_val = val_buf.trim().parse::<i32>().unwrap_or(0);
                        if !self
                            .state
                            .global_constants
                            .iter()
                            .any(|(n, _)| n == &name_trimmed)
                        {
                            self.state
                                .global_constants
                                .push((name_trimmed.clone(), parsed_val));
                            let mut vars = crate::overlay::RUNTIME_VARIABLES.lock();
                            vars.insert(name_trimmed, parsed_val);
                            name_buf.clear();
                            val_buf.clear();
                            self.persist();
                        }
                    }
                }
                ui.memory_mut(|mem| {
                    mem.data.insert_temp(id_const_name, name_buf);
                    mem.data.insert_temp(id_const_val, val_buf);
                });
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            // Collect referenced variables statically + dynamic runtime variables
            let mut all_vars_set = std::collections::HashSet::new();
            for v in self.collect_all_macro_referenced_variables() {
                all_vars_set.insert(v);
            }
            {
                let vars = crate::overlay::RUNTIME_VARIABLES.lock();
                for k in vars.keys() {
                    if !self.state.global_constants.iter().any(|(n, _)| n == k) {
                        all_vars_set.insert(k.clone());
                    }
                }
            }
            let mut vars_list: Vec<String> = all_vars_set.into_iter().collect();
            vars_list.sort();
            if !vars_list.is_empty() {
                egui::ScrollArea::vertical()
                    .id_salt("macro_vars_scroll")
                    .max_height(160.0)
                    .show(ui, |ui| {
                        egui::Grid::new("macro_vars_grid")
                            .num_columns(3)
                            .spacing([8.0, 6.0])
                            .striped(true)
                            .show(ui, |ui| {
                                let mut to_remove = None;
                                let mut to_update = None;
                                for name in &vars_list {
                                    ui.label(
                                        RichText::new(name)
                                            .monospace()
                                            .color(Color32::from_rgb(243, 156, 18)),
                                    );
                                    let runtime_val = {
                                        let vars = crate::overlay::RUNTIME_VARIABLES.lock();
                                        vars.get(name).copied().unwrap_or(0)
                                    };
                                    let id_editing = ui.id().with(("var-edit", name));
                                    let mut val_str = ui.memory(|mem| mem.data.get_temp::<String>(id_editing))
                                        .unwrap_or_else(|| runtime_val.to_string());
                                    let response = ui.add(
                                        egui::TextEdit::singleline(&mut val_str)
                                            .desired_width(70.0)
                                            .font(egui::FontId::monospace(14.0)),
                                    );
                                    if response.changed() {
                                        ui.memory_mut(|mem| mem.data.insert_temp(id_editing, val_str.clone()));
                                    }
                                    if response.lost_focus() || response.clicked_elsewhere() {
                                        if let Ok(new_val) = val_str.trim().parse::<i32>() {
                                            to_update = Some((name.clone(), new_val));
                                        }
                                        ui.memory_mut(|mem| mem.data.remove_temp::<String>(id_editing));
                                    }
                                    if ui
                                        .button(Self::material_icon_text(0xe872, 14.0))
                                        .on_hover_text(Self::tr_lang(language, "Delete", "Xóa"))
                                        .clicked()
                                    {
                                        to_remove = Some(name.clone());
                                    }
                                    ui.end_row();
                                }
                                if let Some(name) = to_remove {
                                    let mut vars = crate::overlay::RUNTIME_VARIABLES.lock();
                                    vars.remove(&name);
                                } else if let Some((name, new_val)) = to_update {
                                    let mut vars = crate::overlay::RUNTIME_VARIABLES.lock();
                                    vars.insert(name, new_val);
                                }
                            });
                    });
            }
            // Quick set dynamic variable at the bottom
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.set_row_height(24.0);
                let id_name = ui.id().with("new_dyn_var_name");
                let id_val = ui.id().with("new_dyn_var_val");
                let mut name_buf =
                    ui.memory(|mem| mem.data.get_temp::<String>(id_name).unwrap_or_default());
                let mut val_buf =
                    ui.memory(|mem| mem.data.get_temp::<String>(id_val).unwrap_or_default());
                let is_dark_theme = self.state.ui_theme == UiThemeMode::Dark;
                let hint_color = if is_dark_theme {
                    Color32::from_rgba_premultiplied(140, 140, 140, 150)
                } else {
                    Color32::from_rgba_premultiplied(100, 100, 100, 150)
                };
                ui.add_sized(
                    [100.0, 20.0],
                    egui::TextEdit::singleline(&mut name_buf).hint_text(
                        RichText::new(Self::tr_lang(language, "Var Name", "Var Name"))
                            .color(hint_color)
                            .weak(),
                    ),
                );
                ui.label("=");
                ui.add_sized(
                    [70.0, 20.0],
                    egui::TextEdit::singleline(&mut val_buf).hint_text(
                        RichText::new(Self::tr_lang(language, "Value", "Value"))
                            .color(hint_color)
                            .weak(),
                    ),
                );
                if ui.button(Self::tr_lang(language, "Set", "Set")).clicked() {
                    let name_trimmed = name_buf.trim().to_string();
                    if !name_trimmed.is_empty() {
                        let parsed_val = val_buf.trim().parse::<i32>().unwrap_or(0);
                        let mut vars = crate::overlay::RUNTIME_VARIABLES.lock();
                        vars.insert(name_trimmed, parsed_val);
                        name_buf.clear();
                        val_buf.clear();
                    }
                }
                ui.memory_mut(|mem| {
                    mem.data.insert_temp(id_name, name_buf);
                    mem.data.insert_temp(id_val, val_buf);
                });
            });
        });
    }
    fn adjust_expression_by_delta(expr: &str, delta: i32) -> String {
        if delta == 0 {
            return expr.to_string();
        }
        let trimmed = expr.trim();
        if trimmed.is_empty() {
            return delta.max(0).to_string();
        }
        if let Ok(val) = trimmed.parse::<i32>() {
            return (val + delta).max(0).to_string();
        }
        if let Some(pos) = trimmed.rfind(|c| c == '+' || c == '-') {
            let (left, right) = trimmed.split_at(pos);
            let op = &right[0..1];
            let num_part = right[1..].trim();
            if let Ok(num) = num_part.parse::<i32>() {
                let signed_num = if op == "-" { -num } else { num };
                let new_num = signed_num + delta;
                let left_trimmed = left.trim_end();
                if new_num == 0 {
                    return left_trimmed.to_string();
                } else if new_num > 0 {
                    return format!("{} + {}", left_trimmed, new_num);
                } else {
                    return format!("{} - {}", left_trimmed, -new_num);
                }
            }
        }
        if delta > 0 {
            format!("{} + {}", trimmed, delta)
        } else {
            format!("{} - {}", trimmed, -delta)
        }
    }
    fn builtin_variable_suggestions() -> &'static [&'static str] {
        &[
            "System",
            "Screen",
            "Mouse",
            "Window",
            "Volume",
            "Clipboard",
        ]
    }
    fn object_property_suggestions(base: &str) -> Option<&'static [&'static str]> {
        match base.to_ascii_lowercase().as_str() {
            "system" => Some(&["date", "time", "year", "month", "day", "hour", "minute", "second", "millisecond"]),
            s if s.starts_with("timer") => Some(&["hour", "minute", "second", "millisecond", "ms", "raw", "total_sec"]),
            "screen" => Some(&["width", "height"]),
            "mouse" => Some(&["x", "y", "sensitivity"]),
            "volume" => Some(&["level"]),
            "window" => Some(&["title", "width", "height"]),
            "clipboard" => Some(&["text"]),
            _ => None,
        }
    }
    fn timer_ref_index(ref_name: &str) -> Option<usize> {
        let normalized = ref_name.trim().replace(' ', "").to_lowercase();
        let idx_str = normalized.strip_prefix("timer")?;
        let idx = idx_str.parse::<usize>().ok()?;
        idx.checked_sub(1)
    }
    fn timer_suggestion_label(suggestion: &str, timer_names: &[String]) -> String {
        if let Some((base, prop)) = suggestion.split_once('.') {
            if let Some(idx) = Self::timer_ref_index(base)
                && let Some(timer_name) = timer_names.get(idx)
            {
                return format!("{}.{} ({})", base, prop, timer_name);
            }
        } else if let Some(idx) = Self::timer_ref_index(suggestion)
            && let Some(timer_name) = timer_names.get(idx)
        {
            return format!("{} ({})", suggestion, timer_name);
        }
        suggestion.to_string()
    }
    fn variable_value_kind(token: &str) -> VariableValueKind {
        let trimmed = token.trim().trim_matches(|c| c == '{' || c == '}');
        if trimmed.is_empty() {
            return VariableValueKind::Neutral;
        }

        if let Some((base, prop)) = trimmed.split_once('.') {
            let base = base.trim().replace(' ', "").to_ascii_lowercase();
            let prop = prop.trim().to_ascii_lowercase();
            if base.is_empty() || prop.is_empty() {
                return VariableValueKind::Neutral;
            }

            return match base.as_str() {
                "system" => match prop.as_str() {
                    "date" | "time" => VariableValueKind::Text,
                    "year" | "month" | "day" | "hour" | "minute" | "second" | "millisecond" | "ms" => VariableValueKind::Number,
                    _ => VariableValueKind::Neutral,
                },
                "screen" => match prop.as_str() {
                    "width" | "height" | "w" | "h" => VariableValueKind::Number,
                    _ => VariableValueKind::Neutral,
                },
                "mouse" => match prop.as_str() {
                    "x" | "y" | "sensitivity" => VariableValueKind::Number,
                    _ => VariableValueKind::Neutral,
                },
                "window" => match prop.as_str() {
                    "title" => VariableValueKind::Text,
                    "width" | "height" | "w" | "h" => VariableValueKind::Number,
                    _ => VariableValueKind::Neutral,
                },
                "volume" => match prop.as_str() {
                    "level" | "percent" | "value" => VariableValueKind::Number,
                    _ => VariableValueKind::Neutral,
                },
                "clipboard" => match prop.as_str() {
                    "text" => VariableValueKind::Text,
                    _ => VariableValueKind::Neutral,
                },
                s if s.starts_with("timer") => match prop.as_str() {
                    "hour" | "minute" | "second" | "millisecond" | "ms" | "raw" | "total_sec" => VariableValueKind::Number,
                    _ => VariableValueKind::Neutral,
                },
                _ => VariableValueKind::Neutral,
            };
        }

        if Self::builtin_variable_suggestions()
            .iter()
            .any(|name| name.eq_ignore_ascii_case(trimmed))
            || Self::timer_ref_index(trimmed).is_some()
        {
            return VariableValueKind::Neutral;
        }

        VariableValueKind::Number
    }
    fn variable_highlight_job(
        ui: &egui::Ui,
        text: &str,
        wrap_width: f32,
    ) -> egui::text::LayoutJob {
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = wrap_width;
        let font_id = egui::TextStyle::Body.resolve(ui.style());
        let default_color = ui.visuals().text_color();

        let mut segment_start = 0;
        let mut chars = text.char_indices().peekable();
        while let Some((idx, ch)) = chars.next() {
            let is_token_char = ch.is_ascii_alphanumeric() || ch == '_' || ch == '.';
            if !is_token_char {
                continue;
            }

            if segment_start < idx {
                job.append(
                    &text[segment_start..idx],
                    0.0,
                    egui::text::TextFormat::simple(font_id.clone(), default_color),
                );
            }

            let mut end = idx + ch.len_utf8();
            while let Some(&(next_idx, next_ch)) = chars.peek() {
                if next_ch.is_ascii_alphanumeric() || next_ch == '_' || next_ch == '.' {
                    chars.next();
                    end = next_idx + next_ch.len_utf8();
                } else {
                    break;
                }
            }

            let token = &text[idx..end];
            let color = match Self::variable_value_kind(token) {
                VariableValueKind::Text => Color32::from_rgb(255, 185, 92),
                VariableValueKind::Number => Color32::from_rgb(86, 198, 255),
                VariableValueKind::Neutral => default_color,
            };
            job.append(
                token,
                0.0,
                egui::text::TextFormat::simple(font_id.clone(), color),
            );
            segment_start = end;
        }

        if segment_start < text.len() {
            job.append(
                &text[segment_start..],
                0.0,
                egui::text::TextFormat::simple(font_id, default_color),
            );
        }

        job
    }
    fn apply_variable_suggestion(
        ui: &mut egui::Ui,
        response: &egui::Response,
        text: &mut String,
        prefix: &str,
        chosen: &str,
        wrap_open: bool,
        after_cursor: &str,
    ) {
        let suffix = if wrap_open && after_cursor.starts_with('}') {
            &after_cursor['}'.len_utf8()..]
        } else {
            after_cursor
        };
        let closing = if wrap_open { "}" } else { "" };
        *text = format!("{}{}{}{}", prefix, chosen, closing, suffix);
        let mut response = response.clone();
        response.mark_changed();
        response.request_focus();

        let char_count = text.chars().count();
        if let Some(mut state) = egui::widgets::text_edit::TextEditState::load(ui.ctx(), response.id) {
            let cursor_pos = egui::text::CCursor::new(char_count);
            state.cursor.set_char_range(Some(egui::text::CCursorRange::two(cursor_pos, cursor_pos)));
            state.store(ui.ctx(), response.id);
        }
    }
    fn render_variable_suggestions(
        ui: &mut egui::Ui,
        response: &egui::Response,
        text: &mut String,
        timer_names: &[String],
        _language: UiLanguage,
    ) {
        let suggestion_names = ui
            .memory(|mem| mem.data.get_temp::<Vec<String>>(egui::Id::new("macro_variable_suggestion_names")))
            .unwrap_or_default();
        let cursor_index = match egui::widgets::text_edit::TextEditState::load(ui.ctx(), response.id)
            .and_then(|state| state.cursor.char_range().and_then(|range| range.single().map(|c| c.index)))
        {
            Some(index) => index,
            None => return,
        };
        let cursor_byte = text
            .char_indices()
            .nth(cursor_index)
            .map(|(byte, _)| byte)
            .unwrap_or(text.len());
        let before_cursor = &text[..cursor_byte];
        let after_cursor = text[cursor_byte..].to_string();

        let mut last_word_start = 0;
        for (i, c) in before_cursor.char_indices() {
            if c.is_whitespace()
                || c == '+'
                || c == '-'
                || c == '*'
                || c == '/'
                || c == '('
                || c == ')'
                || c == ','
                || c == '{'
                || c == '}'
            {
                last_word_start = i + c.len_utf8();
            }
        }
        let prefix = before_cursor[..last_word_start].to_string();
        let last_word_trimmed = before_cursor[last_word_start..].trim().to_string();
        let wrap_open = prefix.ends_with('{');
        
        if last_word_trimmed.is_empty() {
            return;
        }

        let mut suggestions = Vec::new();

        if last_word_trimmed.contains('.') {
            let parts: Vec<&str> = last_word_trimmed.split('.').collect();
            let obj_part = parts[0].to_lowercase();
            let prop_part = parts[1].to_lowercase();

            let timer_exists = Self::timer_ref_index(&parts[0]).is_some()
                || timer_names.iter().any(|name| name.replace(" ", "").to_lowercase() == obj_part);

            let props: Vec<&str> = if timer_exists {
                vec!["hour", "minute", "second", "millisecond", "ms", "raw", "total_sec"]
            } else {
                Self::object_property_suggestions(&parts[0]).map_or_else(Vec::new, |props| props.to_vec())
            };

            for prop in props {
                let full_prop = format!("{}.{}", parts[0], prop);
                if prop.starts_with(&prop_part) && full_prop.to_lowercase() != last_word_trimmed.to_lowercase() {
                    suggestions.push(full_prop);
                }
            }
        } else {
            for name in &suggestion_names {
                let name_no_space = name.replace(" ", "");
                if name_no_space.to_lowercase().starts_with(&last_word_trimmed.to_lowercase()) 
                   && name_no_space.to_lowercase() != last_word_trimmed.to_lowercase() {
                    suggestions.push(name_no_space);
                }
            }
        }

        if suggestions.is_empty() {
            return;
        }

        let popup_open_key = response.id.with("popup_open");
        let mut popup_open = ui.memory(|mem| mem.data.get_temp::<bool>(popup_open_key)).unwrap_or(false);
        
        if response.has_focus() {
            popup_open = true;
        } else {
            let popup_rect = ui.memory(|mem| mem.data.get_temp::<egui::Rect>(response.id.with("popup_rect")));
            if let Some(rect) = popup_rect {
                let hover = ui.input(|i| i.pointer.hover_pos().map_or(false, |pos| rect.contains(pos)));
                let mouse_down = ui.input(|i| i.pointer.any_down());
                if !hover && mouse_down {
                    popup_open = false;
                }
            } else {
                popup_open = false;
            }
        }

        if !popup_open {
            return;
        }

        let mut selected_index = ui.memory(|mem| mem.data.get_temp::<usize>(response.id)).unwrap_or(0);
        let mut confirm_selected = false;
        let mut selection_changed = false;
        let sug_count = suggestions.len();
        
        if selected_index >= sug_count {
            selected_index = 0;
        }

        if response.has_focus() {
            let enter_pressed = ui.memory(|mem| mem.data.get_temp::<bool>(egui::Id::new("enter_pressed"))).unwrap_or(false);
            if enter_pressed {
                confirm_selected = true;
            }
            let arrow_up_pressed = ui.memory(|mem| mem.data.get_temp::<bool>(egui::Id::new("arrow_up_pressed"))).unwrap_or(false);
            let arrow_down_pressed = ui.memory(|mem| mem.data.get_temp::<bool>(egui::Id::new("arrow_down_pressed"))).unwrap_or(false);
            if arrow_down_pressed {
                selected_index = (selected_index + 1) % sug_count;
                selection_changed = true;
            }
            if arrow_up_pressed {
                selected_index = if selected_index == 0 { sug_count - 1 } else { selected_index - 1 };
                selection_changed = true;
            }
            ui.memory_mut(|mem| mem.data.insert_temp(response.id, selected_index));
        }

        if confirm_selected {
            let chosen = &suggestions[selected_index];
            Self::apply_variable_suggestion(ui, response, text, &prefix, chosen, wrap_open, &after_cursor);
            ui.memory_mut(|mem| {
                mem.data.insert_temp(egui::Id::new("macro_variable_suggestion_committed"), true);
            });
            popup_open = false;
            ui.memory_mut(|mem| {
                mem.data.insert_temp(popup_open_key, popup_open);
                mem.data.insert_temp(egui::Id::new("enter_pressed"), false);
            });
            return;
        }

        let popup_id = response.id.with("sug_popup");
        let popup_position = response.rect.left_bottom();
        let mut clicked_choice: Option<String> = None;
        let popup_max_height = (ui.ctx().content_rect().bottom() - popup_position.y - 8.0).max(120.0);
        
        let area_res = egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .fixed_pos(popup_position)
            .show(ui.ctx(), |ui| {
                let frame_res = egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(200.0);
                    egui::ScrollArea::vertical()
                        .max_height(popup_max_height)
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                for (idx, sug) in suggestions.iter().enumerate() {
                                    let is_selected = idx == selected_index;
                                    let label = Self::timer_suggestion_label(sug, timer_names);
                                    let color = match Self::variable_value_kind(sug) {
                                        VariableValueKind::Text => Color32::from_rgb(255, 185, 92),
                                        VariableValueKind::Number => Color32::from_rgb(86, 198, 255),
                                        VariableValueKind::Neutral => ui.visuals().text_color(),
                                    };
                                    let mut resp = ui.selectable_label(
                                        is_selected,
                                        RichText::new(label).color(color),
                                    );
                                    if is_selected && selection_changed {
                                        resp.scroll_to_me(None);
                                    }
                                    if resp.clicked() {
                                        clicked_choice = Some(sug.clone());
                                    }
                                }
                            });
                        });
                });
                
                let rect = frame_res.response.rect;
                ui.memory_mut(|mem| mem.data.insert_temp(response.id.with("popup_rect"), rect));
            });

        if let Some(chosen) = clicked_choice {
            Self::apply_variable_suggestion(ui, response, text, &prefix, &chosen, wrap_open, &after_cursor);
            ui.memory_mut(|mem| {
                mem.data.insert_temp(egui::Id::new("macro_variable_suggestion_committed"), true);
            });
            popup_open = false;
            ui.memory_mut(|mem| {
                mem.data.insert_temp(popup_open_key, popup_open);
                mem.data.insert_temp(egui::Id::new("enter_pressed"), false);
            });
            return;
        }
            
        ui.memory_mut(|mem| {
            mem.data.insert_temp(popup_open_key, popup_open);
            mem.data.insert_temp(egui::Id::new("any_popup_open"), true);
        });
    }

    fn render_variable_suggestions_raw(
        ui: &mut egui::Ui,
        response: &egui::Response,
        text: &mut String,
        timer_names: &[String],
        _language: UiLanguage,
    ) {
        let suggestion_names = ui
            .memory(|mem| mem.data.get_temp::<Vec<String>>(egui::Id::new("macro_variable_writable_suggestion_names")))
            .unwrap_or_default();
        let cursor_index = match egui::widgets::text_edit::TextEditState::load(ui.ctx(), response.id)
            .and_then(|state| state.cursor.char_range().and_then(|range| range.single().map(|c| c.index)))
        {
            Some(index) => index,
            None => return,
        };
        let cursor_byte = text
            .char_indices()
            .nth(cursor_index)
            .map(|(byte, _)| byte)
            .unwrap_or(text.len());
        let before_cursor = &text[..cursor_byte];
        let after_cursor = text[cursor_byte..].to_string();

        let mut last_word_start = 0;
        for (i, c) in before_cursor.char_indices() {
            if c.is_whitespace()
                || c == '+'
                || c == '-'
                || c == '*'
                || c == '/'
                || c == '('
                || c == ')'
                || c == ','
                || c == '{'
                || c == '}'
            {
                last_word_start = i + c.len_utf8();
            }
        }
        let prefix = before_cursor[..last_word_start].to_string();
        let last_word = before_cursor[last_word_start..].trim().to_string();
        let wrap_open = prefix.ends_with('{');

        if last_word.is_empty() {
            return;
        }

        let mut suggestions = Vec::new();

        if last_word.contains('.') {
            let parts: Vec<&str> = last_word.split('.').collect();
            let prop_part = parts.get(1).map(|s| s.to_lowercase()).unwrap_or_default();
            if Self::timer_ref_index(parts[0]).is_some() {
                for prop in ["hour", "minute", "second", "millisecond", "ms", "raw", "total_sec"] {
                    let full_prop = format!("{}.{}", parts[0], prop);
                    if prop.starts_with(&prop_part) && full_prop.to_lowercase() != last_word.to_lowercase() {
                        suggestions.push(full_prop);
                    }
                }
            }
        } else {
            for name in &suggestion_names {
                let name_no_space = name.replace(" ", "");
                if name_no_space.to_lowercase().starts_with(&last_word.to_lowercase()) 
                   && name_no_space.to_lowercase() != last_word.to_lowercase() {
                    suggestions.push(name_no_space);
                }
            }
        }

        if suggestions.is_empty() {
            return;
        }

        let popup_open_key = response.id.with("popup_open_raw");
        let mut popup_open = ui.memory(|mem| mem.data.get_temp::<bool>(popup_open_key)).unwrap_or(false);
        
        if response.has_focus() {
            popup_open = true;
        } else {
            let popup_rect = ui.memory(|mem| mem.data.get_temp::<egui::Rect>(response.id.with("popup_rect_raw")));
            if let Some(rect) = popup_rect {
                let hover = ui.input(|i| i.pointer.hover_pos().map_or(false, |pos| rect.contains(pos)));
                let mouse_down = ui.input(|i| i.pointer.any_down());
                if !hover && mouse_down {
                    popup_open = false;
                }
            } else {
                popup_open = false;
            }
        }

        if !popup_open {
            return;
        }

        let mut selected_index = ui.memory(|mem| mem.data.get_temp::<usize>(response.id)).unwrap_or(0);
        let mut confirm_selected = false;
        let mut selection_changed = false;
        let sug_count = suggestions.len();
        
        if selected_index >= sug_count {
            selected_index = 0;
        }

        if response.has_focus() {
            let enter_pressed = ui.memory(|mem| mem.data.get_temp::<bool>(egui::Id::new("enter_pressed"))).unwrap_or(false);
            if enter_pressed {
                confirm_selected = true;
            }
            let arrow_up_pressed = ui.memory(|mem| mem.data.get_temp::<bool>(egui::Id::new("arrow_up_pressed"))).unwrap_or(false);
            let arrow_down_pressed = ui.memory(|mem| mem.data.get_temp::<bool>(egui::Id::new("arrow_down_pressed"))).unwrap_or(false);
            if arrow_down_pressed {
                selected_index = (selected_index + 1) % sug_count;
                selection_changed = true;
            }
            if arrow_up_pressed {
                selected_index = if selected_index == 0 { sug_count - 1 } else { selected_index - 1 };
                selection_changed = true;
            }
            ui.memory_mut(|mem| mem.data.insert_temp(response.id, selected_index));
        }

        if confirm_selected {
            let chosen = &suggestions[selected_index];
            let suffix = if wrap_open && after_cursor.starts_with('}') {
                &after_cursor['}'.len_utf8()..]
            } else {
                after_cursor.as_str()
            };
            let closing = if wrap_open { "}" } else { "" };
            *text = format!("{}{}{}{}", prefix, chosen, closing, suffix);
            response.request_focus();
            
            let char_count = text.chars().count();
            if let Some(mut state) = egui::widgets::text_edit::TextEditState::load(ui.ctx(), response.id) {
                let cursor_pos = egui::text::CCursor::new(char_count);
                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(cursor_pos, cursor_pos)));
                state.store(ui.ctx(), response.id);
            }

            popup_open = false;
            ui.memory_mut(|mem| {
                mem.data.insert_temp(popup_open_key, popup_open);
                mem.data.insert_temp(egui::Id::new("enter_pressed"), false);
            });
            return;
        }

        let popup_id = response.id.with("sug_popup_raw");
        let popup_position = response.rect.left_bottom();
        let mut clicked_choice: Option<String> = None;
        let popup_max_height = (ui.ctx().content_rect().bottom() - popup_position.y - 8.0).max(120.0);
        
        let area_res = egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .fixed_pos(popup_position)
            .show(ui.ctx(), |ui| {
                let frame_res = egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(200.0);
                    egui::ScrollArea::vertical()
                        .max_height(popup_max_height)
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                for (idx, sug) in suggestions.iter().enumerate() {
                                    let is_selected = idx == selected_index;
                                    let label = Self::timer_suggestion_label(sug, timer_names);
                                    let color = match Self::variable_value_kind(sug) {
                                        VariableValueKind::Text => Color32::from_rgb(255, 185, 92),
                                        VariableValueKind::Number => Color32::from_rgb(86, 198, 255),
                                        VariableValueKind::Neutral => ui.visuals().text_color(),
                                    };
                                    let mut resp = ui.selectable_label(
                                        is_selected,
                                        RichText::new(label).color(color),
                                    );
                                    if is_selected && selection_changed {
                                        resp.scroll_to_me(None);
                                    }
                                    if resp.clicked() {
                                        clicked_choice = Some(sug.clone());
                                    }
                                }
                            });
                        });
                });
                
                let rect = frame_res.response.rect;
                ui.memory_mut(|mem| mem.data.insert_temp(response.id.with("popup_rect_raw"), rect));
            });

        if let Some(chosen) = clicked_choice {
            Self::apply_variable_suggestion(ui, response, text, &prefix, &chosen, wrap_open, &after_cursor);
            popup_open = false;
            ui.memory_mut(|mem| {
                mem.data.insert_temp(popup_open_key, popup_open);
                mem.data.insert_temp(egui::Id::new("enter_pressed"), false);
            });
            return;
        }
            
        ui.memory_mut(|mem| {
            mem.data.insert_temp(popup_open_key, popup_open);
            mem.data.insert_temp(egui::Id::new("any_popup_open"), true);
        });
    }

    fn render_expandable_text_edit_impl(
        ui: &mut egui::Ui,
        text: &mut String,
        id: egui::Id,
        normal_width: f32,
        expanded_width: f32,
        normal_height: f32,
        expanded_height: f32,
        hint: &str,
        multiline_on_focus: bool,
        highlight_variables: bool,
    ) -> egui::Response {
        let focus_key = id.with("expand-focus");
        let has_focus = ui.memory(|mem| mem.data.get_temp::<bool>(focus_key)).unwrap_or(false);

        let target_width = if has_focus { expanded_width } else { normal_width };

        // Calculate dynamic height based on text content when focused
        let target_height = if has_focus {
            let chars_per_line = ((expanded_width / 7.2) as usize).max(10);
            let mut estimated_rows = 0;
            for line in text.split('\n') {
                let line_len = line.chars().count();
                estimated_rows += 1 + line_len / chars_per_line;
            }
            let rows = estimated_rows.clamp(1, 12);
            
            if multiline_on_focus || rows > 1 {
                (rows as f32 * 18.0 + 6.0).max(expanded_height)
            } else {
                expanded_height
            }
        } else {
            normal_height
        };

        let animated_width = ui.ctx().animate_value_with_time(id.with("w"), target_width, 0.20);
        let animated_height = ui.ctx().animate_value_with_time(id.with("h"), target_height, 0.20);

        let is_multiline = has_focus && (multiline_on_focus || (text.chars().count() > (expanded_width / 7.2) as usize) || text.contains('\n'));

        let text_edit = if is_multiline {
            let chars_per_line = ((expanded_width / 7.2) as usize).max(10);
            let mut estimated_rows = 0;
            for line in text.split('\n') {
                let line_len = line.chars().count();
                estimated_rows += 1 + line_len / chars_per_line;
            }
            let rows = estimated_rows.clamp(1, 12);

            egui::TextEdit::multiline(text)
                .hint_text(hint)
                .desired_rows(rows)
                .id(id)
        } else {
            egui::TextEdit::singleline(text)
                .hint_text(hint)
                .id(id)
        };

        let mut layouter = |ui: &egui::Ui, string: &dyn TextBuffer, wrap_width: f32| {
            let job = Self::variable_highlight_job(ui, string.as_str(), wrap_width);
            ui.fonts_mut(|fonts| fonts.layout_job(job))
        };
        let text_edit = if highlight_variables {
            text_edit.layouter(&mut layouter)
        } else {
            text_edit
        };

        // Temporarily clear override_text_color so hint/placeholder text is properly dimmed.
        // Preset cards set override_text_color for their content, which bleeds into TextEdit
        // and makes hint text appear at full brightness instead of the dimmed weak_text_color.
        let prev_override = ui.visuals().override_text_color;
        ui.visuals_mut().override_text_color = None;
        let response = ui.add_sized([animated_width, animated_height], text_edit);
        ui.visuals_mut().override_text_color = prev_override;

        let now_focused = response.has_focus();
        if now_focused != has_focus {
            ui.memory_mut(|mem| mem.data.insert_temp(focus_key, now_focused));
        }

        response
    }
    fn render_expandable_text_edit(
        ui: &mut egui::Ui,
        text: &mut String,
        id: egui::Id,
        normal_width: f32,
        expanded_width: f32,
        normal_height: f32,
        expanded_height: f32,
        hint: &str,
        multiline_on_focus: bool,
    ) -> egui::Response {
        Self::render_expandable_text_edit_impl(
            ui,
            text,
            id,
            normal_width,
            expanded_width,
            normal_height,
            expanded_height,
            hint,
            multiline_on_focus,
            false,
        )
    }
    fn render_variable_text_edit(
        ui: &mut egui::Ui,
        text: &mut String,
        id: egui::Id,
        normal_width: f32,
        expanded_width: f32,
        normal_height: f32,
        expanded_height: f32,
        hint: &str,
        multiline_on_focus: bool,
    ) -> egui::Response {
        Self::render_expandable_text_edit_impl(
            ui,
            text,
            id,
            normal_width,
            expanded_width,
            normal_height,
            expanded_height,
            hint,
            multiline_on_focus,
            true,
        )
    }

    fn render_expandable_command_text_edit(
        ui: &mut egui::Ui,
        text: &mut String,
        id: egui::Id,
        hint: &str,
    ) -> egui::Response {
        let focus_key = id.with("expand-focus");
        let has_focus = ui.memory(|mem| mem.data.get_temp::<bool>(focus_key)).unwrap_or(false);

        let target_height = if has_focus { 160.0 } else { 72.0 };
        let animated_height = ui.ctx().animate_value_with_time(id.with("h"), target_height, 0.20);

        let response = ui.add_sized(
            [300.0, animated_height],
            egui::TextEdit::multiline(text)
                .hint_text(hint)
                .desired_rows(if has_focus { 7 } else { 3 })
                .id(id),
        );

        let now_focused = response.has_focus();
        if now_focused != has_focus {
            ui.memory_mut(|mem| mem.data.insert_temp(focus_key, now_focused));
        }

        response
    }
}
