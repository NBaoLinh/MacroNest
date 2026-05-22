use crate::model::*;
use crate::ui::{
    CrosshairApp, MacroActionSubmenuKind, MacroStepDragPayload, MacroGroupFavoriteFilter,
    MouseMoveAbsoluteCaptureTarget, MATERIAL_ICONS_FONT,
};
use crate::ai;
use crate::hotkey;
use eframe::egui::{self, *};

impl CrosshairApp {
    fn loop_is_infinite(step: &MacroStep) -> bool {
        matches!(
            step.key.trim().to_ascii_lowercase().as_str(),
            "infinite" | "inf" | "forever" | "-1"
        )
    }



    fn macro_loop_colors(steps: &[MacroStep]) -> Vec<Option<Color32>> {
        let palette = [
            Color32::from_rgba_premultiplied(120, 180, 255, 120),
            Color32::from_rgba_premultiplied(255, 180, 120, 120),
            Color32::from_rgba_premultiplied(160, 220, 140, 120),
            Color32::from_rgba_premultiplied(220, 140, 220, 120),
        ];
        let mut colors = vec![None; steps.len()];
        let mut stack: Vec<Color32> = Vec::new();
        let mut next_loop_color = 0usize;

        for (index, step) in steps.iter().enumerate() {
            match step.action {
                MacroAction::LoopStart | MacroAction::IfStart => {
                    let color = palette[next_loop_color % palette.len()];
                    next_loop_color += 1;
                    stack.push(color);
                    colors[index] = Some(color);
                }
                MacroAction::LoopEnd | MacroAction::IfEnd => {
                    if let Some(color) = stack.last().copied() {
                        colors[index] = Some(color);
                    }
                    stack.pop();
                }
                _ => {
                    if let Some(color) = stack.last().copied() {
                        colors[index] = Some(color);
                    }
                }
            }
        }

        colors
    }


    fn render_macro_action_option(
        ui: &mut egui::Ui,
        language: UiLanguage,
        current: &mut MacroAction,
        candidate: MacroAction,
        live_sync: &mut bool,
    ) {
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
                ui.label(
                    RichText::new(Self::macro_action_short_label(candidate, language))
                        .size(9.0)
                        .color(label_color),
                );
                response
            },
        );
        let response = inner.inner;
        Self::show_instant_hover_tooltip(
            ui,
            &response,
            format!(
                "{}\n{}",
                Self::macro_action_label(candidate),
                Self::macro_action_tooltip(candidate, language)
            ),
        );
        if response.clicked() {
            *current = candidate;
            *live_sync = true;
            ui.close();
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



    fn render_mouse_action_group_option(
        ui: &mut egui::Ui,
        language: UiLanguage,
        id_source: impl std::hash::Hash + Copy,
        current: &mut MacroAction,
        live_sync: &mut bool,
    ) {
        let selected = Self::macro_action_is_mouse(*current);
        let owner_id = ui.make_persistent_id("macro-action-submenu-owner");
        let popup_id = ui.make_persistent_id((id_source, "mouse-submenu-popup"));
        let image_popup_id = ui.make_persistent_id((id_source, "image-search-submenu-popup"));
        let timer_popup_id = ui.make_persistent_id((id_source, "timer-submenu-popup"));
        let active_owner = ui
            .ctx()
            .data(|data| data.get_temp::<MacroActionSubmenuKind>(owner_id));
        let mut open = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(popup_id))
            .unwrap_or(false);
        if active_owner.is_some_and(|kind| kind != MacroActionSubmenuKind::Mouse) {
            open = false;
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
                    open = true;
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(owner_id, MacroActionSubmenuKind::Mouse));
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(image_popup_id, false));
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(timer_popup_id, false));
                }
                let popup_response = egui::Popup::from_response(&response)
                    .id(popup_id)
                    .open_bool(&mut open)
                    .align(egui::RectAlign::BOTTOM_START)
                    .layout(egui::Layout::top_down_justified(egui::Align::Min))
                    .width(372.0)
                    .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
                    .show(|ui| {
                        egui::Grid::new((id_source, "mouse-action-grid"))
                            .num_columns(6)
                            .spacing([6.0, 6.0])
                            .show(ui, |ui| {
                                for (index, action) in
                                    Self::mouse_macro_actions().iter().copied().enumerate()
                                {
                                    Self::render_macro_action_option(
                                        ui, language, current, action, live_sync,
                                    );
                                    if (index + 1) % 8 == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
                if open && let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
                    let mut keep_open_rect = response.rect.expand(10.0);
                    if let Some(popup) = &popup_response {
                        keep_open_rect = keep_open_rect.union(popup.response.rect.expand(10.0));
                        if popup.response.rect.contains(pointer_pos) {
                            ui.ctx().data_mut(|data| {
                                data.insert_temp(owner_id, MacroActionSubmenuKind::Mouse)
                            });
                        }
                    }
                    if !keep_open_rect.contains(pointer_pos) {
                        open = false;
                    }
                }
                ui.ctx().data_mut(|data| data.insert_temp(popup_id, open));
                let label_color = if selected {
                    ui.visuals().strong_text_color()
                } else {
                    ui.visuals().text_color()
                };
                ui.label(
                    RichText::new(Self::tr_lang(language, "Mouse", "Chuột"))
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
                    "Chuột\nMở các action click, lăn và di chuyển chuột.",
                ),
            );
        }
    }

    fn image_search_macro_actions() -> &'static [MacroAction] {
        &[
            MacroAction::StartVisionSearch,
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
    ) {
        let selected = Self::macro_action_is_image_search(*current);
        let owner_id = ui.make_persistent_id("macro-action-submenu-owner");
        let popup_id = ui.make_persistent_id((id_source, "image-search-submenu-popup"));
        let mouse_popup_id = ui.make_persistent_id((id_source, "mouse-submenu-popup"));
        let timer_popup_id = ui.make_persistent_id((id_source, "timer-submenu-popup"));
        let active_owner = ui
            .ctx()
            .data(|data| data.get_temp::<MacroActionSubmenuKind>(owner_id));
        let mut open = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(popup_id))
            .unwrap_or(false);
        if active_owner.is_some_and(|kind| kind != MacroActionSubmenuKind::ImageSearch) {
            open = false;
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
                    open = true;
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(owner_id, MacroActionSubmenuKind::ImageSearch)
                    });
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(mouse_popup_id, false));
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(timer_popup_id, false));
                }
                let popup_response = egui::Popup::from_response(&response)
                    .id(popup_id)
                    .open_bool(&mut open)
                    .align(egui::RectAlign::BOTTOM_START)
                    .layout(egui::Layout::top_down_justified(egui::Align::Min))
                    .width(220.0)
                    .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
                    .show(|ui| {
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
                                        ui, language, current, action, live_sync,
                                    );
                                    if (index + 1) % 3 == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
                if open && let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
                    let mut keep_open_rect = response.rect.expand(10.0);
                    if let Some(popup) = &popup_response {
                        keep_open_rect = keep_open_rect.union(popup.response.rect.expand(10.0));
                        if popup.response.rect.contains(pointer_pos) {
                            ui.ctx().data_mut(|data| {
                                data.insert_temp(owner_id, MacroActionSubmenuKind::ImageSearch)
                            });
                        }
                    }
                    if !keep_open_rect.contains(pointer_pos) {
                        open = false;
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
                    "Image\nMở các action bắt đầu, trigger và dừng image search.",
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
    ) {
        let selected = Self::macro_action_is_timer(*current);
        let owner_id = ui.make_persistent_id("macro-action-submenu-owner");
        let popup_id = ui.make_persistent_id((id_source, "timer-submenu-popup"));
        let mouse_popup_id = ui.make_persistent_id((id_source, "mouse-submenu-popup"));
        let image_popup_id = ui.make_persistent_id((id_source, "image-search-submenu-popup"));
        let active_owner = ui
            .ctx()
            .data(|data| data.get_temp::<MacroActionSubmenuKind>(owner_id));
        let mut open = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(popup_id))
            .unwrap_or(false);
        if active_owner.is_some_and(|kind| kind != MacroActionSubmenuKind::Timer) {
            open = false;
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
                    open = true;
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(owner_id, MacroActionSubmenuKind::Timer)
                    });
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(mouse_popup_id, false));
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(image_popup_id, false));
                }
                let popup_response = egui::Popup::from_response(&response)
                    .id(popup_id)
                    .open_bool(&mut open)
                    .align(egui::RectAlign::BOTTOM_START)
                    .layout(egui::Layout::top_down_justified(egui::Align::Min))
                    .width(220.0)
                    .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
                    .show(|ui| {
                        egui::Grid::new((id_source, "timer-action-grid"))
                            .num_columns(3)
                            .spacing([6.0, 6.0])
                            .show(ui, |ui| {
                                for (index, action) in Self::timer_macro_actions()
                                    .iter()
                                    .copied()
                                    .enumerate()
                                {
                                    Self::render_macro_action_option(
                                        ui, language, current, action, live_sync,
                                    );
                                    if (index + 1) % 3 == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
                if open && let Some(pointer_pos) = ui.ctx().pointer_hover_pos() {
                    let mut keep_open_rect = response.rect.expand(10.0);
                    if let Some(popup) = &popup_response {
                        keep_open_rect = keep_open_rect.union(popup.response.rect.expand(10.0));
                        if popup.response.rect.contains(pointer_pos) {
                            ui.ctx().data_mut(|data| {
                                data.insert_temp(owner_id, MacroActionSubmenuKind::Timer)
                            });
                        }
                    }
                    if !keep_open_rect.contains(pointer_pos) {
                        open = false;
                    }
                }
                ui.ctx().data_mut(|data| data.insert_temp(popup_id, open));
                let label_color = if selected {
                    ui.visuals().strong_text_color()
                } else {
                    ui.visuals().text_color()
                };
                ui.label(
                    RichText::new(Self::tr_lang(language, "Timer", "Hẹn giờ"))
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
                    "Hẹn giờ\nMở các action bắt đầu, tạm dừng và dừng hẹn giờ.",
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
        if step.command_preset_use_powershell {
            step.command_preset_use_powershell = false;
            changed = true;
        }
        let is_saved_custom_preset = resolved_preset.as_ref().is_some_and(|preset| {
            preset.command.trim() == step.command_preset_command.trim()
                && !step.command_preset_command.trim().is_empty()
                && !preset.use_powershell
        });

        if open {
            let popup_size = vec2(300.0, 132.0);
            let mut pos = anchor_response.rect.left_top() - vec2(popup_size.x + 8.0, 0.0);
            let screen_rect = ui.ctx().content_rect();
            if pos.x < screen_rect.left() {
                pos.x = anchor_response.rect.right() + 8.0;
            }
            let area = egui::Area::new(popup_id)
                .order(egui::Order::Foreground)
                .fixed_pos(pos)
                .interactable(true);
            let area_response = area.show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(300.0);
                    let mut trigger_ai = false;
                    ui.horizontal(|ui| {
                        ui.label(Self::tr_lang(language, "Custom command", "Custom command"));
                        let ai_btn = egui::Button::new(Self::ai_badge_text(true))
                            .fill(Self::ai_badge_fill())
                            .stroke(Self::ai_badge_stroke());
                        if ui.add(ai_btn)
                            .on_hover_text(Self::tr_lang(language, "Generate or edit command with AI", "Tạo hoặc sửa câu lệnh bằng AI"))
                            .clicked()
                        {
                            trigger_ai = true;
                        }
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
                            let command_text = ai::normalize_command_text(&step.command_preset_command);
                            save_and_open_ai_request = Some((
                                step_index,
                                preset_name,
                                command_text,
                                step.command_preset_use_powershell,
                                true, // is_ad_hoc
                            ));
                        }
                    }
                    let command_changed = ui
                        .add_sized(
                            [280.0, 72.0],
                            TextEdit::multiline(&mut step.command_preset_command)
                                .desired_rows(3)
                                .hint_text("shutdown /s /t 0"),
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
                                        .hint_text(Self::tr_lang(language, "Enter name...", "Nhập tên..."))
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
                                Self::tr_lang(language, "Save as custom preset", "Lưu thành preset mới")
                            };
                            if ui
                                .add_enabled(
                                    save_enabled,
                                    egui::Button::new(btn_text),
                                )
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
                                    false,
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
            let popup_rect = area_response.response.rect.expand(6.0);
            let hover_popup = pointer_pos.is_some_and(|pos| popup_rect.contains(pos));
            open = response.hovered() || hover_popup;
            ui.ctx().data_mut(|data| data.insert_temp(popup_id, open));
            return (changed, save_request, save_and_open_ai_request, open_ai_preset_id);
        }

        ui.ctx().data_mut(|data| data.insert_temp(popup_id, open));
        (changed, save_request, save_and_open_ai_request, open_ai_preset_id)
    }





    pub(crate) fn render_macro_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
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
        ui.horizontal_wrapped(|ui| {
            ui.label(Self::material_icon_text(0xe8b6, 18.0));
            ui.label(Self::tr_lang(language, "Search", "Tìm"));
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
            Self::apply_vietnamese_input_if_changed(
                &response,
                self.state.vietnamese_input_enabled,
                self.state.vietnamese_input_mode,
                &mut self.macro_preset_search_query,
            );

            if active_folder_for_controls.is_none() {
                if Self::sized_button(
                    ui,
                    112.0,
                    Self::tr_lang(language, "+ Add folder", "+ Thêm thư mục"),
                )
                .clicked()
                {
                    self.add_macro_folder();
                    self.persist();
                    self.macro_folders_panel_open = true;
                    self.active_macro_folder_view = None;
                }
            }
            if Self::sized_button(
                ui,
                138.0,
                Self::tr_lang(language, "+ Add macro group", "+ Thêm macro group"),
            )
            .clicked()
            {
                if let Some(folder_id) = active_folder_for_controls {
                    self.add_macro_group_to_folder(folder_id);
                } else {
                    self.add_macro_group();
                }
                self.persist();
            }
            if let Some(folder_id) = active_folder_for_controls
                && Self::sized_button(
                    ui,
                    138.0,
                    Self::tr_lang(language, "Enable All Groups", "Bật tất cả group"),
                )
                .clicked()
            {
                for group in self
                    .state
                    .macro_groups
                    .iter_mut()
                    .filter(|group| group.folder_id == Some(folder_id))
                {
                    group.enabled = true;
                }
                self.persist_macro_presets();
            }

            let paste_enabled = !self.macro_group_clipboard.is_empty();
            if ui
                .add_enabled(
                    paste_enabled,
                    Button::new(Self::tr_lang(language, "Paste", "Paste"))
                        .min_size(egui::vec2(112.0, 24.0)),
                )
                .clicked()
            {
                self.paste_macro_groups_into_folder(paste_target_folder);
            }

            let copy_enabled = !self.selected_macro_groups.is_empty();
            if ui
                .add_enabled(
                    copy_enabled,
                    Button::new(Self::tr_lang(language, "Copy", "Copy"))
                        .min_size(egui::vec2(112.0, 24.0)),
                )
                .clicked()
            {
                self.copy_selected_macro_groups();
            }

            let cut_enabled = !self.selected_macro_groups.is_empty();
            if ui
                .add_enabled(
                    cut_enabled,
                    Button::new(Self::tr_lang(language, "Cut", "Cut"))
                        .min_size(egui::vec2(112.0, 24.0)),
                )
                .clicked()
            {
                self.cut_selected_macro_groups();
            }
        });

        ui.add_space(8.0);


        let mut release_folder_id = None;
        let mut delete_folder_id = None;
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
                    "Chỉ hiện nhóm đã favorite",
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

            let variable_inspector_active = self.variable_inspector_open;
            if ui
                .add_sized(
                    [28.0, 28.0],
                    Button::new(Self::material_icon_text(0xe868, 18.0)) // bug icon
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
                    "Variable Inspector / Debugger (Real-time)",
                    "Trình theo dõi biến thời gian thực (Real-time)",
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
                    "Xóa các macro group đã chọn",
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
                    Self::tr_lang(language, "and all", "và toàn bộ"),
                    Self::tr_lang(
                        language,
                        "macro group(s) inside it",
                        "macro group bên trong",
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
                    Self::tr_lang(language, "and move", "và chuyển"),
                    Self::tr_lang(
                        language,
                        "macro group(s) out of it",
                        "macro group ra khỏi nó",
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
            for folder in &self.state.macro_folders {
                render_items.push(RenderItem::FolderHeader(folder.id));
                if !folder.collapsed {
                    for (index, group) in self.state.macro_groups.iter().enumerate() {
                        if group.folder_id == Some(folder.id) {
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
                "Không có macro group nào ngoài thư mục.",
            ));
        }

        let mut toggle_collapsed_folder_id = None;
        let mut add_group_to_folder_id = None;
        let mut renamed_folder: Option<(u32, String)> = None;
        let mut toggle_folder_enabled_id = None;
        let mut enable_all_folder_id = None;
        let mut disable_all_folder_id = None;
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
                    let card_active = folder.enabled && folder_has_enabled_content;
                    Self::show_preset_card(ui, card_active, |ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized(
                                    [28.0, 24.0],
                                    Button::new(Self::folder_icon_text(!folder.collapsed, 18.0)),
                                )
                                .clicked()
                            {
                                toggle_collapsed_folder_id = Some(folder_id);
                            }
                            let mut folder_enabled = folder.enabled;
                            if ui.checkbox(&mut folder_enabled, "").changed() {
                                toggle_folder_enabled_id = Some(folder_id);
                            }
                            let response =
                                ui.add_sized([220.0, 24.0], TextEdit::singleline(&mut folder_name));
                            Self::apply_vietnamese_input_if_changed(
                                &response,
                                self.state.vietnamese_input_enabled,
                                self.state.vietnamese_input_mode,
                                &mut folder_name,
                            );
                            if response.changed() {
                                renamed_folder = Some((folder_id, folder_name.clone()));
                            }
                            ui.add_sized(
                                [96.0, 24.0],
                                egui::Label::new(match language {
                                    UiLanguage::Vietnamese => format!("{folder_group_count} nhóm"),
                                    _ => format!("{folder_group_count} group(s)"),
                                }),
                            );

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if Self::sound_style_remove_button(ui).clicked() {
                                    if folder_group_count > 0 {
                                        self.confirm_delete_folder_id = Some(folder_id);
                                    } else {
                                        delete_folder_id = Some(folder_id);
                                    }
                                }
                                let is_collapsed = folder.collapsed;
                                let button_text = if is_collapsed {
                                    Self::tr_lang(language, "Show", "Hiện")
                                } else {
                                    Self::tr_lang(language, "Hide", "Ẩn")
                                };
                                if ui
                                    .add_sized(
                                        [70.0, 24.0],
                                        Button::new(button_text),
                                    )
                                    .clicked()
                                {
                                    toggle_collapsed_folder_id = Some(folder_id);
                                }
                                if ui
                                    .add_sized(
                                        [75.0, 24.0],
                                        Button::new(Self::tr_lang(language, "Disable All", "Tắt hết")),
                                    )
                                    .clicked()
                                {
                                    disable_all_folder_id = Some(folder_id);
                                }
                                if ui
                                    .add_sized(
                                        [70.0, 24.0],
                                        Button::new(Self::tr_lang(language, "Enable All", "Bật hết")),
                                    )
                                    .clicked()
                                {
                                    enable_all_folder_id = Some(folder_id);
                                }
                                if ui
                                    .add_sized(
                                        [86.0, 24.0],
                                        Button::new(Self::tr_lang(language, "+ Group", "+ Nhóm")),
                                    )
                                    .clicked()
                                {
                                    add_group_to_folder_id = Some(folder_id);
                                }
                            });
                        });
                    });
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
                    let mut selection_after_move = None;
                    let mut selection_after_paste = None;
                    let mut clear_step_selection = None;
                    let mut copy_selected_steps = None;
                    let mut delete_selected_steps = None;
                    let mut paste_step_after = None;
                    let mut copy_single_step = None;
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
                    let group = &mut self.state.macro_groups[group_index];
                    Self::show_preset_card(ui, group.enabled, |ui| {
                        ui.horizontal(|ui| {
                            if self.macro_folders_panel_open {
                                ui.add_space(16.0);
                            }
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
                                            ui.label(RichText::new(Self::tr_lang(language, "CRITICAL WARNING", "CẢNH BÁO NGUY HIỂM")).strong().color(Color32::from_rgb(255, 10, 10)));
                                        });
                                        if has_group_inf_loop {
                                            ui.label(Self::tr_lang(
                                                language,
                                                "This group contains one or more enabled infinite loop macros! Enabling this group could lead to persistent looping upon keypress.",
                                                "Nhóm macro này chứa một hoặc nhiều macro bị lặp vô tận đang bật! Kích hoạt nhóm này có thể dẫn tới lặp vĩnh viễn khi bấm phím."
                                            ));
                                        }
                                        if has_group_vision_leak {
                                            ui.label(Self::tr_lang(
                                                language,
                                                "This group contains one or more macros that start image search (Press/Release trigger) but never stop it! This could cause background CPU thread leaks.",
                                                "Nhóm macro này chứa một hoặc nhiều macro bắt đầu tìm ảnh (kích hoạt bằng Nhấn/Thả) nhưng không dừng lại! Điều này có thể gây chạy luồng ngầm liên tục hao CPU."
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
                                    TextEdit::singleline(&mut group.name),
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

                                    let enabled_icon = if group.enabled { 0xe834 } else { 0xe835 };
                                    let enabled_fill = if group.enabled {
                                        Color32::from_rgba_premultiplied(72, 156, 116, 120)
                                    } else {
                                        ui.visuals().faint_bg_color
                                    };
                                    let enabled_stroke = if group.enabled {
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
                                            "Enable / disable group",
                                            "Enable / disable group",
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

                                        if Self::sized_button(
                                            ui,
                                            92.0,
                                            Self::tr_lang(language, "+ Preset", "+ Preset"),
                                        )
                                        .clicked()
                                        {
                                            add_preset_to_group = Some(group.id);
                                        }
                                    }
                                },
                            );
                        });
                        if group.collapsed {
                            return;
                        }
                        ui.separator();
                        egui::Grid::new((group.id, "group-target-row"))
                            .num_columns(2)
                            .spacing([8.0, 8.0])
                            .show(ui, |ui| {
                                ui.label(Self::tr_lang(language, "Target Window", "Target Window"));
                                live_sync |= Self::render_multi_window_targets_with_duplicate_mode(
                                    ui,
                                    language,
                                    (group.id, "macro-group-window-target"),
                                    Self::tr_lang(
                                        language,
                                        "Any focused window",
                                        "Cửa sổ đang focus",
                                    ),
                                    &mut group.target_window_title,
                                    &mut group.extra_target_window_titles,
                                    &mut group.match_duplicate_window_titles,
                                    &self.open_windows,
                                );
                                ui.end_row();
                            });

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
                            Self::show_macro_preset_card(ui, group.enabled, preset.enabled, |ui| {
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
                                                             "Thả"
                                                         } else {
                                                             "Kích hoạt"
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

                                            let mouse_trigger_options = [
                                                (
                                                    "MouseLeft",
                                                    Self::tr_lang(language, "Left Click", "Click Trái"),
                                                ),
                                                (
                                                    "MouseRight",
                                                    Self::tr_lang(language, "Right Click", "Click Phải"),
                                                ),
                                                (
                                                    "MouseMiddle",
                                                    Self::tr_lang(language, "Middle Click", "Click Giữa"),
                                                ),
                                                ("MouseX1", Self::tr_lang(language, "Mouse X1", "Nút Phụ 1 (X1)")),
                                                ("MouseX2", Self::tr_lang(language, "Mouse X2", "Nút Phụ 2 (X2)")),
                                                (
                                                    "MouseWheelUp",
                                                    Self::tr_lang(language, "Wheel Up", "Cuộn Lên"),
                                                ),
                                                (
                                                    "MouseWheelDown",
                                                    Self::tr_lang(language, "Wheel Down", "Cuộn Xuống"),
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
                                            let enabled_icon = if preset.enabled { 0xe834 } else { 0xe835 };
                                            let enabled_fill = if preset.enabled {
                                                Color32::from_rgba_premultiplied(72, 156, 116, 120)
                                            } else {
                                                ui.visuals().faint_bg_color
                                            };
                                            let enabled_stroke = if preset.enabled {
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
                                                    "Enable / disable preset",
                                                    "Bật / tắt macro",
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
                                                             ui.label(RichText::new(Self::tr_lang(language, "MACRO WARNING", "CẢNH BÁO MACRO")).strong().color(Color32::from_rgb(255, 90, 0)));
                                                         });
                                                         if has_preset_inf_loop {
                                                             ui.label(Self::tr_lang(
                                                                 language,
                                                                 "This macro contains an infinite loop and is active. Ensure you know how to stop it to avoid system hang!",
                                                                 "Macro này chứa vòng lặp vô hạn và đang ở chế độ tự kích hoạt. Hãy đảm bảo bạn đã biết cách dừng nó để tránh treo máy!"
                                                             ));
                                                         }
                                                         if has_preset_vision_leak {
                                                             ui.label(Self::tr_lang(
                                                                 language,
                                                                 "This macro starts image search (Press/Release trigger) but does not contain a 'StopImageSearch' action! This could lead to a persistent background CPU thread. Add a 'StopImageSearch' step or change trigger to 'Hold'.",
                                                                 "Macro này bắt đầu tìm kiếm hình ảnh (chế độ Nhấn/Thả) nhưng không có bước dừng tìm ảnh! Điều này có thể dẫn tới luồng chạy ngầm liên tục gây hao CPU. Hãy thêm bước dừng tìm ảnh hoặc đổi trigger sang Giữ (Hold)."
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
                                        ui.label(RichText::new(Self::tr_lang(language, "Active Variables:", "Biến đang dùng:")).size(11.0).weak());
                                        let vars_map = crate::overlay::RUNTIME_VARIABLES.lock();
                                        for var_name in &referenced_vars {
                                            let val = vars_map.get(var_name).copied();
                                            let val_str = val.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
                                            
                                            let bg_color = if val.is_some() {
                                                Color32::from_rgba_premultiplied(0, 191, 255, 20)
                                            } else {
                                                Color32::from_rgba_premultiplied(128, 128, 128, 20)
                                            };
                                            let stroke_color = if val.is_some() {
                                                Color32::from_rgb(0, 191, 255)
                                            } else {
                                                Color32::from_rgb(128, 128, 128)
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
                                                            .color(if val.is_some() { Color32::from_rgb(0, 191, 255) } else { Color32::from_rgb(160, 160, 160) })
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
                                .selected_text(match (language, preset.trigger_mode) {
                                    (UiLanguage::Vietnamese, MacroTriggerMode::Press) => "Nhấn",
                                    (UiLanguage::Vietnamese, MacroTriggerMode::Hold) => "Giữ",
                                    (UiLanguage::Vietnamese, MacroTriggerMode::Release) => "Thả",
                                     (_, _) => Self::macro_trigger_mode_label(preset.trigger_mode, language),
                                })
                                .show_ui(ui, |ui| {
                                    for mode in [
                                        MacroTriggerMode::Press,
                                        MacroTriggerMode::Hold,
                                        MacroTriggerMode::Release,
                                    ] {
                                        if ui
                                            .selectable_label(
                                                preset.trigger_mode == mode,
                                                match (language, mode) {
                                                    (UiLanguage::Vietnamese, MacroTriggerMode::Press) => "Nhấn",
                                                    (UiLanguage::Vietnamese, MacroTriggerMode::Hold) => "Giữ",
                                                    (UiLanguage::Vietnamese, MacroTriggerMode::Release) => "Thả",
                                                     (_, _) => Self::macro_trigger_mode_label(mode, language),
                                                },
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
                                            "Chạy một action nếu hold dừng sớm",
                                        ),
                                    )
                                    .on_hover_text(
                                        Self::tr_lang(
                                            language,
                                            "If this hold macro is interrupted before it finishes all steps, run this extra action once on stop.",
                                            "Nếu macro hold này bị ngắt trước khi chạy hết các bước, hãy chạy thêm action này một lần khi dừng.",
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
                                            "Đợi các phím khác nhả ra rồi mới kích hoạt",
                                        ),
                                    )
                                    .on_hover_text(
                                        Self::tr_lang(
                                            language,
                                            "If enabled, releasing the trigger key or mouse button will not fire while any other key or mouse button is still held down.",
                                            "Nếu bật, khi bạn thả phím kích hoạt ra, macro sẽ chưa chạy ngay nếu vẫn còn các phím/nút chuột khác đang được giữ. Nó sẽ đợi cho đến khi toàn bộ các phím đó được nhả ra hết rồi mới chính thức kích hoạt.",
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
                                        let is_dark_theme = self.state.ui_theme == UiThemeMode::Dark;
                                        let hint_color = if is_dark_theme {
                                            Color32::from_rgba_unmultiplied(110, 110, 110, 90)
                                        } else {
                                            Color32::from_rgba_unmultiplied(130, 130, 130, 90)
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
                                                    "🔄 Tự động bật/tắt bước khi chạy"
                                                )).changed();
                                                ui.separator();
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
                                                              MacroAction::IfStart,
                                                              MacroAction::Else,
                                                              MacroAction::IfEnd,
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
                                                        );
                                                        Self::render_image_search_action_group_option(
                                                            ui,
                                                            language,
                                                            (group.id, preset.id, "hold-stop-image-search-group"),
                                                            &mut step.action,
                                                            &mut live_sync,
                                );
                                                         Self::render_timer_action_group_option(
                                                             ui,
                                                             language,
                                                             (group.id, preset.id, "hold-stop-timer-group"),
                                                             &mut step.action,
                                                             &mut live_sync,
                                                         );
                                });
                            });
                                            Self::show_instant_hover_tooltip(
                                                ui,
                                                &hold_stop_combo.response,
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
                                                        .unwrap_or_else(|| {
                                                            Self::tr_lang(language, "Select window preset", "Chọn preset cửa sổ").to_owned()
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
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .window_focus_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            Self::tr_lang(language, "Select focus preset", "Chọn preset focus").to_owned()
                                                        });
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-focus-window-preset"))
                                                        .width(146.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.window_focus_presets {
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
                                                            Self::tr_lang(language, "Select macro preset", "Select macro preset").to_owned()
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
                                                                Self::tr_lang(
                                                                    language,
                                                                    "Select custom preset",
                                                                    "Chọn preset câu lệnh",
                                                                )
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
                                                    let (custom_draft_changed, custom_save_request, custom_save_and_open_ai_request, open_ai_preset_id) = Self::render_custom_preset_step_draft_popup(
                                                         ui,
                                                         &custom_preset_combo.response,
                                                         &custom_preset_combo.response,
                                                         step,
                                                         (group.id, preset.id, "hold-stop"),
                                                         None,
                                                         language,
                                                         &command_presets_snapshot,
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
                                                                Self::tr_lang(language, "Select macro preset", "Select macro preset").to_owned()
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
                                                        let preset_label = group_preset_options.iter()
                                                            .find(|(id, _)| *id == current_preset_id)
                                                            .map(|(_, label)| label.clone())
                                                            .unwrap_or_else(|| Self::tr_lang(language, "Select preset", "Chọn preset").to_owned());
                                                            
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
                                                            Self::tr_lang(language, "Select steps", "Chọn steps").to_owned()
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
                                                        .unwrap_or_else(|| Self::tr_lang(language, "Select timer", "Chọn hẹn giờ").to_owned());
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
                                                        Self::tr_lang(language, "Select crosshair preset", "Select crosshair preset").to_owned()
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
                                                            Self::tr_lang(language, "Select pin preset", "Chọn preset ghim").to_owned()
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
                                                            Self::tr_lang(language, "Select mouse path", "Chọn preset đường chuột").to_owned()
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
                                                                "Chọn preset image search",
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
                                                } else if step.action == MacroAction::ApplyMouseSensitivityPreset {
                                                    live_sync |= ui.checkbox(&mut step.manual_mouse_sensitivity, Self::tr_lang(language, "Manual", "Nhập tay")).changed();
                                                    if step.manual_mouse_sensitivity {
                                                        ui.vertical(|ui| {
                                                            let response = ui.add_sized(
                                                                [110.0, 22.0],
                                                                TextEdit::singleline(&mut step.key)
                                                                    .hint_text(RichText::new(Self::tr_lang(language, "value/expr", "giá trị")).color(hint_color).weak()),
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
                                                                UiLanguage::Vietnamese => format!("Kết quả: {} (giới hạn: {} trong 1..20)", evaluated, clamped),
                                                                _ => format!("Evaluated: {} (clamped to: {} within 1..20)", evaluated, clamped),
                                                            };
                                                            response.on_hover_text(tooltip_text);

                                                            Self::render_variable_suggestions(ui, &mut step.key, language);
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
                                                                Self::tr_lang(
                                                                    language,
                                                                    "Select mouse sensitivity preset",
                                                                    "Chọn preset độ nhạy",
                                                                )
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
                                                            Self::tr_lang(language, "Select sound preset", "Chọn preset âm thanh").to_owned()
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
                                                } else if matches!(step.action, MacroAction::LockKeys | MacroAction::UnlockKeys) {
                                                    let response = ui.add_sized(
                                                        [160.0, 22.0],
                                                        TextEdit::singleline(&mut step.key)
                                                            .hint_text(RichText::new("A,S,W,D").color(hint_color).italics()),
                                                    );
                                                    Self::apply_vietnamese_input_if_changed(
                                                        &response,
                                                        self.state.vietnamese_input_enabled,
                                                        self.state.vietnamese_input_mode,
                                                        &mut step.key,
                                                    );
                                                    live_sync |= response.changed();
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
                                                             .color(Color32::BLACK)
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
                                                              let response = ui.add_sized(
                                                                  [96.0, 22.0],
                                                                  TextEdit::singleline(&mut step.key).hint_text(
                                                                      RichText::new(Self::tr_lang(
                                                                          language,
                                                                          "Loop count",
                                                                          "Số lần lặp",
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
                                                              Self::render_variable_suggestions(ui, &mut step.key, language);
                                                          });
                                                      }
                                                 } else if step.action == MacroAction::StopIfKeyPressed {
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
                                                                    "Select HUD preset",
                                                                    "Chọn HUD preset",
                                                                )
                                                                .to_owned()
                                                            } else {
                                                                format!("Cũ: {}", step.key)
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
                                                        let response = ui.add_sized(
                                                            [120.0, 22.0],
                                                            TextEdit::singleline(&mut step.text_override)
                                                                .hint_text(RichText::new(Self::tr_lang(language, "Text override", "Ghi đè văn bản")).color(hint_color).italics()),
                                                        );
                                                        Self::apply_vietnamese_input_if_changed(
                                                            &response,
                                                            self.state.vietnamese_input_enabled,
                                                            self.state.vietnamese_input_mode,
                                                            &mut step.text_override,
                                                        );
                                                        live_sync |= response.changed();
                                                    });
                                                } else if step.action == MacroAction::TypeText {
                                                    ui.vertical(|ui| {
                                                        let response = ui.add_sized(
                                                            [220.0, 22.0],
                                                            TextEdit::singleline(&mut step.key).hint_text(
                                                                RichText::new(Self::tr_lang(
                                                                    language,
                                                                    "Text to type",
                                                                    "Văn bản cần gõ",
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
                                                        Self::render_variable_suggestions(ui, &mut step.key, language);
                                                    });
                                                } else if step.action == MacroAction::DisableCrosshair {
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.spacing_mut().interact_size.y = 22.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        ui.horizontal(|ui| {
                                                            let response = ui.checkbox(&mut step.lock_mouse_left, Self::tr_lang(language, "All", "Tắt hết"));
                                                            live_sync |= response.changed();
                                                            if !step.lock_mouse_left {
                                                                let selected_label = if step.key.trim().is_empty() {
                                                                    Self::tr_lang(language, "Select profile", "Chọn profile").to_owned()
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
                                                            let response = ui.checkbox(&mut step.lock_mouse_left, Self::tr_lang(language, "All", "Tắt hết"));
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
                                                                        Self::tr_lang(language, "Select pin", "Chọn preset ghim").to_owned()
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
                                                } else if matches!(step.action, MacroAction::DisableZoom | MacroAction::Else | MacroAction::IfEnd) {
                                                    ui.add_sized(
                                                        [110.0, 22.0],
                                                        egui::Label::new(Self::tr_lang(language, "No input", "No input")),
                                                    );
                                                } else if step.action == MacroAction::IfStart {
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.spacing_mut().interact_size.y = 22.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        ui.vertical(|ui| {
                                                            ui.horizontal(|ui| {
                                                                let response = ui.add_sized(
                                                                    [64.0, 22.0],
                                                                    TextEdit::singleline(&mut step.if_variable_name)
                                                                        .hint_text(RichText::new(Self::tr_lang(language, "variable", "biến")).color(hint_color).weak()),
                                                                );
                                                                Self::apply_vietnamese_input_if_changed(
                                                                    &response,
                                                                    self.state.vietnamese_input_enabled,
                                                                    self.state.vietnamese_input_mode,
                                                                    &mut step.if_variable_name,
                                                                );
                                                                live_sync |= response.changed();
                                                                
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
                                                                
                                                                live_sync |= ui.add_sized(
                                                                    [46.0, 22.0],
                                                                    DragValue::new(&mut step.if_compare_value).range(-1_000_000..=1_000_000),
                                                                ).changed();

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
                                                            Self::render_variable_suggestions_raw(ui, &mut step.if_variable_name, language);
                                                        });
                                                    });
                                                } else if step.action == MacroAction::SetVariable {
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.spacing_mut().interact_size.y = 22.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        ui.vertical(|ui| {
                                                            ui.horizontal(|ui| {
                                                                let response = ui.add_sized(
                                                                    [64.0, 22.0],
                                                                    TextEdit::singleline(&mut step.if_variable_name)
                                                                        .hint_text(RichText::new(Self::tr_lang(language, "variable", "biến")).color(hint_color).weak()),
                                                                );
                                                                Self::apply_vietnamese_input_if_changed(
                                                                    &response,
                                                                    self.state.vietnamese_input_enabled,
                                                                    self.state.vietnamese_input_mode,
                                                                    &mut step.if_variable_name,
                                                                );
                                                                live_sync |= response.changed();

                                                                ui.label(" = ");

                                                                let response2 = ui.add_sized(
                                                                    [76.0, 22.0],
                                                                    TextEdit::singleline(&mut step.key)
                                                                        .hint_text(RichText::new(Self::tr_lang(language, "value/expr", "giá trị")).color(hint_color).weak()),
                                                                );
                                                                Self::apply_vietnamese_input_if_changed(
                                                                    &response2,
                                                                    self.state.vietnamese_input_enabled,
                                                                    self.state.vietnamese_input_mode,
                                                                    &mut step.key,
                                                                );
                                                                live_sync |= response2.changed();

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
                                                            Self::render_variable_suggestions_raw(ui, &mut step.if_variable_name, language);
                                                            Self::render_variable_suggestions(ui, &mut step.key, language);
                                                        });
                                                    });
                                                } else {
                                                    live_sync |= ui
                                                        .add_sized([160.0, 22.0], TextEdit::singleline(&mut step.key))
                                                        .changed();
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
                                                live_sync |= ui
                                                    .checkbox(&mut step.timed_override, "T")
                                                    .on_hover_text("Timed display")
                                                    .changed();
                                                ui.add_enabled_ui(step.timed_override, |ui| {
                                                    live_sync |= ui
                                                        .add_sized(
                                                            [72.0, 22.0],
                                                            DragValue::new(&mut step.duration_override_ms)
                                                                .range(50..=60_000)
                                                                .suffix(" ms"),
                                                        )
                                                        .changed();
                                                });
                                            } else {
                                                ui.add_sized([24.0, 22.0], egui::Label::new(""));
                                                ui.add_sized([24.0, 22.0], egui::Label::new(""));
                                            }

                                            if action_supports_capture {
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
                                                        "Bắt phím giữ",
                                                        "Bắt phím cho action khi dừng giữ",
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
                                                     
                                                     ui.menu_button(Self::tr_lang(language, "Letters (A-Z)", "Chữ cái (A-Z)"), |ui| {
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

                                                     ui.menu_button(Self::tr_lang(language, "Numbers & Symbols", "Số & Kí tự"), |ui| {
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

                                                     ui.menu_button(Self::tr_lang(language, "Navigation", "Điều hướng & Phím tắt"), |ui| {
                                                         ui.set_max_width(160.0);
                                                         for key in ["Escape", "Enter", "Space", "Backspace", "Tab", "Insert", "Delete", "Home", "End", "PageUp", "PageDown", "Left", "Up", "Right", "Down", "PrintScreen", "Pause"] {
                                                             if ui.button(key).clicked() {
                                                                 step.key = key.to_string();
                                                                 live_sync = true;
                                                                 ui.close_menu();
                                                             }
                                                         }
                                                     });

                                                     ui.menu_button(Self::tr_lang(language, "Function (F1-F24)", "Phím chức năng"), |ui| {
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

                                                     ui.menu_button(Self::tr_lang(language, "Numpad", "Bàn phím số phụ"), |ui| {
                                                         ui.set_max_width(160.0);
                                                         for key in ["Numpad0", "Numpad1", "Numpad2", "Numpad3", "Numpad4", "Numpad5", "Numpad6", "Numpad7", "Numpad8", "Numpad9", "NumpadMultiply", "NumpadAdd", "NumpadSubtract", "NumpadDecimal", "NumpadDivide"] {
                                                             if ui.button(key).clicked() {
                                                                 step.key = key.to_string();
                                                                 live_sync = true;
                                                                 ui.close_menu();
                                                             }
                                                         }
                                                     });

                                                     ui.menu_button(Self::tr_lang(language, "Modifiers & Locks", "Phím khóa & bổ trợ"), |ui| {
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
                                                     "Chọn phím thủ công"
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
                                        if ui
                                            .add_sized([22.0, 20.0], Button::new(Self::material_icon_text(0xe145, 12.0)))
                                            .on_hover_text(Self::tr_lang(
                                                language,
                                                "Add step",
                                                "Thêm một bước vào đầu preset này",
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
                                            ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                                        }
                                        let record_text = if is_recording_this {
                                            Self::tr_lang(language, "Stop", "Dừng")
                                        } else {
                                            Self::tr_lang(language, "Record", "Ghi")
                                        };
                                        let record_btn = Button::new(
                                            RichText::new(format!("{} {}", Self::material_icon_text(record_icon, 10.0).text(), record_text))
                                                .color(dot_color)
                                                .strong()
                                        );
                                        if ui.add_sized([82.0, 20.0], record_btn)
                                            .on_hover_text(Self::tr_lang(
                                                language,
                                                "Record your keyboard and mouse clicks globally to automatically generate macro steps",
                                                "Ghi lại thao tác phím và click chuột toàn màn hình để tự động tạo bước macro",
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
                                        let capture_target = CaptureRequest::MacroPresetRecordHotkey(group.id, preset.id);
                                        let has_rec_hotkey = preset.record_hotkey.is_some();
                                        
                                        let capture_active = self.capture_target.as_ref() == Some(&capture_target);
                                        let pulse = if capture_active {
                                            let capture_time = ui.ctx().input(|input| input.time) as f32;
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
                                            ui.visuals().widgets.inactive.bg_fill
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
                                             ui.visuals().widgets.inactive.text_color()
                                         };

                                         let hover_text = if let Some(binding) = &preset.record_hotkey {
                                             let key_ui = Self::format_binding_ui(language, Some(binding));
                                             let fmt = Self::tr_lang(
                                                 language,
                                                 "Bound trigger key: {} (Click to clear)",
                                                 "Phím tắt đã gán: {} (Nhấp để xóa)",
                                             );
                                             fmt.replace("{}", &key_ui)
                                         } else {
                                             Self::tr_lang(
                                                 language,
                                                 "Click to bind a keyboard key to start/stop macro recording dynamically",
                                                 "Nhấp để gán phím tắt bắt đầu/dừng ghi macro nhanh",
                                             ).to_string()
                                         };

                                         let clicked = ui.scope(|ui| {
                                             ui.spacing_mut().button_padding = egui::vec2(6.0, 0.0);
                                             let kbd_btn = Button::new(
                                                 RichText::new(kbd_btn_text)
                                                     .color(text_color)
                                                     .strong()
                                                     .size(10.0)
                                             )
                                             .fill(capture_fill)
                                             .min_size(egui::vec2(20.0, 20.0));
                                             ui.add(kbd_btn)
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
                                         
                                         // Overlay status text label: "Recording..." / "Đang ghi..."
                                         if is_recording_this {
                                             let label_color = Color32::from_rgb(255, 96, 96);
                                             let is_even = (std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap_or_default().as_millis() / 500) % 2 == 0;
                                             let text_color = if is_even { label_color } else { label_color.linear_multiply(0.6) };
                                             ui.label(
                                                 RichText::new(Self::tr_lang(language, "Recording...", "Đang ghi..."))
                                                     .color(text_color)
                                                     .strong()
                                             );
                                         }
                                        ui.add_sized([30.0, 18.0], egui::Label::new(RichText::new("#").strong()));
                                        ui.add_sized([54.0, 18.0], egui::Label::new(RichText::new(Self::tr_lang(language, "Delay", "Delay")).strong()));
                                        ui.add_sized([154.0, 18.0], egui::Label::new(RichText::new(Self::tr_lang(language, "Action", "Action")).strong()));
                                        ui.add_sized([146.0, 18.0], egui::Label::new(""));
                                        let has_selected_steps = selected_steps_snapshot.iter().any(|(g_id, p_id, _)| *g_id == group.id && *p_id == preset.id);
                                         if has_selected_steps {
                                             let delete_btn = Button::new(Self::tr_lang(language, "Delete", "Xóa"))
                                                 .min_size(egui::vec2(64.0, 20.0));
                                             if ui
                                                 .add(delete_btn)
                                                 .on_hover_text(Self::tr_lang(language, "Delete selected steps", "Xóa các bước đã chọn"))
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
                                                .on_hover_text(Self::tr_lang(language, "Clear hotkey", "Xóa phím tắt"))
                                                .clicked()
                                            {
                                                preset.record_hotkey = None;
                                                live_sync = true;
                                            }
                                        }
                                    });
                                });

                            let loop_colors = Self::macro_loop_colors(&preset.steps);
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
                                } else if let Some(color) =
                                    loop_colors.get(step_index).and_then(|color| *color)
                                {
                                    color
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
                                            if ui
                                                .add_sized([22.0, 20.0], Button::new(Self::material_icon_text(0xe145, 12.0)))
                                                .on_hover_text(Self::tr_lang(language, "Add a new step below this one", "Thêm một bước mới phía dưới"))
                                                .clicked()
                                            {
                                                insert_step_after = Some((preset.id, step_index));
                                            }

                                            let select_icon = if is_selected {
                                                Self::material_icon_text(0xe834, 12.0).color(Color32::from_rgb(96, 232, 255))
                                            } else {
                                                Self::material_icon_text(0xe835, 12.0).color(Color32::from_rgb(180, 180, 180))
                                            };
                                            if ui
                                                .add_sized(
                                                    [22.0, 20.0],
                                                    Button::new(select_icon),
                                                )
                                                .on_hover_text(Self::tr_lang(language, "Select step", "Chọn bước này"))
                                                .clicked()
                                            {
                                                pending_step_selection = Some((
                                                    group.id,
                                                    preset.id,
                                                    step_index,
                                                    ui.input(|input| input.modifiers.ctrl),
                                                    ui.input(|input| input.modifiers.shift),
                                                ));
                                            }
                                            let is_dark_theme = self.state.ui_theme == UiThemeMode::Dark;
                                            let hint_color = if is_dark_theme {
                                                Color32::from_rgba_unmultiplied(110, 110, 110, 90)
                                            } else {
                                                Color32::from_rgba_unmultiplied(130, 130, 130, 90)
                                            };
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

                                                let enabled_icon = if step.enabled {
                                                    Self::material_icon_text(0xe834, 16.0).color(Color32::from_rgb(0, 255, 170))
                                                } else {
                                                    Self::material_icon_text(0xe835, 16.0).color(Color32::from_rgb(180, 180, 180))
                                                };
                                                if ui
                                                    .add_sized([22.0, 20.0], Button::new(enabled_icon))
                                                    .on_hover_text(Self::tr_lang(language, "Toggle step enabled", "Bật/Tắt bước này"))
                                                    .clicked()
                                                {
                                                    step.enabled = !step.enabled;
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
                                                        "Xóa bước này",
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
                                            if has_infinite_loop_warning || has_step_vision_leak || has_step_break_loop_warning {
                                                  let warn_color = if has_infinite_loop_warning || has_step_vision_leak {
                                                      Color32::from_rgb(255, 90, 0)
                                                  } else {
                                                      Color32::from_rgb(255, 200, 0)
                                                  };
                                                  let response = ui.add_sized([20.0, 20.0], egui::Button::new(
                                                      Self::material_icon_text(0xe002, 16.0).color(warn_color)
                                                  ).frame(false));
                                                  
                                                  if response.contains_pointer() {
                                                      egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), response.id.with("step-tip"), |ui| {
                                                          ui.horizontal(|ui| {
                                                              ui.label(Self::material_icon_text(0xe002, 14.0).color(warn_color));
                                                              ui.label(RichText::new(Self::tr_lang(language, "STEP WARNING", "CẢNH BÁO BƯỚC")).strong().color(warn_color));
                                                          });
                                                          if has_infinite_loop_warning {
                                                              ui.label(Self::tr_lang(
                                                                  language,
                                                                  "This step starts an infinite loop without an end point. The macro will run forever until you manually stop it.",
                                                                  "Bước này khởi đầu một vòng lặp vô tận mà không có điểm dừng, macro sẽ chạy mãi mãi cho đến khi bạn chủ động bấm dừng."
                                                              ));
                                                          }
                                                          if has_step_vision_leak {
                                                              ui.label(Self::tr_lang(
                                                                  language,
                                                                  "This step starts image search under Press/Release trigger, but there is no 'StopImageSearch' action in this macro! This could lead to a persistent background CPU thread. Add a 'StopImageSearch' step or change trigger to 'Hold'.",
                                                                  "Bước này bắt đầu tìm ảnh (chế độ Nhấn/Thả) nhưng macro không có bước dừng tìm ảnh! Điều này có thể gây chạy ngầm hao CPU. Hãy thêm bước dừng tìm ảnh hoặc đổi trigger sang Giữ (Hold)."
                                                              ));
                                                          }
                                                          if has_step_break_loop_warning {
                                                              ui.label(Self::tr_lang(
                                                                  language,
                                                                  "This step breaks a loop, but it is not placed inside any Loop Start / Loop End block! It will have no effect.",
                                                                  "Bước này thoát vòng lặp, nhưng nó hiện không nằm trong cặp khối Lặp (Loop Start) / Hết lặp (Loop End) nào! Nó sẽ không có tác dụng."
                                                              ));
                                                          }
                                                      });
                                                  }
                                              }
                                            if is_active {
                                                ui.add_sized([16.0, 20.0], egui::Label::new(
                                                    RichText::new("●")
                                                        .color(Color32::from_rgb(0, 255, 170))
                                                        .size(14.0)
                                                ))
                                                .on_hover_text(Self::tr_lang(language, "Step is running/active", "Bước này đang chạy/hoạt động"));
                                            } else {
                                                ui.allocate_space(vec2(16.0, 20.0));
                                            }
                                            let step_num_text = format!("{}", step_index + 1);
                                            let label_width = if has_infinite_loop_warning || has_step_vision_leak || has_step_break_loop_warning { 18.0 } else { 30.0 };
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
                                                ),
                                            );
                                            live_sync |= ui
                                                .add_sized(
                                                    [54.0, 18.0],
                                                    DragValue::new(&mut step.delay_ms).range(0..=600000),
                                                )
                                                .changed();
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
                                                        "🔄 Tự động bật/tắt bước khi chạy"
                                                    )).changed();
                                                    ui.separator();
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
                                                                 MacroAction::IfStart,
                                                                 MacroAction::Else,
                                                                 MacroAction::IfEnd,
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
                                                            );
                                                            Self::render_image_search_action_group_option(
                                                                ui,
                                                                language,
                                                                (group.id, preset.id, step_index, "image-search-group"),
                                                                &mut step.action,
                                                                &mut live_sync,
                                                            );
                                                            Self::render_timer_action_group_option(
                                                                ui,
                                                                language,
                                                                (group.id, preset.id, step_index, "timer-group"),
                                                                &mut step.action,
                                                                &mut live_sync,
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
                                                        .unwrap_or_else(|| Self::tr_lang(language, "Select window preset", "Chọn preset cửa sổ").to_owned());
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
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .window_focus_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| Self::tr_lang(language, "Select focus preset", "Chọn preset focus").to_owned());
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "focus-window-preset-step"))
                                                        .width(160.0)
                                                        .selected_text(selected_label)
                                                        .show_ui(ui, |ui| {
                                                            for preset_option in &self.state.window_focus_presets {
                                                                if ui
                                                                    .selectable_label(selected_id == Some(preset_option.id), &preset_option.name)
                                                                    .clicked()
                                                                {
                                                                    step.key = preset_option.id.to_string();
                                                                    live_sync = true;
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
                                                            Self::tr_lang(language, "Select macro preset", "Select macro preset").to_owned()
                                                        });
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "trigger-macro-preset-step"))
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
                                                                Self::tr_lang(
                                                                    language,
                                                                    "Select custom preset",
                                                                    "Chọn preset câu lệnh",
                                                                )
                                                                .to_owned()
                                                            } else {
                                                                step.key.clone()
                                                            }
                                                        });
                                                    let custom_preset_combo = egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "trigger-custom-preset-step"))
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
                                                    let (custom_draft_changed, custom_save_request, custom_save_and_open_ai_request, open_ai_preset_id) = Self::render_custom_preset_step_draft_popup(
                                                         ui,
                                                         &custom_preset_combo.response,
                                                         &custom_preset_combo.response,
                                                         step,
                                                         (group.id, preset.id, step_index),
                                                         Some(step_index),
                                                         language,
                                                         &command_presets_snapshot,
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
                                                            Self::tr_lang(language, "Select macro preset", "Select macro preset").to_owned()
                                                        });
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "macro-enable-preset-step"))
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
                                                        let preset_label = group_preset_options.iter()
                                                            .find(|(id, _)| *id == current_preset_id)
                                                            .map(|(_, label)| label.clone())
                                                            .unwrap_or_else(|| Self::tr_lang(language, "Select preset", "Chọn preset").to_owned());
                                                            
                                                        egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "step-preset-select"))
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
                                                            Self::tr_lang(language, "Select steps", "Chọn steps").to_owned()
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
                                                        .unwrap_or_else(|| Self::tr_lang(language, "Select timer", "Chọn hẹn giờ").to_owned());
                                                    egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "step-timer-preset-select"))
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
                                                        Self::tr_lang(language, "Select crosshair preset", "Select crosshair preset").to_owned()
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
                                                        .unwrap_or_else(|| Self::tr_lang(language, "Select pin preset", "Chọn preset ghim").to_owned());
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
                                                        .unwrap_or_else(|| Self::tr_lang(language, "Select mouse path", "Chọn preset đường chuột").to_owned());
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
                                                        .unwrap_or_else(|| "Select image search preset".to_owned());
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
                                                                    "Di chuyển chuột tới ảnh tìm thấy rồi mới tiếp tục.",
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
                                                                    "Tiếp tục dò cho tới khi thấy ảnh.",
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
                                                                    "Kích hoạt một preset macro khác trong cùng group.",
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
                                                                    .unwrap_or_else(|| "Select macro preset".to_owned());
                                                                egui::ComboBox::from_id_salt((
                                                                    group.id,
                                                                    preset.id,
                                                                    step_index,
                                                                    "image-search-trigger-macro-preset",
                                                                    ))
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
                                                        .unwrap_or_else(|| "Select zoom preset".to_owned());
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
                                                        .unwrap_or_else(|| Self::tr_lang(language, "Select sound preset", "Chọn preset âm thanh").to_owned());
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
                                                } else if step.action == MacroAction::ApplyMouseSensitivityPreset {
                                                    live_sync |= ui.checkbox(&mut step.manual_mouse_sensitivity, Self::tr_lang(language, "Manual", "Nhập tay")).changed();
                                                    if step.manual_mouse_sensitivity {
                                                        ui.vertical(|ui| {
                                                            let response = ui.add_sized(
                                                                [96.0, 18.0],
                                                                TextEdit::singleline(&mut step.key)
                                                                    .hint_text(RichText::new(Self::tr_lang(language, "value/expr", "giá trị")).color(hint_color).weak()),
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
                                                                UiLanguage::Vietnamese => format!("Kết quả: {} (giới hạn: {} trong 1..20)", evaluated, clamped),
                                                                _ => format!("Evaluated: {} (clamped to: {} within 1..20)", evaluated, clamped),
                                                            };
                                                            response.on_hover_text(tooltip_text);

                                                            Self::render_variable_suggestions(ui, &mut step.key, language);
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
                                                                Self::tr_lang(
                                                                    language,
                                                                    "Select mouse sensitivity preset",
                                                                    "Chọn preset độ nhạy",
                                                                )
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
                                                } else if matches!(step.action, MacroAction::LockKeys | MacroAction::UnlockKeys) {
                                                    let response = ui.add_sized(
                                                        [146.0, 18.0],
                                                        TextEdit::singleline(&mut step.key)
                                                            .hint_text(RichText::new(Self::tr_lang(language, "A,S,W,D", "A,S,W,D")).color(hint_color).italics()),
                                                    );
                                                    Self::apply_vietnamese_input_if_changed(
                                                        &response,
                                                        self.state.vietnamese_input_enabled,
                                                        self.state.vietnamese_input_mode,
                                                        &mut step.key,
                                                     );
                                                     live_sync |= response.changed();
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
                                                             .color(Color32::from_rgb(20, 20, 20)),
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
                                                              let response = ui.add_sized(
                                                                  [80.0, 18.0],
                                                                  TextEdit::singleline(&mut step.key).hint_text(
                                                                      RichText::new(Self::tr_lang(
                                                                          language,
                                                                          "Loop count",
                                                                          "Số lần lặp",
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
                                                              Self::render_variable_suggestions(ui, &mut step.key, language);
                                                          });
                                                      }
                                                 } else if step.action == MacroAction::StopIfKeyPressed {
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
                                                                    "Select HUD preset",
                                                                    "Chọn HUD preset",
                                                                )
                                                                .to_owned()
                                                            } else {
                                                                match language {
                                                                    UiLanguage::Vietnamese => format!("Cũ: {}", step.key),
                                                                    _ => format!("Legacy: {}", step.key),
                                                                }
                                                            }
                                                        });
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.spacing_mut().interact_size.y = 18.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "toolbox-preset-step"))
                                                            .width(104.0)
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
                                                        let response = ui.add_sized(
                                                            [122.0, 18.0],
                                                            TextEdit::singleline(&mut step.text_override)
                                                                .hint_text(RichText::new(Self::tr_lang(language, "Text override", "Ghi đè văn bản")).color(hint_color).italics()),
                                                        );
                                                        Self::apply_vietnamese_input_if_changed(
                                                            &response,
                                                            self.state.vietnamese_input_enabled,
                                                            self.state.vietnamese_input_mode,
                                                            &mut step.text_override,
                                                        );
                                                        live_sync |= response.changed();
                                                     });
                                                } else if step.action == MacroAction::TypeText {
                                                     ui.vertical(|ui| {
                                                         let response = ui.add_sized(
                                                             [146.0, 18.0],
                                                             TextEdit::singleline(&mut step.key)
                                                                 .hint_text(RichText::new(Self::tr_lang(language, "Text to type", "Text to type")).color(hint_color).italics()),
                                                         );
                                                         Self::apply_vietnamese_input_if_changed(
                                                             &response,
                                                             self.state.vietnamese_input_enabled,
                                                             self.state.vietnamese_input_mode,
                                                             &mut step.key,
                                                         );
                                                         live_sync |= response.changed();
                                                         Self::render_variable_suggestions(ui, &mut step.key, language);
                                                     });
                                                } else if step.action == MacroAction::DisableCrosshair {
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.spacing_mut().interact_size.y = 18.0;
                                                        ui.spacing_mut().button_padding.y = 0.0;
                                                        ui.horizontal(|ui| {
                                                            let response = ui.checkbox(&mut step.lock_mouse_left, Self::tr_lang(language, "All", "Tắt hết"));
                                                            live_sync |= response.changed();
                                                            if !step.lock_mouse_left {
                                                                let selected_label = if step.key.trim().is_empty() {
                                                                    Self::tr_lang(language, "Select profile", "Chọn profile").to_owned()
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
                                                            let response = ui.checkbox(&mut step.lock_mouse_left, Self::tr_lang(language, "All", "Tắt hết"));
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
                                                                        Self::tr_lang(language, "Select pin", "Chọn preset ghim").to_owned()
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
                                                } else if matches!(step.action, MacroAction::DisableZoom | MacroAction::Else | MacroAction::IfEnd) {
                                                    ui.add_sized(
                                                        [146.0, 18.0],
                                                        egui::Label::new(Self::tr_lang(language, "No input", "No input")),
                                                    );
                                                 } else if step.action == MacroAction::IfStart {
                                                     ui.scope(|ui| {
                                                         ui.spacing_mut().item_spacing.x = 4.0;
                                                         ui.spacing_mut().interact_size.y = 22.0;
                                                         ui.spacing_mut().button_padding.y = 0.0;
                                                         ui.vertical(|ui| {
                                                             ui.horizontal(|ui| {
                                                                 let response = ui.add_sized(
                                                                     [64.0, 22.0],
                                                                     TextEdit::singleline(&mut step.if_variable_name)
                                                                         .hint_text(RichText::new(Self::tr_lang(language, "variable", "biến")).color(hint_color).weak()),
                                                                 );
                                                                 Self::apply_vietnamese_input_if_changed(
                                                                     &response,
                                                                     self.state.vietnamese_input_enabled,
                                                                     self.state.vietnamese_input_mode,
                                                                     &mut step.if_variable_name,
                                                                 );
                                                                 live_sync |= response.changed();
                                                                 
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
                                                                 
                                                                 live_sync |= ui.add_sized(
                                                                     [46.0, 22.0],
                                                                     DragValue::new(&mut step.if_compare_value).range(-1_000_000..=1_000_000),
                                                                 ).changed();

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
                                                             Self::render_variable_suggestions_raw(ui, &mut step.if_variable_name, language);
                                                         });
                                                     });
                                                 } else if step.action == MacroAction::SetVariable {
                                                     ui.scope(|ui| {
                                                         ui.spacing_mut().item_spacing.x = 4.0;
                                                         ui.spacing_mut().interact_size.y = 22.0;
                                                         ui.spacing_mut().button_padding.y = 0.0;
                                                         ui.vertical(|ui| {
                                                             ui.horizontal(|ui| {
                                                                 let response = ui.add_sized(
                                                                     [64.0, 22.0],
                                                                     TextEdit::singleline(&mut step.if_variable_name)
                                                                         .hint_text(RichText::new(Self::tr_lang(language, "variable", "biến")).color(hint_color).weak()),
                                                                 );
                                                                 Self::apply_vietnamese_input_if_changed(
                                                                     &response,
                                                                     self.state.vietnamese_input_enabled,
                                                                     self.state.vietnamese_input_mode,
                                                                     &mut step.if_variable_name,
                                                                 );
                                                                 live_sync |= response.changed();

                                                                 ui.label(" = ");

                                                                 let response2 = ui.add_sized(
                                                                     [76.0, 22.0],
                                                                     TextEdit::singleline(&mut step.key)
                                                                         .hint_text(RichText::new(Self::tr_lang(language, "value/expr", "giá trị")).color(hint_color).weak()),
                                                                 );
                                                                 Self::apply_vietnamese_input_if_changed(
                                                                     &response2,
                                                                     self.state.vietnamese_input_enabled,
                                                                     self.state.vietnamese_input_mode,
                                                                     &mut step.key,
                                                                 );
                                                                 live_sync |= response2.changed();

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
                                                             Self::render_variable_suggestions_raw(ui, &mut step.if_variable_name, language);
                                                             Self::render_variable_suggestions(ui, &mut step.key, language);
                                                         });
                                                     });
                                                } else if matches!(step.action, MacroAction::StartVisionSearch | MacroAction::TriggerVisionMove | MacroAction::StopVision | MacroAction::StopVisionWait) {
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
                                                                    "Select vision preset",
                                                                    "Chọn preset hình ảnh",
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

                                                    let mut text_edit = TextEdit::singleline(&mut display_key);
                                                    if step_capture_active {
                                                        text_edit = text_edit.hint_text(Self::tr_lang(
                                                            language,
                                                            "Capturing...",
                                                            "Đang lấy phím...",
                                                        ));
                                                    }

                                                    let response =
                                                        ui.add_sized([146.0, 18.0], text_edit);
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
                                                            "Thu nhỏ app rồi bấm vào bất kỳ vị trí nào trên màn hình để lấy X/Y.",
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
                                                        "Di chuyển chuột với tốc độ đều",
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
                                                live_sync |= ui
                                                    .checkbox(&mut step.timed_override, "T")
                                                    .on_hover_text(Self::tr_lang(
                                                        language,
                                                        "Timed display",
                                                        "Dùng thời gian hiển thị riêng cho step này",
                                                        ))
                                                    .changed();
                                                ui.add_enabled_ui(step.timed_override, |ui| {
                                                    live_sync |= ui
                                                        .add_sized(
                                                            [72.0, 18.0],
                                                            DragValue::new(&mut step.duration_override_ms)
                                                                .range(50..=60_000)
                                                                .suffix(" ms"),
                                                        )
                                                        .changed();
                                                });
                                            } else if action_supports_capture {
                                                let step_capture_target = CaptureRequest::MacroStepInput {
                                                    group_id: group.id,
                                                    preset_id: preset.id,
                                                    step_index,
                                                };
                                                let step_capture_active =
                                                    capture_target_snapshot.as_ref() == Some(&step_capture_target);
                                                let step_capture_button = if step_capture_active {
                                                    Button::new(Self::material_icon_text(0xe312, 12.0))
                                                        .fill(Color32::from_rgb(88, 84, 44))
                                                } else {
                                                    Button::new(Self::material_icon_text(0xe312, 12.0))
                                                };
                                                if ui
                                                    .add_sized([22.0, 18.0], step_capture_button)
                                                    .on_hover_text(Self::tr_lang(
                                                        language,
                                                        "Bắt input",
                                                        "Bắt phím cho bước này",
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
                                                    
                                                    ui.menu_button(Self::tr_lang(language, "Letters (A-Z)", "Chữ cái (A-Z)"), |ui| {
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

                                                    ui.menu_button(Self::tr_lang(language, "Numbers & Symbols", "Số & Kí tự"), |ui| {
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

                                                    ui.menu_button(Self::tr_lang(language, "Navigation", "Điều hướng & Phím tắt"), |ui| {
                                                        ui.set_max_width(160.0);
                                                        for key in ["Escape", "Enter", "Space", "Backspace", "Tab", "Insert", "Delete", "Home", "End", "PageUp", "PageDown", "Left", "Up", "Right", "Down", "PrintScreen", "Pause"] {
                                                            if ui.button(key).clicked() {
                                                                step.key = key.to_string();
                                                                live_sync = true;
                                                                ui.close_menu();
                                                            }
                                                        }
                                                    });

                                                    ui.menu_button(Self::tr_lang(language, "Function (F1-F24)", "Phím chức năng"), |ui| {
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

                                                    ui.menu_button(Self::tr_lang(language, "Numpad", "Bàn phím số phụ"), |ui| {
                                                        ui.set_max_width(160.0);
                                                        for key in ["Numpad0", "Numpad1", "Numpad2", "Numpad3", "Numpad4", "Numpad5", "Numpad6", "Numpad7", "Numpad8", "Numpad9", "NumpadMultiply", "NumpadAdd", "NumpadSubtract", "NumpadDecimal", "NumpadDivide"] {
                                                            if ui.button(key).clicked() {
                                                                step.key = key.to_string();
                                                                live_sync = true;
                                                                ui.close_menu();
                                                            }
                                                        }
                                                    });

                                                    ui.menu_button(Self::tr_lang(language, "Modifiers & Locks", "Phím khóa & bổ trợ"), |ui| {
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
                                                    "Chọn phím thủ công"
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
                                                    .add(
                                                        Button::new(Self::tr_lang(language, "Copy", "Copy"))
                                                            .min_size(vec2(paste_button_width, 18.0)),
                                                     )
                                                     .on_hover_text(Self::tr_lang(
                                                         language,
                                                         "Copy this step.",
                                                         "Copy step này.",
                                                     ))
                                                     .clicked()
                                                 {
                                                     copy_single_step = Some((group.id, preset.id, step_index));
                                                 }

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
                                                                "Tự động bật/tắt bước khi chạy (trạng thái chạy lặp/chạy tiếp)"
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
                            }
                            if drag_payload.is_some() && !preview_drawn {
                                preview_drop_index = steps_len;
                                paint_drop_preview(ui);
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
                                    selection_after_move = Some((
                                        group.id,
                                        preset_id,
                                        (insert_at..insert_at + indices.len()).collect::<Vec<_>>(),
                                    ));
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
                    if let Some((group_id, preset_id, moved_indices)) = selection_after_move {
                        self.clear_macro_step_selection_for_preset(group_id, preset_id);
                        for moved_index in moved_indices {
                            self.selected_macro_steps
                                .insert((group_id, preset_id, moved_index));
                        }
                    }
                    if let Some((group_id, preset_id, pasted_indices)) = selection_after_paste {
                        self.clear_macro_step_selection_for_preset(group_id, preset_id);
                        for pasted_index in pasted_indices {
                            self.selected_macro_steps
                                .insert((group_id, preset_id, pasted_index));
                        }
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
        ui.add_space((ui.ctx().screen_rect().height() - 250.0).max(0.0));
    }

    pub(crate) fn render_variable_inspector(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        ui.vertical(|ui| {
            ui.add_space(4.0);
            
            // Header with some actions
            ui.horizontal(|ui| {
                ui.label(RichText::new(Self::tr_lang(
                    language,
                    "Active Runtime Variables",
                    "Các biến đang hoạt động",
                )).strong());
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(Self::tr_lang(language, "Clear All", "Xóa hết")).clicked() {
                        let mut vars = crate::overlay::RUNTIME_VARIABLES.lock();
                        vars.clear();
                    }
                });
            });
            ui.separator();

            // Render variables table
            let vars_snapshot = {
                let vars = crate::overlay::RUNTIME_VARIABLES.lock();
                let mut list: Vec<(String, i32)> = vars.iter().map(|(k, v)| (k.clone(), *v)).collect();
                list.sort_by(|a, b| a.0.cmp(&b.0));
                list
            };

            if vars_snapshot.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(RichText::new(Self::tr_lang(
                        language,
                        "No variables active yet.\n(Run a macro or set a variable)",
                        "Chưa có biến nào hoạt động.\n(Chạy macro hoặc thiết lập biến)",
                    )).italics().color(ui.visuals().weak_text_color()));
                    ui.add_space(20.0);
                });
            } else {
                egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                    egui::Grid::new("variable_inspector_grid")
                        .num_columns(3)
                        .spacing([8.0, 6.0])
                        .striped(true)
                        .show(ui, |ui| {
                            // Headers
                            ui.label(RichText::new(Self::tr_lang(language, "Name", "Tên biến")).strong());
                            ui.label(RichText::new(Self::tr_lang(language, "Value", "Giá trị")).strong());
                            ui.label(""); // Actions column
                            ui.end_row();

                            let mut to_remove = None;
                            let mut to_update = None;

                            for (name, val) in &vars_snapshot {
                                ui.label(RichText::new(name).monospace());
                                
                                // Direct value editing
                                let mut val_str = val.to_string();
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut val_str)
                                        .desired_width(70.0)
                                        .font(egui::FontId::monospace(14.0))
                                );
                                if response.lost_focus() || response.clicked_elsewhere() {
                                    if let Ok(new_val) = val_str.trim().parse::<i32>() {
                                        to_update = Some((name.clone(), new_val));
                                    }
                                }

                                // Delete variable button
                                if ui.button(Self::material_icon_text(0xe872, 14.0)) // trash
                                    .on_hover_text(Self::tr_lang(language, "Delete variable", "Xóa biến"))
                                    .clicked() 
                                {
                                    to_remove = Some(name.clone());
                                }
                                ui.end_row();
                            }

                            if let Some(name) = to_remove {
                                let mut vars = crate::overlay::RUNTIME_VARIABLES.lock();
                                vars.remove(&name);
                                if let Some((ref up_name, _)) = to_update {
                                    if up_name == &name {
                                        to_update = None;
                                    }
                                }
                            }

                            if let Some((name, new_val)) = to_update {
                                let mut vars = crate::overlay::RUNTIME_VARIABLES.lock();
                                vars.insert(name, new_val);
                            }
                        });
                });
            }

            // Quick set variable utility at the bottom
            ui.separator();
            ui.horizontal(|ui| {
                ui.set_row_height(24.0);
                
                let id_name = ui.id().with("new_var_name");
                let id_val = ui.id().with("new_var_val");
                
                let mut name_buf = ui.memory(|mem| mem.data.get_temp::<String>(id_name).unwrap_or_default());
                let mut val_buf = ui.memory(|mem| mem.data.get_temp::<String>(id_val).unwrap_or_default());

                let is_dark_theme = self.state.ui_theme == UiThemeMode::Dark;
                let hint_color = if is_dark_theme {
                    Color32::from_rgba_unmultiplied(110, 110, 110, 90)
                } else {
                    Color32::from_rgba_unmultiplied(130, 130, 130, 90)
                };

                ui.add_sized(
                    [100.0, 20.0],
                    egui::TextEdit::singleline(&mut name_buf)
                        .hint_text(RichText::new(Self::tr_lang(language, "Name", "Tên")).color(hint_color).weak())
                );
                ui.label("=");
                ui.add_sized(
                    [70.0, 20.0],
                    egui::TextEdit::singleline(&mut val_buf)
                        .hint_text(RichText::new(Self::tr_lang(language, "Value", "Giá trị")).color(hint_color).weak())
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

    fn render_variable_suggestions(ui: &mut egui::Ui, text: &mut String, language: UiLanguage) {
        let vars_snapshot = {
            let vars = crate::overlay::RUNTIME_VARIABLES.lock();
            let mut list: Vec<String> = vars.keys().cloned().collect();
            list.sort();
            list
        };

        if !vars_snapshot.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new(Self::tr_lang(
                    language,
                    "Insert variable:",
                    "Chèn nhanh biến:",
                )).size(11.0).color(ui.visuals().weak_text_color()));
                
                for var_name in vars_snapshot {
                    let btn_text = format!("{{{}}}", var_name);
                    let button = egui::Button::new(RichText::new(&btn_text).size(11.0).monospace())
                        .small()
                        .fill(ui.visuals().widgets.noninteractive.bg_fill);
                    let hover_text = match language {
                        UiLanguage::Vietnamese => format!("Bấm để chèn {{{}}} vào vị trí", var_name),
                        _ => format!("Click to insert {{{}}}", var_name),
                    };
                    if ui.add(button).on_hover_text(hover_text).clicked() {
                        if text.is_empty() {
                            *text = btn_text;
                        } else {
                            text.push_str(&btn_text);
                        }
                    }
                }
            });
        }
    }

    fn render_variable_suggestions_raw(ui: &mut egui::Ui, text: &mut String, language: UiLanguage) {
        let vars_snapshot = {
            let vars = crate::overlay::RUNTIME_VARIABLES.lock();
            let mut list: Vec<String> = vars.keys().cloned().collect();
            list.sort();
            list
        };

        if !vars_snapshot.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new(Self::tr_lang(
                    language,
                    "Choose variable:",
                    "Chọn nhanh biến:",
                )).size(11.0).color(ui.visuals().weak_text_color()));
                
                for var_name in vars_snapshot {
                    let button = egui::Button::new(RichText::new(&var_name).size(11.0).monospace())
                        .small()
                        .fill(ui.visuals().widgets.noninteractive.bg_fill);
                    let hover_text = match language {
                        UiLanguage::Vietnamese => format!("Bấm để chọn {}", var_name),
                        _ => format!("Click to select {}", var_name),
                    };
                    if ui.add(button).on_hover_text(hover_text).clicked() {
                        *text = var_name;
                    }
                }
            });
        }
    }
}
