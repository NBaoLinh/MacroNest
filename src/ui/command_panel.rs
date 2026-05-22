use eframe::egui::{self, Button, RichText, TextEdit, Color32};
use crate::model::*;
use crate::overlay::OverlayCommand;
use crate::ai;
use crate::ui::CrosshairApp;


impl CrosshairApp {
    pub(crate) fn render_commands_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        let previous_item_spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            if ui
                .button(self.tr("+ Add command preset", "+ Thêm preset lệnh"))
                .clicked()
            {
                self.add_custom_preset();
                self.persist_command_presets();
            }
        });

        ui.add_space(8.0);

        let mut remove_id = None;
        let mut changed = false;
        let mut open_ai_dialog: Option<u32> = None;
        let open_windows = self.open_windows.clone();
        for index in 0..self.state.command_presets.len() {
            ui.add_space(6.0);
            let preset = &mut self.state.command_presets[index];
            preset.enabled = true;
            Self::show_preset_card(ui, true, |ui| {
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
                    if Self::sound_style_toggle_button(
                        ui,
                        Self::tr_lang(language, "Run", "Chạy"),
                    )
                    .on_hover_text(Self::tr_lang(
                        language,
                        "Execute this custom preset immediately",
                        "Chạy câu lệnh này ngay lập tức",
                    ))
                    .clicked()
                    {
                        let command_text = ai::normalize_command_text(&preset.command);
                        if !command_text.is_empty() {
                            crate::overlay::spawn_custom_command(preset.use_powershell, command_text);
                        }
                    }
                    ui.add_space(6.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_sized(
                                [40.0, 24.0],
                                Button::new(Self::ai_badge_text(false))
                                    .fill(Self::ai_badge_fill())
                                    .stroke(Self::ai_badge_stroke())
                            )
                            .clicked()
                        {
                            open_ai_dialog = Some(preset.id);
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
                        if Self::sound_style_remove_button(ui).clicked() {
                            remove_id = Some(preset.id);
                        }
                    });
                });

                if preset.collapsed {
                    return;
                }

                if preset.use_powershell {
                    preset.use_powershell = false;
                    changed = true;
                }

                egui::Grid::new((preset.id, "custom-preset-grid"))
                    .num_columns(2)
                    .spacing([14.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(Self::tr_lang(language, "Target Window", "Target Window"));
                        changed |= Self::render_multi_window_targets_with_duplicate_mode(
                            ui,
                            language,
                            (preset.id, "custom-target-window"),
                            Self::tr_lang(language, "Any focused window", "Cửa sổ đang focus"),
                            &mut preset.target_window_title,
                            &mut preset.extra_target_window_titles,
                            &mut preset.match_duplicate_window_titles,
                            &open_windows,
                        );
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Shell", "Shell (Dòng lệnh)"));
                        ui.label(Self::material_icon_text(0xeb8e, 15.0));
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Command", "Câu lệnh"));
                        let command_hint = RichText::new(Self::tr_lang(
                            language,
                            "Example: shutdown /s /t 0",
                            "Ví dụ: shutdown /s /t 0",
                        ))
                        .italics()
                        .color(Color32::from_rgba_unmultiplied(120, 120, 120, 140));
                        changed |= ui
                            .add_sized(
                                [ui.available_width().max(240.0), 92.0],
                                TextEdit::multiline(&mut preset.command)
                                    .desired_rows(4)
                                    .hint_text(command_hint),
                            )
                            .changed();
                        ui.end_row();
                    });
            });
        }

        if let Some(id) = remove_id {
            self.state.command_presets.retain(|preset| preset.id != id);
            changed = true;
        }
        if let Some(preset_id) = open_ai_dialog {
            self.open_command_ai_dialog_for_preset(preset_id);
        }
        if changed {
            self.persist_command_presets();
        }
        ui.spacing_mut().item_spacing = previous_item_spacing;
    }

    pub(crate) fn add_custom_preset(&mut self) {
        let id = self.state.next_command_preset_id.max(1);
        self.state.next_command_preset_id = id + 1;
        self.state.command_presets.push(CommandPreset::new(id));
        self.sync_command_presets();
        self.status = format!("Added custom preset {id}.");
    }

    pub(crate) fn persist_command_presets(&mut self) {
        self.sync_command_presets();
        self.persist();
    }

    pub(crate) fn sync_command_presets(&self) {
        let _ = self.overlay_tx.send(OverlayCommand::UpdateCommandPresets(
            self.state.command_presets.clone(),
        ));
    }
}
