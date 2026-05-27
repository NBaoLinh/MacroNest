use crate::model::*;
use crate::overlay::OverlayCommand;
use crate::ui::{CrosshairApp, AudioCardOutcome, AudioEditorTarget, video_duration};
use crate::audio;
use eframe::egui::{self, *};

impl CrosshairApp {
    fn render_audio_trim_bar(
        ui: &mut egui::Ui,
        id_source: impl std::hash::Hash + Copy,
        clip: &mut AudioClipSettings,
        total_ms: u64,
        waveform: Option<&[f32]>,
        desired_height: f32,
    ) -> bool {
        Self::trim_audio_bounds(clip, total_ms);
        let desired_size = vec2(ui.available_width().max(220.0), desired_height);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, 8.0, ui.visuals().extreme_bg_color);

        if let Some(waveform) = waveform.filter(|waveform| !waveform.is_empty()) {
            let bar_width = rect.width() / waveform.len() as f32;
            for (index, level) in waveform.iter().enumerate() {
                let amplitude = level.clamp(0.04, 1.0);
                let center_x = rect.left() + (index as f32 + 0.5) * bar_width;
                let half_height = amplitude * rect.height() * 0.42;
                let wave_rect = egui::Rect::from_min_max(
                    egui::pos2(
                        center_x - (bar_width * 0.35).max(1.0),
                        rect.center().y - half_height,
                    ),
                    egui::pos2(
                        center_x + (bar_width * 0.35).max(1.0),
                        rect.center().y + half_height,
                    ),
                );
                painter.rect_filled(wave_rect, 1.0, Color32::from_rgb(96, 172, 224));
            }
        } else {
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.center().y),
                    egui::pos2(rect.right(), rect.center().y),
                ],
                egui::Stroke::new(2.0, Color32::from_gray(120)),
            );
        }

        let total_ms_f32 = total_ms as f32;
        let start_t = if total_ms == 0 {
            0.0
        } else {
            clip.start_ms as f32 / total_ms_f32
        };
        let end_t = if total_ms == 0 {
            1.0
        } else {
            clip.end_ms as f32 / total_ms_f32
        };
        let start_x = rect.left() + rect.width() * start_t.clamp(0.0, 1.0);
        let end_x = rect.left() + rect.width() * end_t.clamp(0.0, 1.0);

        let selected_rect = egui::Rect::from_min_max(
            egui::pos2(start_x, rect.top()),
            egui::pos2(end_x.max(start_x + 2.0), rect.bottom()),
        );
        painter.rect_filled(
            selected_rect,
            8.0,
            Color32::from_rgba_premultiplied(72, 198, 120, 70),
        );
        painter.line_segment(
            [
                egui::pos2(start_x, rect.top()),
                egui::pos2(start_x, rect.bottom()),
            ],
            egui::Stroke::new(2.0, Color32::from_rgb(255, 232, 96)),
        );
        painter.line_segment(
            [
                egui::pos2(end_x, rect.top()),
                egui::pos2(end_x, rect.bottom()),
            ],
            egui::Stroke::new(2.0, Color32::from_rgb(255, 232, 96)),
        );

        let start_handle_rect = egui::Rect::from_center_size(
            egui::pos2(start_x, rect.center().y),
            vec2(20.0, rect.height()),
        );
        let end_handle_rect = egui::Rect::from_center_size(
            egui::pos2(end_x, rect.center().y),
            vec2(20.0, rect.height()),
        );
        let start_response = ui.interact(
            start_handle_rect,
            ui.make_persistent_id((id_source, "trim-start")),
            Sense::click_and_drag(),
        );
        let end_response = ui.interact(
            end_handle_rect,
            ui.make_persistent_id((id_source, "trim-end")),
            Sense::click_and_drag(),
        );

        let mut changed = false;
        if total_ms > 0
            && let Some(pointer) = start_response.interact_pointer_pos()
            && (start_response.clicked() || start_response.dragged())
        {
            let ratio = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let next_ms = (ratio * total_ms_f32).round() as u64;
            clip.start_ms = next_ms.min(clip.end_ms);
            changed = true;
        } else if total_ms > 0
            && let Some(pointer) = end_response.interact_pointer_pos()
            && (end_response.clicked() || end_response.dragged())
        {
            let ratio = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let next_ms = (ratio * total_ms_f32).round() as u64;
            clip.end_ms = next_ms.max(clip.start_ms);
            changed = true;
        } else if response.clicked()
            && total_ms > 0
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let ratio = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let next_ms = (ratio * total_ms_f32).round() as u64;
            if (pointer.x - start_x).abs() <= (pointer.x - end_x).abs() {
                clip.start_ms = next_ms.min(clip.end_ms);
            } else {
                clip.end_ms = next_ms.max(clip.start_ms);
            }
            changed = true;
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(format!("Start: {}", Self::format_ms(clip.start_ms)));
            ui.separator();
            ui.label(format!("End: {}", Self::format_ms(clip.end_ms)));
            ui.separator();
            ui.label(format!(
                "Selected: {}",
                Self::format_ms(clip.end_ms.saturating_sub(clip.start_ms))
            ));
        });

        changed
    }



    fn render_audio_trim_timeline(
        ui: &mut egui::Ui,
        language: UiLanguage,
        id_source: impl std::hash::Hash + Copy,
        clip: &mut AudioClipSettings,
        total_ms: u64,
        waveform: Option<&[f32]>,
        preview_cursor_ms: &mut u64,
        trim_timeline_zoom: &mut f32,
        interactive: bool,
        desired_height: f32,
    ) -> bool {
        Self::trim_audio_bounds(clip, total_ms);
        if total_ms > 0 {
            *preview_cursor_ms =
                (*preview_cursor_ms).clamp(clip.start_ms, clip.end_ms.max(clip.start_ms + 1));
        } else {
            *preview_cursor_ms = 0;
        }
        *trim_timeline_zoom = (*trim_timeline_zoom).clamp(1.0, 8.0);

        ui.horizontal(|ui| {
            ui.label(Self::material_icon_text(0xe14e, 14.0));
            ui.add_space(4.0);
            ui.label(
                RichText::new(Self::tr_lang(language, "Trim", "Trim"))
                    .size(13.0)
                    .strong(),
            );
            ui.add_space(4.0);
            let help = ui.add_sized(
                [24.0, 24.0],
                Button::new(Self::material_icon_text(0xe887, 16.0))
                    .fill(ui.visuals().faint_bg_color)
                    .stroke(Stroke::new(
                        1.0,
                        ui.visuals().widgets.noninteractive.bg_stroke.color,
                    )),
            );
            if help.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Help);
            }
            help.on_hover_ui_at_pointer(|ui| {
                ui.set_max_width(280.0);
                ui.label(
                    RichText::new(Self::tr_lang(language, "Trim shortcuts", "Trim shortcuts"))
                        .size(13.0)
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label("Space: preview or stop at playhead");
                ui.label("S: preview from the trim start");
                ui.label("Q: move the left trim to the mouse");
                ui.label("W: move the right trim to the mouse");
                ui.label("A / D: pan timeline left or right");
                ui.label("Ctrl + mouse wheel: zoom around the hover playhead");
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{:.1}x", *trim_timeline_zoom))
                        .size(12.0)
                        .color(ui.visuals().weak_text_color()),
                );
            });
        });

        ui.add_space(4.0);
        let viewport_width = (ui.available_width() - 24.0).max(296.0);
        let zoom_scroll_offset_id = egui::Id::new((id_source, "trim-zoom-offset"));
        let trim_adjusting_id = egui::Id::new((id_source, "trim-adjusting"));
        let trim_hotkey_adjusting_id = egui::Id::new((id_source, "trim-hotkey-adjusting"));
        let playhead_drag_id = egui::Id::new((id_source, "trim-playhead-drag"));
        let stored_zoom_scroll_offset = ui
            .ctx()
            .data(|data| data.get_temp::<f32>(zoom_scroll_offset_id));
        let mut next_scroll_offset = stored_zoom_scroll_offset;
        let timeline_size = vec2(
            (viewport_width * *trim_timeline_zoom).max(viewport_width),
            desired_height,
        );
        let dark_theme = ui.visuals().dark_mode;
        let mut changed = false;
        let total_ms_f32 = total_ms.max(1) as f32;

        ui.allocate_ui_with_layout(
            vec2(viewport_width, timeline_size.y + 6.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                let mut scroll_area = egui::ScrollArea::horizontal()
                    .id_salt((id_source, "trim-timeline-scroll"))
                    .auto_shrink([false, false]);
                if let Some(offset) = stored_zoom_scroll_offset {
                    scroll_area = scroll_area.horizontal_scroll_offset(offset);
                }
                scroll_area.show(ui, |ui| {
                    let (rect, response) =
                        ui.allocate_exact_size(timeline_size, Sense::click_and_drag());
                    let viewport_rect = rect.intersect(ui.clip_rect());
                    let painter = ui.painter_at(rect);
                    let timeline_fill = if dark_theme {
                        Color32::from_rgb(16, 18, 24)
                    } else {
                        Color32::from_rgb(255, 255, 255)
                    };
                    let timeline_stroke = if dark_theme {
                        Color32::from_rgb(58, 66, 78)
                    } else {
                        Color32::from_rgb(235, 223, 232)
                    };
                    painter.rect_filled(rect, 18.0, timeline_fill);
                    painter.rect_stroke(
                        rect,
                        18.0,
                        Stroke::new(1.0, timeline_stroke),
                        StrokeKind::Outside,
                    );

                    let start_t = clip.start_ms as f32 / total_ms_f32;
                    let end_t = clip.end_ms as f32 / total_ms_f32;
                    let start_x = rect.left() + rect.width() * start_t.clamp(0.0, 1.0);
                    let end_x = rect.left() + rect.width() * end_t.clamp(0.0, 1.0);

                    let selected_rect = egui::Rect::from_min_max(
                        egui::pos2(start_x, rect.top()),
                        egui::pos2(end_x.max(start_x + 2.0), rect.bottom()),
                    );
                    painter.rect_filled(
                        selected_rect,
                        8.0,
                        if dark_theme {
                            Color32::from_rgba_premultiplied(34, 83, 92, 110)
                        } else {
                            Color32::from_rgba_premultiplied(72, 198, 120, 70)
                        },
                    );

                    if let Some(waveform) = waveform.filter(|waveform| !waveform.is_empty()) {
                        let bar_width = rect.width() / waveform.len().max(1) as f32;
                        let wave_color = if dark_theme {
                            Color32::from_rgb(112, 188, 206)
                        } else {
                            Color32::from_rgb(86, 118, 160)
                        };
                        for (index, level) in waveform.iter().enumerate() {
                            let amplitude = level.clamp(0.04, 1.0);
                            let center_x = rect.left() + (index as f32 + 0.5) * bar_width;
                            let half_height = amplitude * rect.height() * 0.42;
                            let wave_rect = egui::Rect::from_min_max(
                                egui::pos2(
                                    center_x - (bar_width * 0.35).max(1.0),
                                    rect.center().y - half_height,
                                ),
                                egui::pos2(
                                    center_x + (bar_width * 0.35).max(1.0),
                                    rect.center().y + half_height,
                                ),
                            );
                            painter.rect_filled(wave_rect, 1.0, wave_color);
                        }
                    } else {
                        painter.line_segment(
                            [
                                egui::pos2(rect.left(), rect.center().y),
                                egui::pos2(rect.right(), rect.center().y),
                            ],
                            Stroke::new(2.0, Color32::from_gray(120)),
                        );
                    }

                    painter.line_segment(
                        [
                            egui::pos2(start_x, rect.top()),
                            egui::pos2(start_x, rect.bottom()),
                        ],
                        Stroke::new(2.0, Color32::from_rgb(255, 232, 96)),
                    );
                    painter.line_segment(
                        [
                            egui::pos2(end_x, rect.top()),
                            egui::pos2(end_x, rect.bottom()),
                        ],
                        Stroke::new(2.0, Color32::from_rgb(255, 232, 96)),
                    );

                    let start_handle_rect = egui::Rect::from_center_size(
                        egui::pos2(start_x, rect.center().y),
                        vec2(20.0, rect.height()),
                    );
                    let end_handle_rect = egui::Rect::from_center_size(
                        egui::pos2(end_x, rect.center().y),
                        vec2(20.0, rect.height()),
                    );
                    let start_response = ui.interact(
                        start_handle_rect,
                        ui.make_persistent_id((id_source, "trim-start")),
                        Sense::click_and_drag(),
                    );
                    let end_response = ui.interact(
                        end_handle_rect,
                        ui.make_persistent_id((id_source, "trim-end")),
                        Sense::click_and_drag(),
                    );

                    let pointer_pos = interactive
                        .then(|| ui.ctx().input(|input| input.pointer.hover_pos()))
                        .flatten();
                    let hovered_pointer_pos =
                        pointer_pos.filter(|pos| viewport_rect.contains(*pos));
                    let pointer_time_ms = pointer_pos.map(|pointer| {
                        let ratio = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                        (ratio * total_ms_f32).round() as u64
                    });
                    let playhead_outline = if dark_theme {
                        Color32::from_rgba_premultiplied(8, 13, 19, 224)
                    } else {
                        Color32::from_rgba_premultiplied(255, 255, 255, 232)
                    };
                    let playhead_color = if dark_theme {
                        Color32::from_rgb(108, 231, 255)
                    } else {
                        Color32::from_rgb(42, 39, 44)
                    };
                    let hover_playhead_color = if dark_theme {
                        Color32::from_rgba_premultiplied(108, 231, 255, 150)
                    } else {
                        Color32::from_rgba_premultiplied(42, 39, 44, 110)
                    };
                    let pan_left = interactive && ui.input(|input| input.key_down(egui::Key::A));
                    let pan_right = interactive && ui.input(|input| input.key_down(egui::Key::D));
                    let keyboard_panning = pan_left ^ pan_right;
                    let timeline_hovered =
                        interactive && (response.hovered() || hovered_pointer_pos.is_some());
                    let showing_hover_preview = hovered_pointer_pos.is_some()
                        && !keyboard_panning
                        && !start_response.is_pointer_button_down_on()
                        && !end_response.is_pointer_button_down_on()
                        && !response.dragged();

                    if showing_hover_preview && let Some(pointer) = hovered_pointer_pos {
                        painter.line_segment(
                            [
                                egui::pos2(pointer.x, rect.top() + 12.0),
                                egui::pos2(pointer.x, rect.bottom() - 12.0),
                            ],
                            Stroke::new(1.0, hover_playhead_color),
                        );
                        painter.circle_filled(
                            egui::pos2(pointer.x, rect.top() + 12.0),
                            4.0,
                            hover_playhead_color,
                        );
                        if let Some(pointer_time_ms) = pointer_time_ms {
                            let text_pos = egui::pos2(
                                (pointer.x + 8.0).clamp(rect.left() + 6.0, rect.right() - 68.0),
                                rect.top() + 12.0,
                            );
                            painter.text(
                                text_pos,
                                egui::Align2::LEFT_TOP,
                                Self::format_ms(pointer_time_ms),
                                egui::FontId::proportional(11.5),
                                if dark_theme {
                                    Color32::from_rgb(208, 244, 255)
                                } else {
                                    Color32::from_rgb(42, 39, 44)
                                },
                            );
                        }
                    }

                    let cursor_ms = (*preview_cursor_ms).clamp(clip.start_ms, clip.end_ms);
                    let cursor_ratio = if total_ms == 0 {
                        0.0
                    } else {
                        cursor_ms as f32 / total_ms_f32
                    };
                    let cursor_x = rect.left() + rect.width() * cursor_ratio.clamp(0.0, 1.0);
                    painter.line_segment(
                        [
                            egui::pos2(cursor_x, rect.top() + 8.0),
                            egui::pos2(cursor_x, rect.bottom() - 8.0),
                        ],
                        Stroke::new(4.0, playhead_outline),
                    );
                    painter.line_segment(
                        [
                            egui::pos2(cursor_x, rect.top() + 8.0),
                            egui::pos2(cursor_x, rect.bottom() - 8.0),
                        ],
                        Stroke::new(2.0, playhead_color),
                    );
                    painter.circle_filled(
                        egui::pos2(cursor_x, rect.top() + 10.0),
                        4.5,
                        playhead_color,
                    );

                    if timeline_hovered && keyboard_panning {
                        ui.ctx().memory_mut(|memory| memory.stop_text_input());
                        let pan_speed = (viewport_rect.width() * 2.4).max(420.0);
                        let pan_step =
                            pan_speed * ui.input(|input| input.stable_dt).max(1.0 / 240.0);
                        let max_offset = (rect.width() - viewport_rect.width()).max(0.0);
                        let delta = match (pan_left, pan_right) {
                            (true, false) => -pan_step,
                            (false, true) => pan_step,
                            _ => 0.0,
                        };
                        let current_offset = next_scroll_offset
                            .unwrap_or_else(|| (viewport_rect.left() - rect.left()).max(0.0));
                        next_scroll_offset = Some((current_offset + delta).clamp(0.0, max_offset));
                        ui.ctx().request_repaint();
                    }

                    if interactive && pointer_pos.is_some() && !ui.ctx().wants_keyboard_input() {
                        let zoom_delta = ui.input(|input| {
                            if input.modifiers.ctrl {
                                input.raw_scroll_delta.y
                            } else {
                                0.0
                            }
                        });
                        if zoom_delta.abs() > 0.0 {
                            let anchor_viewport_x = hovered_pointer_pos
                                .map(|pointer| {
                                    (pointer.x - viewport_rect.left())
                                        .clamp(0.0, viewport_rect.width())
                                })
                                .unwrap_or(viewport_rect.width() * cursor_ratio.clamp(0.0, 1.0));
                            let anchor_content_x = hovered_pointer_pos
                                .map(|pointer| (pointer.x - rect.left()).clamp(0.0, rect.width()))
                                .unwrap_or((cursor_ratio * rect.width()).clamp(0.0, rect.width()));
                            let factor = if zoom_delta > 0.0 { 1.12 } else { 1.0 / 1.12 };
                            *trim_timeline_zoom = (*trim_timeline_zoom * factor).clamp(1.0, 8.0);
                            let next_timeline_width =
                                (viewport_width * *trim_timeline_zoom).max(viewport_width);
                            let next_anchor_content_x =
                                (anchor_content_x / rect.width().max(1.0)) * next_timeline_width;
                            let max_offset = (next_timeline_width - viewport_width).max(0.0);
                            next_scroll_offset = Some(
                                (next_anchor_content_x - anchor_viewport_x).clamp(0.0, max_offset),
                            );
                            ui.ctx().request_repaint();
                        }
                    }

                    let move_left = interactive && ui.input(|input| input.key_down(egui::Key::Q));
                    let move_right = interactive && ui.input(|input| input.key_down(egui::Key::W));
                    if let Some(pointer_time_ms) = pointer_time_ms {
                        if move_left {
                            clip.start_ms = pointer_time_ms.min(clip.end_ms.saturating_sub(50));
                            Self::trim_audio_bounds(clip, total_ms);
                            changed = true;
                            ui.ctx()
                                .data_mut(|data| data.insert_temp(trim_hotkey_adjusting_id, true));
                        }
                        if move_right {
                            clip.end_ms = pointer_time_ms.max(clip.start_ms + 50);
                            Self::trim_audio_bounds(clip, total_ms);
                            changed = true;
                            ui.ctx()
                                .data_mut(|data| data.insert_temp(trim_hotkey_adjusting_id, true));
                        }
                    }
                    if !move_left
                        && !move_right
                        && ui
                            .ctx()
                            .data(|data| data.get_temp::<bool>(trim_hotkey_adjusting_id))
                            .unwrap_or(false)
                    {
                        ui.ctx()
                            .data_mut(|data| data.remove::<bool>(trim_hotkey_adjusting_id));
                    }

                    if interactive
                        && total_ms > 0
                        && let Some(pointer) = start_response.interact_pointer_pos()
                        && (start_response.clicked() || start_response.dragged())
                    {
                        let ratio = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                        let next_ms = (ratio * total_ms_f32).round() as u64;
                        clip.start_ms = next_ms.min(clip.end_ms.saturating_sub(50));
                        Self::trim_audio_bounds(clip, total_ms);
                        changed = true;
                        *preview_cursor_ms = clip.start_ms;
                        ui.ctx()
                            .data_mut(|data| data.insert_temp(trim_adjusting_id, true));
                        ui.ctx()
                            .data_mut(|data| data.remove::<bool>(playhead_drag_id));
                    } else if interactive
                        && total_ms > 0
                        && let Some(pointer) = end_response.interact_pointer_pos()
                        && (end_response.clicked() || end_response.dragged())
                    {
                        let ratio = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                        let next_ms = (ratio * total_ms_f32).round() as u64;
                        clip.end_ms = next_ms.max(clip.start_ms + 50);
                        Self::trim_audio_bounds(clip, total_ms);
                        changed = true;
                        *preview_cursor_ms = clip.end_ms;
                        ui.ctx()
                            .data_mut(|data| data.insert_temp(trim_adjusting_id, true));
                        ui.ctx()
                            .data_mut(|data| data.remove::<bool>(playhead_drag_id));
                    } else if interactive
                        && !start_response.is_pointer_button_down_on()
                        && !end_response.is_pointer_button_down_on()
                        && total_ms > 0
                        && let Some(pointer) = response.interact_pointer_pos()
                        && (response.clicked() || response.dragged())
                    {
                        let ratio = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                        let next_ms = (ratio * total_ms_f32).round() as u64;
                        *preview_cursor_ms = next_ms.clamp(clip.start_ms, clip.end_ms);
                        if response.dragged() {
                            ui.ctx()
                                .data_mut(|data| data.insert_temp(playhead_drag_id, true));
                        }
                    }

                    if interactive
                        && response.drag_stopped()
                        && ui
                            .ctx()
                            .data(|data| data.get_temp::<bool>(playhead_drag_id))
                            .unwrap_or(false)
                    {
                        ui.ctx()
                            .data_mut(|data| data.remove::<bool>(playhead_drag_id));
                    }

                    if interactive
                        && (start_response.drag_stopped() || end_response.drag_stopped())
                        && ui
                            .ctx()
                            .data(|data| data.get_temp::<bool>(trim_adjusting_id))
                            .unwrap_or(false)
                    {
                        ui.ctx()
                            .data_mut(|data| data.remove::<bool>(trim_adjusting_id));
                    }

                    if !interactive || !ui.input(|input| input.pointer.primary_down()) {
                        ui.ctx()
                            .data_mut(|data| data.remove::<bool>(playhead_drag_id));
                        if ui
                            .ctx()
                            .data(|data| data.get_temp::<bool>(trim_adjusting_id))
                            .unwrap_or(false)
                        {
                            ui.ctx()
                                .data_mut(|data| data.remove::<bool>(trim_adjusting_id));
                        }
                    }

                    *preview_cursor_ms = (*preview_cursor_ms).clamp(clip.start_ms, clip.end_ms);

                    if next_scroll_offset.is_none() {
                        next_scroll_offset = Some((viewport_rect.left() - rect.left()).max(0.0));
                    }
                });
                if let Some(offset) = next_scroll_offset {
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(zoom_scroll_offset_id, offset);
                    });
                }
            },
        );

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(Self::format_ms(clip.start_ms))
                    .size(13.0)
                    .color(ui.visuals().weak_text_color()),
            );
            ui.separator();
            ui.label(
                RichText::new(Self::format_ms(clip.end_ms.saturating_sub(clip.start_ms)))
                    .size(13.0)
                    .color(ui.visuals().weak_text_color()),
            );
            ui.separator();
            ui.label(
                RichText::new(Self::format_ms(clip.end_ms))
                    .size(13.0)
                    .color(ui.visuals().weak_text_color()),
            );
        });

        changed
    }


    fn render_audio_clip_card(
        ui: &mut egui::Ui,
        language: UiLanguage,
        title: &str,
        clip: &mut AudioClipSettings,
        duration_ms: &mut Option<u64>,
        editor_open: &mut bool,
        _waveform: Option<&[f32]>,
    ) -> AudioCardOutcome {
        let mut outcome = AudioCardOutcome::default();
        let previewing = audio::is_previewing(clip);

        Self::show_preset_card(ui, clip.enabled, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).strong());
                if !clip.file_path.trim().is_empty() {
                    ui.monospace(Self::format_ms(clip.end_ms.saturating_sub(clip.start_ms)));
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(Self::material_icon_text(0xe145, 18.0))
                    .on_hover_text(Self::tr_lang(
                        language,
                        "Choose audio file",
                        "Chọn file âm thanh",
                    ))
                    .clicked()
                {
                    outcome.choose_file = true;
                }
                if ui
                    .add_enabled(
                        !clip.file_path.trim().is_empty(),
                        Button::new(Self::material_icon_text(0xe3c9, 18.0)),
                    )
                    .on_hover_text(Self::tr_lang(
                        language,
                        "Open Media editor",
                        "Mở trình sửa Media",
                    ))
                    .clicked()
                {
                    outcome.open_editor = true;
                }
                if ui
                    .add_enabled(
                        !clip.file_path.trim().is_empty(),
                        Button::new(if previewing {
                            Self::material_icon_text(0xe034, 18.0)
                        } else {
                            Self::material_icon_text(0xe037, 18.0)
                        }),
                    )
                    .on_hover_text(if previewing {
                        Self::tr_lang(language, "Stop preview", "Dừng nghe thử")
                    } else {
                        Self::tr_lang(language, "Preview audio", "Nghe thử âm thanh")
                    })
                    .clicked()
                {
                    match audio::toggle_preview(clip.clone()) {
                        Ok(true) => {
                            outcome.status = Some(match language {
                                UiLanguage::Vietnamese => {
                                    format!("Đang nghe thử {title}.")
                                }
                                _ => format!("Previewing {title}."),
                            })
                        }
                        Ok(false) => {
                            outcome.status = Some(match language {
                                UiLanguage::Vietnamese => format!("Đã dừng nghe thử {title}."),
                                _ => format!("Stopped {title} preview."),
                            })
                        }
                        Err(error) => {
                            outcome.status = Some(match language {
                                UiLanguage::Vietnamese => {
                                    format!("Nghe thử thất bại: {error}")
                                }
                                _ => format!("Preview failed: {error}"),
                            })
                        }
                    }
                }
            });

            ui.label(if clip.file_path.is_empty() {
                Self::tr_lang(
                    language,
                    "No audio file selected.",
                    "Chưa chọn file âm thanh.",
                )
            } else {
                clip.file_path.as_str()
            });

            if let Some(total_ms) = *duration_ms {
                Self::trim_audio_bounds(clip, total_ms);
                ui.label(format!(
                    "{} {}  |  {} {}",
                    Self::tr_lang(language, "Total:", "Total:"),
                    Self::format_ms(total_ms),
                    Self::tr_lang(language, "Slice", "Đoạn hiện tại"),
                    Self::format_ms(clip.end_ms.saturating_sub(clip.start_ms))
                ));
            }

            let _ = editor_open;
        });

        outcome
    }

    fn trim_video_bounds(clip: &mut VideoClipSettings, total_ms: u64) {
        if total_ms == 0 {
            clip.start_ms = 0;
            clip.end_ms = 0;
            return;
        }
        clip.start_ms = clip.start_ms.min(total_ms);
        clip.end_ms = clip.end_ms.min(total_ms);
        if clip.end_ms < clip.start_ms {
            clip.end_ms = clip.start_ms;
        }
    }

    fn render_video_trim_bar(
        ui: &mut egui::Ui,
        id_source: impl std::hash::Hash + Copy,
        clip: &mut VideoClipSettings,
        total_ms: u64,
        preview_cursor_ms: &mut u64,
        desired_height: f32,
    ) -> bool {
        Self::trim_video_bounds(clip, total_ms);
        let desired_size = vec2(ui.available_width().max(220.0), desired_height);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, 8.0, ui.visuals().extreme_bg_color);
        painter.line_segment(
            [
                egui::pos2(rect.left(), rect.center().y),
                egui::pos2(rect.right(), rect.center().y),
            ],
            egui::Stroke::new(2.0, Color32::from_gray(120)),
        );

        let total_ms_f32 = total_ms.max(1) as f32;
        let start_x = rect.left() + rect.width() * (clip.start_ms as f32 / total_ms_f32);
        let end_x = rect.left() + rect.width() * (clip.end_ms as f32 / total_ms_f32);
        let selected_rect = egui::Rect::from_min_max(
            egui::pos2(start_x, rect.top()),
            egui::pos2(end_x.max(start_x + 2.0), rect.bottom()),
        );
        painter.rect_filled(
            selected_rect,
            8.0,
            Color32::from_rgba_premultiplied(72, 198, 120, 70),
        );
        painter.line_segment(
            [egui::pos2(start_x, rect.top()), egui::pos2(start_x, rect.bottom())],
            egui::Stroke::new(2.0, Color32::from_rgb(255, 232, 96)),
        );
        painter.line_segment(
            [egui::pos2(end_x, rect.top()), egui::pos2(end_x, rect.bottom())],
            egui::Stroke::new(2.0, Color32::from_rgb(255, 232, 96)),
        );

        let start_handle_rect = egui::Rect::from_center_size(
            egui::pos2(start_x, rect.center().y),
            vec2(20.0, rect.height()),
        );
        let end_handle_rect = egui::Rect::from_center_size(
            egui::pos2(end_x, rect.center().y),
            vec2(20.0, rect.height()),
        );
        let start_response = ui.interact(
            start_handle_rect,
            ui.make_persistent_id((id_source, "video-trim-start")),
            Sense::click_and_drag(),
        );
        let end_response = ui.interact(
            end_handle_rect,
            ui.make_persistent_id((id_source, "video-trim-end")),
            Sense::click_and_drag(),
        );

        let mut changed = false;
        if let Some(pointer) = start_response.interact_pointer_pos()
            && (start_response.clicked() || start_response.dragged())
        {
            let ratio = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            clip.start_ms = ((ratio * total_ms_f32).round() as u64).min(clip.end_ms);
            *preview_cursor_ms = clip.start_ms; // Sync playhead to start bound during drag
            changed = true;
        } else if let Some(pointer) = end_response.interact_pointer_pos()
            && (end_response.clicked() || end_response.dragged())
        {
            let ratio = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            clip.end_ms = ((ratio * total_ms_f32).round() as u64).max(clip.start_ms);
            *preview_cursor_ms = clip.end_ms; // Sync playhead to end bound during drag
            changed = true;
        } else if response.clicked() || response.dragged() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let ratio = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                *preview_cursor_ms = ((ratio * total_ms_f32).round() as u64).min(total_ms);
            }
        }

        // Draw Playhead
        let playhead_x = rect.left() + rect.width() * (*preview_cursor_ms as f32 / total_ms_f32);
        painter.line_segment(
            [egui::pos2(playhead_x, rect.top()), egui::pos2(playhead_x, rect.bottom())],
            egui::Stroke::new(2.5, Color32::from_rgb(255, 60, 60)),
        );
        painter.circle_filled(
            egui::pos2(playhead_x, rect.top()),
            4.0,
            Color32::from_rgb(255, 60, 60),
        );

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(format!("Start: {}", Self::format_ms(clip.start_ms)));
            ui.separator();
            ui.label(format!("End: {}", Self::format_ms(clip.end_ms)));
            ui.separator();
            ui.label(format!(
                "Selected: {}",
                Self::format_ms(clip.end_ms.saturating_sub(clip.start_ms))
            ));
        });

        changed
    }



    pub(crate) fn render_sound_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        let previous_item_spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
        ui.add_space(2.0);
        let mut changed = false;
        ui.heading(Self::tr_lang(language, "Media", "Media"));
        ui.label(Self::tr_lang(
            language,
            "Manage reusable audio and video presets for macro steps.",
            "Quản lý preset âm thanh và video dùng lại cho các bước macro.",
        ));
        ui.add_space(6.0);
        let mut import_video_for_new_preset = None;
        ui.horizontal(|ui| {

            if ui
                .button(self.tr("+ Add sound preset", "+ Thêm preset âm thanh"))
                .clicked()
            {
                let mut id = 1;
                while self.state.audio_settings.presets.iter().any(|p| p.id == id) {
                    id += 1;
                }
                self.state.audio_settings.next_preset_id = (self.state.audio_settings.presets.iter().map(|p| p.id).max().unwrap_or(0) + 1).max(id + 1);
                self.state.audio_settings.presets.push(SoundPreset::new(id));
                self.show_sound_preset_audio_editor.insert(id);
                changed = true;
            }
            if ui
                .button(Self::tr_lang(language, "+ Add Video Preset", "+ Thêm preset video"))
                .clicked()
            {
                let mut id = 1;
                while self.state.audio_settings.video_presets.iter().any(|p| p.id == id) {
                    id += 1;
                }
                self.state.audio_settings.next_video_preset_id = (self
                    .state
                    .audio_settings
                    .video_presets
                    .iter()
                    .map(|p| p.id)
                    .max()
                    .unwrap_or(0)
                    + 1)
                    .max(id + 1);
                self.state.audio_settings.video_presets.push(VideoPreset::new(id));
                import_video_for_new_preset = Some(id);
                changed = true;
            }
        });

        ui.add_space(8.0);
        ui.label(RichText::new(Self::tr_lang(language, "Sound Presets", "Preset âm thanh")).strong());

        let mut remove_sound_preset = None;
        for index in 0..self.state.audio_settings.presets.len() {
            let mut choose_file_for = None;
            let mut open_editor_target = None;
            let preset_id = self.state.audio_settings.presets[index].id;
            let waveform_path = self.state.audio_settings.presets[index]
                .clip
                .file_path
                .trim()
                .to_owned();
            self.refresh_audio_waveform_for_path(&waveform_path);
            let preset = &mut self.state.audio_settings.presets[index];
            let waveform = self.audio_waveforms.get(&waveform_path).cloned();
            let mut duration = self
                .sound_preset_clip_duration_ms
                .get(&preset_id)
                .copied()
                .flatten();
            let mut show_editor = self.show_sound_preset_audio_editor.contains(&preset.id);
            if !preset.clip.enabled {
                preset.clip.enabled = true;
                changed = true;
            }

            ui.add_space(6.0);
            Self::show_preset_card(ui, false, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    let name_width = Self::preset_header_name_width(ui);
                    let response = ui.add_sized(
                        [name_width, 24.0],
                        TextEdit::singleline(&mut preset.name),
                    );
                    Self::apply_vietnamese_input_if_changed(
                        &response,
                        self.state.vietnamese_input_enabled,
                        self.state.vietnamese_input_mode,
                        &mut preset.name,
                    );
                    changed |= response.changed();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_sized(
                                [36.0, 24.0],
                                Button::new(Self::material_icon_text(0xe872, 18.0)),
                            )
                            .on_hover_text(Self::tr_lang(
                                language,
                                "Delete sound preset",
                                "Xóa sound preset",
                            ))
                            .clicked()
                        {
                            remove_sound_preset = Some(preset.id);
                        }
                        if ui
                            .add_sized(
                                [84.0, 24.0],
                                Button::new(if preset.collapsed {
                                    Self::tr_lang(language, "Show", "Show")
                                } else {
                                    Self::tr_lang(language, "Hide", "Hide")
                                }),
                            )
                            .clicked()
                        {
                            preset.collapsed = !preset.collapsed;
                        }
                    });
                });
                if preset.collapsed {
                    return;
                }
                let outcome = Self::render_audio_clip_card(
                    ui,
                    language,
                    Self::tr_lang(language, "Sound Preset", "Sound Preset"),
                    &mut preset.clip,
                    &mut duration,
                    &mut show_editor,
                    waveform.as_deref(),
                );
                changed |= outcome.changed;
                if let Some(status) = outcome.status {
                    self.status = status;
                }
                if outcome.choose_file {
                    choose_file_for = Some(preset.id);
                }
                if outcome.open_editor {
                    open_editor_target = Some(AudioEditorTarget::Preset(preset.id));
                }
            });

            self.sound_preset_clip_duration_ms
                .insert(preset.id, duration);
            if show_editor {
                self.show_sound_preset_audio_editor.insert(preset.id);
            } else {
                self.show_sound_preset_audio_editor.remove(&preset.id);
            }
            if let Some(preset_id) = choose_file_for {
                self.choose_audio_file_for_sound_preset(preset_id);
            }
            if let Some(target) = open_editor_target {
                self.open_audio_editor(target);
            }
        }

        if let Some(preset_id) = remove_sound_preset {
            audio::stop_preview();
            self.state
                .audio_settings
                .presets
                .retain(|preset| preset.id != preset_id);
            self.sound_preset_clip_duration_ms.remove(&preset_id);
            self.show_sound_preset_audio_editor.remove(&preset_id);
            changed = true;
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label(RichText::new(Self::tr_lang(language, "Video Presets", "Preset video")).strong());

        let mut remove_video_preset = None;
        let mut preview_video_preset = None;
        for index in 0..self.state.audio_settings.video_presets.len() {
            let preset_id = self.state.audio_settings.video_presets[index].id;
            let clip_snapshot = self.state.audio_settings.video_presets[index].clip.clone();
            let mut choose_video_for = None;
            let mut preview_now = false;
            let mut duration = self
                .video_preset_clip_duration_ms
                .get(&preset_id)
                .copied()
                .flatten()
                .or_else(|| video_duration(&clip_snapshot));
            let mut preview_cursor_ms = self
                .video_preview_cursor_ms
                .get(&preset_id)
                .copied()
                .unwrap_or(clip_snapshot.start_ms);
            if let Some(total_ms) = duration {
                preview_cursor_ms = preview_cursor_ms.min(total_ms);
            }
            let preview_key = self.ensure_video_preview_frame(
                ui.ctx(),
                clip_snapshot.file_path.trim(),
                preview_cursor_ms,
                720,
                420,
            );
            let preview_frame = preview_key
                .as_ref()
                .and_then(|key| self.video_preview_cache.get(key))
                .cloned();
            let preset = &mut self.state.audio_settings.video_presets[index];
            if !preset.clip.enabled {
                preset.clip.enabled = true;
                changed = true;
            }

            ui.add_space(6.0);
            Self::show_preset_card(ui, false, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    let name_width = Self::preset_header_name_width(ui);
                    let response = ui.add_sized([name_width, 24.0], TextEdit::singleline(&mut preset.name));
                    Self::apply_vietnamese_input_if_changed(
                        &response,
                        self.state.vietnamese_input_enabled,
                        self.state.vietnamese_input_mode,
                        &mut preset.name,
                    );
                    changed |= response.changed();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_sized([36.0, 24.0], Button::new(Self::material_icon_text(0xe872, 18.0)))
                            .on_hover_text(Self::tr_lang(language, "Delete video preset", "Xóa preset video"))
                            .clicked()
                        {
                            remove_video_preset = Some(preset.id);
                        }
                        if ui
                            .add_sized(
                                [84.0, 24.0],
                                Button::new(if preset.collapsed {
                                    Self::tr_lang(language, "Show", "Show")
                                } else {
                                    Self::tr_lang(language, "Hide", "Hide")
                                }),
                            )
                            .clicked()
                        {
                            preset.collapsed = !preset.collapsed;
                        }
                    });
                });
                if preset.collapsed {
                    return;
                }

                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_sized(
                            [118.0, 26.0],
                            Button::new(Self::tr_lang(language, "Import video", "Import video")),
                        )
                        .on_hover_text(Self::tr_lang(language, "Choose video file", "Chọn file video"))
                        .clicked()
                    {
                        choose_video_for = Some(preset.id);
                    }
                    if ui
                        .add_enabled(
                            !preset.clip.file_path.trim().is_empty(),
                            Button::new(Self::tr_lang(language, "Preview", "Xem thử")),
                        )
                        .on_hover_text(Self::tr_lang(language, "Preview fullscreen", "Xem thử fullscreen"))
                        .clicked()
                    {
                        preview_now = true;
                    }
                    let pick_active = self.video_chroma_pick_preset_id == Some(preset.id);
                    if ui
                        .add_enabled(
                            !preset.clip.file_path.trim().is_empty(),
                            Button::new(Self::tr_lang(
                                language,
                                if pick_active {
                                    "Picking color..."
                                } else {
                                    "Pick chroma color"
                                },
                                if pick_active {
                                    "Đang lấy màu..."
                                } else {
                                    "Lấy màu chroma"
                                },
                            )),
                        )
                        .clicked()
                    {
                        self.video_chroma_pick_preset_id = if pick_active {
                            None
                        } else {
                            Some(preset.id)
                        };
                    }
                });

                ui.label(if preset.clip.file_path.is_empty() {
                    Self::tr_lang(language, "No video file selected.", "Chưa chọn file video.")
                } else {
                    preset.clip.file_path.as_str()
                });

                if let Some(total_ms) = duration {
                    preview_cursor_ms = preview_cursor_ms.min(total_ms);
                    ui.add_space(4.0);
                    ui.label(format!(
                        "{} {}",
                        Self::tr_lang(language, "Preview frame", "Khung xem"),
                        Self::format_ms(preview_cursor_ms)
                    ));
                    ui.add(
                        Slider::new(&mut preview_cursor_ms, 0..=total_ms)
                            .show_value(false)
                            .clamping(egui::SliderClamping::Always),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .button(Self::tr_lang(language, "Set start", "Đặt đầu"))
                            .clicked()
                        {
                            preset.clip.start_ms = preview_cursor_ms.min(preset.clip.end_ms);
                            changed = true;
                        }
                        if ui
                            .button(Self::tr_lang(language, "Set end", "Đặt cuối"))
                            .clicked()
                        {
                            preset.clip.end_ms = preview_cursor_ms.max(preset.clip.start_ms);
                            changed = true;
                        }
                        if ui
                            .button(Self::tr_lang(language, "Jump to start", "Tới đầu trim"))
                            .clicked()
                        {
                            preview_cursor_ms = preset.clip.start_ms.min(total_ms);
                        }
                        if ui
                            .button(Self::tr_lang(language, "Jump to end", "Tới cuối trim"))
                            .clicked()
                        {
                            preview_cursor_ms = preset.clip.end_ms.min(total_ms);
                        }
                    });
                }
                if preset.clip.file_path.trim().is_empty() {
                    ui.label(
                        RichText::new(Self::tr_lang(
                            language,
                            "Import a video first to unlock preview, trim, and chroma key picking.",
                            "Import video trước để mở preview, trim và chọn màu xóa phông.",
                        ))
                        .small()
                        .color(ui.visuals().weak_text_color()),
                    );
                }
                if let Some(preview) = preview_frame.as_ref() {
                    ui.add_space(4.0);
                    ui.label(Self::tr_lang(
                        language,
                        "Preview frame: click to pick chroma key color",
                        "Khung xem trước: bấm để lấy màu xóa phông",
                    ));
                    let pick_active = self.video_chroma_pick_preset_id == Some(preset.id);
                    let scale = (ui.available_width().min(720.0) / preview.width as f32)
                        .clamp(0.5, 1.0);
                    let size =
                        vec2(preview.width as f32 * scale, preview.height as f32 * scale);
                    let response = ui.add(
                        Image::new((preview.texture.id(), size))
                            .sense(Sense::click())
                            .max_size(size),
                    );
                    if pick_active && response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    }
                    if response.clicked()
                        && pick_active
                        && let Some(pointer) = response.interact_pointer_pos()
                    {
                        let local_x = ((pointer.x - response.rect.left()) / response.rect.width()
                            * preview.width as f32)
                            .floor()
                            .clamp(0.0, preview.width.saturating_sub(1) as f32)
                            as usize;
                        let local_y = ((pointer.y - response.rect.top()) / response.rect.height()
                            * preview.height as f32)
                            .floor()
                            .clamp(0.0, preview.height.saturating_sub(1) as f32)
                            as usize;
                        let pixel_index = (local_y * preview.width + local_x) * 4;
                        if pixel_index + 3 < preview.rgba.len() {
                            preset.clip.chroma_key_color = RgbaColor {
                                r: preview.rgba[pixel_index],
                                g: preview.rgba[pixel_index + 1],
                                b: preview.rgba[pixel_index + 2],
                                a: 255,
                            };
                            preset.clip.chroma_key_enabled = true;
                            self.video_chroma_pick_preset_id = None;
                            changed = true;
                        }
                    }
                }

                if let Some(total_ms) = duration {
                    Self::trim_video_bounds(&mut preset.clip, total_ms);
                    ui.label(format!(
                        "{} {}  |  {} {}",
                        Self::tr_lang(language, "Total:", "Tổng:"),
                        Self::format_ms(total_ms),
                        Self::tr_lang(language, "Slice", "Đoạn"),
                        Self::format_ms(preset.clip.end_ms.saturating_sub(preset.clip.start_ms))
                    ));
                    changed |= Self::render_video_trim_bar(
                        ui,
                        ("video-trim", preset.id),
                        &mut preset.clip,
                        total_ms,
                        &mut preview_cursor_ms,
                        44.0,
                    );
                    ui.horizontal(|ui| {
                        ui.label(Self::tr_lang(language, "Start", "Bắt đầu"));
                        changed |= ui
                            .add(DragValue::new(&mut preset.clip.start_ms).range(0..=total_ms))
                            .changed();
                        ui.label(Self::tr_lang(language, "End", "Kết thúc"));
                        changed |= ui
                            .add(DragValue::new(&mut preset.clip.end_ms).range(0..=total_ms))
                            .changed();
                    });
                    Self::trim_video_bounds(&mut preset.clip, total_ms);
                }

                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    changed |= ui
                        .checkbox(
                            &mut preset.clip.chroma_key_enabled,
                            Self::tr_lang(language, "Chroma Key", "Xóa phông màu"),
                        )
                        .changed();
                    ui.label(Self::tr_lang(language, "Color", "Màu"));
                    let mut key_rgba = [
                        preset.clip.chroma_key_color.r,
                        preset.clip.chroma_key_color.g,
                        preset.clip.chroma_key_color.b,
                        255,
                    ];
                    if ui.color_edit_button_srgba_unmultiplied(&mut key_rgba).changed() {
                        preset.clip.chroma_key_color = RgbaColor {
                            r: key_rgba[0],
                            g: key_rgba[1],
                            b: key_rgba[2],
                            a: 255,
                        };
                        changed = true;
                    }
                    ui.label(Self::tr_lang(language, "Tolerance", "Ngưỡng"));
                    changed |= ui
                        .add(Slider::new(&mut preset.clip.chroma_key_tolerance, 0..=128))
                        .changed();
                });
            });

            self.video_preset_clip_duration_ms.insert(preset.id, duration);
            self.video_preview_cursor_ms
                .insert(preset.id, preview_cursor_ms);
            if let Some(preset_id) = choose_video_for {
                self.choose_video_file_for_preset(preset_id);
            }
            if preview_now {
                preview_video_preset = Some(preset_id);
            }
        }

        if let Some(preset_id) = remove_video_preset {
            self.state
                .audio_settings
                .video_presets
                .retain(|preset| preset.id != preset_id);
            self.video_preset_clip_duration_ms.remove(&preset_id);
            self.video_preview_cursor_ms.remove(&preset_id);
            if self.video_chroma_pick_preset_id == Some(preset_id) {
                self.video_chroma_pick_preset_id = None;
            }
            changed = true;
        }

        if let Some(preset_id) = import_video_for_new_preset {
            self.choose_video_file_for_preset(preset_id);
        }

        if let Some(preset_id) = preview_video_preset {
            let _ = self.overlay_tx.send(OverlayCommand::PlayVideoPreset(preset_id));
            self.status = Self::tr_lang(
                language,
                "Playing video preset fullscreen.",
                "Đang phát preset video fullscreen.",
            )
            .to_owned();
        }

        if changed {
            self.sync_audio_settings();
            self.persist();
        }
        ui.spacing_mut().item_spacing = previous_item_spacing;
    }


    fn render_audio_media_editor(
        ui: &mut egui::Ui,
        language: UiLanguage,
        target: AudioEditorTarget,
        id_source: impl std::hash::Hash + Copy,
        title: &str,
        clip: &mut AudioClipSettings,
        duration_ms: &mut Option<u64>,
        waveform: Option<&[f32]>,
        preview_cursor: &mut Option<(AudioEditorTarget, u64)>,
        trim_timeline_zoom: &mut f32,
    ) -> AudioCardOutcome {
        let mut outcome = AudioCardOutcome::default();
        let previewing = audio::is_previewing(clip);
        let previous_item_spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
        let space_pressed = ui.input(|input| input.key_pressed(egui::Key::Space));
        let s_pressed = ui.input(|input| input.key_pressed(egui::Key::S));
        let mut preview_cursor_ms = Self::preview_cursor_ms_for(preview_cursor, target, clip);
        if previewing && let Some(position_ms) = audio::preview_position_ms(clip) {
            preview_cursor_ms = position_ms;
            ui.ctx().request_repaint();
        }

        ui.heading(Self::tr_lang(language, "Media", "Media"));
        ui.label(RichText::new(title).strong());
        ui.add_space(3.0);

        if !clip.file_path.trim().is_empty() {
            if s_pressed {
                let preview_start_ms = clip.start_ms;
                Self::set_preview_cursor_ms(preview_cursor, target, preview_start_ms, clip);
                match audio::start_preview_from_ms(clip.clone(), preview_start_ms) {
                    Ok(()) => {
                        outcome.status = Some(match language {
                            UiLanguage::Vietnamese => {
                                format!("Đang nghe lại {title} từ đầu.")
                            }
                            _ => format!("Restarting {title} from the start."),
                        })
                    }
                    Err(error) => {
                        outcome.status = Some(match language {
                            UiLanguage::Vietnamese => {
                                format!("Nghe thử thất bại: {error}")
                            }
                            _ => format!("Preview failed: {error}"),
                        })
                    }
                }
            } else if space_pressed {
                if previewing {
                    audio::stop_preview();
                    outcome.status = Some(match language {
                        UiLanguage::Vietnamese => format!("Đã dừng nghe thử {title}."),
                        _ => format!("Stopped {title} preview."),
                    });
                } else {
                    let preview_start_ms = preview_cursor_ms;
                    Self::set_preview_cursor_ms(preview_cursor, target, preview_start_ms, clip);
                    match audio::toggle_preview_from_ms(clip.clone(), preview_start_ms) {
                        Ok(true) => {
                            outcome.status = Some(match language {
                                UiLanguage::Vietnamese => {
                                    format!("Đang nghe thử {title}.")
                                }
                                _ => format!("Previewing {title}."),
                            })
                        }
                        Ok(false) => {
                            outcome.status = Some(match language {
                                UiLanguage::Vietnamese => {
                                    format!("Đã dừng nghe thử {title}.")
                                }
                                _ => format!("Stopped {title} preview."),
                            })
                        }
                        Err(error) => {
                            outcome.status = Some(match language {
                                UiLanguage::Vietnamese => {
                                    format!("Nghe thử thất bại: {error}")
                                }
                                _ => format!("Preview failed: {error}"),
                            })
                        }
                    }
                }
            }
        }

        ui.label(if clip.file_path.is_empty() {
            Self::tr_lang(
                language,
                "No audio file selected.",
                "Chưa chọn file âm thanh.",
            )
        } else {
            clip.file_path.as_str()
        });
        ui.add_space(2.0);

        if let Some(total_ms) = *duration_ms {
            Self::trim_audio_bounds(clip, total_ms);
            Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .stroke(Stroke::new(
                    1.0,
                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                ))
                .corner_radius(16.0)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.label(format!(
                        "{} {}  |  {} {}",
                        Self::tr_lang(language, "Total:", "Total:"),
                        Self::format_ms(total_ms),
                        Self::tr_lang(language, "Slice", "Slice"),
                        Self::format_ms(clip.end_ms.saturating_sub(clip.start_ms))
                    ));
                    ui.add_space(1.0);
                    outcome.changed |= Self::render_audio_trim_timeline(
                        ui,
                        language,
                        (id_source, "trim"),
                        clip,
                        total_ms,
                        waveform,
                        &mut preview_cursor_ms,
                        trim_timeline_zoom,
                        true,
                        112.0,
                    );
                    ui.add_space(1.0);
                    ui.horizontal(|ui| {
                        ui.label(Self::tr_lang(language, "Start", "Bắt đầu"));
                        outcome.changed |= ui
                            .add(DragValue::new(&mut clip.start_ms).range(0..=total_ms))
                            .changed();
                        ui.label(Self::tr_lang(language, "End", "End"));
                        outcome.changed |= ui
                            .add(DragValue::new(&mut clip.end_ms).range(0..=total_ms))
                            .changed();
                    });
                });
            Self::trim_audio_bounds(clip, total_ms);
        }

        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !clip.file_path.trim().is_empty(),
                    Button::new(if previewing {
                        Self::material_icon_text(0xe034, 18.0)
                    } else {
                        Self::material_icon_text(0xe037, 18.0)
                    }),
                )
                .on_hover_text(if previewing {
                    Self::tr_lang(language, "Stop preview", "Dừng nghe thử")
                } else {
                    Self::tr_lang(language, "Preview audio", "Nghe thử âm thanh")
                })
                .clicked()
            {
                match audio::toggle_preview(clip.clone()) {
                    Ok(true) => {
                        outcome.status = Some(match language {
                            UiLanguage::Vietnamese => {
                                format!("Đang nghe thử {title}.")
                            }
                            _ => format!("Previewing {title}."),
                        })
                    }
                    Ok(false) => {
                        outcome.status = Some(match language {
                            UiLanguage::Vietnamese => format!("Đã dừng nghe thử {title}."),
                            _ => format!("Stopped {title} preview."),
                        })
                    }
                    Err(error) => {
                        outcome.status = Some(match language {
                            UiLanguage::Vietnamese => format!("Nghe thử thất bại: {error}"),
                            _ => format!("Preview failed: {error}"),
                        })
                    }
                }
            }
            ui.label(Self::tr_lang(language, "Volume", "Âm lượng"));
            outcome.changed |= ui
                .add_sized(
                    [170.0, 24.0],
                    Slider::new(&mut clip.volume, 0.0..=2.0)
                        .text("x")
                        .clamping(egui::SliderClamping::Always),
                )
                .changed();
            ui.label(Self::tr_lang(language, "Speed", "Tốc độ"));
            outcome.changed |= ui
                .add_sized(
                    [170.0, 24.0],
                    Slider::new(&mut clip.speed, 0.25..=3.0)
                        .text("x")
                        .clamping(egui::SliderClamping::Always),
                )
                .changed();
        });

        if clip.file_path.trim().is_empty() {
            *preview_cursor = None;
        } else {
            *preview_cursor = Some((target, preview_cursor_ms));
        }
        ui.spacing_mut().item_spacing = previous_item_spacing;
        outcome
    }


    pub(crate) fn render_media_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        let Some(target) = self.active_audio_editor else {
            self.state.active_panel = AppPanel::Sound;
            self.render_sound_panel(ui);
            return;
        };
        let mut preview_cursor = self.preview_cursor;
        let mut trim_timeline_zoom = self.trim_timeline_zoom;

        ui.horizontal(|ui| {
            if ui.button(self.tr("Back", "Quay lại")).clicked() {
                self.close_audio_editor();
            }
            if ui
                .button(self.tr("Choose audio file", "Chọn file âm thanh"))
                .clicked()
            {
                match target {
                    AudioEditorTarget::Preset(preset_id) => {
                        self.choose_audio_file_for_sound_preset(preset_id)
                    }
                    AudioEditorTarget::Library(item_id) => {
                        self.choose_audio_file_for_library_item(item_id)
                    }
                    AudioEditorTarget::Startup => self.choose_audio_file(true),
                    AudioEditorTarget::Exit => self.choose_audio_file(false),
                }
            }
        });
        ui.separator();

        match target {
            AudioEditorTarget::Preset(preset_id) => {
                let waveform_path = self
                    .state
                    .audio_settings
                    .presets
                    .iter()
                    .find(|preset| preset.id == preset_id)
                    .map(|preset| preset.clip.file_path.trim().to_owned())
                    .unwrap_or_default();
                self.refresh_audio_waveform_for_path(&waveform_path);
                let waveform = self.audio_waveforms.get(&waveform_path).cloned();
                let mut choose_file_for = None;
                if let Some(preset) = self
                    .state
                    .audio_settings
                    .presets
                    .iter_mut()
                    .find(|preset| preset.id == preset_id)
                {
                    let mut duration = self
                        .sound_preset_clip_duration_ms
                        .get(&preset.id)
                        .copied()
                        .flatten();
                    let outcome = Self::render_audio_media_editor(
                        ui,
                        language,
                        AudioEditorTarget::Preset(preset.id),
                        ("preset", preset.id),
                        &format!(
                            "{}: {}",
                            Self::tr_lang(language, "Sound Preset", "Preset âm thanh"),
                            preset.name
                        ),
                        &mut preset.clip,
                        &mut duration,
                        waveform.as_deref(),
                        &mut preview_cursor,
                        &mut trim_timeline_zoom,
                    );
                    self.sound_preset_clip_duration_ms
                        .insert(preset.id, duration);
                    if outcome.choose_file {
                        choose_file_for = Some(preset.id);
                    }
                    if let Some(status) = outcome.status {
                        self.status = status;
                    }
                    if outcome.changed {
                        self.sync_audio_settings();
                        self.persist();
                    }
                } else {
                    self.close_audio_editor();
                }
                if let Some(preset_id) = choose_file_for {
                    self.choose_audio_file_for_sound_preset(preset_id);
                }
            }
            _ => {
                self.close_audio_editor();
            }
        }
        self.preview_cursor = preview_cursor;
        self.trim_timeline_zoom = trim_timeline_zoom;
    }

}
