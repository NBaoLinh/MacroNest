use std::time::{Duration, Instant};
use std::fs;
use std::process::Command;
use std::sync::atomic::Ordering;
use eframe::egui::{self, Button, RichText, Stroke, Margin, TextEdit, Color32, vec2, Frame, TextBuffer, Order, Shadow};
use anyhow::Result;
use crate::model::*;
use crate::overlay::UiCommand;
use crate::ui::{
    CrosshairApp, UpdateStatus,
};


impl CrosshairApp {
    pub(crate) fn render_settings_popup(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        egui::ScrollArea::vertical()
            .max_height(ui.available_height())
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(4.0);
                    let mut groq_changed = false;
                    Self::settings_card_frame(ui).show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.selectable_value(
                                    &mut self.state.groq_settings.details_open,
                                    true,
                                    RichText::new("API (Groq / OpenAI)").strong().size(14.0),
                                );
                                if self.state.groq_settings.details_open {
                                    if ui.small_button("Close").clicked() {
                                        self.state.groq_settings.details_open = false;
                                    }
                                }
                            });
                            if self.state.groq_settings.details_open {
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.label("API Key");
                                    let key_editor =
                                        TextEdit::singleline(&mut self.state.groq_settings.api_key)
                                            .hint_text("gsk_...");
                                    let response = ui.add_sized(
                                        [(ui.available_width() - 96.0).max(180.0), 24.0],
                                        if self.state.groq_settings.show_api_key {
                                            key_editor
                                        } else {
                                            key_editor.password(true)
                                        },
                                    );
                                    Self::apply_vietnamese_input_if_changed(
                                        &response,
                                        self.state.vietnamese_input_enabled,
                                        self.state.vietnamese_input_mode,
                                        &mut self.state.groq_settings.api_key,
                                    );
                                    groq_changed |= response.changed();
                                    if ui
                                        .button(if self.state.groq_settings.show_api_key {
                                            Self::tr_lang(language, "Hide", "Ẩn")
                                        } else {
                                            Self::tr_lang(language, "Show", "Hiện")
                                        })
                                        .clicked()
                                    {
                                        self.state.groq_settings.show_api_key =
                                            !self.state.groq_settings.show_api_key;
                                        groq_changed = true;
                                    }
                                });
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.label("Model");
                                    let selected_text = {
                                        let model = self.state.groq_settings.model.trim();
                                        Self::groq_model_catalog()
                                            .iter()
                                            .find(|(_, model_id)| {
                                                model_id.eq_ignore_ascii_case(model)
                                            })
                                            .map(|(label, model_id)| {
                                                format!("{label} ({model_id})")
                                            })
                                            .unwrap_or_else(|| model.to_owned())
                                    };
                                    egui::ComboBox::from_id_salt("groq-model-picker")
                                        .selected_text(selected_text)
                                        .width((ui.available_width() - 96.0).max(200.0))
                                        .show_ui(ui, |ui| {
                                            for (label, model_id) in Self::groq_model_catalog() {
                                                let selected =
                                                    self.state.groq_settings.model.trim().eq(*model_id);
                                                if ui
                                                    .selectable_label(
                                                        selected,
                                                        format!("{label} ({model_id})"),
                                                    )
                                                    .clicked()
                                                {
                                                    self.state.groq_settings.model =
                                                        (*model_id).to_owned();
                                                    groq_changed = true;
                                                    ui.close();
                                                }
                                            }
                                        });
                                });
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    let response = ui.add_sized(
                                        [ui.available_width() - 110.0, 24.0],
                                        TextEdit::singleline(&mut self.state.groq_settings.model)
                                            .hint_text("openai/gpt-oss-120b"),
                                    );
                                    Self::apply_vietnamese_input_if_changed(
                                        &response,
                                        self.state.vietnamese_input_enabled,
                                        self.state.vietnamese_input_mode,
                                        &mut self.state.groq_settings.model,
                                    );
                                    groq_changed |= response.changed();
                                    if ui.button("Get API key").clicked() {
                                        let _ = crate::platform::open_url_in_browser(
                                            "https://console.groq.com/keys",
                                        );
                                    }
                                });
                            } else {
                                ui.label(
                                    RichText::new("Click to expand API configurations.")
                                        .small()
                                        .weak(),
                                );
                            }
                        });
                    });
                    if groq_changed {
                        self.persist();
                    }
                    
                    ui.add_space(12.0);
                    Self::settings_card_frame(ui).show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(Self::tr_lang(
                                    language,
                                    "Vietnamese input",
                                    "Gõ tiếng Việt",
                                ))
                                .strong()
                                .size(14.0),
                            );
                            ui.add_space(8.0);
                            let mut vietnamese_input_changed = false;
                            ui.horizontal(|ui| {
                                vietnamese_input_changed |= ui
                                    .radio_value(
                                        &mut self.state.vietnamese_input_mode,
                                        VietnameseInputMode::Telex,
                                        "Telex",
                                    )
                                    .changed();
                                ui.add_space(12.0);
                                vietnamese_input_changed |= ui
                                    .radio_value(
                                        &mut self.state.vietnamese_input_mode,
                                        VietnameseInputMode::Vni,
                                        "VNI",
                                    )
                                    .changed();
                            });
                            if vietnamese_input_changed {
                                self.persist();
                            }
                        });
                    });
                    ui.add_space(12.0);
                    Self::settings_card_frame(ui).show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(Self::tr_lang(language, "App data", "Thư mục dữ liệu"))
                                    .strong()
                                    .size(14.0),
                            );
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .button(Self::tr_lang(
                                        language,
                                        "Open data folder",
                                        "Mở thư mục dữ liệu",
                                    ))
                                    .clicked()
                                {
                                    self.open_app_data_folder();
                                }
                                ui.add_space(8.0);
                                let is_copied = self.copy_folder_feedback_until
                                    .map(|until| Instant::now() < until)
                                    .unwrap_or(false);

                                let btn_label = if is_copied {
                                    Self::tr_lang(language, "Copied!", "Đã sao chép!")
                                } else {
                                    Self::tr_lang(language, "Copy folder", "Sao chép thư mục")
                                };

                                if is_copied {
                                    ui.ctx().request_repaint_after(Duration::from_millis(200));
                                }

                                if ui.button(btn_label).clicked() {
                                    if let Err(e) = crate::platform::copy_folder_to_clipboard(&self.paths.root) {
                                        self.status = format!("Failed to copy folder: {e}");
                                    } else {
                                        self.status = Self::tr_lang(language, "Folder copied to clipboard.", "Đã chép thư mục vào clipboard.").to_owned();
                                        self.copy_folder_feedback_until = Some(Instant::now() + Duration::from_secs(2));
                                    }
                                }
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(self.paths.root.display().to_string())
                                        .monospace()
                                        .small()
                                        .weak(),
                                );
                            });
                        });
                    });
                    ui.add_space(12.0);
                    self.render_advanced_settings(ui);
                    ui.add_space(12.0);
                    self.render_opencv_settings(ui);
                    ui.add_space(12.0);
                    self.render_interception_settings(ui);
                    ui.add_space(12.0);
                    let ctx_clone = ui.ctx().clone();
                    self.render_update_settings(ui, &ctx_clone);
                    ui.add_space(8.0);
                });
            });
    }

    pub(crate) fn render_advanced_settings(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        Self::settings_card_frame(ui).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let header_text = RichText::new(Self::tr_lang(language, "Advanced", "Nâng cao"))
                        .strong()
                        .size(14.0);
                    if ui.selectable_label(self.advanced_settings_open, header_text).clicked() {
                        self.advanced_settings_open = !self.advanced_settings_open;
                    }
                });

                if self.advanced_settings_open {
                    ui.add_space(8.0);
                    let explanation_en = "Note: Some games might not register inputs if the delays are set too low (e.g., 0ms). You can adjust these values if your macros do not work correctly in-game.";
                    let explanation_vi = "Lưu ý: Một số trò chơi có thể không nhận phản hồi từ chuột hoặc phím nếu đặt độ trễ quá thấp (ví dụ: 0ms). Bạn có thể chỉnh lại các thông số này nếu macro không hoạt động chính xác trong game.";
                    ui.label(
                        RichText::new(Self::tr_lang(language, explanation_en, explanation_vi))
                            .small()
                            .weak(),
                    );
                    ui.add_space(8.0);

                    let mut delay_changed = false;
                    ui.horizontal(|ui| {
                        ui.label(Self::tr_lang(language, "Mouse Click Delay:", "Độ trễ click chuột:"));
                        let slider = egui::Slider::new(&mut self.state.macro_mouse_click_delay_ms, 0..=500)
                            .suffix(" ms");
                        let res = ui.add(slider);
                        if res.changed() {
                            delay_changed = true;
                        }
                    });

                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label(Self::tr_lang(language, "Keyboard Press Delay:", "Độ trễ nhấn phím:"));
                        let slider = egui::Slider::new(&mut self.state.macro_keyboard_key_press_delay_ms, 0..=500)
                            .suffix(" ms");
                        let res = ui.add(slider);
                        if res.changed() {
                            delay_changed = true;
                        }
                    });

                    ui.add_space(6.0);

                    let mut interception_changed = false;
                    ui.horizontal(|ui| {
                        let res = ui.checkbox(
                            &mut self.state.vision_settings.use_interception,
                            Self::tr_lang(
                                language,
                                "Use Interception Driver (Mouse clicks/movement in games)",
                                "Sử dụng Driver Interception (Di chuyển/click chuột trong game)"
                            )
                        );
                        if res.changed() {
                            if !self.interception_installed {
                                // Block and revert
                                self.state.vision_settings.use_interception = false;
                                self.status = Self::tr_lang(
                                    language,
                                    "Please download and install the Interception Driver wrapper first!",
                                    "Vui lòng tải xuống và cài đặt wrapper Interception Driver trước!"
                                ).to_owned();
                            } else {
                                interception_changed = true;
                            }
                        }
                    });

                    if delay_changed {
                        self.sync_macro_delay_settings();
                        self.persist();
                    }
                    if interception_changed {
                        self.sync_vision_settings();
                        self.persist();
                    }
                } else {
                    ui.label(
                        RichText::new(Self::tr_lang(
                            language,
                            "Click to expand advanced configuration (input delays).",
                            "Nhấn vào để mở rộng cấu hình nâng cao (độ trễ đầu vào)."
                        ))
                        .small()
                        .weak(),
                    );
                }
            });
        });
    }

    pub(crate) fn render_opencv_settings(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        Self::settings_card_frame(ui).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical(|ui| {
                ui.label(RichText::new("Vision Support (OpenCV)").strong().size(14.0));
                ui.add_space(8.0);

            if self.opencv_installed {
                ui.horizontal(|ui| {
                    ui.label(Self::tr_lang(language, "Status: Installed", "Trạng thái: Đã cài đặt"));
                    if ui.button(Self::tr_lang(language, "Delete", "Xóa")).clicked() {
                        let _ = fs::remove_file(&self.paths.opencv_dll);
                        self.opencv_installed = false;
                        self.status = Self::tr_lang(language, "OpenCV DLL deleted.", "Đã xóa file OpenCV DLL.").to_owned();
                    }
                });
                ui.label(RichText::new(self.paths.opencv_dll.display().to_string()).small().weak());
            } else if self.opencv_download_job.is_some() {
                let progress = self.opencv_download_progress.load(Ordering::SeqCst) as f32 / 1000.0;
                ui.horizontal(|ui| {
                    ui.label(Self::tr_lang(language, "Downloading...", "Đang tải..."));
                    ui.add(egui::ProgressBar::new(progress).show_percentage());
                });
                ui.ctx().request_repaint();
            } else {
                ui.vertical(|ui| {
                    ui.label(Self::tr_lang(
                        language, 
                        "Vision features require OpenCV (~60MB).", 
                        "Tính năng Vision cần thư viện OpenCV (~60MB)."
                    ));
                    if ui.button(RichText::new(Self::tr_lang(language, "Download OpenCV", "Tải OpenCV")).strong()).clicked() {
                        self.start_opencv_download();
                    }
                });
            }
        });
    });
}

    pub(crate) fn render_interception_settings(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        Self::settings_card_frame(ui).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical(|ui| {
                ui.label(RichText::new("Interception Driver").strong().size(14.0));
                ui.add_space(8.0);

            if self.interception_installed {
                ui.horizontal(|ui| {
                    ui.label(Self::tr_lang(language, "Status: Installed", "Trạng thái: Đã cài đặt"));
                    if ui.button(Self::tr_lang(language, "Delete", "Xóa")).clicked() {
                        let _ = fs::remove_file(&self.paths.interception_dll);
                        self.interception_installed = false;
                        self.status = Self::tr_lang(language, "Interception wrapper deleted.", "Đã xóa wrapper Interception.").to_owned();
                    }
                });
                ui.label(RichText::new(self.paths.interception_dll.display().to_string()).small().weak());
            } else if self.interception_download_job.is_some() {
                let progress = self.interception_download_progress.load(Ordering::SeqCst) as f32 / 1000.0;
                ui.horizontal(|ui| {
                    ui.label(Self::tr_lang(language, "Downloading...", "Đang tải..."));
                    ui.add(egui::ProgressBar::new(progress).show_percentage());
                });
                ui.ctx().request_repaint();
            } else {
                ui.vertical(|ui| {
                    ui.label(Self::tr_lang(
                        language, 
                        "Low-level driver input requires interception.dll (~200KB).", 
                        "Điều khiển cấp thấp cần interception.dll (~200KB)."
                    ));
                    if ui.button(RichText::new(Self::tr_lang(language, "Download Interception Wrapper", "Tải Interception Wrapper")).strong()).clicked() {
                        self.start_interception_download();
                    }
                });
            }
        });
    });
}

    pub(crate) fn render_update_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let language = self.state.ui_language;
        Self::settings_card_frame(ui).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(Self::tr_lang(language, "Update", "Cập nhật"))
                        .strong()
                        .size(14.0),
                );
                ui.add_space(8.0);
                match &self.update_status {
                    UpdateStatus::Idle => {
                        if ui
                            .button(Self::tr_lang(
                                language,
                                "Check for update",
                                "Kiểm tra cập nhật",
                            ))
                            .clicked()
                        {
                            self.check_for_update(ctx);
                        }
                    }
                    UpdateStatus::Checking => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(Self::tr_lang(
                                language,
                                "Checking for updates...",
                                "Đang kiểm tra cập nhật...",
                            ));
                        });
                    }
                    UpdateStatus::Available(version, body, url) => {
                        ui.label(
                            RichText::new(format!("New version available: v{}", version))
                                .color(Color32::GREEN),
                        );
                        if !body.is_empty() {
                            ui.label(RichText::new(body).small().weak());
                        }
                        if ui
                            .button(Self::tr_lang(
                                language,
                                "Download and Update",
                                "Tải xuống và Cập nhật",
                            ))
                            .clicked()
                        {
                            self.start_download_update(ctx, url.clone());
                        }
                    }
                    UpdateStatus::Downloading => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(Self::tr_lang(
                                language,
                                "Downloading update...",
                                "Đang tải cập nhật...",
                            ));
                        });
                    }
                    UpdateStatus::ReadyToRestart(path) => {
                        ui.label(RichText::new("Update downloaded!").color(Color32::GREEN));
                        let path = path.clone();
                        if ui
                            .button(
                                RichText::new(Self::tr_lang(
                                    language,
                                    "Restart App",
                                    "Khởi động lại",
                                ))
                                .strong(),
                            )
                            .clicked()
                        {
                            self.restart_and_apply_update(path);
                        }
                    }
                    UpdateStatus::UpToDate => {
                        ui.label(Self::tr_lang(
                            language,
                            "App is up to date.",
                            "Ứng dụng đã ở bản mới nhất.",
                        ));
                        ui.add_space(4.0);
                        if ui
                            .button(Self::tr_lang(language, "Check again", "Kiểm tra lại"))
                            .clicked()
                        {
                            self.check_for_update(ctx);
                        }
                    }
                    UpdateStatus::Error(e) => {
                        ui.label(RichText::new(format!("Error: {}", e)).color(Color32::RED));
                        ui.add_space(4.0);
                        if ui.button(Self::tr_lang(language, "Retry", "Thử lại")).clicked() {
                            self.check_for_update(ctx);
                        }
                    }
                }
            });
        });
    }

    pub(crate) fn render_custom_ai_modal(&mut self, ctx: &egui::Context) {
        let generating = self.command_ai_job.is_some();
        let Some(dialog_preset_id) = self
            .command_ai_dialog
            .as_ref()
            .map(|dialog| dialog.preset_id)
        else {
            return;
        };
        let Some(preset_name) = self
            .state
            .command_presets
            .iter()
            .find(|preset| preset.id == dialog_preset_id)
            .map(|preset| preset.name.clone())
        else {
            self.command_ai_dialog = None;
            self.status = "Custom preset was removed.".to_owned();
            return;
        };

        if self.capture_target.is_none()
            && ctx.input(|input| input.key_pressed(egui::Key::Escape))
        {
            self.command_ai_dialog = None;
            return;
        }

        self.render_modal_backdrop(ctx, true);
        let (panel_size, panel_pos) =
            Self::centered_modal_placement(ctx, vec2(560.0, 220.0), vec2(480.0, 180.0));
        let mut close_request = false;
        let mut generate_request = false;
        let dark_theme = self.state.ui_theme == UiThemeMode::Dark;
        let vietnamese_input_mode = self.state.vietnamese_input_mode;
        {
            let Some(dialog) = self.command_ai_dialog.as_mut() else {
                return;
            };
            egui::Area::new(egui::Id::new("custom-ai-modal"))
                .order(Order::Foreground)
                .fixed_pos(panel_pos)
                .interactable(true)
                .show(ctx, |ui| {
                    ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::Default);
                    Frame::new()
                        .fill(if dark_theme {
                            Color32::from_rgba_premultiplied(24, 26, 32, 248)
                        } else {
                            Color32::from_rgba_premultiplied(248, 248, 250, 248)
                        })
                        .stroke(Stroke::new(
                            1.0,
                            Color32::from_rgba_premultiplied(90, 94, 108, 180),
                        ))
                        .shadow(Shadow {
                            offset: [0, 14],
                            blur: 32,
                            spread: 0,
                            color: Color32::from_rgba_premultiplied(12, 12, 16, 72),
                        })
                        .corner_radius(24.0)
                        .inner_margin(Margin::same(16))
                        .show(ui, |ui| {
                            ui.set_min_width(panel_size.x);
                            ui.set_max_width(panel_size.x);
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new("AI Custom").strong());
                                        ui.label(
                                            RichText::new(preset_name.clone())
                                                .small()
                                                .color(ui.visuals().weak_text_color()),
                                        );
                                    });
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .add_sized(
                                                    [34.0, 28.0],
                                                    Button::new(Self::material_icon_text(
                                                        0xe5cd, 18.0,
                                                    )),
                                                )
                                                .clicked()
                                            {
                                                close_request = true;
                                            }
                                        },
                                    );
                                });
                                let original_weak_color = ui.style().visuals.weak_text_color;
                                ui.style_mut().visuals.weak_text_color = if dark_theme {
                                    Some(Color32::from_gray(85))
                                } else {
                                    Some(Color32::from_gray(175))
                                };
                                let original_extreme_bg = ui.visuals().extreme_bg_color;
                                ui.visuals_mut().extreme_bg_color = if dark_theme {
                                    Color32::from_rgba_unmultiplied(12, 13, 16, 50)
                                } else {
                                    Color32::from_rgba_unmultiplied(240, 240, 242, 50)
                                };
                                let response = ui.add_sized(
                                    [ui.available_width(), 92.0],
                                    TextEdit::multiline(&mut dialog.prompt)
                                        .desired_rows(4)
                                        .text_color(if dark_theme {
                                            Color32::from_gray(210)
                                        } else {
                                            Color32::from_gray(60)
                                        })
                                        .hint_text(
                                            egui::RichText::new(Self::tr_lang(
                                                self.state.ui_language,
                                                "Example: Open Excel, write text to cell A1, then save...",
                                                "Ví dụ: Mở Excel, ghi nội dung vào ô A1, sau đó lưu lại...",
                                            ))
                                            .color(if dark_theme {
                                                Color32::from_rgba_unmultiplied(120, 120, 120, 140)
                                            } else {
                                                Color32::from_rgba_unmultiplied(140, 140, 140, 180)
                                            })
                                            .italics(),
                                        ),
                                );
                                ui.style_mut().visuals.weak_text_color = original_weak_color;
                                ui.visuals_mut().extreme_bg_color = original_extreme_bg;
                                Self::apply_vietnamese_input_if_changed(
                                    &response,
                                    self.state.vietnamese_input_enabled,
                                    vietnamese_input_mode,
                                    &mut dialog.prompt,
                                );

                                let enter_generate = !generating
                                    && !dialog.prompt.trim().is_empty()
                                    && ctx.input(|input| input.key_pressed(egui::Key::Enter));
                                if generating {
                                    ui.horizontal(|ui| {
                                        ui.spinner();
                                        ui.label("Generating...");
                                    });
                                } else if let Some(feedback) = self.command_ai_feedback.as_ref() {
                                    ui.label(
                                        RichText::new(feedback)
                                            .small()
                                            .color(ui.visuals().strong_text_color()),
                                    );
                                }
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    let can_generate =
                                        !generating && !dialog.prompt.trim().is_empty();
                                    if ui
                                        .add_enabled(
                                            can_generate,
                                            Button::new("Generate").min_size(vec2(100.0, 28.0)),
                                        )
                                        .clicked()
                                    {
                                        generate_request = true;
                                    }
                                    if ui
                                        .add_enabled(
                                            true,
                                            Button::new("Close").min_size(vec2(100.0, 28.0)),
                                        )
                                        .clicked()
                                    {
                                        close_request = true;
                                    }
                                });
                                if enter_generate {
                                    generate_request = true;
                                }
                            });
                        });
                });
            ctx.set_cursor_icon(egui::CursorIcon::Default);
        }

        if generate_request {
            self.start_custom_ai_generation(ctx);
        }
        if close_request {
            self.command_ai_dialog = None;
        }
        if self.command_ai_dialog.is_none() {
            self.command_ai_step_target = None;
            self.state.command_presets.retain(|preset| preset.id != 999999);
        }
        if self.command_ai_dialog.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }

    pub(crate) fn start_opencv_download(&mut self) {
        if self.opencv_download_job.is_some() {
            return;
        }

        let paths = self.paths.clone();
        let progress = self.opencv_download_progress.clone();
        progress.store(0, Ordering::SeqCst);

        let job = std::thread::spawn(move || -> Result<()> {
            let url = "https://github.com/Baolinh0305/MacroNest/releases/download/v0.1/opencv_world4100.dll";
            let mut response = reqwest::blocking::get(url)?.error_for_status()?;
            let total_size = response.content_length().unwrap_or(64 * 1024 * 1024);

            let mut file = fs::File::create(&paths.opencv_dll)?;
            let mut downloaded: u64 = 0;
            let mut buffer = [0u8; 16384];

            use std::io::{Read, Write};
            loop {
                let n = response.read(&mut buffer)?;
                if n == 0 { break; }
                file.write_all(&buffer[..n])?;
                downloaded += n as u64;
                let p = (downloaded as f32 / total_size as f32 * 1000.0) as u32;
                progress.store(p, Ordering::SeqCst);
            }

            Ok(())
        });

        self.opencv_download_job = Some(job);
    }

    pub(crate) fn start_interception_download(&mut self) {
        if self.interception_download_job.is_some() {
            return;
        }

        let paths = self.paths.clone();
        let progress = self.interception_download_progress.clone();
        progress.store(0, Ordering::SeqCst);

        let job = std::thread::spawn(move || -> Result<()> {
            let url = "https://github.com/Baolinh0305/MacroNest/releases/download/v0.1/interception.dll";
            let mut response = reqwest::blocking::get(url)?.error_for_status()?;
            let total_size = response.content_length().unwrap_or(200 * 1024);

            let mut file = fs::File::create(&paths.interception_dll)?;
            let mut downloaded: u64 = 0;
            let mut buffer = [0u8; 16384];

            use std::io::{Read, Write};
            loop {
                let n = response.read(&mut buffer)?;
                if n == 0 { break; }
                file.write_all(&buffer[..n])?;
                downloaded += n as u64;
                let p = (downloaded as f32 / total_size as f32 * 1000.0) as u32;
                progress.store(p, Ordering::SeqCst);
            }

            Ok(())
        });

        self.interception_download_job = Some(job);
    }

    pub(crate) fn check_for_update(&mut self, ctx: &egui::Context) {
        if matches!(self.update_status, UpdateStatus::Checking | UpdateStatus::Downloading) {
            return;
        }
        self.update_status = UpdateStatus::Checking;
        let ui_tx = self.ui_tx.clone();
        let ctx = ctx.clone();
        let current_version = self.app_version_label().to_owned();
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .user_agent("MacroNest")
                .build()
                .map_err(|e| e.to_string());
            let result = client.and_then(|c| {
                let resp = c.get("https://api.github.com/repos/Baolinh0305/MacroNest/releases/latest")
                    .send()
                    .map_err(|e| e.to_string())?;
                
                if resp.status() == reqwest::StatusCode::NOT_FOUND {
                    return Err("No releases found on GitHub.".to_owned());
                }

                if !resp.status().is_success() {
                    return Err(format!("GitHub API error: {}", resp.status()));
                }

                let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
                let latest_version = json["tag_name"].as_str()
                    .unwrap_or("")
                    .trim_start_matches('v')
                    .to_owned();
                if latest_version.is_empty() {
                    return Err("Failed to parse version from GitHub".to_owned());
                }
                if Self::versions_are_equal(&latest_version, &current_version) {
                    let _ = ui_tx.send(UiCommand::UpdateUpToDate);
                    return Ok(());
                }
                let body = json["body"].as_str().unwrap_or("").to_owned();
                let download_url = json["assets"].as_array()
                    .and_then(|assets| {
                        assets.iter().find(|a| {
                            a["name"].as_str().map(|n| n.ends_with(".exe")).unwrap_or(false)
                        })
                    })
                    .and_then(|a| a["browser_download_url"].as_str())
                    .map(|s| s.to_owned());
                if let Some(url) = download_url {
                    let _ = ui_tx.send(UiCommand::UpdateAvailable(latest_version, body, url));
                } else {
                    let _ = ui_tx.send(UiCommand::UpdateError("No executable found in the latest release".to_owned()));
                }
                Ok(())
            });
            if let Err(e) = result {
                let _ = ui_tx.send(UiCommand::UpdateError(e));
            }
            ctx.request_repaint();
        });
    }

    pub(crate) fn start_download_update(&mut self, ctx: &egui::Context, download_url: String) {
        self.update_status = UpdateStatus::Downloading;
        let ui_tx = self.ui_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .user_agent("MacroNest")
                .build();
            let result = client.map_err(|e| e.to_string()).and_then(|c| {
                let mut resp = c.get(download_url)
                    .send()
                    .map_err(|e| e.to_string())?;
                let temp_dir = std::env::temp_dir();
                let temp_path = temp_dir.join("macronest_update.exe");
                let mut file = fs::File::create(&temp_path).map_err(|e| e.to_string())?;
                std::io::copy(&mut resp, &mut file).map_err(|e| e.to_string())?;
                let _ = ui_tx.send(UiCommand::UpdateDownloadFinished(temp_path.to_string_lossy().to_string()));
                Ok(())
            });
            if let Err(e) = result {
                let _ = ui_tx.send(UiCommand::UpdateError(e));
            }
            ctx.request_repaint();
        });
    }

    pub(crate) fn restart_and_apply_update(&mut self, new_exe_path: String) {
        let current_exe = std::env::current_exe().unwrap_or_default();
        let old_exe = current_exe.with_extension("exe.old");
        let result: anyhow::Result<()> = (|| {
            if old_exe.exists() {
                let _ = fs::remove_file(&old_exe);
            }
            fs::rename(&current_exe, &old_exe)?;
            fs::copy(&new_exe_path, &current_exe)?;
            Command::new(&current_exe).spawn()?;
            std::process::exit(0);
        })();
        if let Err(e) = result {
            self.status = format!("Failed to apply update: {e}");
        }
    }

    pub(crate) fn open_app_data_folder(&mut self) {
        match crate::platform::open_folder_in_explorer(&self.paths.root) {
            Ok(()) => {
                self.status = format!("Opened data folder: {}.", self.paths.root.display());
            }
            Err(error) => {
                self.status = format!("Failed to open data folder: {error}");
            }
        }
    }

    pub(crate) fn open_ai_debug_folder(&mut self) {}
}
