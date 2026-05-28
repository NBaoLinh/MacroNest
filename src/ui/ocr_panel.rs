use eframe::egui::{self, RichText, Sense, Color32, vec2};
use crate::ui::CrosshairApp;
use crate::ocr::{perform_ocr, OcrResult};
use crate::window_list::capture_virtual_screen_region;

impl CrosshairApp {
    pub(crate) fn render_ocr_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        ui.add_space(4.0);

        // Header Title
        ui.horizontal(|ui| {
            ui.heading(
                RichText::new(self.tr("Native Windows OCR (Beta)", "Nhận dạng chữ Windows (OCR)"))
                    .strong()
                    .color(Color32::from_rgb(0, 255, 170)),
            );
        });

        ui.add_space(8.0);

        // Description
        ui.label(
            self.tr(
                "Windows OCR uses Microsoft's native OS engine. It's fast, offline, and takes 0 resources.",
                "Windows OCR sử dụng engine gốc tích hợp sẵn trong hệ điều hành Windows. Xử lý cực nhanh, không tốn tài nguyên và hoạt động offline."
            )
        );

        ui.add_space(12.0);

        // Settings Grid
        Self::show_preset_card(ui, false, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(self.tr("Test OCR Engine Configuration", "Cấu hình thử nghiệm OCR")).strong());
                ui.add_space(6.0);

                egui::Grid::new("ocr-test-settings-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        // Language Code
                        ui.label(self.tr("Language Code (e.g. 'en', 'vi')", "Mã ngôn ngữ (ví dụ 'en', 'vi')"));
                        ui.horizontal(|ui| {
                            let mut lang_str = self.state.ocr_test_lang.clone().unwrap_or_default();
                            let resp = ui.add_sized([180.0, 22.0], egui::TextEdit::singleline(&mut lang_str));
                            if resp.changed() {
                                self.state.ocr_test_lang = if lang_str.trim().is_empty() {
                                    None
                                } else {
                                    Some(lang_str.trim().to_string())
                                };
                                self.persist();
                            }
                            ui.label(RichText::new(self.tr("(Leave blank for Auto)", "(Để trống để tự động nhận dạng)")).weak().small());
                        });
                        ui.end_row();

                        // X, Y, Width, Height
                        ui.label(self.tr("Scan Region (X, Y, W, H)", "Vùng quét OCR (X, Y, W, H)"));
                        ui.horizontal(|ui| {
                            let mut changed = false;
                            ui.label("X:");
                            changed |= ui.add(egui::DragValue::new(&mut self.state.ocr_test_x).range(0..=10000)).changed();
                            ui.add_space(6.0);
                            ui.label("Y:");
                            changed |= ui.add(egui::DragValue::new(&mut self.state.ocr_test_y).range(0..=10000)).changed();
                            ui.add_space(6.0);
                            ui.label("W:");
                            changed |= ui.add(egui::DragValue::new(&mut self.state.ocr_test_width).range(10..=5000)).changed();
                            ui.add_space(6.0);
                            ui.label("H:");
                            changed |= ui.add(egui::DragValue::new(&mut self.state.ocr_test_height).range(10..=5000)).changed();
                            
                            if changed {
                                self.persist();
                            }
                        });
                        ui.end_row();
                    });

                ui.add_space(10.0);

                // Region Editor Preview
                ui.label(RichText::new(self.tr("Visual Region Adjuster", "Điều chỉnh Vùng quét trực quan")).strong());
                ui.add_space(4.0);

                // Re-using the premium rect editor to adjust X, Y, W, H visually
                let changed = Self::render_zoom_rect_editor(
                    ui,
                    "ocr-test-rect-editor",
                    "",
                    &mut self.state.ocr_test_x,
                    &mut self.state.ocr_test_y,
                    &mut self.state.ocr_test_width,
                    &mut self.state.ocr_test_height,
                    Self::screen_size(),
                    None,
                    None,
                    None,
                );

                if changed {
                    self.persist();
                }

                ui.add_space(12.0);

                // Trigger Button
                ui.horizontal(|ui| {
                    if ui.button(
                        RichText::new(self.tr("⚡ Test Capture and OCR Scan", "⚡ Chụp và Quét thử OCR"))
                            .strong()
                            .color(Color32::from_rgb(0, 255, 170))
                    ).clicked() {
                        self.run_ocr_test();
                    }

                    if self.state.ocr_test_running {
                        ui.spinner();
                        ui.label(self.tr("Scanning...", "Đang nhận diện..."));
                    }
                });
            });
        });

        ui.add_space(12.0);

        // Results Section
        Self::show_preset_card(ui, false, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(self.tr("OCR Scan Results", "Kết quả quét OCR")).strong());
                ui.add_space(8.0);

                if let Some(ref err) = self.state.ocr_test_error {
                    ui.label(
                        RichText::new(format!("❌ Error: {err}"))
                            .color(Color32::from_rgb(255, 85, 85))
                            .strong()
                    );
                } else if let Some(ref res) = self.state.ocr_test_result {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(self.tr("Full Extracted Text:", "Toàn bộ đoạn chữ quét được:")).strong());
                        });
                        ui.add_space(4.0);
                        
                        let text_val = if res.text.trim().is_empty() {
                            self.tr("[No text found in region]", "[Không tìm thấy chữ nào trong vùng quét]").to_string()
                        } else {
                            res.text.clone()
                        };

                        ui.text_edit_multiline(&mut text_val.as_str());

                        // Number parsing test helper
                        if !res.text.trim().is_empty() {
                            let mut parsed_numbers = Vec::new();
                            let mut current_num = String::new();
                            for c in res.text.chars() {
                                if c.is_ascii_digit() {
                                    current_num.push(c);
                                } else if !current_num.is_empty() {
                                    if let Ok(n) = current_num.parse::<i32>() {
                                        parsed_numbers.push(n);
                                    }
                                    current_num.clear();
                                }
                            }
                            if !current_num.is_empty() {
                                if let Ok(n) = current_num.parse::<i32>() {
                                    parsed_numbers.push(n);
                                }
                            }

                            if !parsed_numbers.is_empty() {
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(self.tr("💡 Extracted Numeric values:", "💡 Các số trích xuất được:")).strong().color(Color32::from_rgb(255, 232, 96)));
                                    ui.label(format!("{:?}", parsed_numbers));
                                });
                            }
                        }
                    });

                    if !res.words.is_empty() {
                        ui.add_space(8.0);
                        ui.label(RichText::new(self.tr("Detailed Words Coordinates:", "Tọa độ chi tiết các từ:")).strong());
                        ui.add_space(4.0);
                        
                        egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                            egui::Grid::new("ocr-test-words-grid")
                                .num_columns(4)
                                .spacing([16.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Word").strong());
                                    ui.label(RichText::new("Pos X, Y (Relative)").strong());
                                    ui.label(RichText::new("Absolute on Screen").strong());
                                    ui.label(RichText::new("Size (W x H)").strong());
                                    ui.end_row();

                                    for word in &res.words {
                                        ui.label(RichText::new(&word.text).color(Color32::from_rgb(0, 255, 170)));
                                        ui.label(format!("{:.0}, {:.0}", word.x, word.y));
                                        
                                        // Calc absolute screen position
                                        let abs_x = self.state.ocr_test_x as f32 + word.x;
                                        let abs_y = self.state.ocr_test_y as f32 + word.y;
                                        ui.label(format!("{:.0}, {:.0}", abs_x, abs_y));
                                        
                                        ui.label(format!("{:.0}x{:.0}", word.width, word.height));
                                        ui.end_row();
                                    }
                                });
                        });
                    }
                } else {
                    ui.label(
                        RichText::new(self.tr("Click button above to run OCR test scan.", "Nhấn nút phía trên để chạy quét thử OCR."))
                            .weak()
                    );
                }
            });
        });
    }

    fn run_ocr_test(&mut self) {
        self.state.ocr_test_running = true;
        self.state.ocr_test_error = None;
        self.state.ocr_test_result = None;

        let x = self.state.ocr_test_x;
        let y = self.state.ocr_test_y;
        let w = self.state.ocr_test_width;
        let h = self.state.ocr_test_height;
        let lang = self.state.ocr_test_lang.clone().unwrap_or_default();

        // Capture virtual screen region
        if let Some(frame) = capture_virtual_screen_region(x, y, w, h) {
            match perform_ocr(&frame.rgba, frame.width as u32, frame.height as u32, &lang) {
                Ok(res) => {
                    self.state.ocr_test_result = Some(res);
                }
                Err(err) => {
                    self.state.ocr_test_error = Some(err.to_string());
                }
            }
        } else {
            self.state.ocr_test_error = Some("Failed to capture screen region.".to_string());
        }

        self.state.ocr_test_running = false;
        self.persist();
    }
}
