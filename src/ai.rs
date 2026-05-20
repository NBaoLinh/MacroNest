use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{
    AiSettings, CrosshairStyle, CommandPreset, GroqSettings, MacroAction, MacroPreset, MacroStep,
    RgbaColor,
};

const API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const GROQ_API_URL: &str = "https://api.groq.com/openai/v1";

#[derive(Serialize)]
struct GenerateRequest<'a> {
    #[serde(rename = "systemInstruction")]
    system_instruction: Content<'a>,
    contents: [Content<'a>; 1],
}

#[derive(Serialize)]
struct Content<'a> {
    parts: [Part<'a>; 1],
}

#[derive(Serialize)]
struct Part<'a> {
    text: &'a str,
}

#[derive(Deserialize)]
struct GenerateResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<ResponseContent>,
}

#[derive(Deserialize)]
struct ResponseContent {
    parts: Option<Vec<ResponsePart>>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: Option<String>,
}

#[derive(Deserialize)]
struct GroqChatResponse {
    choices: Vec<GroqChoice>,
}

#[derive(Deserialize)]
struct GroqChoice {
    message: GroqMessage,
}

#[derive(Deserialize)]
struct GroqMessage {
    content: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CrosshairStylePatch {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub x_offset: Option<i32>,
    #[serde(default)]
    pub y_offset: Option<i32>,
    #[serde(default)]
    pub horizontal_length: Option<f32>,
    #[serde(default)]
    pub vertical_length: Option<f32>,
    #[serde(default)]
    pub arm_length: Option<f32>,
    #[serde(default)]
    pub thickness: Option<f32>,
    #[serde(default)]
    pub gap: Option<f32>,
    #[serde(default)]
    pub outline_enabled: Option<bool>,
    #[serde(default)]
    pub outline_thickness: Option<f32>,
    #[serde(default)]
    pub outline_color: Option<CrosshairColorPatch>,
    #[serde(default)]
    pub center_dot: Option<bool>,
    #[serde(default)]
    pub center_dot_size: Option<f32>,
    #[serde(default)]
    pub opacity: Option<f32>,
    #[serde(default)]
    pub color: Option<CrosshairColorPatch>,
    #[serde(default)]
    pub custom_asset: Option<Option<String>>,
    #[serde(default)]
    pub custom_scale: Option<f32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CrosshairColorPatch {
    #[serde(default)]
    pub r: Option<u8>,
    #[serde(default)]
    pub g: Option<u8>,
    #[serde(default)]
    pub b: Option<u8>,
    #[serde(default)]
    pub a: Option<u8>,
}

impl CrosshairColorPatch {
    fn apply_to(&self, color: &mut RgbaColor) {
        if let Some(r) = self.r {
            color.r = r;
        }
        if let Some(g) = self.g {
            color.g = g;
        }
        if let Some(b) = self.b {
            color.b = b;
        }
        if let Some(a) = self.a {
            color.a = a;
        }
    }
}

impl CrosshairStylePatch {
    pub fn apply_to(&self, style: &mut CrosshairStyle) {
        if let Some(enabled) = self.enabled {
            style.enabled = enabled;
        }
        if let Some(x_offset) = self.x_offset {
            style.x_offset = x_offset;
        }
        if let Some(y_offset) = self.y_offset {
            style.y_offset = y_offset;
        }
        if let Some(horizontal_length) = self.horizontal_length {
            style.horizontal_length = horizontal_length;
        }
        if let Some(vertical_length) = self.vertical_length {
            style.vertical_length = vertical_length;
        }
        if let Some(arm_length) = self.arm_length {
            style.arm_length = arm_length;
        }
        if let Some(thickness) = self.thickness {
            style.thickness = thickness;
        }
        if let Some(gap) = self.gap {
            style.gap = gap;
        }
        if let Some(outline_enabled) = self.outline_enabled {
            style.outline_enabled = outline_enabled;
        }
        if let Some(outline_thickness) = self.outline_thickness {
            style.outline_thickness = outline_thickness;
        }
        if let Some(outline_color) = &self.outline_color {
            outline_color.apply_to(&mut style.outline_color);
        }
        if let Some(center_dot) = self.center_dot {
            style.center_dot = center_dot;
        }
        if let Some(center_dot_size) = self.center_dot_size {
            style.center_dot_size = center_dot_size;
        }
        if let Some(opacity) = self.opacity {
            style.opacity = opacity;
        }
        if let Some(color) = &self.color {
            color.apply_to(&mut style.color);
        }
        if let Some(custom_asset) = &self.custom_asset {
            style.custom_asset = custom_asset.clone();
        }
        if let Some(custom_scale) = self.custom_scale {
            style.custom_scale = custom_scale;
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CommandPresetPatch {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub collapsed: Option<bool>,
    #[serde(default)]
    pub target_window_title: Option<String>,
    #[serde(default)]
    pub extra_target_window_titles: Option<Vec<String>>,
    #[serde(default)]
    pub match_duplicate_window_titles: Option<bool>,
    #[serde(default)]
    pub use_powershell: Option<bool>,
}

impl CommandPresetPatch {
    pub fn apply_to(&self, preset: &mut CommandPreset) {
        if let Some(name) = self.name.as_ref() {
            let next = name.trim();
            if !next.is_empty() {
                preset.name = next.to_owned();
            }
        }
        if let Some(command) = self.command.as_ref() {
            preset.command = normalize_command_text(command);
        }
        if let Some(enabled) = self.enabled {
            preset.enabled = enabled;
        }
        if let Some(collapsed) = self.collapsed {
            preset.collapsed = collapsed;
        }
        if let Some(target_window_title) = self.target_window_title.as_ref() {
            let next = target_window_title.trim();
            preset.target_window_title = if next.is_empty() {
                None
            } else {
                Some(next.to_owned())
            };
        }
        if let Some(extra_target_window_titles) = self.extra_target_window_titles.as_ref() {
            preset.extra_target_window_titles = extra_target_window_titles
                .iter()
                .map(|title| title.trim())
                .filter(|title| !title.is_empty())
                .map(|title| title.to_owned())
                .collect();
        }
        if let Some(match_duplicate_window_titles) = self.match_duplicate_window_titles {
            preset.match_duplicate_window_titles = match_duplicate_window_titles;
        }
        preset.use_powershell = false;
    }
}

#[derive(Serialize)]
struct AiTraceReport<'a> {
    timestamp_unix_ms: u128,
    model: &'a str,
    prompt: &'a str,
    system_instruction: &'a str,
    raw_response_text: Option<&'a str>,
    parsed_steps: Option<&'a [MacroStep]>,
    error: Option<&'a str>,
}

#[derive(Deserialize)]
struct AiMacroStepDraft {
    #[serde(default)]
    key: String,
    #[serde(default)]
    action: Option<MacroAction>,
    #[serde(default)]
    delay_ms: u64,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    x: i32,
    #[serde(default)]
    y: i32,
    #[serde(default)]
    text_override: String,
    #[serde(default)]
    command_preset_command: String,
    #[serde(default)]
    command_preset_use_powershell: bool,
    #[serde(default)]
    timed_override: bool,
    #[serde(default)]
    duration_override_ms: u64,
    #[serde(default)]
    smooth_mouse_path: bool,
    #[serde(default)]
    mouse_speed_percent: u32,
    #[serde(default)]
    vision_move_cursor_on_match: bool,
    #[serde(default)]
    vision_wait_until_found: bool,
    #[serde(default)]
    vision_trigger_macro_enabled: bool,
    #[serde(default)]
    vision_trigger_macro_preset_id: Option<u32>,
}

impl AiMacroStepDraft {
    fn into_macro_step(self) -> Option<MacroStep> {
        let action = self.action?;
        let mut step = MacroStep::default();
        let mut key = self.key.trim().to_owned();
        let text_override = self.text_override.trim().to_owned();

        if action == MacroAction::TypeText && key.is_empty() && !text_override.is_empty() {
            key = text_override.clone();
        }

        if Self::action_uses_key(action) && key.trim().is_empty() {
            return None;
        }

        step.key = key.trim().to_owned();
        step.action = action;
        step.delay_ms = self.delay_ms;
        if let Some(enabled) = self.enabled {
            step.enabled = enabled;
        }
        step.x = self.x;
        step.y = self.y;
        step.text_override = if action == MacroAction::TypeText {
            String::new()
        } else {
            self.text_override
        };
        step.command_preset_command = normalize_command_text(&self.command_preset_command);
        step.command_preset_use_powershell = self.command_preset_use_powershell;
        step.timed_override = self.timed_override;
        step.duration_override_ms = self.duration_override_ms;
        step.smooth_mouse_path = self.smooth_mouse_path;
        step.mouse_speed_percent = self.mouse_speed_percent;
        step.vision_move_cursor_on_match = self.vision_move_cursor_on_match;
        step.vision_wait_until_found = self.vision_wait_until_found;
        step.vision_trigger_macro_enabled = self.vision_trigger_macro_enabled;
        step.vision_trigger_macro_preset_id = self.vision_trigger_macro_preset_id;
        Some(step)
    }

    fn action_uses_key(action: MacroAction) -> bool {
        matches!(
            action,
            MacroAction::KeyPress
                | MacroAction::KeyDown
                | MacroAction::KeyUp
                | MacroAction::TypeText
                | MacroAction::ApplyWindowPreset
                | MacroAction::FocusWindowPreset
                | MacroAction::TriggerMacroPreset
                | MacroAction::TriggerCommandPreset
                | MacroAction::EnableCrosshairProfile
                | MacroAction::EnablePinPreset
                | MacroAction::PlayMousePathPreset
                | MacroAction::ApplyMouseSensitivityPreset
                | MacroAction::EnableZoomPreset
                | MacroAction::PlaySoundPreset
                | MacroAction::EnableMacroPreset
                | MacroAction::DisableMacroPreset
                | MacroAction::StartVisionSearch
                | MacroAction::TriggerVisionMove
                | MacroAction::StopVisionWait
                | MacroAction::StopVision
                | MacroAction::LoopStart
                | MacroAction::StopIfKeyPressed
                | MacroAction::ShowHud
                | MacroAction::LockKeys
                | MacroAction::UnlockKeys
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct MacroAiPlan {
    pub steps: Vec<MacroStep>,
    pub command_presets: Vec<CommandPresetPatch>,
}

#[derive(Deserialize, Default)]
struct AiMacroPlanDraft {
    #[serde(default)]
    steps: Vec<AiMacroStepDraft>,
    #[serde(default)]
    command_presets: Vec<CommandPresetPatch>,
}

pub fn generate_macro_steps(settings: &AiSettings, prompt: &str) -> Result<Vec<MacroStep>> {
    let _ = (settings, prompt);
    bail!("AI model support was removed. Use Groq in settings.")
}

pub fn generate_macro_steps_groq(
    settings: &GroqSettings,
    prompt: &str,
    system_instruction: &str,
) -> Result<Vec<MacroStep>> {
    let text = groq_chat_completion_text(
        settings,
        prompt,
        system_instruction,
        Some(serde_json::json!({"type": "json_object"})),
    )?;
    parse_steps_json(&text, prompt)
}

pub fn generate_macro_plan_groq(
    settings: &GroqSettings,
    prompt: &str,
    system_instruction: &str,
) -> Result<MacroAiPlan> {
    let text = groq_chat_completion_text(settings, prompt, system_instruction, None)?;
    match parse_macro_ai_plan_script(&text, prompt) {
        Ok(plan) => Ok(plan),
        Err(script_error) => match parse_macro_ai_plan_json(&text, prompt) {
            Ok(plan) => Ok(plan),
            Err(json_error) => {
                let repair_prompt = format!(
                    "Rewrite the following MacroNest macro request as a compact line-by-line script.\n\
                     Return plain text only, one command per line.\n\
                     Use commands like wait_100, press_D, hold_key_down_A, key_up_A, type_hello world, trigger_Open Edge, and custom_Open Edge = start msedge.\n\
                     Do not return JSON, markdown fences, bullets, or explanations.\n\
                     \n\
                     User request: {}\n\
                     \n\
                     Original AI response:\n\
                     {}\n",
                    extract_user_request(prompt),
                    text.trim()
                );
                let repair_system_instruction = "You rewrite malformed MacroNest macro AI output into a compact plain-text script. Return only the script, one command per line. Do not add prose or JSON.";
                let repaired_text = groq_chat_completion_text(
                    settings,
                    &repair_prompt,
                    repair_system_instruction,
                    None,
                )?;
                parse_macro_ai_plan_script(&repaired_text, prompt).or_else(|_repair_script_error| {
                    parse_macro_ai_plan_json(&repaired_text, prompt).with_context(|| {
                        format!(
                            "{script_error}; {json_error}; repair attempt also failed to produce a valid macro script"
                        )
                    })
                })
            }
        },
    }
}

pub fn generate_crosshair_style_patch_groq(
    settings: &GroqSettings,
    prompt: &str,
    system_instruction: &str,
) -> Result<CrosshairStylePatch> {
    let text = groq_chat_completion_text(
        settings,
        prompt,
        system_instruction,
        Some(serde_json::json!({"type": "json_object"})),
    )?;
    parse_crosshair_style_patch_json(&text, prompt)
}

pub fn generate_command_preset_patch_groq(
    settings: &GroqSettings,
    prompt: &str,
    system_instruction: &str,
) -> Result<CommandPresetPatch> {
    let text = groq_chat_completion_text(
        settings,
        prompt,
        system_instruction,
        Some(serde_json::json!({"type": "json_object"})),
    )?;
    parse_custom_preset_patch_json(&text, prompt)
}

pub fn normalize_command_text(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let lines = trimmed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if lines.len() > 1 {
        return lines.join(" ");
    }

    let single_line = lines.first().copied().unwrap_or(trimmed);
    if single_line.len() >= 2 {
        let mut chars = single_line.chars();
        let first = chars.next().unwrap_or_default();
        let last = single_line.chars().last().unwrap_or_default();
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            let inner = single_line[1..single_line.len() - 1].trim();
            return inner.to_owned();
        }
    }

    single_line.to_owned()
}

pub fn prompt_contains_action_intent(prompt: &str) -> bool {
    let normalized = prompt.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }

    const ACTION_WORDS: &[&str] = &[
        "press",
        "hold",
        "wait",
        "delay",
        "type",
        "click",
        "move",
        "open",
        "launch",
        "close",
        "minimize",
        "maximize",
        "shutdown",
        "restart",
        "sleep",
        "lock",
        "unlock",
        "enable",
        "disable",
        "toggle",
        "show",
        "hide",
        "focus",
        "run",
        "kill",
        "pin",
        "unpin",
        "loop",
        "repeat",
        "start",
        "stop",
        "switch",
        "activate",
        "deactivate",
    ];

    normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .any(|word| ACTION_WORDS.contains(&word))
}

pub fn normalize_trigger_command_preset_keys(
    steps: &mut [MacroStep],
    command_presets: &[CommandPreset],
) {
    let mut catalog = HashMap::new();
    for preset in command_presets {
        let name = preset.name.trim();
        if !name.is_empty() {
            catalog.insert(name.to_ascii_lowercase(), preset.id);
        }
    }

    for step in steps {
        if step.action != MacroAction::TriggerCommandPreset {
            continue;
        }
        let key = step.key.trim();
        if key.is_empty() || key.parse::<u32>().is_ok() {
            continue;
        }
        if let Some(id) = catalog.get(&key.to_ascii_lowercase()) {
            step.key = id.to_string();
        }
    }
}

pub fn attach_command_preset_drafts_to_steps(
    steps: &mut [MacroStep],
    command_presets: &[CommandPresetPatch],
) {
    let mut catalog = HashMap::new();
    for preset in command_presets {
        let Some(name) = preset.name.as_ref() else {
            continue;
        };
        let key = name.trim();
        if !key.is_empty() {
            catalog.insert(key.to_ascii_lowercase(), preset.clone());
        }
    }

    for step in steps {
        if step.action != MacroAction::TriggerCommandPreset {
            continue;
        }
        let key = step.key.trim();
        if key.is_empty() {
            continue;
        }
        let Some(patch) = catalog.get(&key.to_ascii_lowercase()) else {
            continue;
        };
        if let Some(command) = patch.command.as_ref() {
            step.command_preset_command = command.trim().to_owned();
        }
        if let Some(use_powershell) = patch.use_powershell {
            step.command_preset_use_powershell = use_powershell;
        }
    }
}

fn normalize_wait_delay_before_command_trigger(user_request: &str, steps: &mut [MacroStep]) {
    let Some(wait_ms) = extract_first_wait_duration_ms(user_request) else {
        return;
    };
    if wait_ms == 0 || steps.is_empty() {
        return;
    }
    if steps.iter().any(|step| step.delay_ms == wait_ms) {
        return;
    }

    let mut saw_non_command_step = false;
    for step in steps {
        if step.action == MacroAction::TriggerCommandPreset {
            if saw_non_command_step && step.delay_ms == 0 {
                step.delay_ms = wait_ms;
                return;
            }
            continue;
        }
        saw_non_command_step = true;
    }
}

fn extract_first_wait_duration_ms(user_request: &str) -> Option<u64> {
    fn extract_duration(text: &str, needle: &str) -> Option<u64> {
        let Some(index) = text.find(needle) else {
            return None;
        };
        let tail = &text[index + needle.len()..];
        let mut digits = String::new();
        let mut seen_digit = false;
        for ch in tail.chars() {
            if ch.is_ascii_digit() {
                digits.push(ch);
                seen_digit = true;
                continue;
            }
            if seen_digit {
                break;
            }
        }
        let value = digits.parse::<u64>().ok()?;
        let tail_lower = tail.to_lowercase();
        if tail_lower.contains("ms")
            || tail_lower.contains("milli")
            || tail_lower.contains("mili")
            || tail_lower.contains("miligiay")
            || tail_lower.contains("milisec")
        {
            return Some(value);
        }
        Some(value.saturating_mul(1000))
    }

    let normalized = user_request.to_lowercase();
    for needle in [
        "wait", "delay", "pause", "sleep", "after", "doi", "đoi", "đợi", "chờ", "cho",
    ] {
        if let Some(value) = extract_duration(&normalized, needle) {
            return Some(value);
        }
    }
    None
}

fn groq_chat_completion_text(
    settings: &GroqSettings,
    prompt: &str,
    system_instruction: &str,
    response_format: Option<serde_json::Value>,
) -> Result<String> {
    if settings.api_key.trim().is_empty() {
        bail!("Enter a Groq API key first");
    }
    let model = settings.model.trim();
    if model.is_empty() {
        bail!("The Groq model name is empty");
    }

    let url = format!("{GROQ_API_URL}/chat/completions");
    let mut body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_instruction.trim()},
            {"role": "user", "content": prompt.trim()}
        ],
        "temperature": 0.0,
    });
    if let Some(response_format) = response_format {
        body["response_format"] = response_format;
    }

    let response = reqwest::blocking::Client::new()
        .post(url)
        .bearer_auth(settings.api_key.trim())
        .json(&body)
        .send()
        .context("Failed to call the Groq API")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().unwrap_or_default();
        let error_message = if error_body.trim().is_empty() {
            format!("Groq returned an error response: HTTP {status}")
        } else {
            format!("Groq returned an error response: HTTP {status}\n{error_body}")
        };
        bail!(error_message);
    }

    let payload: GroqChatResponse = response
        .json()
        .context("Failed to parse the Groq response")?;

    payload
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.message.content)
        .context("Groq did not return any text")
}

fn generate_macro_steps_gemini(settings: &AiSettings, prompt: &str) -> Result<Vec<MacroStep>> {
    if settings.api_key.trim().is_empty() {
        write_ai_trace(
            settings.model.trim(),
            prompt,
            AiTraceReport {
                timestamp_unix_ms: unix_ms_now(),
                model: settings.model.trim(),
                prompt,
                system_instruction: settings.system_instruction.trim(),
                raw_response_text: None,
                parsed_steps: None,
                error: Some("Enter a Gemini API key first"),
            },
        );
        bail!("Enter a Gemini API key first");
    }
    let model = settings.model.trim();
    if model.is_empty() {
        write_ai_trace(
            model,
            prompt,
            AiTraceReport {
                timestamp_unix_ms: unix_ms_now(),
                model,
                prompt,
                system_instruction: settings.system_instruction.trim(),
                raw_response_text: None,
                parsed_steps: None,
                error: Some("The AI model name is empty"),
            },
        );
        bail!("The AI model name is empty");
    }

    let url = format!(
        "{API_URL}/{model}:generateContent?key={}",
        settings.api_key.trim()
    );
    let body = GenerateRequest {
        system_instruction: Content {
            parts: [Part {
                text: settings.system_instruction.trim(),
            }],
        },
        contents: [Content {
            parts: [Part {
                text: prompt.trim(),
            }],
        }],
    };

    let response = reqwest::blocking::Client::new()
        .post(url)
        .json(&body)
        .send()
        .context("Failed to call the Gemini API")
        .inspect_err(|error| {
            write_ai_trace(
                model,
                prompt,
                AiTraceReport {
                    timestamp_unix_ms: unix_ms_now(),
                    model,
                    prompt,
                    system_instruction: settings.system_instruction.trim(),
                    raw_response_text: None,
                    parsed_steps: None,
                    error: Some(&format!("Failed to call the Gemini API: {error}")),
                },
            );
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().unwrap_or_default();
        let error_message = if error_body.trim().is_empty() {
            format!("Gemini returned an error response: HTTP {status}")
        } else {
            format!("Gemini returned an error response: HTTP {status}\n{error_body}")
        };
        write_ai_trace(
            model,
            prompt,
            AiTraceReport {
                timestamp_unix_ms: unix_ms_now(),
                model,
                prompt,
                system_instruction: settings.system_instruction.trim(),
                raw_response_text: Some(&error_body),
                parsed_steps: None,
                error: Some(&error_message),
            },
        );
        bail!(error_message);
    }

    let payload: GenerateResponse = response
        .json()
        .context("Failed to parse the Gemini response")
        .inspect_err(|error| {
            write_ai_trace(
                model,
                prompt,
                AiTraceReport {
                    timestamp_unix_ms: unix_ms_now(),
                    model,
                    prompt,
                    system_instruction: settings.system_instruction.trim(),
                    raw_response_text: None,
                    parsed_steps: None,
                    error: Some(&format!("Failed to parse the Gemini response: {error}")),
                },
            );
        })?;

    let text = payload
        .candidates
        .and_then(|mut list| list.drain(..).next())
        .and_then(|candidate| candidate.content)
        .and_then(|content| content.parts)
        .and_then(|mut parts| parts.drain(..).find_map(|part| part.text))
        .context("Gemini did not return any text")
        .inspect_err(|error| {
            write_ai_trace(
                model,
                prompt,
                AiTraceReport {
                    timestamp_unix_ms: unix_ms_now(),
                    model,
                    prompt,
                    system_instruction: settings.system_instruction.trim(),
                    raw_response_text: None,
                    parsed_steps: None,
                    error: Some(&format!("Gemini did not return any text: {error}")),
                },
            );
        })?;

    let result = parse_steps_json(&text, prompt);
    match &result {
        Ok(steps) => {
            write_ai_trace(
                model,
                prompt,
                AiTraceReport {
                    timestamp_unix_ms: unix_ms_now(),
                    model,
                    prompt,
                    system_instruction: settings.system_instruction.trim(),
                    raw_response_text: Some(&text),
                    parsed_steps: Some(steps),
                    error: None,
                },
            );
        }
        Err(error) => {
            write_ai_trace(
                model,
                prompt,
                AiTraceReport {
                    timestamp_unix_ms: unix_ms_now(),
                    model,
                    prompt,
                    system_instruction: settings.system_instruction.trim(),
                    raw_response_text: Some(&text),
                    parsed_steps: None,
                    error: Some(&error.to_string()),
                },
            );
        }
    }
    result
}

fn parse_steps_json(text: &str, prompt: &str) -> Result<Vec<MacroStep>> {
    let mut attempts = Vec::new();
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        attempts.push(trimmed.to_owned());
    }

    if let Some(block) = extract_fenced_json_block(trimmed) {
        attempts.push(block);
    }

    if let Some(candidate) = extract_json_span(trimmed) {
        attempts.push(candidate);
    }

    let mut last_error = None;
    for candidate in attempts {
        match parse_steps_json_candidate(&candidate) {
            Ok(steps) => {
                let steps = sanitize_steps_for_prompt(prompt, steps)?;
                return Ok(steps);
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("AI output did not contain valid macro JSON")))
}

fn parse_steps_json_candidate(text: &str) -> Result<Vec<MacroStep>> {
    let value: serde_json::Value =
        serde_json::from_str(text).context("AI output was not valid macro JSON")?;
    let raw_steps = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(map) => map
            .get("steps")
            .and_then(|steps| steps.as_array())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("AI output must be a JSON array of steps"))?,
        _ => bail!("AI output must be a JSON array of steps"),
    };

    let steps = raw_steps
        .into_iter()
        .filter_map(|value| serde_json::from_value::<AiMacroStepDraft>(value).ok())
        .filter_map(AiMacroStepDraft::into_macro_step)
        .collect::<Vec<_>>();

    if steps.is_empty() {
        bail!("AI output did not contain any valid macro steps");
    }

    Ok(steps)
}

#[derive(Debug)]
enum MacroAiScriptEntry {
    Step(MacroStep),
    CommandPreset(CommandPresetPatch),
    Delay(u64),
}

fn parse_macro_ai_plan_script(text: &str, prompt: &str) -> Result<MacroAiPlan> {
    let mut steps = Vec::new();
    let mut command_presets = Vec::new();
    let mut pending_delay_ms = 0u64;

    for line in text.lines() {
        let Some(entry) = parse_macro_ai_script_line(line)? else {
            continue;
        };
        match entry {
            MacroAiScriptEntry::Delay(delay_ms) => {
                pending_delay_ms = pending_delay_ms.saturating_add(delay_ms);
            }
            MacroAiScriptEntry::Step(mut step) => {
                if pending_delay_ms > 0 {
                    step.delay_ms = step.delay_ms.saturating_add(pending_delay_ms);
                    pending_delay_ms = 0;
                }
                steps.push(step);
            }
            MacroAiScriptEntry::CommandPreset(preset) => command_presets.push(preset),
        }
    }

    if steps.is_empty() && command_presets.is_empty() {
        bail!("AI output did not contain any macro script commands");
    }

    let user_request = extract_user_request(prompt);
    if steps.is_empty() && !command_presets.is_empty() {
        steps = synthesize_command_trigger_steps(&user_request, &command_presets)?;
        if pending_delay_ms > 0 {
            if let Some(first_step) = steps.first_mut() {
                first_step.delay_ms = first_step.delay_ms.saturating_add(pending_delay_ms);
            }
        }
    }

    let mut plan = MacroAiPlan {
        steps,
        command_presets,
    };
    attach_command_preset_drafts_to_steps(&mut plan.steps, &plan.command_presets);
    let steps = sanitize_steps_for_prompt_allow_empty(prompt, plan.steps)?;
    plan.steps = steps;
    attach_command_preset_drafts_to_steps(&mut plan.steps, &plan.command_presets);
    normalize_wait_delay_before_command_trigger(&user_request, &mut plan.steps);
    if plan.steps.is_empty() && plan.command_presets.is_empty() {
        bail!("AI output did not contain any macro script commands");
    }

    Ok(plan)
}

fn parse_macro_ai_script_line(line: &str) -> Result<Option<MacroAiScriptEntry>> {
    let trimmed = strip_macro_ai_script_prefix(line).trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.starts_with('#') || trimmed.starts_with("//") {
        return Ok(None);
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "wait_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "wait "))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "delay_"))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "delay "))
    {
        let Some(delay_ms) = parse_macro_ai_duration_ms(rest) else {
            return Ok(None);
        };
        return Ok(Some(MacroAiScriptEntry::Delay(delay_ms)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "press_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "press "))
    {
        return Ok(parse_macro_ai_key_step(MacroAction::KeyPress, rest));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "hold_key_down_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "hold_key_down "))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "key_down_"))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "key_down "))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "keydown_"))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "keydown "))
    {
        return Ok(parse_macro_ai_key_step(MacroAction::KeyDown, rest));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "key_up_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "key_up "))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "keyup_"))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "keyup "))
    {
        return Ok(parse_macro_ai_key_step(MacroAction::KeyUp, rest));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "type_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "type "))
    {
        let text = rest.trim();
        if text.is_empty() {
            return Ok(None);
        }
        let mut step = MacroStep::default();
        step.action = MacroAction::TypeText;
        step.key = text.to_owned();
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "apply_window_preset_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "apply_window_preset "))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "window_preset_"))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "window_preset "))
    {
        return Ok(parse_macro_ai_key_step(
            MacroAction::ApplyWindowPreset,
            rest,
        ));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "focus_window_preset_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "focus_window_preset "))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "focus_preset_"))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "focus_preset "))
    {
        return Ok(parse_macro_ai_key_step(
            MacroAction::FocusWindowPreset,
            rest,
        ));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "play_mouse_path_preset_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "play_mouse_path_preset "))
    {
        return Ok(parse_macro_ai_key_step(
            MacroAction::PlayMousePathPreset,
            rest,
        ));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "apply_mouse_sensitivity_preset_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "apply_mouse_sensitivity_preset "))
    {
        return Ok(parse_macro_ai_key_step(
            MacroAction::ApplyMouseSensitivityPreset,
            rest,
        ));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "start_image_search_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "start_image_search "))
    {
        return Ok(parse_macro_ai_key_step(MacroAction::StartVisionSearch, rest));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "trigger_image_search_move_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "trigger_image_search_move "))
    {
        return Ok(parse_macro_ai_key_step(
            MacroAction::TriggerVisionMove,
            rest,
        ));
    }


    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "stop_image_search_wait_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "stop_image_search_wait "))
    {
        return Ok(parse_macro_ai_key_step(
            MacroAction::StopVisionWait,
            rest,
        ));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "stop_image_search_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "stop_image_search "))
    {
        return Ok(parse_macro_ai_key_step(MacroAction::StopVision, rest));
    }

    if trimmed.eq_ignore_ascii_case("lock_mouse") || trimmed.eq_ignore_ascii_case("lock mouse") {
        let mut step = MacroStep::default();
        step.action = MacroAction::LockMouse;
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if trimmed.eq_ignore_ascii_case("unlock_mouse") || trimmed.eq_ignore_ascii_case("unlock mouse")
    {
        let mut step = MacroStep::default();
        step.action = MacroAction::UnlockMouse;
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "mouse_move_relative_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "mouse_move_relative "))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "move_relative_"))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "move_relative "))
    {
        if let Some((x, y)) = parse_macro_ai_coordinates(rest) {
            let mut step = MacroStep::default();
            step.action = MacroAction::MouseMoveRelative;
            step.x = x;
            step.y = y;
            return Ok(Some(MacroAiScriptEntry::Step(step)));
        }
        return Ok(None);
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "mouse_move_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "mouse_move "))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "move_"))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "move "))
    {
        if let Some((x, y)) = parse_macro_ai_coordinates(rest) {
            let mut step = MacroStep::default();
            step.action = MacroAction::MouseMoveAbsolute;
            step.x = x;
            step.y = y;
            return Ok(Some(MacroAiScriptEntry::Step(step)));
        }
        return Ok(None);
    }

    for (action_name, action) in [
        ("left_click", MacroAction::MouseLeftClick),
        ("mouse_left_click", MacroAction::MouseLeftClick),
        ("left_down", MacroAction::MouseLeftDown),
        ("mouse_left_down", MacroAction::MouseLeftDown),
        ("left_up", MacroAction::MouseLeftUp),
        ("mouse_left_up", MacroAction::MouseLeftUp),
        ("right_click", MacroAction::MouseRightClick),
        ("mouse_right_click", MacroAction::MouseRightClick),
        ("right_down", MacroAction::MouseRightDown),
        ("mouse_right_down", MacroAction::MouseRightDown),
        ("right_up", MacroAction::MouseRightUp),
        ("mouse_right_up", MacroAction::MouseRightUp),
        ("middle_click", MacroAction::MouseMiddleClick),
        ("mouse_middle_click", MacroAction::MouseMiddleClick),
        ("middle_down", MacroAction::MouseMiddleDown),
        ("mouse_middle_down", MacroAction::MouseMiddleDown),
        ("middle_up", MacroAction::MouseMiddleUp),
        ("mouse_middle_up", MacroAction::MouseMiddleUp),
        ("x1_click", MacroAction::MouseX1Click),
        ("mouse_x1_click", MacroAction::MouseX1Click),
        ("x1_down", MacroAction::MouseX1Down),
        ("mouse_x1_down", MacroAction::MouseX1Down),
        ("x1_up", MacroAction::MouseX1Up),
        ("mouse_x1_up", MacroAction::MouseX1Up),
        ("x2_click", MacroAction::MouseX2Click),
        ("mouse_x2_click", MacroAction::MouseX2Click),
        ("x2_down", MacroAction::MouseX2Down),
        ("mouse_x2_down", MacroAction::MouseX2Down),
        ("x2_up", MacroAction::MouseX2Up),
        ("mouse_x2_up", MacroAction::MouseX2Up),
        ("wheel_up", MacroAction::MouseWheelUp),
        ("mouse_wheel_up", MacroAction::MouseWheelUp),
        ("wheel_down", MacroAction::MouseWheelDown),
        ("mouse_wheel_down", MacroAction::MouseWheelDown),
    ] {
        if trimmed.eq_ignore_ascii_case(action_name) {
            return Ok(Some(MacroAiScriptEntry::Step({
                let mut step = MacroStep::default();
                step.action = action;
                step
            })));
        }
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "trigger_macro_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "trigger_macro "))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "macro_"))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "macro "))
    {
        let key = rest.trim();
        if key.is_empty() {
            return Ok(None);
        }
        let mut step = MacroStep::default();
        step.action = MacroAction::TriggerMacroPreset;
        step.key = key.to_owned();
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "trigger_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "trigger "))
    {
        let key = rest.trim();
        if key.is_empty() {
            return Ok(None);
        }
        let mut step = MacroStep::default();
        step.action = MacroAction::TriggerCommandPreset;
        step.key = key.to_owned();
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "custom_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "custom "))
    {
        let Some((name_part, command_part)) = rest.split_once('=') else {
            return Ok(None);
        };
        let name = name_part.trim();
        let command = normalize_command_text(command_part);
        if name.is_empty() || command.is_empty() {
            return Ok(None);
        }
        let patch = CommandPresetPatch {
            name: Some(name.to_owned()),
            command: Some(command.to_owned()),
            enabled: None,
            collapsed: None,
            target_window_title: None,
            extra_target_window_titles: None,
            match_duplicate_window_titles: None,
            use_powershell: Some(false),
        };
        return Ok(Some(MacroAiScriptEntry::CommandPreset(patch)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "loop_start_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "loop_start "))
    {
        let mut step = MacroStep::default();
        step.action = MacroAction::LoopStart;
        let key = rest.trim();
        step.key = if key.is_empty() {
            "1".to_owned()
        } else {
            key.to_owned()
        };
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if trimmed.eq_ignore_ascii_case("loop_end") || trimmed.eq_ignore_ascii_case("loopend") {
        let mut step = MacroStep::default();
        step.action = MacroAction::LoopEnd;
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if trimmed.eq_ignore_ascii_case("stop_if_trigger_pressed_again")
        || trimmed.eq_ignore_ascii_case("stop if trigger pressed again")
    {
        let mut step = MacroStep::default();
        step.action = MacroAction::StopIfTriggerPressedAgain;
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "stop_if_key_pressed_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "stop_if_key_pressed "))
    {
        let key = rest.trim();
        if key.is_empty() {
            return Ok(None);
        }
        let mut step = MacroStep::default();
        step.action = MacroAction::StopIfKeyPressed;
        step.key = key.to_owned();
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "show_hud_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "show_hud "))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "show_toolbox_"))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "show_toolbox "))
    {
        let key = rest.trim();
        if key.is_empty() {
            return Ok(None);
        }
        let mut step = MacroStep::default();
        step.action = MacroAction::ShowHud;
        step.key = key.to_owned();
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if trimmed.eq_ignore_ascii_case("hide_hud")
        || trimmed.eq_ignore_ascii_case("hide hud")
        || trimmed.eq_ignore_ascii_case("hide_toolbox")
    {
        let mut step = MacroStep::default();
        step.action = MacroAction::HideHud;
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "lock_keys_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "lock_keys "))
    {
        let key = rest.trim();
        if key.is_empty() {
            return Ok(None);
        }
        let mut step = MacroStep::default();
        step.action = MacroAction::LockKeys;
        step.key = key.to_owned();
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "unlock_keys_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "unlock_keys "))
    {
        let key = rest.trim();
        if key.is_empty() {
            return Ok(None);
        }
        let mut step = MacroStep::default();
        step.action = MacroAction::UnlockKeys;
        step.key = key.to_owned();
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "enable_macro_preset_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "enable_macro_preset "))
    {
        let key = rest.trim();
        if key.is_empty() {
            return Ok(None);
        }
        let mut step = MacroStep::default();
        step.action = MacroAction::EnableMacroPreset;
        step.key = key.to_owned();
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "disable_macro_preset_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "disable_macro_preset "))
    {
        let key = rest.trim();
        if key.is_empty() {
            return Ok(None);
        }
        let mut step = MacroStep::default();
        step.action = MacroAction::DisableMacroPreset;
        step.key = key.to_owned();
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if trimmed.eq_ignore_ascii_case("disable_crosshair")
        || trimmed.eq_ignore_ascii_case("disable crosshair")
    {
        let mut step = MacroStep::default();
        step.action = MacroAction::DisableCrosshair;
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if trimmed.eq_ignore_ascii_case("disable_pin") || trimmed.eq_ignore_ascii_case("disable pin") {
        let mut step = MacroStep::default();
        step.action = MacroAction::DisablePin;
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if trimmed.eq_ignore_ascii_case("disable_zoom") || trimmed.eq_ignore_ascii_case("disable zoom")
    {
        let mut step = MacroStep::default();
        step.action = MacroAction::DisableZoom;
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if trimmed.eq_ignore_ascii_case("disable_macro")
        || trimmed.eq_ignore_ascii_case("disable macro")
    {
        let mut step = MacroStep::default();
        step.action = MacroAction::DisableMacroPreset;
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if trimmed.eq_ignore_ascii_case("enable_crosshair")
        || trimmed.eq_ignore_ascii_case("enable crosshair")
    {
        let mut step = MacroStep::default();
        step.action = MacroAction::EnableCrosshairProfile;
        step.key = "Default".to_owned();
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "enable_crosshair_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "enable_crosshair "))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "enable_profile_"))
        .or_else(|| strip_case_insensitive_prefix(trimmed, "enable_profile "))
    {
        let key = rest.trim();
        if key.is_empty() {
            return Ok(None);
        }
        let mut step = MacroStep::default();
        step.action = MacroAction::EnableCrosshairProfile;
        step.key = key.to_owned();
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "enable_pin_preset_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "enable_pin_preset "))
    {
        let key = rest.trim();
        if key.is_empty() {
            return Ok(None);
        }
        let mut step = MacroStep::default();
        step.action = MacroAction::EnablePinPreset;
        step.key = key.to_owned();
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "enable_zoom_preset_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "enable_zoom_preset "))
    {
        let key = rest.trim();
        if key.is_empty() {
            return Ok(None);
        }
        let mut step = MacroStep::default();
        step.action = MacroAction::EnableZoomPreset;
        step.key = key.to_owned();
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    if let Some(rest) = strip_case_insensitive_prefix(trimmed, "play_sound_preset_")
        .or_else(|| strip_case_insensitive_prefix(trimmed, "play_sound_preset "))
    {
        let key = rest.trim();
        if key.is_empty() {
            return Ok(None);
        }
        let mut step = MacroStep::default();
        step.action = MacroAction::PlaySoundPreset;
        step.key = key.to_owned();
        return Ok(Some(MacroAiScriptEntry::Step(step)));
    }

    Ok(None)
}

fn parse_macro_ai_key_step(action: MacroAction, rest: &str) -> Option<MacroAiScriptEntry> {
    let key = rest.trim();
    if key.is_empty() {
        return None;
    }
    let mut step = MacroStep::default();
    step.action = action;
    step.key = key.to_owned();
    Some(MacroAiScriptEntry::Step(step))
}

fn parse_macro_ai_duration_ms(text: &str) -> Option<u64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut multiplier = 1u64;
    let mut number_text = trimmed;
    let mut parts = trimmed.split_whitespace();
    let first = parts.next()?;
    let second = parts.next();
    if parts.next().is_none() {
        if let Some(unit) = second {
            number_text = first;
            let unit = unit.to_ascii_lowercase();
            if unit == "s"
                || unit == "sec"
                || unit == "secs"
                || unit == "second"
                || unit == "seconds"
            {
                multiplier = 1_000;
            }
        }
    }

    let lower = number_text.to_ascii_lowercase();
    let digits_text = if let Some(value) = lower.strip_suffix("ms") {
        value.trim()
    } else if let Some(value) = lower.strip_suffix("s") {
        multiplier = 1_000;
        value.trim()
    } else {
        number_text
    };

    let digits = digits_text
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u64>().ok()?.checked_mul(multiplier)
}

fn parse_macro_ai_coordinates(text: &str) -> Option<(i32, i32)> {
    let mut numbers = text
        .split(|ch: char| !ch.is_ascii_digit() && ch != '-')
        .filter_map(|part| part.trim().parse::<i32>().ok());
    let x = numbers.next()?;
    let y = numbers.next()?;
    Some((x, y))
}

fn strip_macro_ai_script_prefix(line: &str) -> &str {
    let trimmed = line.trim_start();
    let Some(first) = trimmed.chars().next() else {
        return trimmed;
    };

    if matches!(first, '-' | '*' | '•') {
        return trimmed[first.len_utf8()..].trim_start();
    }

    let mut digits_end = 0usize;
    for (index, ch) in trimmed.char_indices() {
        if ch.is_ascii_digit() {
            digits_end = index + ch.len_utf8();
            continue;
        }
        break;
    }

    if digits_end > 0 {
        let remainder = trimmed[digits_end..].trim_start();
        if let Some(rest) = remainder
            .strip_prefix('.')
            .or_else(|| remainder.strip_prefix(')'))
        {
            return rest.trim_start();
        }
    }

    trimmed
}

fn strip_case_insensitive_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.get(..prefix.len())
        .filter(|candidate| candidate.eq_ignore_ascii_case(prefix))
        .map(|_| &text[prefix.len()..])
}

fn parse_macro_ai_plan_json(text: &str, prompt: &str) -> Result<MacroAiPlan> {
    let mut attempts = Vec::new();
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        attempts.push(trimmed.to_owned());
    }

    if let Some(block) = extract_fenced_json_block(trimmed) {
        attempts.push(block);
    }

    if let Some(candidate) = extract_json_span(trimmed) {
        attempts.push(candidate);
    }

    let mut last_error = None;
    for candidate in attempts {
        match parse_macro_ai_plan_json_candidate(&candidate) {
            Ok(plan) => {
                let user_request = extract_user_request(prompt);
                let steps = sanitize_steps_for_prompt_allow_empty(prompt, plan.steps)?;
                if steps.is_empty() {
                    let synthesized_steps =
                        synthesize_command_trigger_steps(&user_request, &plan.command_presets)?;
                    if !synthesized_steps.is_empty() {
                        let mut plan = MacroAiPlan {
                            steps: synthesized_steps,
                            command_presets: plan.command_presets,
                        };
                        attach_command_preset_drafts_to_steps(&mut plan.steps, &plan.command_presets);
                        normalize_wait_delay_before_command_trigger(&user_request, &mut plan.steps);
                        return Ok(plan);
                    }
                    if !plan.command_presets.is_empty() {
                        bail!(
                            "AI output only contained custom presets, but the request did not include an actionable macro step"
                        );
                    }
                }
                let mut plan = MacroAiPlan {
                    steps,
                    command_presets: plan.command_presets,
                };
                attach_command_preset_drafts_to_steps(&mut plan.steps, &plan.command_presets);
                normalize_wait_delay_before_command_trigger(&user_request, &mut plan.steps);
                return Ok(plan);
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error
        .unwrap_or_else(|| anyhow::anyhow!("AI output did not contain valid macro plan JSON")))
}

fn parse_macro_ai_plan_json_candidate(text: &str) -> Result<MacroAiPlan> {
    let value: serde_json::Value =
        serde_json::from_str(text).context("AI output was not valid macro JSON")?;
    match value {
        serde_json::Value::Array(items) => {
            let steps = items
                .into_iter()
                .filter_map(|value| serde_json::from_value::<AiMacroStepDraft>(value).ok())
                .filter_map(AiMacroStepDraft::into_macro_step)
                .collect::<Vec<_>>();
            if steps.is_empty() {
                bail!("AI output did not contain any valid macro steps");
            }
            Ok(MacroAiPlan {
                steps,
                command_presets: Vec::new(),
            })
        }
        serde_json::Value::Object(_) => {
            let draft: AiMacroPlanDraft =
                serde_json::from_str(text).context("AI output was not valid macro JSON")?;
            let steps = draft
                .steps
                .into_iter()
                .filter_map(AiMacroStepDraft::into_macro_step)
                .collect::<Vec<_>>();
            Ok(MacroAiPlan {
                steps,
                command_presets: draft.command_presets,
            })
        }
        _ => bail!("AI output must be a JSON array or object"),
    }
}

fn parse_crosshair_style_patch_json(text: &str, _prompt: &str) -> Result<CrosshairStylePatch> {
    let mut attempts = Vec::new();
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        attempts.push(trimmed.to_owned());
    }

    if let Some(block) = extract_fenced_json_block(trimmed) {
        attempts.push(block);
    }

    if let Some(candidate) = extract_json_span(trimmed) {
        attempts.push(candidate);
    }

    let mut last_error = None;
    for candidate in attempts {
        match serde_json::from_str::<CrosshairStylePatch>(&candidate) {
            Ok(patch) => return Ok(patch),
            Err(error) => last_error = Some(error.into()),
        }
    }

    Err(last_error
        .unwrap_or_else(|| anyhow::anyhow!("AI output did not contain valid crosshair JSON")))
}

fn parse_custom_preset_patch_json(text: &str, _prompt: &str) -> Result<CommandPresetPatch> {
    let mut attempts = Vec::new();
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        attempts.push(trimmed.to_owned());
    }

    if let Some(block) = extract_fenced_json_block(trimmed) {
        attempts.push(block);
    }

    if let Some(candidate) = extract_json_span(trimmed) {
        attempts.push(candidate);
    }

    let mut last_error = None;
    for candidate in attempts {
        match serde_json::from_str::<CommandPresetPatch>(&candidate) {
            Ok(patch) => return Ok(patch),
            Err(error) => last_error = Some(error.into()),
        }
    }

    Err(last_error
        .unwrap_or_else(|| anyhow::anyhow!("AI output did not contain valid custom preset JSON")))
}

fn extract_fenced_json_block(text: &str) -> Option<String> {
    let start = text.find("```")?;
    let rest = &text[start + 3..];
    let rest = rest.strip_prefix("json").unwrap_or(rest).trim_start();
    let end = rest.find("```")?;
    Some(rest[..end].trim().to_owned())
}

fn extract_json_span(text: &str) -> Option<String> {
    let array_start = text.find('[');
    let object_start = text.find('{');
    let start = match (array_start, object_start) {
        (Some(a), Some(o)) => a.min(o),
        (Some(a), None) => a,
        (None, Some(o)) => o,
        (None, None) => return None,
    };
    let tail = &text[start..];
    let end = if tail.starts_with('[') {
        text.rfind(']')?
    } else {
        text.rfind('}')?
    };
    if end < start {
        return None;
    }
    Some(text[start..=end].trim().to_owned())
}

fn unix_ms_now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn write_ai_trace(model: &str, prompt: &str, report: AiTraceReport<'_>) {
    let _ = (model, prompt, report);
}

fn sanitize_steps_for_prompt(prompt: &str, steps: Vec<MacroStep>) -> Result<Vec<MacroStep>> {
    sanitize_steps_for_prompt_impl(prompt, steps, false)
}

fn sanitize_steps_for_prompt_allow_empty(
    prompt: &str,
    steps: Vec<MacroStep>,
) -> Result<Vec<MacroStep>> {
    sanitize_steps_for_prompt_impl(prompt, steps, true)
}

fn sanitize_steps_for_prompt_impl(
    prompt: &str,
    mut steps: Vec<MacroStep>,
    allow_empty: bool,
) -> Result<Vec<MacroStep>> {
    let user_request = extract_user_request(prompt);
    if user_request.to_ascii_lowercase().contains("hold ") {
        normalize_hold_delay_to_key_up(&mut steps);
    }
    normalize_tap_keydown_keyup_pairs(&mut steps);
    normalize_absolute_click_request(&user_request, &mut steps);
    let catalogs = extract_prompt_catalogs(prompt);
    normalize_named_target_keys(&catalogs, &mut steps);
    normalize_press_repeat_request(&user_request, &mut steps);
    normalize_loop_repeat_request(&user_request, &mut steps);
    normalize_simple_type_text_request(&user_request, &mut steps);

    if steps.is_empty() && !allow_empty {
        bail!("AI output did not contain any macro body steps");
    }

    Ok(steps)
}

fn synthesize_command_trigger_steps(
    _user_request: &str,
    command_presets: &[CommandPresetPatch],
) -> Result<Vec<MacroStep>> {
    let custom_names = command_presets
        .iter()
        .filter_map(|preset| preset.name.as_ref())
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(|name| name.to_owned())
        .collect::<Vec<_>>();

    if custom_names.is_empty() {
        return Ok(Vec::new());
    }

    let mut steps = Vec::with_capacity(custom_names.len());
    for name in custom_names {
        let mut step = MacroStep::default();
        step.action = MacroAction::TriggerCommandPreset;
        step.key = name;
        steps.push(step);
    }
    Ok(steps)
}

fn extract_user_request(prompt: &str) -> String {
    let Some((_, rest)) = prompt.split_once("User request:") else {
        return prompt.trim().to_owned();
    };
    rest.lines().next().unwrap_or_default().trim().to_owned()
}

fn normalize_hold_delay_to_key_up(steps: &mut [MacroStep]) {
    for index in 0..steps.len().saturating_sub(1) {
        let (left, right) = steps.split_at_mut(index + 1);
        let current = &mut left[index];
        let next = &mut right[0];
        if current.action == MacroAction::KeyDown
            && next.action == MacroAction::KeyUp
            && !current.key.trim().is_empty()
            && current.key.trim().eq_ignore_ascii_case(next.key.trim())
            && current.delay_ms > 0
            && next.delay_ms == 0
        {
            next.delay_ms = current.delay_ms;
            current.delay_ms = 0;
            return;
        }
    }
}

fn normalize_tap_keydown_keyup_pairs(steps: &mut Vec<MacroStep>) {
    let mut rewritten = Vec::with_capacity(steps.len());
    let mut index = 0usize;
    while index < steps.len() {
        let current = steps[index].clone();
        if index + 1 < steps.len() {
            let next = &steps[index + 1];
            if current.action == MacroAction::KeyDown
                && next.action == MacroAction::KeyUp
                && current.delay_ms == 0
                && next.delay_ms == 0
                && current.key.trim().eq_ignore_ascii_case(next.key.trim())
            {
                let mut tap = current.clone();
                tap.action = MacroAction::KeyPress;
                tap.delay_ms = 0;
                rewritten.push(tap);
                index += 2;
                continue;
            }
        }
        rewritten.push(current);
        index += 1;
    }
    *steps = rewritten;
}

fn normalize_absolute_click_request(user_request: &str, steps: &mut Vec<MacroStep>) {
    let Some((x, y, click_action)) = parse_absolute_click_request(user_request) else {
        return;
    };

    let mut move_step = MacroStep::default();
    move_step.action = MacroAction::MouseMoveAbsolute;
    move_step.x = x;
    move_step.y = y;
    move_step.delay_ms = 0;
    let mut click_step = MacroStep::default();
    click_step.action = click_action;
    click_step.delay_ms = 0;

    let has_move = steps
        .iter()
        .any(|step| step.action == MacroAction::MouseMoveAbsolute && step.x == x && step.y == y);
    let has_click = steps.iter().any(|step| step.action == click_action);

    if has_move && has_click {
        return;
    }

    if !has_move && !has_click {
        steps.push(move_step);
        steps.push(click_step);
        return;
    }

    if !has_move {
        if let Some(click_index) = steps.iter().position(|step| step.action == click_action) {
            steps.insert(click_index, move_step);
        } else {
            steps.insert(0, move_step);
        }
    }

    if !has_click {
        if let Some(move_index) = steps.iter().position(|step| {
            step.action == MacroAction::MouseMoveAbsolute && step.x == x && step.y == y
        }) {
            steps.insert(move_index + 1, click_step);
        } else {
            steps.push(click_step);
        }
    }
}

fn parse_absolute_click_request(user_request: &str) -> Option<(i32, i32, MacroAction)> {
    let lower = user_request.to_ascii_lowercase();
    if !lower.contains("click") || !lower.contains(" at ") {
        return None;
    }

    let click_action = if lower.contains("right click") {
        MacroAction::MouseRightClick
    } else if lower.contains("middle click") {
        MacroAction::MouseMiddleClick
    } else if lower.contains("x1 click") {
        MacroAction::MouseX1Click
    } else if lower.contains("x2 click") {
        MacroAction::MouseX2Click
    } else {
        MacroAction::MouseLeftClick
    };

    let at_index = lower.rfind(" at ")?;
    let tail = user_request.get(at_index + 4..)?.trim();
    let mut coords = tail
        .split(|ch: char| !ch.is_ascii_digit() && ch != '-')
        .filter_map(|part| part.trim().parse::<i32>().ok());
    let x = coords.next()?;
    let y = coords.next()?;
    Some((x, y, click_action))
}

fn normalize_press_repeat_request(user_request: &str, steps: &mut Vec<MacroStep>) {
    let repeat_specs = extract_press_repeat_specs(user_request);
    if repeat_specs.is_empty() || steps.is_empty() {
        return;
    }

    let Some((_, normalized_repeat_key, repeat_count)) = repeat_specs.first() else {
        return;
    };

    let Some((tap_key, _)) = extract_repeatable_tap_key(steps) else {
        return;
    };

    if !tap_key.eq_ignore_ascii_case(normalized_repeat_key) {
        return;
    }

    let mut rewritten = Vec::with_capacity(*repeat_count);
    for _ in 0..*repeat_count {
        let mut repeated = MacroStep::default();
        repeated.action = MacroAction::KeyPress;
        repeated.key = normalized_repeat_key.clone();
        repeated.delay_ms = 0;
        rewritten.push(repeated);
    }
    *steps = rewritten;
}

fn normalize_loop_repeat_request(user_request: &str, steps: &mut Vec<MacroStep>) {
    if steps.is_empty() || !request_looks_like_loop(user_request) {
        return;
    }
    if steps
        .iter()
        .any(|step| matches!(step.action, MacroAction::LoopStart | MacroAction::LoopEnd))
    {
        return;
    }

    let expected_repeat_count = extract_loop_repeat_count(user_request);
    let Some((block_len, repeat_count)) = detect_repeated_loop_block(steps, expected_repeat_count)
    else {
        return;
    };
    if repeat_count < 2 || block_len == 0 {
        return;
    }

    let mut rewritten = Vec::with_capacity(block_len + 2);
    let mut loop_start = MacroStep::default();
    loop_start.action = MacroAction::LoopStart;
    loop_start.key = repeat_count.to_string();
    rewritten.push(loop_start);
    rewritten.extend_from_slice(&steps[..block_len]);
    let mut loop_end = MacroStep::default();
    loop_end.action = MacroAction::LoopEnd;
    rewritten.push(loop_end);
    *steps = rewritten;
}

fn request_looks_like_loop(user_request: &str) -> bool {
    let lower = user_request.to_ascii_lowercase();
    lower.contains("loop")
        || lower.contains("repeat")
        || lower.contains("repeate")
        || lower.contains("lặp")
        || lower.contains("lap ")
        || lower.contains("lap lai")
}

fn extract_loop_repeat_count(user_request: &str) -> Option<usize> {
    let lower = user_request.to_ascii_lowercase();
    let keywords = ["loop", "repeat", "lặp", "lap"];
    for keyword in keywords {
        let Some(index) = lower.find(keyword) else {
            continue;
        };
        let tail = &lower[index + keyword.len()..];
        let mut digits = String::new();
        let mut seen_digit = false;
        for ch in tail.chars() {
            if ch.is_ascii_digit() {
                digits.push(ch);
                seen_digit = true;
                continue;
            }
            if seen_digit {
                break;
            }
        }
        if let Ok(count) = digits.parse::<usize>()
            && count >= 2
        {
            return Some(count);
        }
    }
    None
}

fn detect_repeated_loop_block(
    steps: &[MacroStep],
    expected_repeat_count: Option<usize>,
) -> Option<(usize, usize)> {
    let total = steps.len();
    if total < 2 {
        return None;
    }

    let mut candidate_lengths = Vec::new();
    if let Some(repeat_count) = expected_repeat_count
        && repeat_count >= 2
        && total % repeat_count == 0
    {
        candidate_lengths.push(total / repeat_count);
    }
    for block_len in 1..=total / 2 {
        if total % block_len == 0 {
            candidate_lengths.push(block_len);
        }
    }
    candidate_lengths.sort_unstable();
    candidate_lengths.dedup();

    for block_len in candidate_lengths {
        let repeat_count = total / block_len;
        if repeat_count < 2 {
            continue;
        }
        if let Some(expected) = expected_repeat_count
            && expected != repeat_count
        {
            continue;
        }
        let block = &steps[..block_len];
        let mut matches = true;
        for chunk_index in 1..repeat_count {
            let start = chunk_index * block_len;
            let end = start + block_len;
            if steps[start..end] != *block {
                matches = false;
                break;
            }
        }
        if matches {
            return Some((block_len, repeat_count));
        }
    }

    None
}

fn extract_press_repeat_specs(user_request: &str) -> Vec<(String, String, usize)> {
    let tokens = user_request
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let mut specs = Vec::new();
    for index in 0..tokens.len().saturating_sub(1) {
        if !tokens[index].eq_ignore_ascii_case("press") {
            continue;
        }
        let token = tokens[index + 1].to_ascii_lowercase();
        let literal = if let Some(repeat_suffix) = token.strip_prefix('x') {
            if !repeat_suffix.is_empty() && repeat_suffix.chars().all(|ch| ch.is_ascii_digit()) {
                token
            } else {
                continue;
            }
        } else if token == "x" {
            let Some(count_token) = tokens.get(index + 2) else {
                continue;
            };
            if !count_token.chars().all(|ch| ch.is_ascii_digit()) {
                continue;
            }
            format!("x{count_token}")
        } else {
            continue;
        };

        let Some(repeat_count) = literal
            .trim_start_matches('x')
            .parse::<usize>()
            .ok()
            .filter(|count| *count >= 2)
        else {
            continue;
        };
        specs.push((literal, "X".to_owned(), repeat_count));
    }
    specs
}

fn normalize_simple_type_text_request(user_request: &str, steps: &mut Vec<MacroStep>) {
    let trimmed = user_request.trim_start();
    let Some(rest) = trimmed.strip_prefix("type ") else {
        let Some(rest) = trimmed.strip_prefix("Type ") else {
            return;
        };
        let _ = rest;
        return;
    };
    let text = rest.trim();
    if text.is_empty() {
        return;
    }

    let mut normalized = text.trim_matches('"').trim_matches('\'').to_owned();
    if normalized.is_empty() {
        return;
    }
    if normalized.len() >= 2 && normalized.starts_with('\"') && normalized.ends_with('\"') {
        normalized = normalized[1..normalized.len() - 1].to_owned();
    }

    let Some(typed_text) = extract_typeable_text_from_steps(steps) else {
        return;
    };

    let mut text_step = MacroStep::default();
    text_step.action = MacroAction::TypeText;
    text_step.key = if normalized.is_empty() {
        typed_text
    } else {
        normalized
    };
    steps.clear();
    steps.push(text_step);
}

fn extract_repeatable_tap_key(steps: &[MacroStep]) -> Option<(String, usize)> {
    if steps.is_empty() {
        return None;
    }

    let mut index = 0usize;
    let mut key: Option<String> = None;
    let mut tap_count = 0usize;

    while index < steps.len() {
        let current = &steps[index];
        match current.action {
            MacroAction::KeyPress => {
                let current_key = current.key.trim();
                if current_key.len() != 1 {
                    return None;
                }
                if let Some(existing) = key.as_ref() {
                    if !existing.eq_ignore_ascii_case(current_key) {
                        return None;
                    }
                } else {
                    key = Some(current_key.to_owned());
                }
                tap_count += 1;
                index += 1;
            }
            MacroAction::KeyDown => {
                let Some(next) = steps.get(index + 1) else {
                    return None;
                };
                if next.action != MacroAction::KeyUp {
                    return None;
                }
                let current_key = current.key.trim();
                if current_key.len() != 1 || !current_key.eq_ignore_ascii_case(next.key.trim()) {
                    return None;
                }
                if let Some(existing) = key.as_ref() {
                    if !existing.eq_ignore_ascii_case(current_key) {
                        return None;
                    }
                } else {
                    key = Some(current_key.to_owned());
                }
                tap_count += 1;
                index += 2;
            }
            _ => return None,
        }
    }

    key.map(|key| (key, tap_count))
}

fn extract_typeable_text_from_steps(steps: &[MacroStep]) -> Option<String> {
    if steps.is_empty() {
        return None;
    }

    let mut index = 0usize;
    let mut typed = String::new();

    while index < steps.len() {
        let current = &steps[index];
        match current.action {
            MacroAction::KeyPress => {
                let key = current.key.trim();
                if key.len() != 1 {
                    return None;
                }
                typed.push_str(key);
                index += 1;
            }
            MacroAction::KeyDown => {
                let Some(next) = steps.get(index + 1) else {
                    return None;
                };
                if next.action != MacroAction::KeyUp {
                    return None;
                }
                let key = current.key.trim();
                if key.len() != 1 || !key.eq_ignore_ascii_case(next.key.trim()) {
                    return None;
                }
                typed.push_str(key);
                index += 2;
            }
            _ => return None,
        }
    }

    if typed.is_empty() { None } else { Some(typed) }
}

#[derive(Default)]
struct PromptCatalogs {
    macro_group_presets: HashMap<String, u32>,
    pin_presets: HashMap<String, u32>,
    hud_presets: HashMap<String, u32>,
    window_presets: HashMap<String, u32>,
    focus_presets: HashMap<String, u32>,
    zoom_presets: HashMap<String, u32>,
    sound_presets: HashMap<String, u32>,
    mouse_sensitivity_presets: HashMap<String, u32>,
    mouse_path_presets: HashMap<String, u32>,
    command_presets: HashMap<String, u32>,
    vision_presets: HashMap<String, u32>,
}

fn extract_prompt_catalogs(prompt: &str) -> PromptCatalogs {
    PromptCatalogs {
        macro_group_presets: extract_named_id_catalog(prompt, "Current macro group presets:"),
        pin_presets: extract_named_id_catalog(prompt, "Available pin presets:"),
        hud_presets: extract_named_id_catalog(prompt, "Available HUD presets:"),
        window_presets: extract_named_id_catalog(prompt, "Available window presets:"),
        focus_presets: extract_named_id_catalog(prompt, "Available focus presets:"),
        zoom_presets: extract_named_id_catalog(prompt, "Available zoom presets:"),
        sound_presets: extract_named_id_catalog(prompt, "Available sound presets:"),
        mouse_sensitivity_presets: extract_named_id_catalog(
            prompt,
            "Available mouse sensitivity presets:",
        ),
        mouse_path_presets: extract_named_id_catalog(prompt, "Available mouse path presets:"),
        command_presets: extract_named_id_catalog(prompt, "Available Command presets:"),
        vision_presets: extract_named_id_catalog(prompt, "Available image search presets:"),
    }
}

fn extract_named_id_catalog(prompt: &str, header: &str) -> HashMap<String, u32> {
    let mut map = HashMap::new();
    let mut in_section = false;
    for line in prompt.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case(header) {
            in_section = true;
            continue;
        }
        if !in_section {
            continue;
        }
        if trimmed.is_empty() {
            break;
        }
        let Some(entry) = trimmed.strip_prefix('-') else {
            break;
        };
        let entry = entry.trim();
        if entry.eq_ignore_ascii_case("none") {
            continue;
        }
        let Some((id_part, name_part)) = entry.split_once('|') else {
            continue;
        };
        let Ok(id) = id_part.trim().parse::<u32>() else {
            continue;
        };
        let name = name_part.trim();
        if !name.is_empty() {
            map.insert(normalize_catalog_name(name), id);
        }
    }
    map
}

fn normalize_catalog_name(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_named_target_keys(catalogs: &PromptCatalogs, steps: &mut [MacroStep]) {
    for step in steps {
        let key = step.key.trim();
        if key.is_empty() {
            continue;
        }
        if key.parse::<u32>().is_ok() {
            continue;
        }

        let resolved = match step.action {
            MacroAction::ApplyWindowPreset => resolve_catalog_id(&catalogs.window_presets, key),
            MacroAction::FocusWindowPreset => resolve_catalog_id(&catalogs.focus_presets, key),
            MacroAction::TriggerMacroPreset
            | MacroAction::EnableMacroPreset
            | MacroAction::DisableMacroPreset => {
                resolve_catalog_id(&catalogs.macro_group_presets, key)
            }
            MacroAction::TriggerCommandPreset => resolve_catalog_id(&catalogs.command_presets, key),
            MacroAction::EnablePinPreset => resolve_catalog_id(&catalogs.pin_presets, key),
            MacroAction::PlayMousePathPreset => {
                resolve_catalog_id(&catalogs.mouse_path_presets, key)
            }
            MacroAction::ApplyMouseSensitivityPreset => {
                resolve_catalog_id(&catalogs.mouse_sensitivity_presets, key)
            }
            MacroAction::EnableZoomPreset => resolve_catalog_id(&catalogs.zoom_presets, key),
            MacroAction::PlaySoundPreset => resolve_catalog_id(&catalogs.sound_presets, key),
            MacroAction::StartVisionSearch
            | MacroAction::TriggerVisionMove
            | MacroAction::StopVision => {
                resolve_catalog_id(&catalogs.vision_presets, key)
            }
            MacroAction::ShowHud => resolve_catalog_id(&catalogs.hud_presets, key),
            _ => None,
        };

        if let Some(id) = resolved {
            step.key = id.to_string();
        }
    }
}

fn resolve_catalog_id(catalog: &HashMap<String, u32>, key: &str) -> Option<u32> {
    catalog.get(&normalize_catalog_name(key)).copied()
}

pub fn apply_steps_to_preset(preset: &mut MacroPreset, steps: Vec<MacroStep>) {
    preset.steps = steps;
}

pub fn merge_steps_into_preset(preset: &mut MacroPreset, steps: Vec<MacroStep>) {
    if preset.steps.is_empty() {
        preset.steps = steps;
        return;
    }

    let existing_len = preset.steps.len();
    let incoming_len = steps.len();
    let shared_len = existing_len.min(incoming_len);

    for index in 0..shared_len {
        preset.steps[index] = steps[index].clone();
    }

    if incoming_len > existing_len {
        preset.steps.extend(steps.into_iter().skip(existing_len));
    }
}

pub fn strip_rewritten_append_prefix(existing_steps: &[MacroStep], steps: &mut Vec<MacroStep>) {
    if existing_steps.is_empty() || steps.is_empty() {
        return;
    }

    let existing_keys = existing_steps
        .iter()
        .filter_map(|step| match step.action {
            MacroAction::KeyPress | MacroAction::KeyDown | MacroAction::KeyUp => {
                let key = step.key.trim();
                (!key.is_empty()).then_some(key.to_ascii_lowercase())
            }
            _ => None,
        })
        .collect::<HashSet<_>>();
    if existing_keys.is_empty() {
        return;
    }

    let mut drop_to = 0usize;
    let mut rewrite_started = false;
    let mut index = 0usize;
    while index < steps.len() {
        match steps[index].action {
            MacroAction::Wait => {
                index += 1;
                if rewrite_started {
                    drop_to = index;
                }
            }
            MacroAction::KeyPress | MacroAction::KeyDown | MacroAction::KeyUp => {
                let key = steps[index].key.trim().to_ascii_lowercase();
                if key.is_empty() {
                    break;
                }
                if existing_keys.contains(&key) {
                    rewrite_started = true;
                    index += 1;
                    drop_to = index;
                } else if rewrite_started {
                    break;
                } else {
                    return;
                }
            }
            _ => {
                if rewrite_started {
                    break;
                }
                return;
            }
        }
    }

    if rewrite_started && drop_to > 0 {
        steps.drain(0..drop_to);
    }
}
