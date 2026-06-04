use crate::model::{
    AudioSensePreset, AudioSensePresetKind, AudioSenseSource, PitchAudioSenseSettings,
    SpatialAudioSenseSettings, UiLanguage,
};
use crate::ui::CrosshairApp;
use eframe::egui::{self, ComboBox, DragValue, TextEdit};

impl CrosshairApp {
    pub(crate) fn render_audiosense_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        ui.add_space(2.0);

        ui.horizontal(|ui| {
            if ui
                .button(Self::tr_lang(language, "+ Pitch preset", "+ Preset cao do"))
                .clicked()
            {
                let id = self
                    .state
                    .audio_sense_presets
                    .iter()
                    .map(|preset| preset.id)
                    .max()
                    .unwrap_or(0)
                    + 1;
                self.state.next_audio_sense_preset_id = id + 1;
                self.state
                    .audio_sense_presets
                    .push(AudioSensePreset::new_pitch(id));
                self.sync_audio_sense_presets();
                self.persist();
            }

            if ui
                .button(Self::tr_lang(language, "+ Spatial preset", "+ Preset am thanh"))
                .clicked()
            {
                let id = self
                    .state
                    .audio_sense_presets
                    .iter()
                    .map(|preset| preset.id)
                    .max()
                    .unwrap_or(0)
                    + 1;
                self.state.next_audio_sense_preset_id = id + 1;
                self.state
                    .audio_sense_presets
                    .push(AudioSensePreset::new_spatial(id));
                self.sync_audio_sense_presets();
                self.persist();
            }
        });

        ui.add_space(8.0);

        let mut remove_id = None;
        let mut changed = false;

        for preset in &mut self.state.audio_sense_presets {
            let is_pitch = preset.kind == AudioSensePresetKind::Pitch;
            let is_preview_running = if is_pitch {
                self.active_pitch_preview_preset_id == Some(preset.id)
                    && self.pitch_monitor.snapshot().running
            } else {
                self.active_spatial_preview_preset_id == Some(preset.id)
                    && self.spatial_monitor.snapshot().running
            };

            ui.add_space(6.0);
            Self::show_preset_card(ui, preset.enabled, |ui| {
                ui.horizontal(|ui| {
                    let name_width = Self::preset_header_name_width(ui);
                    let response =
                        ui.add_sized([name_width, 21.0], TextEdit::singleline(&mut preset.name));
                    Self::apply_vietnamese_input_if_changed(
                        &response,
                        self.state.vietnamese_input_enabled,
                        self.state.vietnamese_input_mode,
                        &mut preset.name,
                    );
                    changed |= response.changed();

                    ui.label(match preset.kind {
                        AudioSensePresetKind::Pitch => {
                            Self::tr_lang(language, "Pitch", "Cao do")
                        }
                        AudioSensePresetKind::Spatial => {
                            Self::tr_lang(language, "Spatial", "Huong am")
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Delete").clicked() {
                            remove_id = Some(preset.id);
                        }
                        if ui
                            .button(if preset.collapsed {
                                Self::tr_lang(language, "Show", "Hien")
                            } else {
                                Self::tr_lang(language, "Hide", "An")
                            })
                            .clicked()
                        {
                            preset.collapsed = !preset.collapsed;
                            changed = true;
                        }
                        if is_preview_running {
                            if ui.button("Stop").clicked() {
                                if is_pitch {
                                    self.pitch_monitor.stop();
                                    self.active_pitch_preview_preset_id = None;
                                } else {
                                    self.spatial_monitor.stop();
                                    self.active_spatial_preview_preset_id = None;
                                }
                            }
                        } else if ui.button("Start").clicked() {
                            if is_pitch {
                                self.spatial_monitor.stop();
                                self.active_spatial_preview_preset_id = None;
                                let _ = self.pitch_monitor.start(preset.pitch.clone());
                                self.active_pitch_preview_preset_id = Some(preset.id);
                            } else {
                                self.pitch_monitor.stop();
                                self.active_pitch_preview_preset_id = None;
                                let _ = self.spatial_monitor.start(preset.spatial.clone());
                                self.active_spatial_preview_preset_id = Some(preset.id);
                            }
                        }
                    });
                });

                if !preset.collapsed {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(Self::tr_lang(language, "Source", "Nguon"));
                        ComboBox::from_id_salt(("audiosense-source", preset.id))
                            .width(120.0)
                            .selected_text(match active_monitor_settings(preset).source {
                                AudioSenseSource::System => Self::tr_lang(language, "System", "He thong"),
                                AudioSenseSource::Microphone => {
                                    Self::tr_lang(language, "Microphone", "Micro")
                                }
                            })
                            .show_ui(ui, |ui| {
                                changed |= ui
                                    .selectable_value(
                                        &mut active_monitor_settings_mut(preset).source,
                                        AudioSenseSource::System,
                                        Self::tr_lang(language, "System", "He thong"),
                                    )
                                    .changed();
                                changed |= ui
                                    .selectable_value(
                                        &mut active_monitor_settings_mut(preset).source,
                                        AudioSenseSource::Microphone,
                                        Self::tr_lang(language, "Microphone", "Micro"),
                                    )
                                    .changed();
                            });

                        if active_monitor_settings(preset).source == AudioSenseSource::Microphone {
                            ComboBox::from_id_salt(("audiosense-device", preset.id))
                                .width(210.0)
                                .selected_text(
                                    active_monitor_settings(preset)
                                        .input_device_name
                                        .clone()
                                        .unwrap_or_else(|| {
                                            Self::tr_lang(
                                                language,
                                                "Default microphone",
                                                "Micro mac dinh",
                                            )
                                            .to_owned()
                                        }),
                                )
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(
                                            active_monitor_settings(preset)
                                                .input_device_name
                                                .is_none(),
                                            Self::tr_lang(
                                                language,
                                                "Default microphone",
                                                "Micro mac dinh",
                                            ),
                                        )
                                        .clicked()
                                    {
                                        active_monitor_settings_mut(preset).input_device_name = None;
                                        changed = true;
                                    }
                                    for device in &self.audio_sense_devices {
                                        if ui
                                            .selectable_label(
                                                active_monitor_settings(preset)
                                                    .input_device_name
                                                    .as_deref()
                                                    == Some(device.as_str()),
                                                device,
                                            )
                                            .clicked()
                                        {
                                            active_monitor_settings_mut(preset).input_device_name =
                                                Some(device.clone());
                                            changed = true;
                                        }
                                    }
                                });
                        }
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Hz");
                        changed |= ui
                            .add(
                                DragValue::new(&mut active_monitor_settings_mut(preset).updates_per_second)
                                    .range(1..=30)
                                    .speed(0.2),
                            )
                            .changed();
                        changed |= ui
                            .checkbox(
                                &mut active_monitor_settings_mut(preset).listen_forever,
                                Self::tr_lang(language, "Listen forever", "Nghe vinh vien"),
                            )
                            .changed();
                        if !active_monitor_settings(preset).listen_forever {
                            ui.label("ms");
                            changed |= ui
                                .add(
                                    DragValue::new(&mut active_monitor_settings_mut(preset).duration_ms)
                                        .range(100..=60_000)
                                        .speed(10.0),
                                )
                                .changed();
                        }
                    });

                    ui.add_space(4.0);
                    match preset.kind {
                        AudioSensePresetKind::Pitch => {
                            changed |= render_pitch_settings_ui(
                                ui,
                                language,
                                &mut preset.pitch,
                                &self.pitch_monitor.snapshot(),
                            );
                        }
                        AudioSensePresetKind::Spatial => {
                            changed |= render_spatial_settings_ui(
                                ui,
                                language,
                                &mut preset.spatial,
                                &self.spatial_monitor.snapshot(),
                            );
                        }
                    }
                }
            });
        }

        if let Some(remove_id) = remove_id {
            self.state.audio_sense_presets.retain(|preset| preset.id != remove_id);
            if self.active_pitch_preview_preset_id == Some(remove_id) {
                self.pitch_monitor.stop();
                self.active_pitch_preview_preset_id = None;
            }
            if self.active_spatial_preview_preset_id == Some(remove_id) {
                self.spatial_monitor.stop();
                self.active_spatial_preview_preset_id = None;
            }
            changed = true;
        }

        if changed {
            self.sync_audio_sense_presets();
            self.persist();
        }
    }
}

fn active_monitor_settings(preset: &AudioSensePreset) -> &crate::model::AudioSenseMonitorSettings {
    match preset.kind {
        AudioSensePresetKind::Pitch => &preset.pitch.monitor,
        AudioSensePresetKind::Spatial => &preset.spatial.monitor,
    }
}

fn active_monitor_settings_mut(
    preset: &mut AudioSensePreset,
) -> &mut crate::model::AudioSenseMonitorSettings {
    match preset.kind {
        AudioSensePresetKind::Pitch => &mut preset.pitch.monitor,
        AudioSensePresetKind::Spatial => &mut preset.spatial.monitor,
    }
}

fn render_pitch_settings_ui(
    ui: &mut egui::Ui,
    language: UiLanguage,
    settings: &mut PitchAudioSenseSettings,
    snapshot: &crate::audiosense::PitchSnapshot,
) -> bool {
    let mut changed = false;
    changed |= ui
        .checkbox(
            &mut settings.show_sharps,
            CrosshairApp::tr_lang(language, "Sharps", "Dau thang"),
        )
        .changed();

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(format!(
            "{}: {}",
            CrosshairApp::tr_lang(language, "Current note", "Cao do hien tai"),
            snapshot.note
        ));
        ui.label(format!("conf {:.2}", snapshot.confidence));
        ui.label(format!("level {:.2}", snapshot.level));
    });
    changed
}

fn render_spatial_settings_ui(
    ui: &mut egui::Ui,
    language: UiLanguage,
    settings: &mut SpatialAudioSenseSettings,
    snapshot: &crate::audiosense::SpatialSnapshot,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Center X");
        changed |= ui
            .add(DragValue::new(&mut settings.center_x).speed(1))
            .changed();
        ui.label("Center Y");
        changed |= ui
            .add(DragValue::new(&mut settings.center_y).speed(1))
            .changed();
        ui.label("Radius");
        changed |= ui
            .add(DragValue::new(&mut settings.radius).range(0..=5000).speed(1))
            .changed();
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(format!("x {}", snapshot.x));
        ui.label(format!("y {}", snapshot.y));
        ui.label(format!("pan {:.2}", snapshot.pan));
        ui.label(format!("level {:.2}", snapshot.level));
        ui.label(CrosshairApp::tr_lang(
            language,
            "Stereo left/right approximation",
            "Xap xi huong trai/phai",
        ));
    });
    changed
}
