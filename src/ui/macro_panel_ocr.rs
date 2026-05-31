use eframe::egui;

use crate::model::{MacroStep, UiLanguage, VietnameseInputMode};

use super::CrosshairApp;

impl CrosshairApp {
    pub(crate) fn render_ocr_outputs_selector(
        ui: &mut egui::Ui,
        language: UiLanguage,
        vietnamese_input_enabled: bool,
        vietnamese_input_mode: VietnameseInputMode,
        group_id: u32,
        preset_id: u32,
        step_index: usize,
        step: &mut MacroStep,
        live_sync: &mut bool,
    ) {
        let outputs_label =
            Self::tr_lang(language, "Outputs", "ÃƒÆ’Ã¢â‚¬Å¾Ãƒâ€šÃ‚ÂÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚ÂºÃƒâ€šÃ‚Â§u ra").to_owned();

        egui::ComboBox::from_id_salt((group_id, preset_id, step_index, "ocr-outputs"))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .width(110.0)
            .selected_text(outputs_label)
            .show_ui(ui, |ui| {
                ui.set_min_width(200.0);

                egui::Grid::new("ocr_outputs_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        let found_label = ui.label(Self::tr_lang(
                            language,
                            "Found Var:",
                            "BiÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚ÂºÃƒâ€šÃ‚Â¿n kÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚ÂºÃƒâ€šÃ‚Â¿t quÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚ÂºÃƒâ€šÃ‚Â£:",
                        ));
                        found_label.on_hover_text(Self::tr_lang(
                            language,
                            "Assigns 1 if the target text was found (or if OCR succeeded when no target is set), 0 otherwise",
                            "GÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¡n 1 nÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚ÂºÃƒâ€šÃ‚Â¿u tÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¬m thÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚ÂºÃƒâ€šÃ‚Â¥y tÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€šÃ‚Â« khÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³a (hoÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚ÂºÃƒâ€šÃ‚Â·c nÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚ÂºÃƒâ€šÃ‚Â¿u quÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â©t OCR thÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â nh cÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â´ng khi khÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â´ng ÃƒÆ’Ã¢â‚¬Å¾ÃƒÂ¢Ã¢â€šÂ¬Ã‹Å“ÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚ÂºÃƒâ€šÃ‚Â·t tÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€šÃ‚Â« tÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¬m), ngÃƒÆ’Ã¢â‚¬Â Ãƒâ€šÃ‚Â°ÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€šÃ‚Â£c lÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚ÂºÃƒâ€šÃ‚Â¡i lÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â  0",
                        ));
                        let found_resp = ui.add(
                            egui::TextEdit::singleline(&mut step.ocr_success_var).hint_text("var_ok"),
                        );
                        Self::apply_vietnamese_input_if_changed(
                            &found_resp,
                            vietnamese_input_enabled,
                            vietnamese_input_mode,
                            &mut step.ocr_success_var,
                        );
                        *live_sync |= found_resp.changed();
                        ui.end_row();

                        let pos_x_label = ui.label("Pos X:");
                        pos_x_label.on_hover_text(Self::tr_lang(
                            language,
                            "Assigns the absolute X coordinate of the center of found text",
                            "GÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¡n tÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€šÃ‚Âa ÃƒÆ’Ã¢â‚¬Å¾ÃƒÂ¢Ã¢â€šÂ¬Ã‹Å“ÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢ X tuyÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â¡t ÃƒÆ’Ã¢â‚¬Å¾ÃƒÂ¢Ã¢â€šÂ¬Ã‹Å“ÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»ÃƒÂ¢Ã¢â€šÂ¬Ã‹Å“i ÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€¦Ã‚Â¸ chÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­nh giÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€šÃ‚Â¯a tÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€šÃ‚Â« tÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¬m thÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚ÂºÃƒâ€šÃ‚Â¥y",
                        ));
                        let pos_x_resp =
                            ui.add(egui::TextEdit::singleline(&mut step.ocr_pos_var_x).hint_text("var_x"));
                        Self::apply_vietnamese_input_if_changed(
                            &pos_x_resp,
                            vietnamese_input_enabled,
                            vietnamese_input_mode,
                            &mut step.ocr_pos_var_x,
                        );
                        *live_sync |= pos_x_resp.changed();
                        ui.end_row();

                        let pos_y_label = ui.label("Pos Y:");
                        pos_y_label.on_hover_text(Self::tr_lang(
                            language,
                            "Assigns the absolute Y coordinate of the center of found text",
                            "GÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¡n tÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€šÃ‚Âa ÃƒÆ’Ã¢â‚¬Å¾ÃƒÂ¢Ã¢â€šÂ¬Ã‹Å“ÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢ Y tuyÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â¡t ÃƒÆ’Ã¢â‚¬Å¾ÃƒÂ¢Ã¢â€šÂ¬Ã‹Å“ÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»ÃƒÂ¢Ã¢â€šÂ¬Ã‹Å“i ÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€¦Ã‚Â¸ chÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­nh giÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€šÃ‚Â¯a tÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€šÃ‚Â« tÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¬m thÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚ÂºÃƒâ€šÃ‚Â¥y",
                        ));
                        let pos_y_resp =
                            ui.add(egui::TextEdit::singleline(&mut step.ocr_pos_var_y).hint_text("var_y"));
                        Self::apply_vietnamese_input_if_changed(
                            &pos_y_resp,
                            vietnamese_input_enabled,
                            vietnamese_input_mode,
                            &mut step.ocr_pos_var_y,
                        );
                        *live_sync |= pos_y_resp.changed();
                        ui.end_row();

                        let text_var_label =
                            ui.label(Self::tr_lang(language, "Text Var:", "Text Var:"));
                        text_var_label.on_hover_text(Self::tr_lang(
                            language,
                            "Stores ALL recognized text into this variable, regardless of the Target Text filter",
                            "Stores all OCR text into this variable regardless of Target Text filter",
                        ));
                        let text_var_resp = ui.add(
                            egui::TextEdit::singleline(&mut step.ocr_text_var).hint_text("var_text"),
                        );
                        Self::apply_vietnamese_input_if_changed(
                            &text_var_resp,
                            vietnamese_input_enabled,
                            vietnamese_input_mode,
                            &mut step.ocr_text_var,
                        );
                        *live_sync |= text_var_resp.changed();
                        ui.end_row();
                    });
            });
    }

    pub(crate) fn render_custom_ocr_inline_controls(
        ui: &mut egui::Ui,
        language: UiLanguage,
        vietnamese_input_enabled: bool,
        vietnamese_input_mode: VietnameseInputMode,
        newly_installed_langs: &[String],
        group_id: u32,
        preset_id: u32,
        step_index: usize,
        step: &mut MacroStep,
        live_sync: &mut bool,
        pending_ocr_step_capture: &mut Option<(u32, u32, usize)>,
    ) {
        let ctrl_height = ui.spacing().interact_size.y;

        ui.add_space(4.0);

        let pick_btn = egui::Button::new("⛶");
        if ui
            .add_sized([ctrl_height, ctrl_height], pick_btn)
            .on_hover_text(Self::tr_lang(
                language,
                "Pick area - Drag on screen to select the OCR scan region",
                "ChÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€šÃ‚Ân vÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¹ng - KÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â©o trÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Âªn mÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â n hÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¬nh ÃƒÆ’Ã¢â‚¬Å¾ÃƒÂ¢Ã¢â€šÂ¬Ã‹Å“ÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€ Ã¢â‚¬â„¢ chÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€šÃ‚Ân vÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¹ng quÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â©t OCR",
            ))
            .clicked()
        {
            *pending_ocr_step_capture = Some((group_id, preset_id, step_index));
        }

        ui.add_space(4.0);

        let popular_langs: &[(&str, &str, &str)] = &[
            ("", "Auto", "Detect language from Windows profile. No install needed."),
            ("en", "English (en)", "English - usually installed by default"),
            ("vi", "Vietnamese (vi)", "Tieng Viet - install via Windows Settings > Language"),
            (
                "zh-Hans",
                "Chinese Simp (zh)",
                "Simplified Chinese - install via Windows Settings > Language",
            ),
            (
                "zh-Hant",
                "Chinese Trad (zht)",
                "Traditional Chinese - install via Windows Settings > Language",
            ),
            ("ja", "Japanese (ja)", "install via Windows Settings > Language"),
            ("ko", "Korean (ko)", "install via Windows Settings > Language"),
            ("fr", "French (fr)", "Francais - install via Windows Settings > Language"),
            ("de", "German (de)", "Deutsch - install via Windows Settings > Language"),
            ("es", "Spanish (es)", "Espanol - install via Windows Settings > Language"),
            ("ru", "Russian (ru)", "install via Windows Settings > Language"),
            ("th", "Thai (th)", "install via Windows Settings > Language"),
        ];

        let available_languages = crate::ocr::available_ocr_languages();
        let current_language = step.ocr_lang.clone().unwrap_or_default();
        let language_label = popular_langs
            .iter()
            .find(|(code, _, _)| *code == current_language.as_str())
            .map(|(_, label, _)| *label)
            .unwrap_or(if current_language.is_empty() {
                "Auto"
            } else {
                current_language.as_str()
            });

        let short_label = match current_language.as_str() {
            "" => "Auto",
            "en" => "EN",
            "vi" => "VI",
            "zh-Hans" => "ZH",
            "zh-Hant" => "ZHT",
            "ja" => "JA",
            "ko" => "KO",
            "fr" => "FR",
            "de" => "DE",
            "es" => "ES",
            "ru" => "RU",
            "th" => "TH",
            other => {
                if other.starts_with("zh-Han") {
                    "ZH"
                } else if let Some(idx) = other.find('-') {
                    &other[..idx]
                } else {
                    other
                }
            }
        };

        let combo_resp = egui::ComboBox::from_id_salt((group_id, preset_id, step_index, "ocr-step-lang"))
            .width(56.0)
            .selected_text(short_label)
            .show_ui(ui, |ui| {
                for (code, label, hint) in popular_langs {
                    let is_selected = current_language.as_str() == *code;
                    let is_installed = code.is_empty()
                        || available_languages
                            .iter()
                            .any(|lang| lang.to_lowercase().starts_with(&code.to_lowercase()))
                        || newly_installed_langs
                            .iter()
                            .any(|lang| lang.to_lowercase() == code.to_lowercase());

                    let display = if is_installed {
                        label.to_string()
                    } else {
                        format!("{} [not installed]", label)
                    };

                    let response = ui.selectable_label(is_selected, &display);
                    let hover_message = if is_installed {
                        hint.to_string()
                    } else {
                        format!(
                            "{} - Language pack NOT installed. Go to Windows Settings > Time & Language > Language & Region > Add a language",
                            hint
                        )
                    };

                    if response.on_hover_text(hover_message).clicked() {
                        step.ocr_lang = if code.is_empty() {
                            None
                        } else {
                            Some(code.to_string())
                        };
                        *live_sync = true;
                    }
                }
            });

        combo_resp.response.on_hover_text(format!(
            "{}: {}",
            Self::tr_lang(language, "Language", "NgÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â´n ngÃƒÆ’Ã‚Â¡Ãƒâ€šÃ‚Â»Ãƒâ€šÃ‚Â¯"),
            language_label
        ));

        ui.add_space(4.0);

        let previous_override = ui.visuals().override_text_color;
        ui.visuals_mut().override_text_color = None;
        let target_resp = ui.add(
            egui::TextEdit::singleline(&mut step.ocr_target_text)
                .desired_width(120.0)
                .hint_text(Self::tr_lang(language, "Target Text", "Van ban can tim")),
        );
        ui.visuals_mut().override_text_color = previous_override;

        Self::apply_vietnamese_input_if_changed(
            &target_resp,
            vietnamese_input_enabled,
            vietnamese_input_mode,
            &mut step.ocr_target_text,
        );
        *live_sync |= target_resp.changed();
    }
}
