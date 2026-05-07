use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};

use arboard::Clipboard;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{
    self, Button, Color32, ColorImage, DragValue, FontData, FontDefinitions, FontFamily, Frame,
    RichText, Sense, Slider, Stroke, StrokeKind, TextEdit, TextureHandle, TextureOptions, pos2,
    vec2,
};

use crate::{
    audio, hotkey,
    model::{
        AppPanel, AppState, AudioClipSettings, CaptureRequest, CapturedInput, CrosshairStyle,
        HotkeyBinding, ImageSearchPreset, ImageSearchTimingPreset, MacroAction, MacroFolder,
        MacroGroup, MacroPreset, MacroStep, MacroTriggerMode,
        MasterMacroGroupState, MasterMacroPresetState, MasterPreset, MasterWindowFocusPresetState,
        MasterWindowPresetState, MasterZoomPresetState, MousePathEvent, MousePathEventKind,
        MousePathPreset, MouseSensitivityPreset, PinOverlayStyle, PinPreset, ProfileRecord,
        RgbaColor, SoundLibraryItem, SoundPreset, ToolboxPreset, UiLanguage, UiThemeMode,
        WindowAnchor, WindowExpandDirection, WindowFocusPreset, WindowPreset, ZoomPreset,
    },
    overlay::{OverlayCommand, UiCommand},
    profile_code,
    storage::AppPaths,
    window_list,
};

#[cfg(windows)]
use windows::Win32::{
    Foundation::POINT,
    UI::{
        Input::KeyboardAndMouse::GetAsyncKeyState,
        WindowsAndMessaging::{GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN},
    },
};

#[derive(Default)]
struct AudioCardOutcome {
    changed: bool,
    choose_file: bool,
    open_editor: bool,
    status: Option<String>,
}

#[derive(Clone, Copy)]
enum AudioEditorTarget {
    Startup,
    Exit,
    Library(u32),
    Preset(u32),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ImageSearchCaptureMode {
    Template,
    SearchRegion,
    ColorSample,
    ColorPriorityAnchor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ImageSearchCaptureTarget {
    Preset(u32),
    TimingPreset(u32),
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct MouseMoveAbsoluteCaptureTarget {
    group_id: u32,
    preset_id: u32,
    step_index: usize,
}

#[derive(Clone)]
struct MacroStepDragPayload {
    group_id: u32,
    preset_id: u32,
    indices: Vec<usize>,
}

struct CloseToTrayAnimation {
    started_at: f64,
    duration_sec: f64,
}

struct OpenFromTrayAnimation {
    start_outer_pos: egui::Pos2,
    start_inner_size: egui::Vec2,
    end_outer_pos: egui::Pos2,
    end_inner_size: egui::Vec2,
    started_at: f64,
    duration_sec: f64,
}

struct StartupSplashState {
    started_at: Option<f64>,
    duration_sec: f64,
}

#[derive(Clone)]
struct ZoomPreviewView {
    texture: TextureHandle,
    title: String,
    screen_x: i32,
    screen_y: i32,
    logical_width: i32,
    logical_height: i32,
}

struct ZoomPreviewCache {
    updated_at: Instant,
    source_window_key: Option<String>,
    source_window_extra_keys: Vec<String>,
    match_duplicate_window_titles: bool,
    view: ZoomPreviewView,
}

#[derive(Clone)]
struct ImageSearchPreviewView {
    texture: TextureHandle,
    file_name: String,
    width: usize,
    height: usize,
}

struct ImageSearchPreviewCache {
    updated_at: Instant,
    source_path: PathBuf,
    source_modified: Option<std::time::SystemTime>,
    view: ImageSearchPreviewView,
}

const MATERIAL_ICONS_FONT: &str = "material_icons";
const UI_SANS_FONT: &str = "ui_sans";
const INTERCEPTION_RELEASE_URL: &str =
    "https://github.com/oblitum/Interception/releases/download/v1.0.1/Interception.zip";

pub fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        UI_SANS_FONT.to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/SegoeUI.ttf"
        ))),
    );
    fonts.font_data.insert(
        MATERIAL_ICONS_FONT.to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/MaterialIcons-Regular.ttf"
        ))),
    );
    let ui_family = FontFamily::Name(UI_SANS_FONT.into());
    fonts
        .families
        .entry(ui_family.clone())
        .or_default()
        .insert(0, UI_SANS_FONT.to_owned());
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, UI_SANS_FONT.to_owned());
    let material_family = FontFamily::Name(MATERIAL_ICONS_FONT.into());
    fonts
        .families
        .entry(material_family.clone())
        .or_default()
        .insert(0, MATERIAL_ICONS_FONT.to_owned());
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push(MATERIAL_ICONS_FONT.to_owned());
    ctx.set_fonts(fonts);
    ctx.style_mut(|style| {
        style.interaction.show_tooltips_only_when_still = false;
        style.interaction.tooltip_delay = 0.0;
        style.interaction.tooltip_grace_time = 0.0;
    });
}

#[derive(Clone, Copy)]
pub enum PopupBlobKind {
    AlreadyRunning,
}

pub struct PopupBlobApp {
    kind: PopupBlobKind,
    theme: UiThemeMode,
    started_at: Option<f64>,
    duration_sec: f64,
    center_next_frame: bool,
}

impl PopupBlobApp {
    pub fn new(kind: PopupBlobKind, theme: UiThemeMode) -> Self {
        Self {
            kind,
            theme,
            started_at: None,
            duration_sec: 1.55,
            center_next_frame: true,
        }
    }

    fn popup_palette(&self) -> (Color32, Color32, Color32, Color32, Color32) {
        match self.theme {
            UiThemeMode::Dark => (
                Color32::from_rgb(108, 244, 226),
                Color32::from_rgb(255, 120, 186),
                Color32::from_rgb(112, 170, 255),
                Color32::from_rgba_premultiplied(4, 8, 18, 230),
                Color32::from_rgba_premultiplied(12, 18, 30, 188),
            ),
            UiThemeMode::Light => (
                Color32::from_rgb(58, 196, 182),
                Color32::from_rgb(236, 102, 152),
                Color32::from_rgb(92, 144, 238),
                Color32::from_rgba_premultiplied(245, 250, 255, 228),
                Color32::from_rgba_premultiplied(220, 236, 246, 190),
            ),
        }
    }

    fn render_message_popup(&self, ctx: &egui::Context, progress: f32) {
        let rect = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("message-popup"),
        ));
        let center = rect.center();
        let time = ctx.input(|input| input.time) as f32;
        let ease_in = 1.0 - (1.0 - (progress / 0.32).clamp(0.0, 1.0)).powi(3);
        let shatter = ((progress - 0.48) / 0.52).clamp(0.0, 1.0);
        let shatter = 1.0 - (1.0 - shatter).powi(3);
        let scale = egui::lerp(0.18..=1.0, ease_in) * (1.0 - shatter * 0.28);
        let fade = 1.0 - shatter * 0.82;
        let (neon_cyan, neon_pink, neon_blue, dark_fill, mid_fill) = self.popup_palette();
        let (title, message) = match self.kind {
            PopupBlobKind::AlreadyRunning => ("MacroNest", "Already running in tray"),
        };

        for layer in 0..3 {
            let layer_t = layer as f32 / 2.0;
            let radius_x = rect.width() * (0.22 + layer_t * 0.12) * scale;
            let radius_y = rect.height() * (0.24 + layer_t * 0.08) * scale;
            let mut points = Vec::with_capacity(96);
            for step in 0..96 {
                let angle = step as f32 / 96.0 * std::f32::consts::TAU;
                let wobble = 1.0
                    + 0.13 * (angle * 3.0 + time * (0.9 + layer_t * 0.3)).sin()
                    + 0.07 * (angle * 5.0 - time * (0.65 + layer_t * 0.22)).cos();
                let blast = 1.0 + shatter * (0.12 + layer_t * 0.08);
                points.push(egui::pos2(
                    center.x + angle.cos() * radius_x * wobble * blast,
                    center.y + angle.sin() * radius_y * wobble * blast,
                ));
            }
            let fill = if layer == 0 {
                Color32::from_rgba_premultiplied(
                    dark_fill.r(),
                    dark_fill.g(),
                    dark_fill.b(),
                    (230.0 * fade) as u8,
                )
            } else if layer == 1 {
                Color32::from_rgba_premultiplied(
                    mid_fill.r(),
                    mid_fill.g(),
                    mid_fill.b(),
                    (168.0 * fade) as u8,
                )
            } else {
                Color32::from_rgba_premultiplied(
                    neon_pink.r(),
                    neon_pink.g(),
                    neon_pink.b(),
                    (52.0 * fade) as u8,
                )
            };
            let stroke = if layer == 2 { neon_pink } else { neon_blue };
            painter.add(egui::Shape::convex_polygon(
                points,
                fill,
                egui::Stroke::new(
                    1.4 - layer as f32 * 0.2,
                    Color32::from_rgba_premultiplied(
                        stroke.r(),
                        stroke.g(),
                        stroke.b(),
                        (110.0 * fade) as u8,
                    ),
                ),
            ));
        }

        for shard_index in 0..18 {
            let frac = shard_index as f32 / 18.0;
            let angle = frac * std::f32::consts::TAU + time * 0.6;
            let distance = rect.width().min(rect.height()) * 0.28 * shatter;
            let pos = egui::pos2(
                center.x + angle.cos() * distance,
                center.y + angle.sin() * distance * 0.72,
            );
            let color = if shard_index % 2 == 0 {
                neon_cyan
            } else {
                neon_pink
            };
            painter.circle_filled(
                pos,
                (1.2 + (shard_index % 4) as f32 * 0.45) * (0.8 + shatter * 0.4),
                Color32::from_rgba_premultiplied(
                    color.r(),
                    color.g(),
                    color.b(),
                    (140.0 * (1.0 - shatter * 0.35)) as u8,
                ),
            );
        }

        painter.text(
            egui::pos2(center.x, rect.top() + rect.height() * 0.38),
            egui::Align2::CENTER_CENTER,
            title,
            egui::FontId::proportional(26.0),
            Color32::from_rgba_premultiplied(244, 247, 255, (255.0 * fade) as u8),
        );
        painter.text(
            egui::pos2(center.x, rect.top() + rect.height() * 0.62),
            egui::Align2::CENTER_CENTER,
            message,
            egui::FontId::proportional(16.0),
            Color32::from_rgba_premultiplied(208, 220, 255, (220.0 * fade) as u8),
        );
    }
}

impl eframe::App for PopupBlobApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.center_next_frame {
            if let Some(center_cmd) = egui::ViewportCommand::center_on_screen(ctx) {
                ctx.send_viewport_cmd(center_cmd);
                self.center_next_frame = false;
            }
        }
        let now = ctx.input(|input| input.time);
        let started_at = self.started_at.get_or_insert(now);
        let progress = ((now - *started_at) / self.duration_sec).clamp(0.0, 1.0) as f32;
        if progress >= 1.0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        ctx.request_repaint();
        self.render_message_popup(ctx, progress);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MacroActionSubmenuKind {
    Mouse,
    ImageSearch,
}

pub struct CrosshairApp {
    pub paths: AppPaths,
    pub state: AppState,
    overlay_tx: Sender<OverlayCommand>,
    ui_rx: Receiver<UiCommand>,
    status: String,
    save_name: String,
    import_code_buffer: String,
    export_code_buffer: String,
    custom_assets: Vec<String>,
    open_windows: Vec<String>,
    quit_requested: bool,
    capture_target: Option<CaptureRequest>,
    startup_clip_duration_ms: Option<u64>,
    exit_clip_duration_ms: Option<u64>,
    startup_sound_played: bool,
    show_startup_audio_editor: bool,
    show_exit_audio_editor: bool,
    startup_sound_collapsed: bool,
    exit_sound_collapsed: bool,
    audio_waveforms: HashMap<String, Vec<f32>>,
    sound_preset_clip_duration_ms: HashMap<u32, Option<u64>>,
    show_sound_preset_audio_editor: HashSet<u32>,
    library_clip_duration_ms: HashMap<u32, Option<u64>>,
    show_library_audio_editor: HashSet<u32>,
    active_audio_editor: Option<AudioEditorTarget>,
    capture_ignored_keys: HashSet<u32>,
    capture_suppress_next_poll: bool,
    capture_wait_for_mouse_release: bool,
    capture_ignore_mouse_until_release: bool,
    capture_suppress_polls_remaining: u8,
    capture_mouse_guard_until: Option<Instant>,
    mouse_move_absolute_capture_target: Option<MouseMoveAbsoluteCaptureTarget>,
    mouse_move_absolute_capture_wait_for_mouse_release: bool,
    mouse_move_absolute_capture_raise_window: bool,
    mouse_move_absolute_restore_inner_size: Option<egui::Vec2>,
    mouse_move_absolute_restore_outer_pos: Option<egui::Pos2>,
    image_search_capture_active: bool,
    image_search_capture_target: Option<ImageSearchCaptureTarget>,
    image_search_capture_mode: Option<ImageSearchCaptureMode>,
    image_search_capture_anchor: Option<egui::Pos2>,
    image_search_capture_current: Option<egui::Pos2>,
    image_search_restore_inner_size: Option<egui::Vec2>,
    image_search_restore_outer_pos: Option<egui::Pos2>,
    selected_macro_steps: HashSet<(u32, u32, usize)>,
    selected_macro_groups: HashSet<u32>,
    macro_preset_search_query: String,
    macro_group_clipboard: Vec<u32>,
    macro_group_clipboard_is_cut: bool,
    macro_preset_clipboard: Option<MacroPreset>,
    macro_step_clipboard: Vec<MacroStep>,
    confirm_delete_folder_id: Option<u32>,
    confirm_release_folder_id: Option<u32>,
    confirm_delete_macro_group_id: Option<u32>,
    center_window_next_frame: bool,
    enforce_square_window_frames: u8,
    last_window_refresh_at: Instant,
    last_active_panel: AppPanel,
    macro_drag_select_anchor: Option<(u32, u32, usize)>,
    active_macro_folder_view: Option<u32>,
    crosshair_panel_collapsed: bool,
    close_to_tray_animation: Option<CloseToTrayAnimation>,
    open_from_tray_animation: Option<OpenFromTrayAnimation>,
    startup_splash: StartupSplashState,
    hidden_window_inner_size: Option<egui::Vec2>,
    hidden_window_outer_pos: Option<egui::Pos2>,
    zoom_preview_cache: HashMap<u32, ZoomPreviewCache>,
    image_search_preview_cache: HashMap<u32, ImageSearchPreviewCache>,
    image_search_color_pick_texture: Option<TextureHandle>,
    image_search_color_pick_preview_color: Option<RgbaColor>,
    active_mouse_record_preset_id: Option<u32>,
    active_toolbox_preview_preset_id: Option<u32>,
    last_applied_theme: Option<UiThemeMode>,
    native_shadow_applied: bool,
}

impl CrosshairApp {
    pub fn new(
        paths: AppPaths,
        state: AppState,
        overlay_tx: Sender<OverlayCommand>,
        ui_rx: Receiver<UiCommand>,
    ) -> Self {
        let custom_assets = paths.list_custom_assets().unwrap_or_default();
        let open_windows = window_list::list_open_windows()
            .into_iter()
            .map(|item| item.selector)
            .collect();
        let save_name = state
            .selected_profile
            .clone()
            .unwrap_or_else(|| "Default".to_owned());
        let startup_clip_duration_ms = audio_duration(&state.audio_settings.startup);
        let exit_clip_duration_ms = audio_duration(&state.audio_settings.exit);
        let initial_active_panel = state.active_panel;

        let ready_status = match state.ui_language {
            UiLanguage::Vietnamese => crate::lang::translate(UiLanguage::Vietnamese, "Ready")
                .unwrap_or("Ready")
                .to_owned(),
            _ => "Ready".to_owned(),
        };

        let mut app = Self {
            paths,
            state,
            overlay_tx,
            ui_rx,
            status: ready_status,
            save_name,
            import_code_buffer: String::new(),
            export_code_buffer: String::new(),
            custom_assets,
            open_windows,
            quit_requested: false,
            capture_target: None,
            startup_clip_duration_ms,
            exit_clip_duration_ms,
            startup_sound_played: false,
            show_startup_audio_editor: false,
            show_exit_audio_editor: false,
            startup_sound_collapsed: true,
            exit_sound_collapsed: true,
            audio_waveforms: HashMap::new(),
            sound_preset_clip_duration_ms: HashMap::new(),
            show_sound_preset_audio_editor: HashSet::new(),
            library_clip_duration_ms: HashMap::new(),
            show_library_audio_editor: HashSet::new(),
            active_audio_editor: None,
            capture_ignored_keys: HashSet::new(),
            capture_suppress_next_poll: false,
            capture_wait_for_mouse_release: false,
            capture_ignore_mouse_until_release: false,
            capture_suppress_polls_remaining: 0,
            capture_mouse_guard_until: None,
            mouse_move_absolute_capture_target: None,
            mouse_move_absolute_capture_wait_for_mouse_release: false,
            mouse_move_absolute_capture_raise_window: false,
            mouse_move_absolute_restore_inner_size: None,
            mouse_move_absolute_restore_outer_pos: None,
            image_search_capture_active: false,
            image_search_capture_target: None,
            image_search_capture_mode: None,
            image_search_capture_anchor: None,
            image_search_capture_current: None,
            image_search_restore_inner_size: None,
            image_search_restore_outer_pos: None,
            selected_macro_steps: HashSet::new(),
            selected_macro_groups: HashSet::new(),
            macro_preset_search_query: String::new(),
            macro_group_clipboard: Vec::new(),
            macro_group_clipboard_is_cut: false,
            macro_preset_clipboard: None,
            macro_step_clipboard: Vec::new(),
            confirm_delete_folder_id: None,
            confirm_release_folder_id: None,
            confirm_delete_macro_group_id: None,
            center_window_next_frame: true,
            enforce_square_window_frames: 8,
            last_window_refresh_at: Instant::now(),
            last_active_panel: initial_active_panel,
            macro_drag_select_anchor: None,
            active_macro_folder_view: None,
            crosshair_panel_collapsed: true,
            close_to_tray_animation: None,
            open_from_tray_animation: None,
            startup_splash: StartupSplashState {
                started_at: None,
                duration_sec: 0.0,
            },
            hidden_window_inner_size: None,
            hidden_window_outer_pos: None,
            zoom_preview_cache: HashMap::new(),
            image_search_preview_cache: HashMap::new(),
            image_search_color_pick_texture: None,
            image_search_color_pick_preview_color: None,
            active_mouse_record_preset_id: None,
            active_toolbox_preview_preset_id: None,
            last_applied_theme: None,
            native_shadow_applied: false,
        };
        app.ensure_master_presets();
        for preset in &app.state.audio_settings.presets {
            if let Some(duration) = audio_duration(&preset.clip) {
                app.sound_preset_clip_duration_ms
                    .insert(preset.id, Some(duration));
            }
        }
        app.sync_crosshair();
        app.sync_window_presets();
        app.sync_mouse_sensitivity_presets();
        app.sync_mouse_driver_settings();
        app.sync_keyboard_arrow_mouse_settings();
        app.sync_profiles();
        app.sync_macro_presets();
        app.sync_audio_settings();
        app.sync_image_search_presets();
        app.sync_image_search_timing_presets();
        app.sync_toolbox_presets();
        app.sync_macro_master_enabled();
        app
    }

    fn sync_crosshair(&self) {
        let _ = self
            .overlay_tx
            .send(OverlayCommand::UpdateProfiles(self.state.profiles.clone()));
    }

    fn sync_window_presets(&self) {
        let _ = self.overlay_tx.send(OverlayCommand::UpdateWindowPresets(
            self.state.window_presets.clone(),
        ));
        let _ = self
            .overlay_tx
            .send(OverlayCommand::UpdateWindowFocusPresets(
                self.state.window_focus_presets.clone(),
            ));
        let _ = self.overlay_tx.send(OverlayCommand::UpdatePinPresets(
            self.state.pin_presets.clone(),
        ));
        let _ = self.overlay_tx.send(OverlayCommand::UpdateMousePathPresets(
            self.state.mouse_path_presets.clone(),
        ));
    }

    fn sync_mouse_sensitivity_presets(&self) {
        let _ = self
            .overlay_tx
            .send(OverlayCommand::UpdateMouseSensitivityPresets(
                self.state.mouse_sensitivity_presets.clone(),
            ));
    }

    fn sync_mouse_sensitivity_settings(&self) {
        let _ = self
            .overlay_tx
            .send(OverlayCommand::UpdateMouseSensitivitySettings {
                restore_on_exit: self.state.mouse_sensitivity_restore_on_exit,
                restore_speed: self.state.mouse_sensitivity_restore_speed,
            });
    }

    fn sync_mouse_driver_settings(&self) {
        let _ = self
            .overlay_tx
            .send(OverlayCommand::UpdateMouseDriverSettings(
                self.state.mouse_use_interception_driver,
            ));
    }

    fn sync_keyboard_arrow_mouse_settings(&self) {
        let _ = self
            .overlay_tx
            .send(OverlayCommand::UpdateKeyboardArrowMouseSettings {
                enabled: self.state.keyboard_arrow_mouse_enabled,
                step_px: self.state.keyboard_arrow_mouse_step_px,
            });
    }

    fn sync_image_search_presets(&self) {
        let preset_ids = self
            .state
            .image_search_presets
            .iter()
            .map(|preset| preset.id)
            .collect::<Vec<_>>();
        let _ = self
            .overlay_tx
            .send(OverlayCommand::InvalidateImageSearchWaits(preset_ids));
        let _ = self
            .overlay_tx
            .send(OverlayCommand::UpdateImageSearchPresets(
                self.state.image_search_presets.clone(),
            ));
    }

    fn sync_image_search_timing_presets(&self) {
        let preset_ids = self
            .state
            .image_search_timing_presets
            .iter()
            .map(|preset| preset.id)
            .collect::<Vec<_>>();
        let _ = self
            .overlay_tx
            .send(OverlayCommand::InvalidateImageSearchTimingWaits(preset_ids));
        let _ = self
            .overlay_tx
            .send(OverlayCommand::UpdateImageSearchTimingPresets(
                self.state.image_search_timing_presets.clone(),
            ));
    }

    fn sync_profiles(&self) {
        let _ = self
            .overlay_tx
            .send(OverlayCommand::UpdateProfiles(self.state.profiles.clone()));
    }

    fn sync_macro_presets(&self) {
        let mut macro_groups = self.state.macro_groups.clone();
        Self::sort_macro_groups(&mut macro_groups);
        let _ = self
            .overlay_tx
            .send(OverlayCommand::UpdateMacroPresets(macro_groups));
    }

    fn sync_macro_master_enabled(&self) {
        let _ = self.overlay_tx.send(OverlayCommand::SetMacrosMasterEnabled(
            self.state.macros_master_enabled,
        ));
    }

    fn sync_audio_settings(&self) {
        let _ = self.overlay_tx.send(OverlayCommand::UpdateAudioSettings(
            self.state.audio_settings.clone(),
        ));
    }

    fn sync_toolbox_presets(&self) {
        let _ = self.overlay_tx.send(OverlayCommand::UpdateToolboxPresets(
            self.state.toolbox_presets.clone(),
        ));
    }

    fn sync_toolbox_preview(&mut self, preset: Option<&ToolboxPreset>) {
        let next_id = preset.map(|preset| preset.id);
        if self.active_toolbox_preview_preset_id == next_id {
            if let Some(preset) = preset {
                let _ = self
                    .overlay_tx
                    .send(OverlayCommand::PreviewToolboxPreset(Some(preset.clone())));
            }
            return;
        }
        self.active_toolbox_preview_preset_id = next_id;
        let _ = self
            .overlay_tx
            .send(OverlayCommand::PreviewToolboxPreset(preset.cloned()));
    }

    fn clear_toolbox_preview(&mut self) {
        if self.active_toolbox_preview_preset_id.take().is_some() {
            let _ = self
                .overlay_tx
                .send(OverlayCommand::PreviewToolboxPreset(None));
        }
    }

    fn persist(&mut self) {
        if let Err(error) = self.paths.save_profiles(&self.state.profiles) {
            self.status = format!("Failed to save profiles: {error}");
            return;
        }
        if let Err(error) = self.paths.save_state(&self.state) {
            self.status = format!("Failed to save app state: {error}");
        }
    }

    fn save_profile(&mut self) {
        let name = self.save_name.trim().to_owned();
        if name.is_empty() {
            self.status = "Enter a profile name before saving.".to_owned();
            return;
        }
        if let Some(existing) = self
            .state
            .profiles
            .iter_mut()
            .find(|profile| profile.name == name)
        {
            existing.style = self.state.active_style.clone();
        } else {
            self.state.profiles.push(ProfileRecord {
                name: name.clone(),
                enabled: self.state.active_style.enabled,
                collapsed: true,
                style: self.state.active_style.clone(),
                target_window_title: None,
                extra_target_window_titles: Vec::new(),
            });
        }
        self.state
            .profiles
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        self.state.selected_profile = Some(name.clone());
        self.sync_profiles();
        self.persist();
        self.status = format!("Saved profile: {name}");
    }

    fn add_profile(&mut self) {
        let mut counter = self.state.profiles.len() + 1;
        let name = loop {
            let candidate = format!("Profile {counter}");
            if self
                .state
                .profiles
                .iter()
                .all(|profile| profile.name != candidate)
            {
                break candidate;
            }
            counter += 1;
        };
        self.state.profiles.push(ProfileRecord {
            name: name.clone(),
            enabled: self.state.active_style.enabled,
            collapsed: true,
            style: self.state.active_style.clone(),
            target_window_title: None,
            extra_target_window_titles: Vec::new(),
        });
        self.state.selected_profile = Some(name.clone());
        self.save_name = name.clone();
        self.sync_profiles();
        self.persist();
        self.status = format!("Added profile: {name}");
    }

    fn delete_profile(&mut self) {
        let Some(selected) = self.state.selected_profile.clone() else {
            self.status = "No profile is selected.".to_owned();
            return;
        };
        self.state
            .profiles
            .retain(|profile| profile.name != selected);
        if self.state.profiles.is_empty() {
            self.state.profiles.push(ProfileRecord {
                name: "Default".to_owned(),
                enabled: true,
                collapsed: true,
                style: CrosshairStyle::default(),
                target_window_title: None,
                extra_target_window_titles: Vec::new(),
            });
        }
        let next = self.state.profiles[0].clone();
        self.state.selected_profile = Some(next.name.clone());
        self.state.active_style = next.style;
        self.save_name = next.name;
        self.sync_crosshair();
        self.sync_profiles();
        self.persist();
        self.status = format!("Deleted profile: {selected}");
    }

    fn export_code(&mut self) {
        match profile_code::encode_style(&self.state.active_style) {
            Ok(code) => {
                self.export_code_buffer = code.clone();
                self.status = "Crosshair code copied to clipboard.".to_owned();
                if let Ok(mut clipboard) = Clipboard::new() {
                    let _ = clipboard.set_text(code);
                }
            }
            Err(error) => self.status = format!("Failed to export code: {error}"),
        }
    }

    fn import_code(&mut self) {
        match profile_code::decode_style(&self.import_code_buffer) {
            Ok(style) => {
                self.state.active_style = style;
                self.sync_crosshair();
                self.persist();
                self.status = "Imported crosshair code.".to_owned();
            }
            Err(error) => self.status = format!("Failed to import code: {error}"),
        }
    }

    fn reload_custom_assets(&mut self) {
        match self.paths.list_custom_assets() {
            Ok(assets) => {
                self.custom_assets = assets;
                self.status = "Reloaded custom crosshair folder.".to_owned();
            }
            Err(error) => self.status = format!("Failed to scan custom folder: {error}"),
        }
    }

    fn reload_open_windows(&mut self) {
        self.open_windows = window_list::list_open_windows()
            .into_iter()
            .map(|item| item.selector)
            .collect();
        self.last_window_refresh_at = Instant::now();
        self.status = "Reloaded open window list.".to_owned();
    }

    fn refresh_open_windows_now(&mut self) {
        self.open_windows = window_list::list_open_windows()
            .into_iter()
            .map(|item| item.selector)
            .collect();
        self.last_window_refresh_at = Instant::now();
    }

    fn window_preview_for_target(
        &mut self,
        ctx: &egui::Context,
        cache_id: u32,
        target_window_title: Option<&String>,
        extra_target_window_titles: &[String],
        match_duplicate_window_titles: bool,
    ) -> Option<ZoomPreviewView> {
        let refresh_every = Duration::from_millis(120);
        if let Some(cache) = self.zoom_preview_cache.get(&cache_id)
            && cache.source_window_key == target_window_title.cloned()
            && cache.source_window_extra_keys == extra_target_window_titles
            && cache.match_duplicate_window_titles == match_duplicate_window_titles
            && cache.updated_at.elapsed() < refresh_every
        {
            return Some(cache.view.clone());
        }

        let frame = window_list::capture_window_preview_with_candidates(
            target_window_title.map(|s| s.as_str()),
            extra_target_window_titles,
            match_duplicate_window_titles,
            720,
        )?;
        let image = ColorImage::from_rgba_unmultiplied([frame.width, frame.height], &frame.rgba);
        let view = if let Some(cache) = self.zoom_preview_cache.get_mut(&cache_id) {
            cache.view.texture.set(image, TextureOptions::LINEAR);
            cache.updated_at = Instant::now();
            cache.source_window_key = target_window_title.cloned();
            cache.source_window_extra_keys = extra_target_window_titles.to_vec();
            cache.match_duplicate_window_titles = match_duplicate_window_titles;
            cache.view.title = frame.title.clone();
            cache.view.screen_x = frame.screen_x;
            cache.view.screen_y = frame.screen_y;
            cache.view.logical_width = frame.logical_width;
            cache.view.logical_height = frame.logical_height;
            cache.view.clone()
        } else {
            let texture = ctx.load_texture(
                format!("window-preview-{cache_id}"),
                image,
                TextureOptions::LINEAR,
            );
            let view = ZoomPreviewView {
                texture,
                title: frame.title.clone(),
                screen_x: frame.screen_x,
                screen_y: frame.screen_y,
                logical_width: frame.logical_width,
                logical_height: frame.logical_height,
            };
            self.zoom_preview_cache.insert(
                cache_id,
                ZoomPreviewCache {
                    updated_at: Instant::now(),
                    source_window_key: target_window_title.cloned(),
                    source_window_extra_keys: extra_target_window_titles.to_vec(),
                    match_duplicate_window_titles,
                    view: view.clone(),
                },
            );
            view
        };
        Some(view)
    }

    fn zoom_preview_for_preset(
        &mut self,
        ctx: &egui::Context,
        preset: &ZoomPreset,
    ) -> Option<ZoomPreviewView> {
        self.window_preview_for_target(
            ctx,
            preset.id,
            preset.target_window_title.as_ref(),
            &preset.extra_target_window_titles,
            false,
        )
    }

    fn image_search_preview_for_preset(
        &mut self,
        ctx: &egui::Context,
        preset: &ImageSearchPreset,
    ) -> Option<ImageSearchPreviewView> {
        let file_path = self.image_search_template_file_for_preset(preset.id);
        let metadata = fs::metadata(&file_path).ok();
        let modified = metadata.and_then(|meta| meta.modified().ok());
        if let Some(cache) = self.image_search_preview_cache.get(&preset.id)
            && cache.source_path == file_path
            && cache.source_modified == modified
            && cache.updated_at.elapsed() < Duration::from_millis(250)
        {
            return Some(cache.view.clone());
        }

        let image = image::open(&file_path).ok()?.to_rgba8();
        let width = image.width() as usize;
        let height = image.height() as usize;
        let color_image = ColorImage::from_rgba_unmultiplied([width, height], image.as_raw());
        let file_name = file_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("template.png")
            .to_owned();

        let view = if let Some(cache) = self.image_search_preview_cache.get_mut(&preset.id) {
            cache.view.texture.set(color_image, TextureOptions::NEAREST);
            cache.updated_at = Instant::now();
            cache.source_path = file_path.clone();
            cache.source_modified = modified;
            cache.view.file_name = file_name.clone();
            cache.view.width = width;
            cache.view.height = height;
            cache.view.clone()
        } else {
            let texture = ctx.load_texture(
                format!("image-search-preview-{}", preset.id),
                color_image,
                TextureOptions::NEAREST,
            );
            let view = ImageSearchPreviewView {
                texture,
                file_name,
                width,
                height,
            };
            self.image_search_preview_cache.insert(
                preset.id,
                ImageSearchPreviewCache {
                    updated_at: Instant::now(),
                    source_path: file_path,
                    source_modified: modified,
                    view: view.clone(),
                },
            );
            view
        };
        Some(view)
    }

    fn image_search_search_area_text(preset: &ImageSearchPreset) -> String {
        match (
            preset.search_region_screen_x,
            preset.search_region_screen_y,
            preset.search_region_width,
            preset.search_region_height,
        ) {
            (Some(x), Some(y), Some(width), Some(height)) if width > 0 && height > 0 => {
                let shape = if preset.search_region_is_circle {
                    "Circle"
                } else {
                    "Rect"
                };
                format!("{shape} {x}, {y}  {width}x{height}")
            }
            _ => "Any screen".to_owned(),
        }
    }

    fn image_search_target_color_text(preset: &ImageSearchPreset) -> String {
        let colors = Self::image_search_target_colors(preset);
        match colors.as_slice() {
            [] => "None".to_owned(),
            [color] => format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b),
            [first, rest @ ..] => format!(
                "#{:02X}{:02X}{:02X} +{}",
                first.r,
                first.g,
                first.b,
                rest.len()
            ),
        }
    }

    fn image_search_timing_color_text(preset: &ImageSearchTimingPreset) -> String {
        let colors = if !preset.target_colors.is_empty() {
            preset.target_colors.clone()
        } else {
            preset.target_color.into_iter().collect()
        };
        match colors.as_slice() {
            [] => "None".to_owned(),
            [color] => format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b),
            [first, rest @ ..] => format!(
                "#{:02X}{:02X}{:02X} +{}",
                first.r,
                first.g,
                first.b,
                rest.len()
            ),
        }
    }

    fn image_search_timing_area_text(preset: &ImageSearchTimingPreset) -> String {
        match (
            preset.search_region_screen_x,
            preset.search_region_screen_y,
            preset.search_region_width,
            preset.search_region_height,
        ) {
            (Some(x), Some(y), Some(width), Some(height)) if width > 0 && height > 0 => {
                let shape = if preset.search_region_is_circle {
                    "Circle"
                } else {
                    "Rect"
                };
                format!("{shape} {x}, {y}  {width}x{height}")
            }
            _ => "Any screen".to_owned(),
        }
    }

    fn image_search_timing_preset_text(preset: &ImageSearchTimingPreset) -> String {
        let colors = if !preset.target_colors.is_empty() {
            preset.target_colors.clone()
        } else {
            preset.target_color.into_iter().collect()
        };
        let color_text = match colors.as_slice() {
            [] => "No color".to_owned(),
            [color] => format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b),
            [first, rest @ ..] => format!(
                "#{:02X}{:02X}{:02X} +{}",
                first.r,
                first.g,
                first.b,
                rest.len()
            ),
        };
        let area = if preset
            .search_region_screen_x
            .zip(preset.search_region_screen_y)
            .zip(preset.search_region_width)
            .zip(preset.search_region_height)
            .is_some()
        {
            "Area set"
        } else {
            "No area"
        };
        let loop_text = if preset.loop_enabled {
            if preset.loop_forever {
                "Loop forever".to_owned()
            } else {
                format!("Loop {} s", preset.loop_duration_secs.max(1))
            }
        } else {
            "One-shot".to_owned()
        };
        format!(
            "{area} | {color_text} | {} ms | {loop_text}",
            preset.timing_cycle_ms.max(1)
        )
    }

    fn image_search_timing_preset_options(&self) -> Vec<(u32, String)> {
        self.state
            .image_search_timing_presets
            .iter()
            .map(|preset| (preset.id, preset.name.clone()))
            .collect()
    }

    fn image_search_timing_preset_label(
        options: &[(u32, String)],
        selected_id: Option<u32>,
        empty_label: &'static str,
    ) -> String {
        selected_id
            .and_then(|id| {
                options
                    .iter()
                    .find(|(preset_id, _)| *preset_id == id)
                    .map(|(_, label)| label.clone())
            })
            .unwrap_or_else(|| empty_label.to_owned())
    }

    fn image_search_target_colors(preset: &ImageSearchPreset) -> Vec<RgbaColor> {
        if !preset.target_colors.is_empty() {
            return preset.target_colors.clone();
        }
        preset.target_color.into_iter().collect()
    }

    fn image_search_target_color_swatch(ui: &mut egui::Ui, color: Option<RgbaColor>) {
        let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
        let fill = color.map_or(Color32::from_rgba_premultiplied(42, 48, 56, 220), |color| {
            Color32::from_rgba_unmultiplied(color.r, color.g, color.b, 255)
        });
        ui.painter().rect_filled(rect, 4.0, fill);
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0, Color32::from_rgb(160, 174, 196)),
            egui::StrokeKind::Outside,
        );
    }

    fn update_image_search_cursor_preview(
        &mut self,
        ctx: &egui::Context,
        pointer: egui::Pos2,
        sample_size: i32,
    ) -> Option<RgbaColor> {
        let (screen_x, screen_y) =
            self.screen_point_from_pos(ctx, pointer, ctx.pixels_per_point())?;
        let sample_size = sample_size.max(3) | 1;
        let left = screen_x - sample_size / 2;
        let top = screen_y - sample_size / 2;
        let capture =
            window_list::capture_virtual_screen_region(left, top, sample_size, sample_size)?;
        if capture.rgba.len() < 4 {
            return None;
        }

        let center_index = (((capture.height / 2) * capture.width) + (capture.width / 2)) * 4;
        if center_index + 3 >= capture.rgba.len() {
            return None;
        }
        let sampled = RgbaColor {
            r: capture.rgba[center_index],
            g: capture.rgba[center_index + 1],
            b: capture.rgba[center_index + 2],
            a: 255,
        };
        let color_image =
            ColorImage::from_rgba_unmultiplied([capture.width, capture.height], &capture.rgba);
        if let Some(texture) = self.image_search_color_pick_texture.as_mut() {
            texture.set(color_image, TextureOptions::NEAREST);
        } else {
            self.image_search_color_pick_texture = Some(ctx.load_texture(
                "image-search-color-pick-preview",
                color_image,
                TextureOptions::NEAREST,
            ));
        }
        self.image_search_color_pick_preview_color = Some(sampled);
        Some(sampled)
    }

    fn image_search_preview_panel_rect(
        viewport_rect: egui::Rect,
        pointer: egui::Pos2,
        panel_size: egui::Vec2,
    ) -> egui::Rect {
        let margin = 18.0;
        let candidates = [
            egui::Rect::from_min_size(
                viewport_rect.right_top() - vec2(panel_size.x + margin, -margin),
                panel_size,
            ),
            egui::Rect::from_min_size(viewport_rect.left_top() + vec2(margin, margin), panel_size),
            egui::Rect::from_min_size(
                viewport_rect.right_bottom() - vec2(panel_size.x + margin, panel_size.y + margin),
                panel_size,
            ),
            egui::Rect::from_min_size(
                viewport_rect.left_bottom() + vec2(margin, -(panel_size.y + margin)),
                panel_size,
            ),
        ];
        let pointer_safe_zone = egui::Rect::from_center_size(pointer, vec2(54.0, 54.0));
        candidates
            .into_iter()
            .find(|candidate| !candidate.intersects(pointer_safe_zone))
            .unwrap_or(candidates[0])
    }

    fn render_image_search_cursor_preview_panel(
        &self,
        painter: &egui::Painter,
        viewport_rect: egui::Rect,
        pointer: egui::Pos2,
        sampled_color: Option<RgbaColor>,
        screen_point: Option<(i32, i32)>,
    ) {
        let Some(texture) = self.image_search_color_pick_texture.as_ref() else {
            return;
        };
        let panel_size = vec2(188.0, 236.0);
        let panel_rect = Self::image_search_preview_panel_rect(viewport_rect, pointer, panel_size);
        painter.rect_filled(
            panel_rect,
            10.0,
            Color32::from_rgba_premultiplied(12, 18, 28, 228),
        );
        painter.rect_stroke(
            panel_rect,
            10.0,
            egui::Stroke::new(1.0, Color32::from_rgb(110, 156, 210)),
            egui::StrokeKind::Outside,
        );
        let preview_rect =
            egui::Rect::from_min_size(panel_rect.min + vec2(12.0, 12.0), vec2(144.0, 144.0));
        painter.image(
            texture.id(),
            preview_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        painter.rect_stroke(
            preview_rect,
            6.0,
            egui::Stroke::new(1.0, Color32::from_rgb(146, 192, 248)),
            egui::StrokeKind::Outside,
        );
        let cell_size = preview_rect.width() / 17.0;
        let center_rect =
            egui::Rect::from_center_size(preview_rect.center(), vec2(cell_size, cell_size));
        painter.rect_stroke(
            center_rect,
            0.0,
            egui::Stroke::new(2.0, Color32::from_rgb(120, 220, 255)),
            egui::StrokeKind::Outside,
        );

        if let Some(color) = sampled_color.or(self.image_search_color_pick_preview_color) {
            let swatch_rect =
                egui::Rect::from_min_size(panel_rect.min + vec2(12.0, 166.0), vec2(24.0, 24.0));
            painter.rect_filled(
                swatch_rect,
                6.0,
                Color32::from_rgb(color.r, color.g, color.b),
            );
            painter.rect_stroke(
                swatch_rect,
                6.0,
                egui::Stroke::new(1.0, Color32::WHITE),
                egui::StrokeKind::Outside,
            );
            painter.text(
                swatch_rect.right_center() + vec2(10.0, -8.0),
                egui::Align2::LEFT_TOP,
                format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b),
                egui::FontId::proportional(15.0),
                Color32::WHITE,
            );
        }
        if let Some((screen_x, screen_y)) = screen_point {
            painter.text(
                panel_rect.min + vec2(12.0, 198.0),
                egui::Align2::LEFT_TOP,
                format!("X:{screen_x}  Y:{screen_y}"),
                egui::FontId::proportional(12.0),
                Color32::from_rgb(188, 206, 230),
            );
        }
        painter.text(
            panel_rect.min + vec2(12.0, 214.0),
            egui::Align2::LEFT_TOP,
            "Center pixel",
            egui::FontId::proportional(12.0),
            Color32::from_rgb(188, 206, 230),
        );
    }

    #[cfg(windows)]
    fn precise_image_search_capture_pointer(&self, ctx: &egui::Context) -> Option<egui::Pos2> {
        let mut point = POINT::default();
        unsafe {
            if GetCursorPos(&mut point).is_err() {
                return None;
            }
        }
        let scale = ctx.pixels_per_point().max(0.5);
        let viewport_min = ctx
            .input(|input| input.viewport().inner_rect.map(|viewport| viewport.min))
            .unwrap_or_else(|| {
                let (left, top, _width, _height) = window_list::virtual_screen_bounds();
                egui::pos2(left as f32 / scale, top as f32 / scale)
            });
        Some(egui::pos2(
            point.x as f32 / scale - viewport_min.x,
            point.y as f32 / scale - viewport_min.y,
        ))
    }

    #[cfg(not(windows))]
    fn precise_image_search_capture_pointer(&self, _ctx: &egui::Context) -> Option<egui::Pos2> {
        None
    }

    fn clear_pin_preview_cache(&mut self) {
        for preset in &self.state.pin_presets {
            self.zoom_preview_cache.remove(&(100_000 + preset.id));
        }
    }

    fn play_startup_sound_once(&mut self) {
        if self.startup_sound_played {
            return;
        }
        self.startup_sound_played = true;
        if self.state.audio_settings.startup.enabled {
            audio::play_clip_async(self.state.audio_settings.startup.clone());
        }
    }

    fn open_audio_editor(&mut self, target: AudioEditorTarget) {
        self.active_audio_editor = Some(target);
        self.state.active_panel = AppPanel::Media;
    }

    fn close_audio_editor(&mut self) {
        self.active_audio_editor = None;
        self.state.active_panel = AppPanel::Sound;
        audio::stop_preview();
    }

    fn capture_is_active(&self, target: &CaptureRequest) -> bool {
        self.capture_target.as_ref() == Some(target)
    }

    #[cfg(windows)]
    fn current_mouse_speed() -> Option<u32> {
        let mut speed = 10u32;
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{
                SPI_GETMOUSESPEED, SystemParametersInfoW,
            };
            SystemParametersInfoW(
                SPI_GETMOUSESPEED,
                0,
                Some((&mut speed as *mut u32).cast()),
                Default::default(),
            )
            .ok()?;
        }
        Some(speed.clamp(1, 20))
    }

    #[cfg(not(windows))]
    fn current_mouse_speed() -> Option<u32> {
        None
    }

    fn mouse_interception_driver_downloaded(&self) -> bool {
        self.paths.interception_installer_exe.exists()
    }

    #[cfg(windows)]
    fn mouse_interception_driver_installed(&self) -> bool {
        let windows_dir = std::env::var_os("WINDIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"));
        let driver_dir = windows_dir.join("System32").join("drivers");
        driver_dir.join("keyboard.sys").exists() && driver_dir.join("mouse.sys").exists()
    }

    #[cfg(not(windows))]
    fn mouse_interception_driver_installed(&self) -> bool {
        false
    }

    #[cfg(windows)]
    fn download_and_install_interception_driver(&mut self) -> anyhow::Result<String> {
        fs::create_dir_all(&self.paths.interception_dir)?;
        let archive = reqwest::blocking::get(INTERCEPTION_RELEASE_URL)?
            .error_for_status()?
            .bytes()?;
        fs::write(&self.paths.interception_zip_file, &archive)?;

        if self.paths.interception_extract_dir.exists() {
            fs::remove_dir_all(&self.paths.interception_extract_dir)?;
        }
        fs::create_dir_all(&self.paths.interception_dir)?;

        let zip = Self::powershell_quote(&self.paths.interception_zip_file.to_string_lossy());
        let extract =
            Self::powershell_quote(&self.paths.interception_extract_dir.to_string_lossy());
        let expand_status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &format!("Expand-Archive -LiteralPath '{zip}' -DestinationPath '{extract}' -Force"),
            ])
            .status()?;
        if !expand_status.success() {
            anyhow::bail!("Failed to extract Interception.zip");
        }
        if !self.paths.interception_installer_exe.exists() {
            anyhow::bail!("install-interception.exe was not found after extraction");
        }

        let install_status = Command::new(&self.paths.interception_installer_exe)
            .arg("/install")
            .current_dir(&self.paths.interception_installer_dir)
            .status()?;
        if !install_status.success() {
            anyhow::bail!("install-interception.exe /install failed");
        }

        Ok(match self.state.ui_language {
            UiLanguage::Vietnamese => {
                "Đã tải và cài Interception driver. Nếu Windows chưa nhận ngay, hãy khởi động lại máy.".to_owned()
            }
            _ => "Interception installed. Restart Windows if needed.".to_owned(),
        })
    }

    #[cfg(not(windows))]
    fn download_and_install_interception_driver(&mut self) -> anyhow::Result<String> {
        anyhow::bail!("Interception is supported on Windows only")
    }

    #[cfg(windows)]
    fn uninstall_and_remove_interception_driver(&mut self) -> anyhow::Result<String> {
        if self.paths.interception_installer_exe.exists() {
            let uninstall_status = Command::new(&self.paths.interception_installer_exe)
                .arg("/uninstall")
                .current_dir(&self.paths.interception_installer_dir)
                .status()?;
            if !uninstall_status.success() {
                anyhow::bail!("install-interception.exe /uninstall failed");
            }
        }

        if self.paths.interception_dir.exists() {
            fs::remove_dir_all(&self.paths.interception_dir)?;
        }
        fs::create_dir_all(&self.paths.interception_dir)?;

        Ok(match self.state.ui_language {
            UiLanguage::Vietnamese => {
                "Đã gỡ Interception driver và xóa bộ cài đã tải. Có thể cần khởi động lại Windows để gỡ hẳn.".to_owned()
            }
            _ => "Removed Interception and deleted the package. Restart Windows if needed.".to_owned(),
        })
    }

    #[cfg(not(windows))]
    fn uninstall_and_remove_interception_driver(&mut self) -> anyhow::Result<String> {
        anyhow::bail!("Interception is supported on Windows only")
    }

    fn powershell_quote(value: &str) -> String {
        value.replace('\'', "''")
    }

    fn choose_audio_file(&mut self, startup: bool) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Audio", &["mp3", "wav", "flac", "ogg", "m4a"])
            .pick_file()
        else {
            return;
        };
        let path_str = path.to_string_lossy().to_string();
        let duration = audio::load_duration_ms(&path_str).ok();
        let clip = if startup {
            &mut self.state.audio_settings.startup
        } else {
            &mut self.state.audio_settings.exit
        };
        clip.file_path = path_str;
        clip.start_ms = 0;
        clip.end_ms = duration.unwrap_or(0);
        if startup {
            self.startup_clip_duration_ms = duration;
            self.show_startup_audio_editor = true;
        } else {
            self.exit_clip_duration_ms = duration;
            self.show_exit_audio_editor = true;
        }
        self.refresh_audio_waveform(startup);
        self.sync_audio_settings();
        self.persist();
    }

    fn choose_audio_file_for_sound_preset(&mut self, preset_id: u32) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Audio", &["mp3", "wav", "flac", "ogg", "m4a"])
            .pick_file()
        else {
            return;
        };
        let path_str = path.to_string_lossy().to_string();
        let duration = audio::load_duration_ms(&path_str).ok();
        if let Some(preset) = self
            .state
            .audio_settings
            .presets
            .iter_mut()
            .find(|preset| preset.id == preset_id)
        {
            preset.clip.file_path = path_str.clone();
            preset.clip.start_ms = 0;
            preset.clip.end_ms = duration.unwrap_or(0);
            preset.clip.enabled = true;
            self.sound_preset_clip_duration_ms
                .insert(preset_id, duration);
            self.show_sound_preset_audio_editor.insert(preset_id);
            self.refresh_audio_waveform_for_path(&path_str);
            self.sync_audio_settings();
            self.persist();
        }
    }

    fn choose_audio_file_for_library_item(&mut self, item_id: u32) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Audio", &["mp3", "wav", "flac", "ogg", "m4a"])
            .pick_file()
        else {
            return;
        };
        let path_str = path.to_string_lossy().to_string();
        let duration = audio::load_duration_ms(&path_str).ok();
        if let Some(item) = self
            .state
            .audio_settings
            .library
            .iter_mut()
            .find(|item| item.id == item_id)
        {
            item.clip.file_path = path_str.clone();
            item.clip.start_ms = 0;
            item.clip.end_ms = duration.unwrap_or(0);
            item.clip.enabled = true;
            self.library_clip_duration_ms.insert(item_id, duration);
            self.show_library_audio_editor.insert(item_id);
            self.refresh_audio_waveform_for_path(&path_str);
            self.sync_audio_settings();
            self.persist();
        }
    }

    fn save_clip_to_library(&mut self, name_prefix: &str, clip: &AudioClipSettings) {
        if clip.file_path.trim().is_empty() {
            self.status = "Choose a sound file before saving it to the library.".to_owned();
            return;
        }
        let id = self.state.audio_settings.next_library_item_id.max(1);
        self.state.audio_settings.next_library_item_id = id + 1;
        let mut item = SoundLibraryItem::new(id);
        item.name = format!("{name_prefix} {id}");
        item.clip = clip.clone();
        item.clip.enabled = true;
        self.state.audio_settings.library.push(item);
        self.library_clip_duration_ms
            .insert(id, audio_duration(clip));
        self.show_library_audio_editor.insert(id);
        self.sync_audio_settings();
        self.persist();
        self.status = format!("Saved sound into library item {id}.");
    }

    fn refresh_audio_waveform(&mut self, startup: bool) {
        let clip = if startup {
            &self.state.audio_settings.startup
        } else {
            &self.state.audio_settings.exit
        };
        let path = clip.file_path.trim();
        if path.is_empty() {
            return;
        }
        if self.audio_waveforms.contains_key(path) {
            return;
        }
        if let Ok(waveform) = audio::load_waveform(path, 320) {
            self.audio_waveforms.insert(path.to_owned(), waveform);
        }
    }

    fn refresh_audio_waveform_for_path(&mut self, path: &str) {
        let trimmed = path.trim();
        if trimmed.is_empty() || self.audio_waveforms.contains_key(trimmed) {
            return;
        }
        if let Ok(waveform) = audio::load_waveform(trimmed, 320) {
            self.audio_waveforms.insert(trimmed.to_owned(), waveform);
        }
    }

    fn trim_audio_bounds(clip: &mut AudioClipSettings, total_ms: u64) {
        clip.start_ms = clip.start_ms.min(total_ms);
        clip.end_ms = if clip.end_ms == 0 {
            total_ms
        } else {
            clip.end_ms.min(total_ms)
        };
        if clip.end_ms < clip.start_ms {
            clip.end_ms = clip.start_ms;
        }
        clip.volume = clip.volume.clamp(0.0, 2.0);
    }

    fn format_ms(ms: u64) -> String {
        format!("{:.2}s", ms as f64 / 1000.0)
    }

    fn preset_frame(ui: &egui::Ui, enabled: bool) -> egui::Frame {
        let fill = if enabled {
            Color32::from_rgba_premultiplied(32, 92, 52, 120)
        } else {
            ui.visuals().faint_bg_color
        };
        let stroke_color = if enabled {
            Color32::from_rgb(108, 224, 148)
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke.color
        };
        egui::Frame::group(ui.style())
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke_color))
    }

    fn preset_body_text_color(dark_mode: bool, enabled: bool) -> Color32 {
        match (dark_mode, enabled) {
            (true, true) => Color32::from_rgb(248, 250, 252),
            (true, false) => Color32::from_rgb(214, 222, 232),
            (false, true) => Color32::from_rgb(250, 250, 250),
            (false, false) => Color32::from_rgb(32, 32, 32),
        }
    }

    fn show_preset_card<R>(
        ui: &mut egui::Ui,
        enabled: bool,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let dark_mode = ui.visuals().dark_mode;
        Self::preset_frame(ui, enabled)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                let previous = ui.visuals().override_text_color;
                if dark_mode {
                    ui.visuals_mut().override_text_color =
                        Some(Self::preset_body_text_color(dark_mode, enabled));
                }
                let output = add_contents(ui);
                ui.visuals_mut().override_text_color = previous;
                output
            })
            .inner
    }

    fn preset_title_text(dark_mode: bool, name: &str, enabled: bool) -> RichText {
        let text = RichText::new(name).strong();
        text.color(Self::preset_body_text_color(dark_mode, enabled))
    }

    fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
        if needle.is_empty() {
            return true;
        }
        haystack.to_lowercase().contains(&needle.to_lowercase())
    }

    fn sort_macro_groups(groups: &mut [MacroGroup]) {
        groups.sort_by(|left, right| {
            right
                .favorite
                .cmp(&left.favorite)
                .then(right.id.cmp(&left.id))
        });
    }

    fn macro_preset_matches_search_query(
        group: &MacroGroup,
        preset: &MacroPreset,
        query: &str,
    ) -> bool {
        if query.trim().is_empty() {
            return true;
        }
        let query = query.trim();
        Self::contains_case_insensitive(&group.name, query)
            || Self::contains_case_insensitive(
                &hotkey::format_binding(preset.hotkey.as_ref()),
                query,
            )
    }

    fn macro_group_matches_search_query(group: &MacroGroup, query: &str) -> bool {
        if query.trim().is_empty() {
            return true;
        }
        let query = query.trim();
        Self::contains_case_insensitive(&group.name, query)
            || group
                .presets
                .iter()
                .any(|preset| Self::macro_preset_matches_search_query(group, preset, query))
    }

    fn desired_window_size() -> egui::Vec2 {
        vec2(980.0, 980.0)
    }

    #[cfg(windows)]
    fn screen_size() -> egui::Vec2 {
        vec2(unsafe { GetSystemMetrics(SM_CXSCREEN) } as f32, unsafe {
            GetSystemMetrics(SM_CYSCREEN)
        }
            as f32)
    }

    #[cfg(not(windows))]
    fn screen_size() -> egui::Vec2 {
        vec2(1920.0, 1080.0)
    }

    fn square_window_size(size: egui::Vec2) -> egui::Vec2 {
        let edge = size.x.max(size.y).max(900.0);
        vec2(edge, edge)
    }

    #[cfg(windows)]
    fn centered_outer_position_for_size(size: egui::Vec2) -> egui::Pos2 {
        let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) } as f32;
        let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) } as f32;
        egui::pos2(
            ((screen_w - size.x) * 0.5).round(),
            ((screen_h - size.y) * 0.5).round(),
        )
    }

    #[cfg(not(windows))]
    fn centered_outer_position_for_size(_size: egui::Vec2) -> egui::Pos2 {
        egui::pos2(120.0, 120.0)
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        if self.last_applied_theme == Some(self.state.ui_theme) {
            return;
        }

        match self.state.ui_theme {
            UiThemeMode::Dark => {
                let mut visuals = egui::Visuals::dark();
                visuals.override_text_color = Some(Color32::from_rgb(232, 238, 248));
                visuals.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(220, 228, 238);
                visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(228, 234, 242);
                visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(240, 246, 252);
                visuals.widgets.active.fg_stroke.color = Color32::from_rgb(248, 250, 252);
                visuals.widgets.open.fg_stroke.color = Color32::from_rgb(240, 246, 252);
                ctx.set_visuals(visuals);
                ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark));
            }
            UiThemeMode::Light => {
                let mut visuals = egui::Visuals::light();
                visuals.override_text_color = Some(Color32::from_rgb(28, 34, 44));
                visuals.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(32, 40, 54);
                visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(28, 36, 48);
                visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(18, 26, 40);
                visuals.widgets.active.fg_stroke.color = Color32::from_rgb(16, 24, 38);
                visuals.widgets.open.fg_stroke.color = Color32::from_rgb(18, 26, 40);
                visuals.hyperlink_color = Color32::from_rgb(26, 92, 164);
                ctx.set_visuals(visuals);
                ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Light));
            }
        }

        self.last_applied_theme = Some(self.state.ui_theme);
    }

    fn cycle_language(&mut self) {
        self.state.ui_language = match self.state.ui_language {
            UiLanguage::English => UiLanguage::Vietnamese,
            UiLanguage::Vietnamese => UiLanguage::English,
            UiLanguage::Icon => UiLanguage::English,
        };
        self.persist();
    }

    fn toggle_theme_mode(&mut self) {
        self.state.ui_theme = match self.state.ui_theme {
            UiThemeMode::Dark => UiThemeMode::Light,
            UiThemeMode::Light => UiThemeMode::Dark,
        };
        self.persist();
    }

    fn tr(&self, english: &'static str, vietnamese: &'static str) -> &'static str {
        Self::tr_lang(self.state.ui_language, english, vietnamese)
    }

    fn normalize_vietnamese(text: &'static str) -> &'static str {
        text
    }

    fn tr_lang(
        language: UiLanguage,
        english: &'static str,
        _vietnamese: &'static str,
    ) -> &'static str {
        match language {
            UiLanguage::Vietnamese => Self::normalize_vietnamese(
                crate::lang::translate(language, english).unwrap_or(english),
            ),
            UiLanguage::English | UiLanguage::Icon => english,
        }
    }

    fn format_binding_ui(language: UiLanguage, binding: Option<&HotkeyBinding>) -> String {
        let label = hotkey::format_binding(binding);
        if label == "Not set" {
            Self::tr_lang(
                language,
                "Not set",
                "Chưa đặt",
            )
            .to_owned()
        } else {
            label
        }
    }

    fn format_macro_trigger_ui(language: UiLanguage, preset: &MacroPreset) -> String {
        let label = if preset.trigger_keys.trim().is_empty() {
            hotkey::format_binding(preset.hotkey.as_ref())
        } else {
            hotkey::format_key_list(&preset.trigger_keys)
        };
        if label == "Not set" {
            Self::tr_lang(
                language,
                "Not set",
                "Chưa đặt",
            )
            .to_owned()
        } else {
            label
        }
    }

    fn pop_key_list_entry(spec: &mut String) -> bool {
        let mut keys = hotkey::split_key_list(spec);
        let Some(_) = keys.pop() else {
            return false;
        };
        *spec = keys.join(", ");
        true
    }

    fn short_key_chip_label(key: &str) -> String {
        match key.trim().to_ascii_uppercase().as_str() {
            "MOUSELEFT" => "LClick".to_owned(),
            "MOUSERIGHT" => "RClick".to_owned(),
            "MOUSEMIDDLE" => "MClick".to_owned(),
            "MOUSEX1" => "X1".to_owned(),
            "MOUSEX2" => "X2".to_owned(),
            "MOUSEWHEELUP" => "WheelUp".to_owned(),
            "MOUSEWHEELDOWN" => "WheelDn".to_owned(),
            "ESCAPE" => "Esc".to_owned(),
            "BACKSPACE" => "Bksp".to_owned(),
            "PAGEUP" => "PgUp".to_owned(),
            "PAGEDOWN" => "PgDn".to_owned(),
            "CONTROL" => "Ctrl".to_owned(),
            "WINDOWS" | "WIN" => "Win".to_owned(),
            other => other.to_owned(),
        }
    }

    fn render_key_list_chips(
        ui: &mut egui::Ui,
        language: UiLanguage,
        spec: &mut String,
        empty_text: &str,
    ) -> bool {
        let keys = hotkey::split_key_list(spec);
        if keys.is_empty() {
            ui.label(empty_text);
            return false;
        }

        let mut remove_index = None;
        ui.horizontal_wrapped(|ui| {
            for (index, key) in keys.iter().enumerate() {
                let label = Self::short_key_chip_label(key);
                if ui
                    .add(Button::new(RichText::new(label).monospace()).min_size(vec2(0.0, 22.0)))
                    .on_hover_text(Self::tr_lang(
                        language,
                        "Click to remove this key",
                        "Click to remove this key",
                    ))
                    .clicked()
                {
                    remove_index = Some(index);
                }
            }
        });

        if let Some(index) = remove_index {
            let mut next_keys = keys;
            next_keys.remove(index);
            *spec = next_keys.join(", ");
            true
        } else {
            false
        }
    }

    fn app_brand_title(&self) -> &'static str {
        "MacroNest"
    }

    fn app_version_label(&self) -> &'static str {
        option_env!("MACRONEST_BUILD_TAG").unwrap_or(env!("CARGO_PKG_VERSION"))
    }

    fn app_brand_subtitle(&self) -> &'static str {
        match self.state.ui_language {
            UiLanguage::English => "Macro control, pin, toolbox, sound, and window tools",
            UiLanguage::Vietnamese => self.tr(
                "Macro control, pin, toolbox, sound, and window tools",
                "Macro control, pin, toolbox, sound, and window tools",
            ),
            UiLanguage::Icon => "Macro control, pin, toolbox, sound, and window tools",
        }
    }

    fn panel_icon(panel: AppPanel) -> u32 {
        match panel {
            AppPanel::Crosshair => 0xe3dc,
            AppPanel::WindowPresets => 0xe8f0,
            AppPanel::Pin | AppPanel::Zoom => 0xe55f,
            AppPanel::Mouse => 0xe323,
            AppPanel::ImageSearch => 0xe8b6,
            AppPanel::Macros | AppPanel::Modes => 0xe312,
            AppPanel::Sound | AppPanel::Media => 0xe050,
            AppPanel::Settings => 0xe8b8,
        }
    }

    fn panel_label(&self, panel: AppPanel) -> &'static str {
        let english = match panel {
            AppPanel::Crosshair => "Crosshair",
            AppPanel::WindowPresets => "Window Control",
            AppPanel::Pin | AppPanel::Zoom => "Pin",
            AppPanel::Mouse => "Mouse",
            AppPanel::ImageSearch => "Image Search",
            AppPanel::Macros | AppPanel::Modes => "Macro",
            AppPanel::Sound => "Sound",
            AppPanel::Media => "Media",
            AppPanel::Settings => "Toolbox",
        };
        Self::tr_lang(self.state.ui_language, english, english)
    }

    fn language_button_text(&self) -> RichText {
        match self.state.ui_language {
            UiLanguage::English => RichText::new("EN").strong(),
            UiLanguage::Vietnamese => RichText::new("VI").strong(),
            UiLanguage::Icon => RichText::new("EN").strong(),
        }
    }

    fn theme_button_text(&self) -> RichText {
        match self.state.ui_theme {
            UiThemeMode::Dark => Self::material_icon_text(0xe51c, 18.0),
            UiThemeMode::Light => Self::material_icon_text(0xe518, 18.0),
        }
    }

    fn startup_loading_text(&self) -> &'static str {
        match self.state.ui_language {
            UiLanguage::English => "loading macro tools, overlays, and UI",
            UiLanguage::Vietnamese => self.tr(
                "loading macro tools, overlays, and UI",
                "loading macro tools, overlays, and UI",
            ),
            UiLanguage::Icon => "loading macro tools, overlays, and UI",
        }
    }

    fn titlebar_language_tooltip(&self) -> &'static str {
        self.tr(
            "Switch language",
            "Đổi ngôn ngữ",
        )
    }

    fn titlebar_theme_tooltip(&self) -> &'static str {
        self.tr(
            "Toggle dark / light theme",
            "Đổi giao diện sáng / tối",
        )
    }

    fn titlebar_minimize_tooltip(&self) -> &'static str {
        self.tr("Minimize", "Minimize")
    }

    fn titlebar_maximize_tooltip(&self, maximized: bool) -> &'static str {
        if maximized {
            self.tr(
                "Restore",
                "Khôi phục",
            )
        } else {
            self.tr("Maximize", "Maximize")
        }
    }

    fn titlebar_hide_tooltip(&self) -> &'static str {
        self.tr(
            "Hide to tray",
            "Ẩn xuống khay",
        )
    }

    fn capture_hint_text(&self) -> &'static str {
        self.tr(
            "Capture mode is active. Press a key now, or press Esc to cancel.",
            "Đang ở chế độ bắt phím. Nhấn phím cần dùng hoặc Esc để hủy.",
        )
    }

    fn titlebar_button(&self, text: RichText, active: bool, danger: bool) -> Button<'static> {
        let (fill, stroke) = match (self.state.ui_theme, active, danger) {
            (_, _, true) => (
                Color32::from_rgba_premultiplied(160, 48, 64, if active { 138 } else { 72 }),
                Color32::from_rgb(222, 106, 126),
            ),
            (UiThemeMode::Dark, true, false) => (
                Color32::from_rgba_premultiplied(74, 146, 118, 166),
                Color32::from_rgb(126, 224, 182),
            ),
            (UiThemeMode::Dark, false, false) => (
                Color32::from_rgba_premultiplied(54, 67, 88, 88),
                Color32::from_rgb(74, 92, 118),
            ),
            (UiThemeMode::Light, true, false) => (
                Color32::from_rgba_premultiplied(72, 156, 116, 120),
                Color32::from_rgb(34, 122, 88),
            ),
            (UiThemeMode::Light, false, false) => (
                Color32::from_rgba_premultiplied(220, 228, 238, 165),
                Color32::from_rgb(188, 198, 214),
            ),
        };
        Button::new(text)
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(8.0)
    }

    fn top_tab_button(&self, text: RichText, selected: bool, emphasized: bool) -> Button<'static> {
        let (fill, stroke) = match (self.state.ui_theme, selected, emphasized) {
            (UiThemeMode::Dark, true, _) => (
                Color32::from_rgba_premultiplied(58, 120, 96, 164),
                Color32::from_rgb(126, 224, 182),
            ),
            (UiThemeMode::Dark, false, true) => (
                Color32::from_rgba_premultiplied(42, 58, 46, 118),
                Color32::from_rgb(92, 180, 148),
            ),
            (UiThemeMode::Dark, false, false) => (
                Color32::from_rgba_premultiplied(34, 42, 56, 72),
                Color32::from_rgb(56, 68, 88),
            ),
            (UiThemeMode::Light, true, _) => (
                Color32::from_rgba_premultiplied(90, 180, 132, 98),
                Color32::from_rgb(34, 122, 88),
            ),
            (UiThemeMode::Light, false, true) => (
                Color32::from_rgba_premultiplied(214, 238, 226, 208),
                Color32::from_rgb(58, 146, 110),
            ),
            (UiThemeMode::Light, false, false) => (
                Color32::from_rgba_premultiplied(230, 236, 242, 165),
                Color32::from_rgb(202, 212, 224),
            ),
        };
        Button::new(text)
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(10.0)
    }

    fn hover_if(response: egui::Response, enabled: bool, text: &str) -> egui::Response {
        if enabled && !text.is_empty() {
            response.on_hover_text(text)
        } else {
            response
        }
    }

    fn render_multi_window_targets(
        ui: &mut egui::Ui,
        id_source: impl std::hash::Hash + Copy,
        label_when_none: &str,
        primary: &mut Option<String>,
        extras: &mut Vec<String>,
        open_windows: &[String],
    ) -> bool {
        let mut changed = false;
        ui.vertical(|ui| {
            egui::ComboBox::from_id_salt((id_source, "primary-target-window"))
                .width(360.0)
                .selected_text(
                    primary
                        .clone()
                        .unwrap_or_else(|| label_when_none.to_owned()),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(primary.is_none(), label_when_none)
                        .clicked()
                    {
                        *primary = None;
                        changed = true;
                    }
                    for title in open_windows {
                        if ui
                            .selectable_label(primary.as_deref() == Some(title), title)
                            .clicked()
                        {
                            *primary = Some(title.clone());
                            changed = true;
                        }
                    }
                });

            let mut remove_index = None;
            for (index, extra) in extras.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt((id_source, "extra-target-window", index))
                        .width(320.0)
                        .selected_text(extra.clone())
                        .show_ui(ui, |ui| {
                            for title in open_windows {
                                if ui.selectable_label(extra == title, title).clicked() {
                                    *extra = title.clone();
                                    changed = true;
                                }
                            }
                        });
                    if ui.button("X").clicked() {
                        remove_index = Some(index);
                    }
                });
            }
            if let Some(index) = remove_index {
                extras.remove(index);
                changed = true;
            }

            if ui.button("+ Window").clicked() {
                let next = open_windows
                    .iter()
                    .find(|title| {
                        primary.as_deref() != Some(title.as_str())
                            && !extras.iter().any(|existing| existing == *title)
                    })
                    .cloned()
                    .or_else(|| open_windows.first().cloned())
                    .unwrap_or_default();
                if !next.is_empty() {
                    extras.push(next);
                    changed = true;
                }
            }
        });
        changed
    }

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
                outcome.changed |= ui
                    .checkbox(
                        &mut clip.enabled,
                        Self::tr_lang(language, "Enabled", "Enabled"),
                    )
                    .changed();
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
                        Self::tr_lang(
                            language,
                            "Stop preview",
                            "Dừng nghe thử",
                        )
                    } else {
                        Self::tr_lang(
                            language,
                            "Preview audio",
                            "Nghe thử âm thanh",
                        )
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
                                UiLanguage::Vietnamese => format!(
                                    "Đã dừng nghe thử {title}."
                                ),
                                _ => format!("Stopped {title} preview."),
                            })
                        }
                        Err(error) => {
                            outcome.status = Some(match language {
                                UiLanguage::Vietnamese => {
                                    format!(
                                        "Nghe thử thất bại: {error}"
                                    )
                                }
                                _ => format!("Preview failed: {error}"),
                            })
                        }
                    }
                }
                if ui
                    .add_enabled(
                        !clip.file_path.trim().is_empty(),
                        Button::new(Self::material_icon_text(0xe15b, 18.0)),
                    )
                    .on_hover_text(Self::tr_lang(
                        language,
                        "Clear audio file",
                        "Xóa file âm thanh",
                    ))
                    .clicked()
                {
                    audio::stop_preview();
                    clip.file_path.clear();
                    clip.start_ms = 0;
                    clip.end_ms = 0;
                    clip.volume = 1.0;
                    *duration_ms = None;
                    *editor_open = false;
                    outcome.changed = true;
                    outcome.status = Some(match language {
                        UiLanguage::Vietnamese => format!("Đã xóa {title}."),
                        _ => format!("Cleared {title}."),
                    });
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
                    Self::tr_lang(
                        language,
                        "Slice",
                        "Đoạn hiện tại"
                    ),
                    Self::format_ms(clip.end_ms.saturating_sub(clip.start_ms))
                ));
            }

            let _ = editor_open;
        });

        outcome
    }

    fn macro_action_label(action: MacroAction) -> &'static str {
        match action {
            MacroAction::KeyPress => "KeyPress",
            MacroAction::KeyDown => "KeyDown",
            MacroAction::KeyUp => "KeyUp",
            MacroAction::TypeText => "TypeText",
            MacroAction::ApplyWindowPreset => "ApplyWindowPreset",
            MacroAction::FocusWindowPreset => "FocusWindowPreset",
            MacroAction::TriggerMacroPreset => "TriggerMacroPreset",
            MacroAction::EnableCrosshairProfile => "EnableCrosshairProfile",
            MacroAction::DisableCrosshair => "DisableCrosshair",
            MacroAction::EnablePinPreset => "EnablePinPreset",
            MacroAction::DisablePin => "DisablePin",
            MacroAction::PlayMousePathPreset => "PlayMousePathPreset",
            MacroAction::ApplyMouseSensitivityPreset => "ApplyMouseSensitivityPreset",
            MacroAction::EnableZoomPreset => "EnableZoomPreset",
            MacroAction::DisableZoom => "DisableZoom",
            MacroAction::PlaySoundPreset => "PlaySoundPreset",
            MacroAction::StartImageSearch => "StartImageSearch",
            MacroAction::TriggerImageSearchMove => "TriggerImageSearchMove",
            MacroAction::TriggerImageSearchTiming => "TriggerImageSearchTiming",
            MacroAction::StopImageSearchWait => "StopImageSearchWait",
            MacroAction::StopImageSearch => "StopImageSearch",
            MacroAction::LoopStart => "LoopStart",
            MacroAction::LoopEnd => "LoopEnd",
            MacroAction::StopIfTriggerPressedAgain => "StopIfTriggerPressedAgain",
            MacroAction::StopIfKeyPressed => "BreakLoopIfKeyPressed",
            MacroAction::ShowToolbox => "ShowToolbox",
            MacroAction::HideToolbox => "HideToolbox",
            MacroAction::LockKeys => "LockKeys",
            MacroAction::UnlockKeys => "UnlockKeys",
            MacroAction::LockMouse => "LockMouse",
            MacroAction::UnlockMouse => "UnlockMouse",
            MacroAction::EnableMacroPreset => "EnableMacroPreset",
            MacroAction::DisableMacroPreset => "DisableMacroPreset",
            MacroAction::MouseLeftClick => "MouseLeftClick",
            MacroAction::MouseLeftDown => "MouseLeftDown",
            MacroAction::MouseLeftUp => "MouseLeftUp",
            MacroAction::MouseRightClick => "MouseRightClick",
            MacroAction::MouseRightDown => "MouseRightDown",
            MacroAction::MouseRightUp => "MouseRightUp",
            MacroAction::MouseMiddleClick => "MouseMiddleClick",
            MacroAction::MouseMiddleDown => "MouseMiddleDown",
            MacroAction::MouseMiddleUp => "MouseMiddleUp",
            MacroAction::MouseX1Click => "MouseX1Click",
            MacroAction::MouseX1Down => "MouseX1Down",
            MacroAction::MouseX1Up => "MouseX1Up",
            MacroAction::MouseX2Click => "MouseX2Click",
            MacroAction::MouseX2Down => "MouseX2Down",
            MacroAction::MouseX2Up => "MouseX2Up",
            MacroAction::MouseWheelUp => "MouseWheelUp",
            MacroAction::MouseWheelDown => "MouseWheelDown",
            MacroAction::MouseMoveAbsolute => "MouseMoveAbsolute",
            MacroAction::MouseMoveRelative => "MouseMoveRelative",
        }
    }

    fn macro_action_tooltip(action: MacroAction) -> &'static str {
        match action {
            MacroAction::KeyPress => "Press and release one keyboard key.",
            MacroAction::KeyDown => "Hold a keyboard key down.",
            MacroAction::KeyUp => "Release a held keyboard key.",
            MacroAction::TypeText => "Type the whole text from the Input field.",
            MacroAction::ApplyWindowPreset => "Run one Window Preset from the selected preset.",
            MacroAction::FocusWindowPreset => {
                "Bring one window forward with the selected focus preset."
            }
            MacroAction::TriggerMacroPreset => {
                "Run another macro preset from the same macro group."
            }
            MacroAction::EnableCrosshairProfile => "Enable one saved crosshair profile.",
            MacroAction::DisableCrosshair => "Turn the overlay crosshair off.",
            MacroAction::EnablePinPreset => "Enable one saved pin preset from the Pin tab.",
            MacroAction::DisablePin => "Turn the pinned app overlay off.",
            MacroAction::PlayMousePathPreset => {
                "Play one recorded mouse path preset from the Mouse tab."
            }
            MacroAction::ApplyMouseSensitivityPreset => {
                "Apply one mouse sensitivity preset from the Mouse tab."
            }
            MacroAction::EnableZoomPreset => "Enable one saved zoom preset.",
            MacroAction::DisableZoom => "Turn the zoom overlay off.",
            MacroAction::PlaySoundPreset => "Play one sound preset from the Sound tab.",
            MacroAction::StartImageSearch => {
                "Start scanning one image-search preset in the background."
            }
            MacroAction::TriggerImageSearchMove => {
                "Move the mouse to the latest image-search match, or run one search now."
            }
            MacroAction::TriggerImageSearchTiming => {
                "Toggle a timing preset loop on or off. It can run for a set number of seconds or forever until you trigger it again."
            }
            MacroAction::StopImageSearchWait => {
                "Stop waiting for one image-search preset to match."
            }
            MacroAction::StopImageSearch => {
                "Stop one image-search preset that is currently scanning."
            }
            MacroAction::LoopStart => {
                "Start looping the next adjacent steps. Input = loop count, or turn on Infinite."
            }
            MacroAction::LoopEnd => "End the current loop block.",
            MacroAction::StopIfTriggerPressedAgain => {
                "Stop the current loop if you press the trigger again."
            }
            MacroAction::StopIfKeyPressed => {
                "Break only the current loop if the key in Input is pressed, then continue with the steps after the loop."
            }
            MacroAction::ShowToolbox => "Show one toolbox preset from the Toolbox tab.",
            MacroAction::HideToolbox => "Hide the currently visible toolbox.",
            MacroAction::LockKeys => "Lock the keys listed in Input.",
            MacroAction::UnlockKeys => "Unlock the keys listed in Input.",
            MacroAction::LockMouse => {
                "Lock mouse movement, clicks, and wheel input until it is unlocked or the macro ends."
            }
            MacroAction::UnlockMouse => "Unlock mouse movement and mouse buttons again.",
            MacroAction::EnableMacroPreset => {
                "Enable one other macro preset from the same macro group."
            }
            MacroAction::DisableMacroPreset => {
                "Disable one other macro preset from the same macro group."
            }
            MacroAction::MouseLeftClick => "Left mouse click.",
            MacroAction::MouseLeftDown => "Hold left mouse button down.",
            MacroAction::MouseLeftUp => "Release left mouse button.",
            MacroAction::MouseRightClick => "Right mouse click.",
            MacroAction::MouseRightDown => "Hold right mouse button down.",
            MacroAction::MouseRightUp => "Release right mouse button.",
            MacroAction::MouseMiddleClick => "Middle mouse click.",
            MacroAction::MouseMiddleDown => "Hold middle mouse button down.",
            MacroAction::MouseMiddleUp => "Release middle mouse button.",
            MacroAction::MouseX1Click => "Mouse button 4 click.",
            MacroAction::MouseX1Down => "Hold mouse button 4 down.",
            MacroAction::MouseX1Up => "Release mouse button 4.",
            MacroAction::MouseX2Click => "Mouse button 5 click.",
            MacroAction::MouseX2Down => "Hold mouse button 5 down.",
            MacroAction::MouseX2Up => "Release mouse button 5.",
            MacroAction::MouseWheelUp => "Scroll mouse wheel up.",
            MacroAction::MouseWheelDown => "Scroll mouse wheel down.",
            MacroAction::MouseMoveAbsolute => "Move the mouse to the exact screen X/Y.",
            MacroAction::MouseMoveRelative => {
                "Move the mouse by the X/Y offset from the current position."
            }
        }
    }

    fn macro_action_icon(action: MacroAction) -> char {
        let codepoint = match action {
            MacroAction::KeyPress => 0xe312,
            MacroAction::KeyDown => 0xe313,
            MacroAction::KeyUp => 0xe316,
            MacroAction::TypeText => 0xe262,
            MacroAction::ApplyWindowPreset => 0xe8b8,
            MacroAction::FocusWindowPreset => 0xe89e,
            MacroAction::TriggerMacroPreset => 0xe8f9,
            MacroAction::EnableCrosshairProfile => 0xe3c5,
            MacroAction::DisableCrosshair => 0xe8f5,
            MacroAction::EnablePinPreset => 0xe89e,
            MacroAction::DisablePin => 0xe8f5,
            MacroAction::PlayMousePathPreset => 0xe913,
            MacroAction::ApplyMouseSensitivityPreset => 0xe837,
            MacroAction::EnableZoomPreset => 0xe8ff,
            MacroAction::DisableZoom => 0xe8f4,
            MacroAction::PlaySoundPreset => 0xe050,
            MacroAction::StartImageSearch => 0xe8b6,
            MacroAction::TriggerImageSearchMove => 0xe8f9,
            MacroAction::TriggerImageSearchTiming => 0xe8f9,
            MacroAction::StopImageSearchWait => 0xe047,
            MacroAction::StopImageSearch => 0xe047,
            MacroAction::LoopStart => 0xe028,
            MacroAction::LoopEnd => 0xe040,
            MacroAction::StopIfTriggerPressedAgain => 0xe047,
            MacroAction::StopIfKeyPressed => 0xe14b,
            MacroAction::ShowToolbox => 0xe8f4,
            MacroAction::HideToolbox => 0xe8f5,
            MacroAction::LockKeys => 0xe897,
            MacroAction::UnlockKeys => 0xe898,
            MacroAction::LockMouse => 0xe323,
            MacroAction::UnlockMouse => 0xe8f5,
            MacroAction::EnableMacroPreset => 0xe86c,
            MacroAction::DisableMacroPreset => 0xe14b,
            MacroAction::MouseLeftClick => 0xe913,
            MacroAction::MouseLeftDown => 0xe764,
            MacroAction::MouseLeftUp => 0xe769,
            MacroAction::MouseRightClick => 0xe323,
            MacroAction::MouseRightDown => 0xe764,
            MacroAction::MouseRightUp => 0xe769,
            MacroAction::MouseMiddleClick => 0xe323,
            MacroAction::MouseMiddleDown => 0xe764,
            MacroAction::MouseMiddleUp => 0xe769,
            MacroAction::MouseX1Click => 0xe762,
            MacroAction::MouseX1Down => 0xe764,
            MacroAction::MouseX1Up => 0xe769,
            MacroAction::MouseX2Click => 0xe762,
            MacroAction::MouseX2Down => 0xe764,
            MacroAction::MouseX2Up => 0xe769,
            MacroAction::MouseWheelUp => 0xe5d8,
            MacroAction::MouseWheelDown => 0xe5db,
            MacroAction::MouseMoveAbsolute => 0xe89f,
            MacroAction::MouseMoveRelative => 0xe8d5,
        };
        char::from_u32(codepoint).unwrap_or('?')
    }

    fn macro_action_icon_text(action: MacroAction) -> RichText {
        Self::material_icon_text(Self::macro_action_icon(action) as u32, 18.0)
    }

    fn macro_action_short_label(action: MacroAction, language: UiLanguage) -> &'static str {
        match language {
            UiLanguage::Vietnamese => Self::normalize_vietnamese(match action {
                MacroAction::KeyPress => "Nhấn",
                MacroAction::KeyDown => "Giữ",
                MacroAction::KeyUp => "Nhả",
                MacroAction::TypeText => "Chữ",
                MacroAction::ApplyWindowPreset => "Áp cửa sổ",
                MacroAction::FocusWindowPreset => "Cửa sổ",
                MacroAction::TriggerMacroPreset => "Tự động",
                MacroAction::EnableCrosshairProfile => "Tâm ngắm",
                MacroAction::DisableCrosshair => "Tắt tâm ngắm",
                MacroAction::EnablePinPreset => "Ghim",
                MacroAction::DisablePin => "Bỏ ghim",
                MacroAction::PlayMousePathPreset => "Đường chuột",
                MacroAction::ApplyMouseSensitivityPreset => "Độ nhạy",
                MacroAction::EnableZoomPreset => "Phóng",
                MacroAction::DisableZoom => "Tắt phóng",
                MacroAction::PlaySoundPreset => "Âm thanh",
                MacroAction::StartImageSearch => "Tìm ảnh",
                MacroAction::TriggerImageSearchMove => "Di chuyển",
                MacroAction::TriggerImageSearchTiming => "Timing",
                MacroAction::StopImageSearchWait => "Chờ",
                MacroAction::StopImageSearch => "Dừng",
                MacroAction::LoopStart => "Lặp",
                MacroAction::LoopEnd => "Kết thúc",
                MacroAction::StopIfTriggerPressedAgain => "Dừng",
                MacroAction::StopIfKeyPressed => "Thoát",
                MacroAction::ShowToolbox => "Công cụ",
                MacroAction::HideToolbox => "Ẩn",
                MacroAction::LockKeys => "Khóa phím",
                MacroAction::UnlockKeys => "Mở phím",
                MacroAction::LockMouse => "Khóa chuột",
                MacroAction::UnlockMouse => "Mở chuột",
                MacroAction::EnableMacroPreset => "Bật preset",
                MacroAction::DisableMacroPreset => "Tắt preset",
                MacroAction::MouseLeftClick => "Trái",
                MacroAction::MouseLeftDown => "Trái↓",
                MacroAction::MouseLeftUp => "Trái↑",
                MacroAction::MouseRightClick => "Phải",
                MacroAction::MouseRightDown => "Phải↓",
                MacroAction::MouseRightUp => "Phải↑",
                MacroAction::MouseMiddleClick => "Giữa",
                MacroAction::MouseMiddleDown => "Giữa↓",
                MacroAction::MouseMiddleUp => "Giữa↑",
                MacroAction::MouseX1Click => "X1",
                MacroAction::MouseX1Down => "X1G",
                MacroAction::MouseX1Up => "X1N",
                MacroAction::MouseX2Click => "X2",
                MacroAction::MouseX2Down => "X2G",
                MacroAction::MouseX2Up => "X2N",
                MacroAction::MouseWheelUp => "Lên",
                MacroAction::MouseWheelDown => "Xuống",
                MacroAction::MouseMoveAbsolute => "Tuyệt đối",
                MacroAction::MouseMoveRelative => "Tương đối",
            }),
            UiLanguage::English => match action {
                MacroAction::KeyPress => "Press",
                MacroAction::KeyDown => "KEY Dn",
                MacroAction::KeyUp => "KEY Up",
                MacroAction::TypeText => "Text",
                MacroAction::ApplyWindowPreset => "Wnd",
                MacroAction::FocusWindowPreset => "Focus",
                MacroAction::TriggerMacroPreset => "Macro",
                MacroAction::EnableCrosshairProfile => "Cross",
                MacroAction::DisableCrosshair => "NoCross",
                MacroAction::EnablePinPreset => "Pin",
                MacroAction::DisablePin => "NoPin",
                MacroAction::PlayMousePathPreset => "Path",
                MacroAction::ApplyMouseSensitivityPreset => "Sense",
                MacroAction::EnableZoomPreset => "Zoom",
                MacroAction::DisableZoom => "NoZoom",
                MacroAction::PlaySoundPreset => "Sound",
                MacroAction::StartImageSearch => "Start",
                MacroAction::TriggerImageSearchMove => "Move",
                MacroAction::TriggerImageSearchTiming => "Timing",
                MacroAction::StopImageSearchWait => "Wait",
                MacroAction::StopImageSearch => "Stop",
                MacroAction::LoopStart => "Loop",
                MacroAction::LoopEnd => "End",
                MacroAction::StopIfTriggerPressedAgain => "Stop",
                MacroAction::StopIfKeyPressed => "Break",
                MacroAction::ShowToolbox => "Tool",
                MacroAction::HideToolbox => "Hide",
                MacroAction::LockKeys => "KL On",
                MacroAction::UnlockKeys => "KL Off",
                MacroAction::LockMouse => "ML On",
                MacroAction::UnlockMouse => "ML Off",
                MacroAction::EnableMacroPreset => "PresetOn",
                MacroAction::DisableMacroPreset => "PresetOff",
                MacroAction::MouseLeftClick => "LClick",
                MacroAction::MouseLeftDown => "LDown",
                MacroAction::MouseLeftUp => "LUp",
                MacroAction::MouseRightClick => "RClick",
                MacroAction::MouseRightDown => "RDown",
                MacroAction::MouseRightUp => "RUp",
                MacroAction::MouseMiddleClick => "MClick",
                MacroAction::MouseMiddleDown => "MDown",
                MacroAction::MouseMiddleUp => "MUp",
                MacroAction::MouseX1Click => "X1",
                MacroAction::MouseX1Down => "X1Dn",
                MacroAction::MouseX1Up => "X1Up",
                MacroAction::MouseX2Click => "X2",
                MacroAction::MouseX2Down => "X2Dn",
                MacroAction::MouseX2Up => "X2Up",
                MacroAction::MouseWheelUp => "WhUp",
                MacroAction::MouseWheelDown => "WhDn",
                MacroAction::MouseMoveAbsolute => "MoveTo",
                MacroAction::MouseMoveRelative => "MoveBy",
            },
            UiLanguage::Icon => match action {
                MacroAction::KeyPress => "Press",
                MacroAction::KeyDown => "KEY Dn",
                MacroAction::KeyUp => "KEY Up",
                MacroAction::TypeText => "Text",
                MacroAction::ApplyWindowPreset => "Wnd",
                MacroAction::FocusWindowPreset => "Focus",
                MacroAction::TriggerMacroPreset => "Macro",
                MacroAction::EnableCrosshairProfile => "Cross",
                MacroAction::DisableCrosshair => "NoCross",
                MacroAction::EnablePinPreset => "Pin",
                MacroAction::DisablePin => "NoPin",
                MacroAction::PlayMousePathPreset => "Path",
                MacroAction::ApplyMouseSensitivityPreset => "Sense",
                MacroAction::EnableZoomPreset => "Zoom",
                MacroAction::DisableZoom => "NoZoom",
                MacroAction::PlaySoundPreset => "Sound",
                MacroAction::StartImageSearch => "Start",
                MacroAction::TriggerImageSearchMove => "Move",
                MacroAction::TriggerImageSearchTiming => "Timing",
                MacroAction::StopImageSearchWait => "Wait",
                MacroAction::StopImageSearch => "Stop",
                MacroAction::LoopStart => "Loop",
                MacroAction::LoopEnd => "End",
                MacroAction::StopIfTriggerPressedAgain => "Stop",
                MacroAction::StopIfKeyPressed => "Break",
                MacroAction::ShowToolbox => "Tool",
                MacroAction::HideToolbox => "Hide",
                MacroAction::LockKeys => "KL On",
                MacroAction::UnlockKeys => "KL Off",
                MacroAction::LockMouse => "ML On",
                MacroAction::UnlockMouse => "ML Off",
                MacroAction::EnableMacroPreset => "PresetOn",
                MacroAction::DisableMacroPreset => "PresetOff",
                MacroAction::MouseLeftClick => "LClick",
                MacroAction::MouseLeftDown => "LDown",
                MacroAction::MouseLeftUp => "LUp",
                MacroAction::MouseRightClick => "RClick",
                MacroAction::MouseRightDown => "RDown",
                MacroAction::MouseRightUp => "RUp",
                MacroAction::MouseMiddleClick => "MClick",
                MacroAction::MouseMiddleDown => "MDown",
                MacroAction::MouseMiddleUp => "MUp",
                MacroAction::MouseX1Click => "X1",
                MacroAction::MouseX1Down => "X1Dn",
                MacroAction::MouseX1Up => "X1Up",
                MacroAction::MouseX2Click => "X2",
                MacroAction::MouseX2Down => "X2Dn",
                MacroAction::MouseX2Up => "X2Up",
                MacroAction::MouseWheelUp => "WhUp",
                MacroAction::MouseWheelDown => "WhDn",
                MacroAction::MouseMoveAbsolute => "MoveTo",
                MacroAction::MouseMoveRelative => "MoveBy",
            },
        }
    }

    fn macro_action_pair_tag(action: MacroAction) -> Option<&'static str> {
        match action {
            MacroAction::KeyDown | MacroAction::KeyUp => Some("KEY"),
            MacroAction::LockKeys | MacroAction::UnlockKeys => Some("KLOCK"),
            MacroAction::LockMouse | MacroAction::UnlockMouse => Some("MLOCK"),
            _ => None,
        }
    }

    fn macro_action_selected_label(action: MacroAction, language: UiLanguage) -> String {
        if let Some(tag) = Self::macro_action_pair_tag(action) {
            match language {
                UiLanguage::Vietnamese => {
                    format!(
                        "[{tag}] {}",
                        Self::macro_action_short_label(action, language)
                    )
                }
                UiLanguage::English => {
                    format!("[{tag}] {}", Self::macro_action_label(action))
                }
                UiLanguage::Icon => {
                    format!("[{tag}] {}", Self::macro_action_label(action))
                }
            }
        } else {
            match language {
                UiLanguage::Vietnamese => {
                    Self::macro_action_short_label(action, language).to_owned()
                }
                UiLanguage::English => {
                    Self::macro_action_label(action).to_owned()
                }
                UiLanguage::Icon => Self::macro_action_label(action).to_owned(),
            }
        }
    }

    fn material_icon_text(codepoint: u32, size: f32) -> RichText {
        RichText::new(char::from_u32(codepoint).unwrap_or('?').to_string())
            .family(FontFamily::Name(MATERIAL_ICONS_FONT.into()))
            .size(size)
    }

    fn folder_icon_text(open: bool, size: f32) -> RichText {
        if open {
            Self::material_icon_text(0xe2c8, size)
        } else {
            Self::material_icon_text(0xe2c7, size)
        }
    }

    fn macro_action_uses_key(action: MacroAction) -> bool {
        matches!(
            action,
            MacroAction::KeyPress
                | MacroAction::KeyDown
                | MacroAction::KeyUp
                | MacroAction::TypeText
                | MacroAction::ApplyWindowPreset
                | MacroAction::FocusWindowPreset
                | MacroAction::TriggerMacroPreset
                | MacroAction::EnableCrosshairProfile
                | MacroAction::EnablePinPreset
                | MacroAction::PlayMousePathPreset
                | MacroAction::ApplyMouseSensitivityPreset
                | MacroAction::EnableZoomPreset
                | MacroAction::PlaySoundPreset
                | MacroAction::EnableMacroPreset
                | MacroAction::DisableMacroPreset
                | MacroAction::StartImageSearch
                | MacroAction::TriggerImageSearchMove
                | MacroAction::TriggerImageSearchTiming
                | MacroAction::StopImageSearchWait
                | MacroAction::StopImageSearch
                | MacroAction::LoopStart
                | MacroAction::StopIfKeyPressed
                | MacroAction::ShowToolbox
                | MacroAction::LockKeys
                | MacroAction::UnlockKeys
        )
    }

    fn macro_action_supports_capture(action: MacroAction) -> bool {
        matches!(
            action,
            MacroAction::KeyPress
                | MacroAction::KeyDown
                | MacroAction::KeyUp
                | MacroAction::StopIfKeyPressed
                | MacroAction::LockKeys
                | MacroAction::UnlockKeys
        )
    }

    fn macro_trigger_mode_label(mode: MacroTriggerMode, language: UiLanguage) -> &'static str {
        match language {
            UiLanguage::Vietnamese => match mode {
                MacroTriggerMode::Press => "Nhấn",
                MacroTriggerMode::Hold => "Giữ",
                MacroTriggerMode::Release => "Thả",
            },
            UiLanguage::English => match mode {
                MacroTriggerMode::Press => "Press",
                MacroTriggerMode::Hold => "Hold",
                MacroTriggerMode::Release => "Release",
            },
            UiLanguage::Icon => match mode {
                MacroTriggerMode::Press => "Press",
                MacroTriggerMode::Hold => "Hold",
                MacroTriggerMode::Release => "Release",
            },
        }
    }

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
                MacroAction::LoopStart => {
                    let color = palette[next_loop_color % palette.len()];
                    next_loop_color += 1;
                    stack.push(color);
                    colors[index] = Some(color);
                }
                MacroAction::LoopEnd => {
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

    fn macro_group_binding_labels(group: &MacroGroup) -> HashMap<u32, String> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for preset in &group.presets {
            let label = if preset.trigger_keys.trim().is_empty() {
                hotkey::format_binding(preset.hotkey.as_ref())
            } else {
                hotkey::format_key_list(&preset.trigger_keys)
            };
            *counts.entry(label).or_insert(0) += 1;
        }

        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut labels = HashMap::new();
        for preset in &group.presets {
            let label = if preset.trigger_keys.trim().is_empty() {
                hotkey::format_binding(preset.hotkey.as_ref())
            } else {
                hotkey::format_key_list(&preset.trigger_keys)
            };
            if counts.get(&label).copied().unwrap_or_default() > 1 && label != "Not set" {
                let entry = seen.entry(label.clone()).or_insert(0);
                *entry += 1;
                labels.insert(preset.id, format!("{label} ({})", *entry));
            } else {
                labels.insert(preset.id, label);
            }
        }
        labels
    }

    fn select_macro_step(
        &mut self,
        group_id: u32,
        preset_id: u32,
        step_index: usize,
        additive: bool,
        currently_selected: bool,
        selected_count_in_preset: usize,
    ) {
        if additive {
            let key = (group_id, preset_id, step_index);
            if !self.selected_macro_steps.insert(key) {
                self.selected_macro_steps.remove(&key);
            }
        } else if currently_selected && selected_count_in_preset <= 1 {
            self.selected_macro_steps.clear();
        } else {
            self.selected_macro_steps.clear();
            self.selected_macro_steps
                .insert((group_id, preset_id, step_index));
        }
    }

    fn clear_macro_step_selection_for_preset(&mut self, group_id: u32, preset_id: u32) {
        self.selected_macro_steps
            .retain(|(selected_group, selected_preset, _)| {
                *selected_group != group_id || *selected_preset != preset_id
            });
    }

    fn set_macro_step_range_selection(
        &mut self,
        group_id: u32,
        preset_id: u32,
        start_index: usize,
        end_index: usize,
    ) {
        self.clear_macro_step_selection_for_preset(group_id, preset_id);
        let start = start_index.min(end_index);
        let end = start_index.max(end_index);
        for step_index in start..=end {
            self.selected_macro_steps
                .insert((group_id, preset_id, step_index));
        }
    }

    fn macro_action_uses_position(action: MacroAction) -> bool {
        matches!(
            action,
            MacroAction::MouseMoveAbsolute | MacroAction::MouseMoveRelative
        )
    }

    fn mouse_path_event_label(event: MousePathEventKind) -> &'static str {
        match event {
            MousePathEventKind::Move => "Move",
            MousePathEventKind::LeftDown => "LDown",
            MousePathEventKind::LeftUp => "LUp",
            MousePathEventKind::RightDown => "RDown",
            MousePathEventKind::RightUp => "RUp",
            MousePathEventKind::MiddleDown => "MDown",
            MousePathEventKind::MiddleUp => "MUp",
            MousePathEventKind::WheelUp => "Wheel+",
            MousePathEventKind::WheelDown => "Wheel-",
        }
    }

    fn sized_button(ui: &mut egui::Ui, width: f32, label: &str) -> egui::Response {
        ui.add_sized([width, 24.0], Button::new(label))
    }

    fn window_anchor_label(anchor: WindowAnchor) -> &'static str {
        match anchor {
            WindowAnchor::Manual => "Manual",
            WindowAnchor::Center => "Center",
            WindowAnchor::TopLeft => "Top Left",
            WindowAnchor::Top => "Top",
            WindowAnchor::TopRight => "Top Right",
            WindowAnchor::Left => "Left",
            WindowAnchor::Right => "Right",
            WindowAnchor::BottomLeft => "Bottom Left",
            WindowAnchor::Bottom => "Bottom",
            WindowAnchor::BottomRight => "Bottom Right",
        }
    }

    fn window_anchor_icon(anchor: WindowAnchor) -> &'static str {
        match anchor {
            WindowAnchor::Manual => "XY",
            WindowAnchor::Center => "\u{25CE}",
            WindowAnchor::TopLeft => "\u{2196}",
            WindowAnchor::Top => "\u{2191}",
            WindowAnchor::TopRight => "\u{2197}",
            WindowAnchor::Left => "\u{2190}",
            WindowAnchor::Right => "\u{2192}",
            WindowAnchor::BottomLeft => "\u{2199}",
            WindowAnchor::Bottom => "\u{2193}",
            WindowAnchor::BottomRight => "\u{2198}",
        }
    }

    fn window_anchor_picker(ui: &mut egui::Ui, preset: &mut WindowPreset) -> bool {
        let mut changed = false;
        let rows = [
            [
                WindowAnchor::TopLeft,
                WindowAnchor::Top,
                WindowAnchor::TopRight,
            ],
            [
                WindowAnchor::Left,
                WindowAnchor::Center,
                WindowAnchor::Right,
            ],
            [
                WindowAnchor::BottomLeft,
                WindowAnchor::Bottom,
                WindowAnchor::BottomRight,
            ],
        ];

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let manual_response = ui
                    .add_sized(
                        [42.0, 24.0],
                        Button::new(Self::window_anchor_icon(WindowAnchor::Manual))
                            .selected(preset.anchor == WindowAnchor::Manual),
                    )
                    .on_hover_text("Manual X/Y position");
                if manual_response.clicked() {
                    preset.anchor = WindowAnchor::Manual;
                    changed = true;
                }

                ui.add_space(6.0);

                egui::Grid::new(("window-anchor-grid", preset.id))
                    .num_columns(3)
                    .spacing([4.0, 4.0])
                    .show(ui, |ui| {
                        for row in rows {
                            for anchor in row {
                                let selected = preset.anchor == anchor;
                                let response = ui
                                    .add_sized(
                                        [32.0, 22.0],
                                        Button::new(Self::window_anchor_icon(anchor))
                                            .selected(selected),
                                    )
                                    .on_hover_text(Self::window_anchor_label(anchor));
                                if response.clicked() {
                                    preset.anchor = anchor;
                                    changed = true;
                                }
                            }
                            ui.end_row();
                        }
                    });
            });

            ui.add_space(2.0);
            ui.label(
                RichText::new(Self::window_anchor_summary(preset.anchor))
                    .small()
                    .italics(),
            );
        });

        changed
    }

    fn window_anchor_summary(anchor: WindowAnchor) -> &'static str {
        match anchor {
            WindowAnchor::Manual => "Manual X/Y",
            WindowAnchor::Center => "Auto: Center",
            WindowAnchor::TopLeft => "Auto: Top Left",
            WindowAnchor::Top => "Auto: Top Edge",
            WindowAnchor::TopRight => "Auto: Top Right",
            WindowAnchor::Left => "Auto: Left Edge",
            WindowAnchor::Right => "Auto: Right Edge",
            WindowAnchor::BottomLeft => "Auto: Bottom Left",
            WindowAnchor::Bottom => "Auto: Bottom Edge",
            WindowAnchor::BottomRight => "Auto: Bottom Right",
        }
    }

    fn window_anchor_preview_position(preset: &WindowPreset) -> Option<(i32, i32)> {
        if preset.anchor == WindowAnchor::Manual {
            return None;
        }

        #[cfg(windows)]
        unsafe {
            let screen_width = GetSystemMetrics(SM_CXSCREEN);
            let screen_height = GetSystemMetrics(SM_CYSCREEN);
            let width = preset.width.max(1);
            let height = preset.height.max(1);
            let position = match preset.anchor {
                WindowAnchor::Manual => (preset.x, preset.y),
                WindowAnchor::Center => ((screen_width - width) / 2, (screen_height - height) / 2),
                WindowAnchor::TopLeft => (0, 0),
                WindowAnchor::Top => (((screen_width - width) / 2), 0),
                WindowAnchor::TopRight => ((screen_width - width), 0),
                WindowAnchor::Left => (0, ((screen_height - height) / 2)),
                WindowAnchor::Right => ((screen_width - width), ((screen_height - height) / 2)),
                WindowAnchor::BottomLeft => (0, (screen_height - height)),
                WindowAnchor::Bottom => (((screen_width - width) / 2), (screen_height - height)),
                WindowAnchor::BottomRight => ((screen_width - width), (screen_height - height)),
            };
            return Some(position);
        }

        #[allow(unreachable_code)]
        None
    }

    fn render_zoom_rect_editor(
        ui: &mut egui::Ui,
        id_source: impl std::hash::Hash,
        label: &str,
        x: &mut i32,
        y: &mut i32,
        width: &mut i32,
        height: &mut i32,
        screen_size: egui::Vec2,
        preview: Option<&ZoomPreviewView>,
        target_preview_source: Option<(i32, i32, i32, i32)>,
        keep_aspect_ratio: Option<f32>,
    ) -> bool {
        let mut changed = false;
        ui.label(RichText::new(label).strong());
        let desired = vec2(ui.available_width().max(420.0), 260.0);
        let (canvas_rect, _) = ui.allocate_exact_size(desired, Sense::hover());
        let draw_rect = canvas_rect.shrink(8.0);
        let scale = (draw_rect.width() / screen_size.x)
            .min(draw_rect.height() / screen_size.y)
            .max(0.0001);
        let preview_size = vec2(screen_size.x * scale, screen_size.y * scale);
        let preview_rect = egui::Rect::from_center_size(draw_rect.center(), preview_size);
        ui.painter().rect_filled(
            preview_rect,
            8.0,
            Color32::from_rgba_premultiplied(24, 36, 30, 220),
        );
        ui.painter().rect_stroke(
            preview_rect,
            8.0,
            egui::Stroke::new(1.0, Color32::from_rgb(112, 156, 128)),
            egui::StrokeKind::Outside,
        );

        let selection_bounds_rect = preview_rect;
        let (coord_width, coord_height, content_scale, preview_content_rect) =
            if let Some(preview_frame) = preview {
                let window_pos = egui::pos2(
                    selection_bounds_rect.left() + (preview_frame.screen_x as f32 * scale),
                    selection_bounds_rect.top() + (preview_frame.screen_y as f32 * scale),
                );
                let window_size = vec2(
                    preview_frame.logical_width.max(1) as f32 * scale,
                    preview_frame.logical_height.max(1) as f32 * scale,
                );
                (
                    screen_size.x,
                    screen_size.y,
                    scale,
                    egui::Rect::from_min_size(window_pos, window_size),
                )
            } else {
                (screen_size.x, screen_size.y, scale, selection_bounds_rect)
            };

        if let Some(preview_frame) = preview {
            ui.painter().image(
                preview_frame.texture.id(),
                preview_content_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            ui.painter().text(
                preview_content_rect.left_top() + vec2(8.0, 8.0),
                egui::Align2::LEFT_TOP,
                &preview_frame.title,
                egui::TextStyle::Small.resolve(ui.style()),
                Color32::WHITE,
            );
        }

        let min_size = vec2(6.0, 6.0);
        let mut rect = egui::Rect::from_min_size(
            egui::pos2(
                selection_bounds_rect.left() + (*x as f32 * content_scale),
                selection_bounds_rect.top() + (*y as f32 * content_scale),
            ),
            vec2(
                (*width).max(1) as f32 * content_scale,
                (*height).max(1) as f32 * content_scale,
            ),
        );
        rect = rect.intersect(selection_bounds_rect);
        if rect.width() < min_size.x {
            rect.max.x = (rect.min.x + min_size.x).min(selection_bounds_rect.right());
        }
        if rect.height() < min_size.y {
            rect.max.y = (rect.min.y + min_size.y).min(selection_bounds_rect.bottom());
        }

        let rect_id = ui.make_persistent_id((id_source, "zoom-rect"));
        let body_response = ui.interact(rect, rect_id, Sense::drag());
        if body_response.dragged() {
            let mut delta = ui.input(|input| input.pointer.delta());
            let shift_down = ui.input(|input| input.modifiers.shift);
            if shift_down {
                if delta.x.abs() >= delta.y.abs() {
                    delta.y = 0.0;
                } else {
                    delta.x = 0.0;
                }
            }
            rect = rect.translate(delta);
            if rect.left() < selection_bounds_rect.left() {
                rect = rect.translate(vec2(selection_bounds_rect.left() - rect.left(), 0.0));
            }
            if rect.top() < selection_bounds_rect.top() {
                rect = rect.translate(vec2(0.0, selection_bounds_rect.top() - rect.top()));
            }
            if rect.right() > selection_bounds_rect.right() {
                rect = rect.translate(vec2(selection_bounds_rect.right() - rect.right(), 0.0));
            }
            if rect.bottom() > selection_bounds_rect.bottom() {
                rect = rect.translate(vec2(0.0, selection_bounds_rect.bottom() - rect.bottom()));
            }
            changed = true;
        }

        let handles = [
            ("nw", rect.left_top()),
            ("n", egui::pos2(rect.center().x, rect.top())),
            ("ne", rect.right_top()),
            ("e", egui::pos2(rect.right(), rect.center().y)),
            ("se", rect.right_bottom()),
            ("s", egui::pos2(rect.center().x, rect.bottom())),
            ("sw", rect.left_bottom()),
            ("w", egui::pos2(rect.left(), rect.center().y)),
        ];
        for (name, pos) in handles {
            let handle_rect = egui::Rect::from_center_size(pos, vec2(8.0, 8.0));
            let response = ui.interact(
                handle_rect,
                ui.make_persistent_id((rect_id, name)),
                Sense::drag(),
            );
            if response.dragged() {
                let delta = ui.input(|input| input.pointer.delta());
                let shift_down = ui.input(|input| input.modifiers.shift);
                match name {
                    "nw" => {
                        rect.min.x += delta.x;
                        rect.min.y += delta.y;
                    }
                    "n" => rect.min.y += delta.y,
                    "ne" => {
                        rect.max.x += delta.x;
                        rect.min.y += delta.y;
                    }
                    "e" => rect.max.x += delta.x,
                    "se" => {
                        rect.max.x += delta.x;
                        rect.max.y += delta.y;
                    }
                    "s" => rect.max.y += delta.y,
                    "sw" => {
                        rect.min.x += delta.x;
                        rect.max.y += delta.y;
                    }
                    "w" => rect.min.x += delta.x,
                    _ => {}
                }
                if shift_down && let Some(aspect_ratio) = keep_aspect_ratio {
                    Self::apply_locked_aspect_ratio(
                        name,
                        aspect_ratio,
                        selection_bounds_rect,
                        min_size,
                        &mut rect,
                    );
                }
                rect.min.x = rect.min.x.clamp(
                    selection_bounds_rect.left(),
                    selection_bounds_rect.right() - min_size.x,
                );
                rect.min.y = rect.min.y.clamp(
                    selection_bounds_rect.top(),
                    selection_bounds_rect.bottom() - min_size.y,
                );
                rect.max.x = rect
                    .max
                    .x
                    .clamp(rect.min.x + min_size.x, selection_bounds_rect.right());
                rect.max.y = rect
                    .max
                    .y
                    .clamp(rect.min.y + min_size.y, selection_bounds_rect.bottom());
                changed = true;
            }
            ui.painter()
                .rect_filled(handle_rect, 2.0, Color32::from_rgb(124, 240, 164));
        }

        if let (Some(preview_frame), Some((src_x, src_y, src_w, src_h))) =
            (preview, target_preview_source)
        {
            let uv = egui::Rect::from_min_max(
                egui::pos2(
                    (src_x as f32 / preview_frame.logical_width.max(1) as f32).clamp(0.0, 1.0),
                    (src_y as f32 / preview_frame.logical_height.max(1) as f32).clamp(0.0, 1.0),
                ),
                egui::pos2(
                    ((src_x + src_w) as f32 / preview_frame.logical_width.max(1) as f32)
                        .clamp(0.0, 1.0),
                    ((src_y + src_h) as f32 / preview_frame.logical_height.max(1) as f32)
                        .clamp(0.0, 1.0),
                ),
            );
            if uv.width() > 0.0 && uv.height() > 0.0 {
                ui.painter()
                    .image(preview_frame.texture.id(), rect, uv, Color32::WHITE);
            }
        }

        ui.painter().rect_stroke(
            rect,
            6.0,
            egui::Stroke::new(2.0, Color32::from_rgb(124, 240, 164)),
            egui::StrokeKind::Outside,
        );

        if changed {
            *x = ((rect.left() - selection_bounds_rect.left()) / content_scale).round() as i32;
            *y = ((rect.top() - selection_bounds_rect.top()) / content_scale).round() as i32;
            *width = (rect.width() / content_scale).round().max(1.0) as i32;
            *height = (rect.height() / content_scale).round().max(1.0) as i32;
            *x = (*x).clamp(0, coord_width.round() as i32);
            *y = (*y).clamp(0, coord_height.round() as i32);
        }

        ui.label(RichText::new(format!("X={} Y={} W={} H={}", *x, *y, *width, *height)).small());
        changed
    }

    fn edit_rgba_color(ui: &mut egui::Ui, color: &mut RgbaColor) -> bool {
        let mut rgba = [color.r, color.g, color.b, color.a];
        let changed = ui.color_edit_button_srgba_unmultiplied(&mut rgba).changed();
        if changed {
            color.r = rgba[0];
            color.g = rgba[1];
            color.b = rgba[2];
            color.a = rgba[3];
        }
        changed
    }

    fn render_toolbox_rect_editor(
        ui: &mut egui::Ui,
        id_source: impl std::hash::Hash,
        preset: &mut ToolboxPreset,
    ) -> bool {
        let mut changed = false;
        let screen_size = Self::screen_size();
        let desired = vec2(ui.available_width().max(560.0), 420.0);
        let (canvas_rect, _) = ui.allocate_exact_size(desired, Sense::hover());
        let draw_rect = canvas_rect.shrink(8.0);
        let scale = (draw_rect.width() / screen_size.x)
            .min(draw_rect.height() / screen_size.y)
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

        let min_size = vec2(4.0, 4.0);
        let mut rect = egui::Rect::from_min_size(
            egui::pos2(
                preview_rect.left() + (preset.x as f32 * scale),
                preview_rect.top() + (preset.y as f32 * scale),
            ),
            vec2(
                preset.width.max(1) as f32 * scale,
                preset.height.max(1) as f32 * scale,
            ),
        )
        .intersect(preview_rect);
        if rect.width() < min_size.x {
            rect.max.x = (rect.min.x + min_size.x).min(preview_rect.right());
        }
        if rect.height() < min_size.y {
            rect.max.y = (rect.min.y + min_size.y).min(preview_rect.bottom());
        }

        let rect_id = ui.make_persistent_id((id_source, "toolbox-rect"));
        let body_response = ui.interact(rect, rect_id, Sense::drag());
        if body_response.dragged() {
            let delta = ui.input(|input| input.pointer.delta());
            rect = rect.translate(delta);
            if rect.left() < preview_rect.left() {
                rect = rect.translate(vec2(preview_rect.left() - rect.left(), 0.0));
            }
            if rect.top() < preview_rect.top() {
                rect = rect.translate(vec2(0.0, preview_rect.top() - rect.top()));
            }
            if rect.right() > preview_rect.right() {
                rect = rect.translate(vec2(preview_rect.right() - rect.right(), 0.0));
            }
            if rect.bottom() > preview_rect.bottom() {
                rect = rect.translate(vec2(0.0, preview_rect.bottom() - rect.bottom()));
            }
            changed = true;
        }

        let handles = [
            ("nw", rect.left_top()),
            ("n", egui::pos2(rect.center().x, rect.top())),
            ("ne", rect.right_top()),
            ("e", egui::pos2(rect.right(), rect.center().y)),
            ("se", rect.right_bottom()),
            ("s", egui::pos2(rect.center().x, rect.bottom())),
            ("sw", rect.left_bottom()),
            ("w", egui::pos2(rect.left(), rect.center().y)),
        ];
        for (name, pos) in handles {
            let handle_rect = egui::Rect::from_center_size(pos, vec2(8.0, 8.0));
            let response = ui.interact(
                handle_rect,
                ui.make_persistent_id((rect_id, name)),
                Sense::drag(),
            );
            if response.dragged() {
                let delta = ui.input(|input| input.pointer.delta());
                match name {
                    "nw" => {
                        rect.min.x += delta.x;
                        rect.min.y += delta.y;
                    }
                    "n" => rect.min.y += delta.y,
                    "ne" => {
                        rect.max.x += delta.x;
                        rect.min.y += delta.y;
                    }
                    "e" => rect.max.x += delta.x,
                    "se" => {
                        rect.max.x += delta.x;
                        rect.max.y += delta.y;
                    }
                    "s" => rect.max.y += delta.y,
                    "sw" => {
                        rect.min.x += delta.x;
                        rect.max.y += delta.y;
                    }
                    "w" => rect.min.x += delta.x,
                    _ => {}
                }
                rect.min.x = rect
                    .min
                    .x
                    .clamp(preview_rect.left(), preview_rect.right() - min_size.x);
                rect.min.y = rect
                    .min
                    .y
                    .clamp(preview_rect.top(), preview_rect.bottom() - min_size.y);
                rect.max.x = rect
                    .max
                    .x
                    .clamp(rect.min.x + min_size.x, preview_rect.right());
                rect.max.y = rect
                    .max
                    .y
                    .clamp(rect.min.y + min_size.y, preview_rect.bottom());
                changed = true;
            }
            ui.painter()
                .rect_filled(handle_rect, 2.0, Color32::from_rgb(124, 240, 164));
        }

        let bg_alpha = (preset.background_opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        let background = Color32::from_rgba_premultiplied(
            ((preset.background_color.r as u32 * bg_alpha as u32) / 255) as u8,
            ((preset.background_color.g as u32 * bg_alpha as u32) / 255) as u8,
            ((preset.background_color.b as u32 * bg_alpha as u32) / 255) as u8,
            bg_alpha,
        );
        let text_color = Color32::from_rgba_premultiplied(
            preset.text_color.r,
            preset.text_color.g,
            preset.text_color.b,
            preset.text_color.a,
        );
        let rounding = if preset.rounded_background { 12.0 } else { 0.0 };
        if bg_alpha > 0 {
            ui.painter().rect_filled(rect, rounding, background);
        }
        ui.painter().rect_stroke(
            rect,
            rounding,
            egui::Stroke::new(2.0, Color32::from_rgb(124, 240, 164)),
            egui::StrokeKind::Outside,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            if preset.text.trim().is_empty() {
                "Toolbox preview"
            } else {
                preset.text.as_str()
            },
            egui::FontId::proportional((preset.font_size * scale).clamp(2.0, 200.0)),
            text_color,
        );

        if changed {
            preset.x = ((rect.left() - preview_rect.left()) / scale).round() as i32;
            preset.y = ((rect.top() - preview_rect.top()) / scale).round() as i32;
            preset.width = (rect.width() / scale).round().max(1.0) as i32;
            preset.height = (rect.height() / scale).round().max(1.0) as i32;
        }

        ui.label(
            RichText::new(format!(
                "X={} Y={} W={} H={}",
                preset.x, preset.y, preset.width, preset.height
            ))
            .small(),
        );
        changed
    }

    fn apply_locked_aspect_ratio(
        handle: &str,
        aspect_ratio: f32,
        bounds: egui::Rect,
        min_size: egui::Vec2,
        rect: &mut egui::Rect,
    ) {
        if aspect_ratio <= 0.0 {
            return;
        }
        match handle {
            "nw" | "ne" | "se" | "sw" => {
                let anchor = match handle {
                    "nw" => rect.right_bottom(),
                    "ne" => rect.left_bottom(),
                    "se" => rect.left_top(),
                    "sw" => rect.right_top(),
                    _ => rect.right_bottom(),
                };
                let moving = match handle {
                    "nw" => rect.left_top(),
                    "ne" => rect.right_top(),
                    "se" => rect.right_bottom(),
                    "sw" => rect.left_bottom(),
                    _ => rect.left_top(),
                };
                let mut dx = moving.x - anchor.x;
                let mut dy = moving.y - anchor.y;
                let width = dx.abs().max(min_size.x);
                let height = dy.abs().max(min_size.y);
                let expected_height = width / aspect_ratio;
                let expected_width = height * aspect_ratio;
                if expected_height >= height {
                    dy = dy.signum() * expected_height.max(min_size.y);
                } else {
                    dx = dx.signum() * expected_width.max(min_size.x);
                }
                let new_corner = egui::pos2(anchor.x + dx, anchor.y + dy);
                *rect = egui::Rect::from_two_pos(anchor, new_corner).intersect(bounds);
            }
            "n" | "s" => {
                let center_x = rect.center().x;
                let anchor_y = if handle == "n" {
                    rect.bottom()
                } else {
                    rect.top()
                };
                let moving_y = if handle == "n" {
                    rect.top()
                } else {
                    rect.bottom()
                };
                let height = (moving_y - anchor_y).abs().max(min_size.y);
                let width = (height * aspect_ratio).max(min_size.x);
                let left = (center_x - width * 0.5).clamp(bounds.left(), bounds.right() - width);
                let right = left + width;
                let top = if handle == "n" {
                    (anchor_y - height).clamp(bounds.top(), bounds.bottom() - height)
                } else {
                    anchor_y.clamp(bounds.top(), bounds.bottom() - height)
                };
                let bottom = top + height;
                *rect = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));
            }
            "e" | "w" => {
                let center_y = rect.center().y;
                let anchor_x = if handle == "w" {
                    rect.right()
                } else {
                    rect.left()
                };
                let moving_x = if handle == "w" {
                    rect.left()
                } else {
                    rect.right()
                };
                let width = (moving_x - anchor_x).abs().max(min_size.x);
                let height = (width / aspect_ratio).max(min_size.y);
                let top = (center_y - height * 0.5).clamp(bounds.top(), bounds.bottom() - height);
                let bottom = top + height;
                let left = if handle == "w" {
                    (anchor_x - width).clamp(bounds.left(), bounds.right() - width)
                } else {
                    anchor_x.clamp(bounds.left(), bounds.right() - width)
                };
                let right = left + width;
                *rect = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));
            }
            _ => {}
        }
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
                Self::macro_action_tooltip(candidate)
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
                                    if (index + 1) % 5 == 0 {
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
                    RichText::new(Self::tr_lang(
                        language,
                        "Mouse",
                        "Chuá»™t",
                    ))
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
            MacroAction::StartImageSearch,
            MacroAction::TriggerImageSearchMove,
            MacroAction::TriggerImageSearchTiming,
            MacroAction::StopImageSearchWait,
            MacroAction::StopImageSearch,
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

    fn capture_button_text(language: UiLanguage, active: bool) -> RichText {
        if active {
            RichText::new(Self::tr_lang(
                language,
                "Capturing...",
                "Đang bắt...",
            ))
            .strong()
            .color(Color32::from_rgb(255, 232, 96))
        } else {
            RichText::new(Self::tr_lang(language, "Capture", "Capture"))
        }
    }

    #[allow(deprecated)]
    fn show_instant_hover_tooltip(
        ui: &egui::Ui,
        response: &egui::Response,
        text: impl Into<String>,
    ) {
        if response.hovered() {
            egui::show_tooltip_at_pointer(
                ui.ctx(),
                ui.layer_id(),
                response.id.with("instant-tip"),
                |ui| {
                    ui.label(text.into());
                },
            );
        }
    }

    fn add_window_preset(&mut self) {
        let id = self.state.next_preset_id.max(1);
        self.state.next_preset_id = id + 1;
        self.state.window_presets.push(WindowPreset::new(id));
        self.reconcile_master_presets();
        self.sync_window_presets();
        self.status = format!("Added window preset {id}.");
    }

    fn add_window_focus_preset(&mut self) {
        let id = self.state.next_window_focus_preset_id.max(1);
        self.state.next_window_focus_preset_id = id + 1;
        self.state
            .window_focus_presets
            .push(WindowFocusPreset::new(id));
        self.reconcile_master_presets();
        self.sync_window_presets();
        self.status = format!("Added window focus preset {id}.");
    }

    fn add_zoom_preset(&mut self) {
        let id = self.state.next_zoom_preset_id.max(1);
        self.state.next_zoom_preset_id = id + 1;
        self.state.zoom_presets.push(ZoomPreset::new(id));
        self.reconcile_master_presets();
        self.sync_window_presets();
        self.status = format!("Added zoom preset {id}.");
    }

    fn add_pin_preset(&mut self) {
        let id = self.state.next_pin_preset_id.max(1);
        self.state.next_pin_preset_id = id + 1;
        self.state.pin_presets.push(PinPreset::new(id));
        self.sync_window_presets();
        self.status = format!("Added pin preset {id}.");
    }

    fn add_mouse_path_preset(&mut self) {
        let id = self.state.next_mouse_path_preset_id.max(1);
        self.state.next_mouse_path_preset_id = id + 1;
        self.state.mouse_path_presets.push(MousePathPreset::new(id));
        self.sync_window_presets();
        self.status = format!("Added mouse path preset {id}.");
    }

    fn add_mouse_sensitivity_preset(&mut self) {
        let id = self.state.next_mouse_sensitivity_preset_id.max(1);
        self.state.next_mouse_sensitivity_preset_id = id + 1;
        self.state
            .mouse_sensitivity_presets
            .push(MouseSensitivityPreset::new(id));
        self.sync_mouse_sensitivity_presets();
        self.status = format!("Added mouse sensitivity preset {id}.");
    }

    fn add_toolbox_preset(&mut self) {
        let id = self.state.next_toolbox_preset_id.max(1);
        self.state.next_toolbox_preset_id = id + 1;
        self.state.toolbox_presets.push(ToolboxPreset::new(id));
        self.sync_toolbox_presets();
        self.status = format!("Added toolbox preset {id}.");
    }

    fn capture_master_preset_snapshot(&self, id: u32, name: String) -> MasterPreset {
        MasterPreset {
            id,
            name,
            collapsed: true,
            macros_master_enabled: self.state.macros_master_enabled,
            window_expand_controls_enabled: self.state.window_expand_controls.enabled,
            window_presets: self
                .state
                .window_presets
                .iter()
                .map(|preset| MasterWindowPresetState {
                    id: preset.id,
                    enabled: preset.enabled,
                    animate_enabled: preset.animate_enabled,
                    restore_titlebar_enabled: preset.restore_titlebar_enabled,
                })
                .collect(),
            window_focus_presets: self
                .state
                .window_focus_presets
                .iter()
                .map(|preset| MasterWindowFocusPresetState {
                    id: preset.id,
                    enabled: preset.enabled,
                })
                .collect(),
            zoom_presets: self
                .state
                .zoom_presets
                .iter()
                .map(|preset| MasterZoomPresetState {
                    id: preset.id,
                    enabled: preset.enabled,
                })
                .collect(),
            macro_groups: self
                .state
                .macro_groups
                .iter()
                .map(|group| MasterMacroGroupState {
                    id: group.id,
                    enabled: group.enabled,
                    presets: group
                        .presets
                        .iter()
                        .map(|preset| MasterMacroPresetState {
                            id: preset.id,
                            enabled: preset.enabled,
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    fn ensure_master_presets(&mut self) {
        if self.state.master_presets.is_empty() {
            let id = self.state.next_master_preset_id.max(1);
            self.state.next_master_preset_id = id + 1;
            self.state
                .master_presets
                .push(self.capture_master_preset_snapshot(id, "Default".to_owned()));
            self.state.selected_master_preset_id = Some(id);
            self.persist();
            return;
        }
        self.reconcile_master_presets();
        if self.state.selected_master_preset_id.is_none() {
            self.state.selected_master_preset_id =
                self.state.master_presets.first().map(|preset| preset.id);
        }
    }

    fn reconcile_master_presets(&mut self) {
        let window_lookup = self
            .state
            .window_presets
            .iter()
            .map(|preset| {
                (
                    preset.id,
                    MasterWindowPresetState {
                        id: preset.id,
                        enabled: preset.enabled,
                        animate_enabled: preset.animate_enabled,
                        restore_titlebar_enabled: preset.restore_titlebar_enabled,
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        let focus_lookup = self
            .state
            .window_focus_presets
            .iter()
            .map(|preset| {
                (
                    preset.id,
                    MasterWindowFocusPresetState {
                        id: preset.id,
                        enabled: preset.enabled,
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        let zoom_lookup = self
            .state
            .zoom_presets
            .iter()
            .map(|preset| {
                (
                    preset.id,
                    MasterZoomPresetState {
                        id: preset.id,
                        enabled: preset.enabled,
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        let macro_lookup = self
            .state
            .macro_groups
            .iter()
            .map(|group| {
                (
                    group.id,
                    MasterMacroGroupState {
                        id: group.id,
                        enabled: group.enabled,
                        presets: group
                            .presets
                            .iter()
                            .map(|preset| MasterMacroPresetState {
                                id: preset.id,
                                enabled: preset.enabled,
                            })
                            .collect(),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        for preset in &mut self.state.master_presets {
            preset
                .window_presets
                .retain(|item| window_lookup.contains_key(&item.id));
            for window_preset in &self.state.window_presets {
                if !preset
                    .window_presets
                    .iter()
                    .any(|item| item.id == window_preset.id)
                    && let Some(item) = window_lookup.get(&window_preset.id)
                {
                    preset.window_presets.push(item.clone());
                }
            }
            preset.window_presets.sort_by_key(|item| {
                self.state
                    .window_presets
                    .iter()
                    .position(|preset| preset.id == item.id)
                    .unwrap_or(usize::MAX)
            });

            preset
                .window_focus_presets
                .retain(|item| focus_lookup.contains_key(&item.id));
            for focus_preset in &self.state.window_focus_presets {
                if !preset
                    .window_focus_presets
                    .iter()
                    .any(|item| item.id == focus_preset.id)
                    && let Some(item) = focus_lookup.get(&focus_preset.id)
                {
                    preset.window_focus_presets.push(item.clone());
                }
            }
            preset.window_focus_presets.sort_by_key(|item| {
                self.state
                    .window_focus_presets
                    .iter()
                    .position(|preset| preset.id == item.id)
                    .unwrap_or(usize::MAX)
            });

            preset
                .zoom_presets
                .retain(|item| zoom_lookup.contains_key(&item.id));
            for zoom_preset in &self.state.zoom_presets {
                if !preset
                    .zoom_presets
                    .iter()
                    .any(|item| item.id == zoom_preset.id)
                    && let Some(item) = zoom_lookup.get(&zoom_preset.id)
                {
                    preset.zoom_presets.push(item.clone());
                }
            }
            preset.zoom_presets.sort_by_key(|item| {
                self.state
                    .zoom_presets
                    .iter()
                    .position(|preset| preset.id == item.id)
                    .unwrap_or(usize::MAX)
            });

            preset
                .macro_groups
                .retain(|item| macro_lookup.contains_key(&item.id));
            for macro_group in &self.state.macro_groups {
                if !preset
                    .macro_groups
                    .iter()
                    .any(|item| item.id == macro_group.id)
                    && let Some(item) = macro_lookup.get(&macro_group.id)
                {
                    preset.macro_groups.push(item.clone());
                }
            }
            for group_state in &mut preset.macro_groups {
                if let Some(group) = self
                    .state
                    .macro_groups
                    .iter()
                    .find(|group| group.id == group_state.id)
                {
                    group_state
                        .presets
                        .retain(|item| group.presets.iter().any(|preset| preset.id == item.id));
                    for preset_item in &group.presets {
                        if !group_state
                            .presets
                            .iter()
                            .any(|item| item.id == preset_item.id)
                        {
                            group_state.presets.push(MasterMacroPresetState {
                                id: preset_item.id,
                                enabled: preset_item.enabled,
                            });
                        }
                    }
                    group_state.presets.sort_by_key(|item| {
                        group
                            .presets
                            .iter()
                            .position(|preset| preset.id == item.id)
                            .unwrap_or(usize::MAX)
                    });
                }
            }
            preset.macro_groups.sort_by_key(|item| {
                self.state
                    .macro_groups
                    .iter()
                    .position(|group| group.id == item.id)
                    .unwrap_or(usize::MAX)
            });
        }
    }

    fn add_master_preset_from_current(&mut self) {
        let id = self.state.next_master_preset_id.max(1);
        self.state.next_master_preset_id = id + 1;
        let preset = self.capture_master_preset_snapshot(id, format!("Mode {id}"));
        self.state.master_presets.push(preset);
        self.state.selected_master_preset_id = Some(id);
        self.persist();
        self.status = format!("Captured current hotkey setup into mode {id}.");
    }

    fn update_master_preset_from_current(&mut self, preset_id: u32) {
        let replacement = self
            .state
            .master_presets
            .iter()
            .find(|preset| preset.id == preset_id)
            .map(|preset| (preset.collapsed, preset.name.clone()));
        let Some((collapsed, name)) = replacement else {
            return;
        };
        let mut snapshot = self.capture_master_preset_snapshot(preset_id, name);
        snapshot.collapsed = collapsed;
        if let Some(existing) = self
            .state
            .master_presets
            .iter_mut()
            .find(|preset| preset.id == preset_id)
        {
            *existing = snapshot;
        }
        self.persist();
        self.status = format!("Updated mode {preset_id} from current toggles.");
    }

    fn apply_master_preset(&mut self, preset_id: u32) {
        let Some(preset) = self
            .state
            .master_presets
            .iter()
            .find(|preset| preset.id == preset_id)
            .cloned()
        else {
            return;
        };
        self.state.selected_master_preset_id = Some(preset_id);
        self.state.macros_master_enabled = preset.macros_master_enabled;
        self.state.window_expand_controls.enabled = preset.window_expand_controls_enabled;

        for item in &preset.window_presets {
            if let Some(window_preset) = self
                .state
                .window_presets
                .iter_mut()
                .find(|preset| preset.id == item.id)
            {
                window_preset.enabled = item.enabled;
                window_preset.animate_enabled = item.animate_enabled;
                window_preset.restore_titlebar_enabled = item.restore_titlebar_enabled;
            }
        }
        for item in &preset.window_focus_presets {
            if let Some(focus_preset) = self
                .state
                .window_focus_presets
                .iter_mut()
                .find(|preset| preset.id == item.id)
            {
                focus_preset.enabled = item.enabled;
            }
        }
        for item in &preset.zoom_presets {
            if let Some(zoom_preset) = self
                .state
                .zoom_presets
                .iter_mut()
                .find(|preset| preset.id == item.id)
            {
                zoom_preset.enabled = item.enabled;
            }
        }
        for group_state in &preset.macro_groups {
            if let Some(group) = self
                .state
                .macro_groups
                .iter_mut()
                .find(|group| group.id == group_state.id)
            {
                group.enabled = group_state.enabled;
                for preset_state in &group_state.presets {
                    if let Some(macro_preset) = group
                        .presets
                        .iter_mut()
                        .find(|preset| preset.id == preset_state.id)
                    {
                        macro_preset.enabled = preset_state.enabled;
                    }
                }
            }
        }
        self.sync_window_presets();
        self.sync_macro_presets();
        self.sync_macro_master_enabled();
        self.persist();
        self.status = format!("Applied mode: {}.", preset.name);
    }

    fn add_macro_group(&mut self) {
        let id = self.state.next_macro_group_id.max(1);
        self.state.next_macro_group_id = id + 1;
        let mut group = MacroGroup::new(id);
        let preset_id = self.state.next_macro_preset_id.max(1);
        self.state.next_macro_preset_id = preset_id + 1;
        group.presets = vec![MacroPreset::new(preset_id)];
        self.state.macro_groups.push(group);
        self.reconcile_master_presets();
        self.sync_macro_presets();
        self.status = format!("Added macro group {id}.");
    }

    fn add_macro_preset_to_group(&mut self, group_id: u32) {
        let id = self.state.next_macro_preset_id.max(1);
        self.state.next_macro_preset_id = id + 1;
        if let Some(group) = self
            .state
            .macro_groups
            .iter_mut()
            .find(|group| group.id == group_id)
        {
            group.presets.push(MacroPreset::new(id));
            self.reconcile_master_presets();
            self.sync_macro_presets();
            self.status = format!("Added macro preset {id}.");
        }
    }

    fn add_macro_folder(&mut self) {
        let id = self.state.next_macro_folder_id.max(1);
        self.state.next_macro_folder_id = id + 1;
        self.state.macro_folders.push(MacroFolder::new(id));
        self.status = format!("Added macro folder {id}.");
    }

    fn add_macro_group_to_folder(&mut self, folder_id: u32) {
        let id = self.state.next_macro_group_id.max(1);
        self.state.next_macro_group_id = id + 1;
        let mut group = MacroGroup::new(id);
        group.folder_id = Some(folder_id);
        let preset_id = self.state.next_macro_preset_id.max(1);
        self.state.next_macro_preset_id = preset_id + 1;
        group.presets = vec![MacroPreset::new(preset_id)];
        self.state.macro_groups.push(group);
        self.reconcile_master_presets();
        self.sync_macro_presets();
        self.status = format!("Added macro group {id} to folder.");
    }

    fn clone_macro_preset_with_new_id(&mut self, source: &MacroPreset) -> MacroPreset {
        let new_preset_id = self.state.next_macro_preset_id.max(1);
        self.state.next_macro_preset_id = new_preset_id + 1;
        let mut preset = source.clone();
        let old_preset_id = preset.id;
        preset.id = new_preset_id;
        preset.collapsed = true;
        Self::remap_macro_step_self_ref(&mut preset.hold_stop_step, old_preset_id, new_preset_id);
        for step in &mut preset.steps {
            Self::remap_macro_step_self_ref(step, old_preset_id, new_preset_id);
        }
        preset
    }

    fn remap_macro_step_self_ref(step: &mut MacroStep, old_preset_id: u32, new_preset_id: u32) {
        if matches!(
            step.action,
            MacroAction::TriggerMacroPreset
                | MacroAction::EnableMacroPreset
                | MacroAction::DisableMacroPreset
        ) && let Ok(id) = step.key.trim().parse::<u32>()
            && id == old_preset_id
        {
            step.key = new_preset_id.to_string();
        }
    }

    fn clone_macro_group_with_new_ids(
        &mut self,
        source_group: &MacroGroup,
        target_folder_id: Option<u32>,
    ) -> MacroGroup {
        let new_group_id = self.state.next_macro_group_id.max(1);
        self.state.next_macro_group_id = new_group_id + 1;

        let mut copied_group = source_group.clone();
        copied_group.id = new_group_id;
        copied_group.name = format!("{} Copy", copied_group.name);
        copied_group.folder_id = target_folder_id;

        let mut preset_id_map = HashMap::new();
        for preset in &mut copied_group.presets {
            let old_id = preset.id;
            let new_preset_id = self.state.next_macro_preset_id.max(1);
            self.state.next_macro_preset_id = new_preset_id + 1;
            preset.id = new_preset_id;
            preset.collapsed = true;
            preset_id_map.insert(old_id, new_preset_id);
        }

        for preset in &mut copied_group.presets {
            Self::remap_macro_step_group_refs(&mut preset.hold_stop_step, &preset_id_map);
            for step in &mut preset.steps {
                Self::remap_macro_step_group_refs(step, &preset_id_map);
            }
        }

        copied_group
    }

    fn remap_macro_step_group_refs(step: &mut MacroStep, preset_id_map: &HashMap<u32, u32>) {
        if matches!(
            step.action,
            MacroAction::TriggerMacroPreset
                | MacroAction::EnableMacroPreset
                | MacroAction::DisableMacroPreset
        ) && let Ok(old_id) = step.key.trim().parse::<u32>()
            && let Some(new_id) = preset_id_map.get(&old_id)
        {
            step.key = new_id.to_string();
        }
    }

    fn set_active_macro_folder_view(&mut self, folder_id: Option<u32>) {
        self.active_macro_folder_view = folder_id;
        self.selected_macro_groups.clear();
    }

    fn copy_selected_macro_groups(&mut self) {
        let mut ids = self
            .selected_macro_groups
            .iter()
            .copied()
            .collect::<Vec<_>>();
        ids.sort_unstable();
        self.macro_group_clipboard = ids;
        self.macro_group_clipboard_is_cut = false;
        self.status = format!(
            "Copied {} macro group(s).",
            self.macro_group_clipboard.len()
        );
    }

    fn copy_selected_macro_steps_for_preset(&mut self, group_id: u32, preset_id: u32) {
        let mut selected_indices = self
            .selected_macro_steps
            .iter()
            .filter_map(|(selected_group, selected_preset, selected_index)| {
                (*selected_group == group_id && *selected_preset == preset_id)
                    .then_some(*selected_index)
            })
            .collect::<Vec<_>>();
        selected_indices.sort_unstable();
        selected_indices.dedup();

        let Some(group) = self
            .state
            .macro_groups
            .iter()
            .find(|group| group.id == group_id)
        else {
            self.status = "Macro group not found.".to_owned();
            return;
        };
        let Some(preset) = group.presets.iter().find(|preset| preset.id == preset_id) else {
            self.status = "Macro preset not found.".to_owned();
            return;
        };

        self.macro_step_clipboard = selected_indices
            .into_iter()
            .filter_map(|step_index| preset.steps.get(step_index).cloned())
            .collect::<Vec<_>>();
        if self.macro_step_clipboard.is_empty() {
            self.status = "No selected steps to copy.".to_owned();
        } else {
            self.status = format!("Copied {} step(s).", self.macro_step_clipboard.len());
        }
    }

    fn paste_macro_steps_after(
        &mut self,
        group_id: u32,
        preset_id: u32,
        step_index: usize,
    ) -> Option<Vec<usize>> {
        if self.macro_step_clipboard.is_empty() {
            self.status = "No steps in clipboard.".to_owned();
            return None;
        }

        let Some(group) = self
            .state
            .macro_groups
            .iter_mut()
            .find(|group| group.id == group_id)
        else {
            self.status = "Macro group not found.".to_owned();
            return None;
        };
        let Some(preset) = group.presets.iter_mut().find(|preset| preset.id == preset_id) else {
            self.status = "Macro preset not found.".to_owned();
            return None;
        };

        let insert_at = (step_index + 1).min(preset.steps.len());
        let clipboard_steps = self.macro_step_clipboard.clone();
        let pasted_count = clipboard_steps.len();
        for (offset, step) in clipboard_steps.into_iter().enumerate() {
            preset.steps.insert(insert_at + offset, step);
        }
        self.status = format!("Pasted {} step(s).", pasted_count);
        Some((insert_at..insert_at + pasted_count).collect::<Vec<_>>())
    }

    fn cut_selected_macro_groups(&mut self) {
        let mut ids = self
            .selected_macro_groups
            .iter()
            .copied()
            .collect::<Vec<_>>();
        ids.sort_unstable();
        self.macro_group_clipboard = ids;
        self.macro_group_clipboard_is_cut = true;
        self.status = format!("Cut {} macro group(s).", self.macro_group_clipboard.len());
    }

    fn paste_macro_groups_into_folder(&mut self, target_folder_id: Option<u32>) {
        if self.macro_group_clipboard.is_empty() {
            self.status = "No macro groups in clipboard.".to_owned();
            return;
        }

        let clipboard_ids = self.macro_group_clipboard.clone();
        if self.macro_group_clipboard_is_cut {
            for group_id in clipboard_ids {
                if let Some(group) = self
                    .state
                    .macro_groups
                    .iter_mut()
                    .find(|group| group.id == group_id)
                {
                    group.folder_id = target_folder_id;
                }
            }
            self.macro_group_clipboard.clear();
            self.macro_group_clipboard_is_cut = false;
            self.status = "Moved macro group selection.".to_owned();
        } else {
            let sources = clipboard_ids
                .iter()
                .filter_map(|group_id| {
                    self.state
                        .macro_groups
                        .iter()
                        .find(|group| group.id == *group_id)
                        .cloned()
                })
                .collect::<Vec<_>>();
            for source in &sources {
                let copied_group = self.clone_macro_group_with_new_ids(source, target_folder_id);
                self.state.macro_groups.push(copied_group);
            }
            self.status = format!("Pasted {} macro group copy(s).", sources.len());
        }

        self.reconcile_master_presets();
        self.sync_macro_presets();
        self.persist_macro_presets();
    }

    fn remove_selected_macro_groups(&mut self) {
        if self.selected_macro_groups.is_empty() {
            self.status = "No macro groups selected.".to_owned();
            return;
        }
        let selected = self.selected_macro_groups.clone();
        self.state
            .macro_groups
            .retain(|group| !selected.contains(&group.id));
        self.selected_macro_groups.clear();
        self.macro_group_clipboard
            .retain(|group_id| !selected.contains(group_id));
        self.reconcile_master_presets();
        self.sync_macro_presets();
        self.persist_macro_presets();
        self.status = "Removed selected macro groups.".to_owned();
    }

    fn begin_capture(&mut self, target: CaptureRequest, status: String) {
        self.capture_target = Some(target.clone());
        self.capture_ignored_keys = self.snapshot_pressed_capture_keys();
        self.capture_ignored_keys
            .extend([0x01, 0x02, 0x04, 0x05, 0x06]);
        self.capture_suppress_next_poll = false;
        self.capture_wait_for_mouse_release = true;
        self.capture_ignore_mouse_until_release = true;
        self.capture_suppress_polls_remaining = 0;
        self.capture_mouse_guard_until = None;
        self.status = if self.capture_request_keeps_open(&target) {
            match self.state.ui_language {
                UiLanguage::Vietnamese => {
                    "Đang bắt nhiều key. Bấm thêm key hoặc Esc để dừng.".to_owned()
                }
                _ => "Capturing multiple keys. Press more keys or Esc to finish.".to_owned(),
            }
        } else {
            status
        };
    }

    fn capture_request_keeps_open(&self, target: &CaptureRequest) -> bool {
        match target {
            CaptureRequest::MacroPresetHotkey(_, _) => true,
            CaptureRequest::MacroPresetReleaseWaitKey(_, _) => true,
            CaptureRequest::MacroPresetHoldStopInput(group_id, preset_id) => self
                .state
                .macro_groups
                .iter()
                .find(|group| group.id == *group_id)
                .and_then(|group| group.presets.iter().find(|preset| preset.id == *preset_id))
                .is_some_and(|preset| {
                    matches!(
                        preset.hold_stop_step.action,
                        MacroAction::LockKeys | MacroAction::UnlockKeys
                    )
                }),
            CaptureRequest::MacroStepInput {
                group_id,
                preset_id,
                step_index,
            } => self
                .state
                .macro_groups
                .iter()
                .find(|group| group.id == *group_id)
                .and_then(|group| group.presets.iter().find(|preset| preset.id == *preset_id))
                .and_then(|preset| preset.steps.get(*step_index))
                .is_some_and(|step| {
                    matches!(step.action, MacroAction::LockKeys | MacroAction::UnlockKeys)
                }),
            _ => false,
        }
    }

    fn capture_request_accepts_mouse(&self, target: &CaptureRequest) -> bool {
        let _ = target;
        false
    }

    fn cancel_capture(&mut self) {
        self.capture_target = None;
        self.capture_suppress_next_poll = false;
        self.capture_wait_for_mouse_release = true;
        self.capture_ignore_mouse_until_release = true;
        self.capture_suppress_polls_remaining = 0;
        self.capture_mouse_guard_until = None;
        self.status = "Capture cancelled.".to_owned();
    }

    fn begin_mouse_move_absolute_capture(
        &mut self,
        ctx: &egui::Context,
        target: MouseMoveAbsoluteCaptureTarget,
    ) {
        self.mouse_move_absolute_capture_target = Some(target);
        self.mouse_move_absolute_capture_wait_for_mouse_release = true;
        let viewport = ctx.input(|input| input.viewport().clone());
        self.mouse_move_absolute_restore_inner_size = viewport
            .inner_rect
            .map(|rect| rect.size())
            .or(Some(Self::desired_window_size()));
        self.mouse_move_absolute_restore_outer_pos = viewport.outer_rect.map(|rect| rect.min);
        self.center_window_next_frame = false;
        self.enforce_square_window_frames = 0;
        let (left, top, width, height) = window_list::virtual_screen_bounds();
        let ppp = ctx.pixels_per_point().max(0.5);
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
            left as f32 / ppp,
            top as f32 / ppp,
        )));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(
            width as f32 / ppp,
            height as f32 / ppp,
        )));
        self.status = Self::tr_lang(
            self.state.ui_language,
            "Click anywhere on screen to capture X/Y. Press Esc to cancel.",
            "Bấm vào bất kỳ vị trí nào trên màn hình để lấy X/Y. Nhấn Esc để hủy.",
        )
        .to_owned();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
            egui::UserAttentionType::Informational,
        ));
        ctx.request_repaint();
    }

    fn cancel_mouse_move_absolute_capture(&mut self, ctx: &egui::Context) {
        if self.mouse_move_absolute_capture_target.is_none() {
            return;
        }
        self.mouse_move_absolute_capture_target = None;
        self.mouse_move_absolute_capture_wait_for_mouse_release = false;
        self.restore_mouse_move_absolute_viewport(ctx);
        self.mouse_move_absolute_capture_raise_window = true;
        self.status = Self::tr_lang(
            self.state.ui_language,
            "Mouse position capture cancelled.",
            "Đã hủy bắt tọa độ chuột.",
        )
        .to_owned();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
            egui::UserAttentionType::Informational,
        ));
        ctx.request_repaint();
    }

    fn finish_mouse_move_absolute_capture(
        &mut self,
        ctx: &egui::Context,
        target: MouseMoveAbsoluteCaptureTarget,
        screen_x: i32,
        screen_y: i32,
    ) {
        let Some(step) = self
            .state
            .macro_groups
            .iter_mut()
            .find(|group| group.id == target.group_id)
            .and_then(|group| {
                group
                    .presets
                    .iter_mut()
                    .find(|preset| preset.id == target.preset_id)
            })
            .and_then(|preset| preset.steps.get_mut(target.step_index))
        else {
            self.cancel_mouse_move_absolute_capture(ctx);
            self.status = Self::tr_lang(
                self.state.ui_language,
                "Mouse position capture target was not found.",
                "Không tìm thấy step để bắt tọa độ chuột.",
            )
            .to_owned();
            return;
        };

        step.x = screen_x;
        step.y = screen_y;
        step.action = MacroAction::MouseMoveAbsolute;
        self.mouse_move_absolute_capture_target = None;
        self.mouse_move_absolute_capture_wait_for_mouse_release = false;
        self.restore_mouse_move_absolute_viewport(ctx);
        self.mouse_move_absolute_capture_raise_window = true;
        self.status = match self.state.ui_language {
            UiLanguage::Vietnamese => {
                format!("Đã lấy tọa độ chuột {}, {}.", screen_x, screen_y)
            }
            _ => format!("Captured mouse position {}, {}.", screen_x, screen_y),
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
            egui::UserAttentionType::Informational,
        ));
        ctx.request_repaint();
        self.persist();
        self.sync_macro_presets();
    }

    fn restore_mouse_move_absolute_viewport(&mut self, ctx: &egui::Context) {
        if let Some(size) = self.mouse_move_absolute_restore_inner_size.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        }
        if let Some(pos) = self.mouse_move_absolute_restore_outer_pos.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        }
    }

    #[cfg(windows)]
    fn poll_mouse_move_absolute_capture(&mut self, ctx: &egui::Context) {
        let Some(target) = self.mouse_move_absolute_capture_target else {
            return;
        };
        ctx.request_repaint_after(Duration::from_millis(16));
        if Self::is_vk_down(0x1B) {
            self.cancel_mouse_move_absolute_capture(ctx);
            return;
        }
        if self.mouse_move_absolute_capture_wait_for_mouse_release {
            if Self::is_vk_down(0x01) {
                return;
            }
            self.mouse_move_absolute_capture_wait_for_mouse_release = false;
            ctx.request_repaint();
            return;
        }
        if !Self::is_vk_down(0x01) {
            return;
        }
        let mut point = POINT::default();
        if unsafe { GetCursorPos(&mut point) }.is_ok() {
            self.finish_mouse_move_absolute_capture(ctx, target, point.x, point.y);
        }
    }

    #[cfg(not(windows))]
    fn poll_mouse_move_absolute_capture(&mut self, _ctx: &egui::Context) {}

    fn pick_point_button_text(language: UiLanguage, active: bool) -> RichText {
        if active {
            RichText::new(Self::tr_lang(language, "Picking...", "Đang chọn..."))
                .strong()
                .color(Color32::from_rgb(255, 232, 96))
        } else {
            RichText::new(Self::tr_lang(language, "Pick", "Pick"))
        }
    }

    fn image_search_template_file_for_preset(&self, preset_id: u32) -> PathBuf {
        self.paths.image_search_template_file_for(preset_id)
    }

    fn begin_image_search_capture(
        &mut self,
        ctx: &egui::Context,
        target: ImageSearchCaptureTarget,
        mode: ImageSearchCaptureMode,
    ) {
        if self.image_search_capture_active {
            return;
        }
        self.image_search_capture_target = Some(target);
        self.image_search_capture_mode = Some(mode);
        let viewport = ctx.input(|input| input.viewport().clone());
        self.image_search_restore_inner_size = viewport
            .inner_rect
            .map(|rect| rect.size())
            .or(Some(Self::desired_window_size()));
        self.image_search_restore_outer_pos = viewport.outer_rect.map(|rect| rect.min);
        self.enforce_square_window_frames = 0;
        self.center_window_next_frame = false;
        let (left, top, width, height) = window_list::virtual_screen_bounds();
        let ppp = ctx.pixels_per_point().max(0.5);
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
            left as f32 / ppp,
            top as f32 / ppp,
        )));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(
            width as f32 / ppp,
            height as f32 / ppp,
        )));
        self.image_search_capture_active = true;
        self.image_search_capture_anchor = None;
        self.image_search_capture_current = None;
        self.status = match mode {
            ImageSearchCaptureMode::Template => {
                "Drag on screen to pick an image template. Press Esc to cancel.".to_owned()
            }
            ImageSearchCaptureMode::SearchRegion => {
                "Drag on screen to pick the image search area. Press Esc to cancel.".to_owned()
            }
            ImageSearchCaptureMode::ColorSample => {
                "Click a pixel on screen to pick a target color. Press Esc to cancel.".to_owned()
            }
            ImageSearchCaptureMode::ColorPriorityAnchor => {
                "Click a point on screen to set the color priority anchor. Press Esc to cancel."
                    .to_owned()
            }
        };
        ctx.request_repaint();
    }

    fn cancel_image_search_capture(&mut self, ctx: &egui::Context) {
        if !self.image_search_capture_active {
            return;
        }
        let mode = self
            .image_search_capture_mode
            .unwrap_or(ImageSearchCaptureMode::Template);
        self.image_search_capture_active = false;
        self.image_search_capture_target = None;
        self.image_search_capture_mode = None;
        self.image_search_capture_anchor = None;
        self.image_search_capture_current = None;
        self.image_search_color_pick_preview_color = None;
        self.restore_image_search_viewport(ctx);
        self.status = match mode {
            ImageSearchCaptureMode::Template => "Image template capture cancelled.".to_owned(),
            ImageSearchCaptureMode::SearchRegion => {
                "Image search area capture cancelled.".to_owned()
            }
            ImageSearchCaptureMode::ColorSample => "Image color pick cancelled.".to_owned(),
            ImageSearchCaptureMode::ColorPriorityAnchor => {
                "Image priority point capture cancelled.".to_owned()
            }
        };
        ctx.request_repaint();
    }

    fn image_search_capture_target_name(&self, target: ImageSearchCaptureTarget) -> Option<String> {
        match target {
            ImageSearchCaptureTarget::Preset(preset_id) => self
                .state
                .image_search_presets
                .iter()
                .find(|preset| preset.id == preset_id)
                .map(|preset| preset.name.clone()),
            ImageSearchCaptureTarget::TimingPreset(preset_id) => self
                .state
                .image_search_timing_presets
                .iter()
                .find(|preset| preset.id == preset_id)
                .map(|preset| preset.name.clone()),
        }
    }

    fn image_search_capture_target_is_circle(&self, target: ImageSearchCaptureTarget) -> bool {
        match target {
            ImageSearchCaptureTarget::Preset(preset_id) => self
                .state
                .image_search_presets
                .iter()
                .find(|preset| preset.id == preset_id)
                .is_some_and(|preset| preset.search_region_is_circle),
            ImageSearchCaptureTarget::TimingPreset(preset_id) => self
                .state
                .image_search_timing_presets
                .iter()
                .find(|preset| preset.id == preset_id)
                .is_some_and(|preset| preset.search_region_is_circle),
        }
    }

    fn restore_image_search_viewport(&mut self, ctx: &egui::Context) {
        if let Some(size) = self.image_search_restore_inner_size.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        }
        if let Some(pos) = self.image_search_restore_outer_pos.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        }
    }

    fn finish_image_search_capture(&mut self, ctx: &egui::Context, rect: egui::Rect) {
        let Some(target) = self.image_search_capture_target else {
            self.cancel_image_search_capture(ctx);
            self.status = "No image search preset is active.".to_owned();
            return;
        };
        let mode = self
            .image_search_capture_mode
            .unwrap_or(ImageSearchCaptureMode::Template);

        self.image_search_capture_active = false;
        self.image_search_capture_target = None;
        self.image_search_capture_mode = None;
        self.image_search_capture_anchor = None;
        self.image_search_capture_current = None;
        match mode {
            ImageSearchCaptureMode::Template => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                let _ = self.overlay_tx.send(OverlayCommand::SetUiVisible(false));
                std::thread::sleep(Duration::from_millis(70));
                let capture =
                    self.capture_screen_region_from_rect(ctx, rect, ctx.pixels_per_point());
                self.restore_image_search_viewport(ctx);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                let _ = self.overlay_tx.send(OverlayCommand::SetUiVisible(true));

                let Some(capture) = capture else {
                    self.status = "Failed to capture the selected screen area.".to_owned();
                    ctx.request_repaint();
                    return;
                };

                let (status, sync_required) = match target {
                    ImageSearchCaptureTarget::Preset(preset_id) => {
                        let template_file = self.image_search_template_file_for_preset(preset_id);
                        if let Some(parent) = template_file.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        let save_result = image::save_buffer(
                            &template_file,
                            &capture.rgba,
                            capture.width as u32,
                            capture.height as u32,
                            image::ColorType::Rgba8,
                        );

                        if let Some(preset) = self
                            .state
                            .image_search_presets
                            .iter_mut()
                            .find(|preset| preset.id == preset_id)
                        {
                            preset.enabled = true;
                            preset.collapsed = false;
                            preset.last_capture_screen_x = Some(capture.screen_x);
                            preset.last_capture_screen_y = Some(capture.screen_y);
                        }
                        self.image_search_preview_cache.remove(&preset_id);
                        (
                            match save_result {
                                Ok(()) => format!(
                                    "Saved template {}x{} for preset #{}.",
                                    capture.width, capture.height, preset_id
                                ),
                                Err(error) => {
                                    format!("Captured template but could not save it: {error}")
                                }
                            },
                            true,
                        )
                    }
                    ImageSearchCaptureTarget::TimingPreset(preset_id) => {
                        if let Some(preset) = self
                            .state
                            .image_search_timing_presets
                            .iter_mut()
                            .find(|preset| preset.id == preset_id)
                        {
                            preset.enabled = true;
                            preset.collapsed = false;
                            preset.search_region_screen_x = Some(capture.screen_x);
                            preset.search_region_screen_y = Some(capture.screen_y);
                            preset.search_region_width = Some(capture.width as i32);
                            preset.search_region_height = Some(capture.height as i32);
                        }
                        (
                            format!(
                                "Saved timing area {}x{} at {}, {} for preset #{}.",
                                capture.width,
                                capture.height,
                                capture.screen_x,
                                capture.screen_y,
                                preset_id
                            ),
                            true,
                        )
                    }
                };
                if sync_required {
                    self.sync_image_search_presets();
                    self.sync_image_search_timing_presets();
                    self.persist();
                }
                self.status = status;
                ctx.request_repaint();
            }
            ImageSearchCaptureMode::SearchRegion => {
                let region = self.screen_region_from_rect(ctx, rect, ctx.pixels_per_point());
                self.restore_image_search_viewport(ctx);
                if let Some((screen_x, screen_y, width, height)) = region {
                    match target {
                        ImageSearchCaptureTarget::Preset(preset_id) => {
                            if let Some(preset) = self
                                .state
                                .image_search_presets
                                .iter_mut()
                                .find(|preset| preset.id == preset_id)
                            {
                                preset.collapsed = false;
                                preset.search_region_screen_x = Some(screen_x);
                                preset.search_region_screen_y = Some(screen_y);
                                preset.search_region_width = Some(width);
                                preset.search_region_height = Some(height);
                            }
                            self.sync_image_search_presets();
                            self.persist();
                            self.status = format!(
                                "Saved search area {}x{} at {}, {} for preset #{}.",
                                width, height, screen_x, screen_y, preset_id
                            );
                        }
                        ImageSearchCaptureTarget::TimingPreset(preset_id) => {
                            if let Some(preset) = self
                                .state
                                .image_search_timing_presets
                                .iter_mut()
                                .find(|preset| preset.id == preset_id)
                            {
                                preset.collapsed = false;
                                preset.search_region_screen_x = Some(screen_x);
                                preset.search_region_screen_y = Some(screen_y);
                                preset.search_region_width = Some(width);
                                preset.search_region_height = Some(height);
                            }
                            self.sync_image_search_timing_presets();
                            self.persist();
                            self.status = format!(
                                "Saved timing area {}x{} at {}, {} for preset #{}.",
                                width, height, screen_x, screen_y, preset_id
                            );
                        }
                    }
                } else {
                    self.status = "Failed to save the selected search area.".to_owned();
                }
                ctx.request_repaint();
            }
            ImageSearchCaptureMode::ColorSample => {
                let center = rect.center();
                self.finish_image_search_color_pick(ctx, center);
            }
            ImageSearchCaptureMode::ColorPriorityAnchor => {
                let center = rect.center();
                self.finish_image_search_color_priority_anchor_pick(ctx, center);
            }
        }
    }

    fn capture_screen_region_from_rect(
        &self,
        ctx: &egui::Context,
        rect: egui::Rect,
        pixels_per_point: f32,
    ) -> Option<window_list::ScreenCaptureFrame> {
        let (capture_left, capture_top, capture_width, capture_height) =
            self.screen_region_from_rect(ctx, rect, pixels_per_point)?;
        window_list::capture_virtual_screen_region(
            capture_left,
            capture_top,
            capture_width,
            capture_height,
        )
    }

    fn screen_point_from_pos(
        &self,
        ctx: &egui::Context,
        pos: egui::Pos2,
        pixels_per_point: f32,
    ) -> Option<(i32, i32)> {
        let (left, top, _width, _height) = window_list::virtual_screen_bounds();
        let scale = pixels_per_point.max(0.5);
        let viewport_origin = ctx
            .input(|input| input.viewport().inner_rect.map(|viewport| viewport.min))
            .unwrap_or_else(|| egui::pos2(left as f32 / scale, top as f32 / scale));
        Some((
            ((viewport_origin.x + pos.x) * scale).round() as i32,
            ((viewport_origin.y + pos.y) * scale).round() as i32,
        ))
    }

    fn screen_region_from_rect(
        &self,
        ctx: &egui::Context,
        rect: egui::Rect,
        pixels_per_point: f32,
    ) -> Option<(i32, i32, i32, i32)> {
        let (left, top, _width, _height) = window_list::virtual_screen_bounds();
        let min = rect.min;
        let max = rect.max;
        let scale = pixels_per_point.max(0.5);
        let viewport_origin = ctx
            .input(|input| input.viewport().inner_rect.map(|viewport| viewport.min))
            .unwrap_or_else(|| egui::pos2(left as f32 / scale, top as f32 / scale));
        let capture_left = ((viewport_origin.x + min.x) * scale).round() as i32;
        let capture_top = ((viewport_origin.y + min.y) * scale).round() as i32;
        let capture_width = ((max.x - min.x).abs() * scale).round().max(1.0) as i32;
        let capture_height = ((max.y - min.y).abs() * scale).round().max(1.0) as i32;
        Some((capture_left, capture_top, capture_width, capture_height))
    }

    fn finish_image_search_color_pick(&mut self, ctx: &egui::Context, pos: egui::Pos2) {
        let Some(target) = self.image_search_capture_target else {
            self.cancel_image_search_capture(ctx);
            self.status = "No image search preset is active.".to_owned();
            return;
        };

        self.image_search_capture_active = false;
        self.image_search_capture_target = None;
        self.image_search_capture_mode = None;
        self.image_search_capture_anchor = None;
        self.image_search_capture_current = None;
        self.image_search_color_pick_preview_color = None;

        let screen_point = self.screen_point_from_pos(ctx, pos, ctx.pixels_per_point());
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        let _ = self.overlay_tx.send(OverlayCommand::SetUiVisible(false));
        std::thread::sleep(Duration::from_millis(70));
        let capture = screen_point.and_then(|(screen_x, screen_y)| {
            window_list::capture_virtual_screen_region(screen_x, screen_y, 1, 1)
        });
        self.restore_image_search_viewport(ctx);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        let _ = self.overlay_tx.send(OverlayCommand::SetUiVisible(true));

        let Some(capture) = capture else {
            self.status = "Failed to sample the selected screen color.".to_owned();
            ctx.request_repaint();
            return;
        };
        if capture.rgba.len() < 4 {
            self.status = "Failed to read the selected screen color.".to_owned();
            ctx.request_repaint();
            return;
        }

        let color = RgbaColor {
            r: capture.rgba[0],
            g: capture.rgba[1],
            b: capture.rgba[2],
            a: 255,
        };
        let status = match target {
            ImageSearchCaptureTarget::Preset(preset_id) => {
                if let Some(preset) = self
                    .state
                    .image_search_presets
                    .iter_mut()
                    .find(|preset| preset.id == preset_id)
                {
                    preset.collapsed = false;
                    preset.use_color_matching = true;
                    if preset.target_colors.is_empty() {
                        if let Some(existing) = preset.target_color {
                            preset.target_colors.push(existing);
                        }
                    }
                    preset.target_colors.push(color);
                    preset.target_color = preset.target_colors.first().copied();
                }
                self.sync_image_search_presets();
                format!(
                    "Picked color #{:02X}{:02X}{:02X} for preset #{}.",
                    color.r, color.g, color.b, preset_id
                )
            }
            ImageSearchCaptureTarget::TimingPreset(preset_id) => {
                if let Some(preset) = self
                    .state
                    .image_search_timing_presets
                    .iter_mut()
                    .find(|preset| preset.id == preset_id)
                {
                    preset.collapsed = false;
                    if preset.target_colors.is_empty() {
                        if let Some(existing) = preset.target_color {
                            preset.target_colors.push(existing);
                        }
                    }
                    preset.target_colors.push(color);
                    preset.target_color = preset.target_colors.first().copied();
                }
                self.sync_image_search_timing_presets();
                format!(
                    "Picked color #{:02X}{:02X}{:02X} for timing preset #{}.",
                    color.r, color.g, color.b, preset_id
                )
            }
        };
        self.persist();
        self.status = status;
        ctx.request_repaint();
    }

    fn finish_image_search_color_priority_anchor_pick(
        &mut self,
        ctx: &egui::Context,
        pos: egui::Pos2,
    ) {
        let Some(target) = self.image_search_capture_target else {
            self.cancel_image_search_capture(ctx);
            self.status = "No image search preset is active.".to_owned();
            return;
        };

        self.image_search_capture_active = false;
        self.image_search_capture_target = None;
        self.image_search_capture_mode = None;
        self.image_search_capture_anchor = None;
        self.image_search_capture_current = None;
        self.image_search_color_pick_preview_color = None;

        let screen_point = self.screen_point_from_pos(ctx, pos, ctx.pixels_per_point());
        self.restore_image_search_viewport(ctx);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        let _ = self.overlay_tx.send(OverlayCommand::SetUiVisible(true));

        let Some((screen_x, screen_y)) = screen_point else {
            self.status = "Failed to read the selected priority point.".to_owned();
            ctx.request_repaint();
            return;
        };

        match target {
            ImageSearchCaptureTarget::Preset(preset_id) => {
                if let Some(preset) = self
                    .state
                    .image_search_presets
                    .iter_mut()
                    .find(|preset| preset.id == preset_id)
                {
                    preset.color_priority_from_anchor = true;
                    preset.color_priority_anchor_screen_x = Some(screen_x);
                    preset.color_priority_anchor_screen_y = Some(screen_y);
                    preset.collapsed = false;
                }
                self.sync_image_search_presets();
                self.persist();
                self.status = format!(
                    "Saved priority point at {}, {} for preset #{}.",
                    screen_x, screen_y, preset_id
                );
            }
            ImageSearchCaptureTarget::TimingPreset(_) => {
                self.status = "Priority point capture is not used for timing presets.".to_owned();
            }
        }
        ctx.request_repaint();
    }

    fn apply_captured_input(&mut self, target: CaptureRequest, captured: CapturedInput) -> bool {
        let keep_capture_open = self.capture_request_keeps_open(&target);
        match (target, captured) {
            (CaptureRequest::WindowPresetHotkey(preset_id), CapturedInput::Binding(binding)) => {
                if let Some(preset) = self
                    .state
                    .window_presets
                    .iter_mut()
                    .find(|preset| preset.id == preset_id)
                {
                    preset.hotkey = Some(binding);
                    self.status = format!("Captured hotkey for {}.", preset.name);
                }
                self.sync_window_presets();
            }
            (
                CaptureRequest::WindowFocusPresetHotkey(preset_id),
                CapturedInput::Binding(binding),
            ) => {
                if let Some(preset) = self
                    .state
                    .window_focus_presets
                    .iter_mut()
                    .find(|preset| preset.id == preset_id)
                {
                    preset.hotkey = Some(binding);
                    self.status = format!("Captured focus hotkey for {}.", preset.name);
                }
                self.sync_window_presets();
            }
            (
                CaptureRequest::WindowPresetAnimateHotkey(preset_id),
                CapturedInput::Binding(binding),
            ) => {
                if let Some(preset) = self
                    .state
                    .window_presets
                    .iter_mut()
                    .find(|preset| preset.id == preset_id)
                {
                    preset.animate_hotkey = Some(binding);
                    self.status = format!("Captured animated hotkey for {}.", preset.name);
                }
                self.sync_window_presets();
            }
            (
                CaptureRequest::WindowPresetTitlebarHotkey(preset_id),
                CapturedInput::Binding(binding),
            ) => {
                if let Some(preset) = self
                    .state
                    .window_presets
                    .iter_mut()
                    .find(|preset| preset.id == preset_id)
                {
                    preset.titlebar_hotkey = Some(binding);
                    self.status = format!("Captured restore title bar hotkey for {}.", preset.name);
                }
                self.sync_window_presets();
            }
            (CaptureRequest::WindowExpandHotkey(direction), CapturedInput::Binding(binding)) => {
                let controls = &mut self.state.window_expand_controls;
                match direction {
                    WindowExpandDirection::Up => controls.up = Some(binding),
                    WindowExpandDirection::Down => controls.down = Some(binding),
                    WindowExpandDirection::Left => controls.left = Some(binding),
                    WindowExpandDirection::Right => controls.right = Some(binding),
                }
                self.sync_window_presets();
                self.status = "Captured window expand hotkey.".to_owned();
            }
            (CaptureRequest::ZoomPresetHotkey(preset_id), CapturedInput::Binding(binding)) => {
                if let Some(preset) = self
                    .state
                    .zoom_presets
                    .iter_mut()
                    .find(|preset| preset.id == preset_id)
                {
                    preset.hotkey = Some(binding);
                    self.status = format!("Captured zoom hotkey for {}.", preset.name);
                }
                self.sync_window_presets();
            }
            (
                CaptureRequest::ImageSearchPresetHotkey(preset_id),
                CapturedInput::Binding(binding),
            ) => {
                if let Some(preset) = self
                    .state
                    .image_search_presets
                    .iter_mut()
                    .find(|preset| preset.id == preset_id)
                {
                    preset.hotkey = Some(binding);
                    self.status = format!("Captured image search hotkey for {}.", preset.name);
                }
                self.sync_image_search_presets();
                self.persist();
            }
            (CaptureRequest::PinPresetHotkey(preset_id), CapturedInput::Binding(binding)) => {
                if let Some(preset) = self
                    .state
                    .pin_presets
                    .iter_mut()
                    .find(|preset| preset.id == preset_id)
                {
                    preset.hotkey = Some(binding);
                    self.status = format!("Captured pin hotkey for {}.", preset.name);
                }
                self.sync_window_presets();
            }
            (CaptureRequest::MousePathRecordHotkey(preset_id), CapturedInput::Binding(binding)) => {
                if let Some(preset) = self
                    .state
                    .mouse_path_presets
                    .iter_mut()
                    .find(|preset| preset.id == preset_id)
                {
                    preset.record_hotkey = Some(binding);
                    self.status = format!("Captured record hotkey for {}.", preset.name);
                }
                self.sync_window_presets();
            }
            (
                CaptureRequest::MouseSensitivityPresetHotkey(preset_id),
                CapturedInput::Binding(binding),
            ) => {
                if let Some(preset) = self
                    .state
                    .mouse_sensitivity_presets
                    .iter_mut()
                    .find(|preset| preset.id == preset_id)
                {
                    preset.hotkey = Some(binding);
                    self.status = format!("Captured mouse sensitivity hotkey for {}.", preset.name);
                }
                self.persist_mouse_sensitivity_presets();
            }
            (
                CaptureRequest::MacroPresetHotkey(group_id, preset_id),
                CapturedInput::Binding(binding),
            ) => {
                if let Some(preset) = self
                    .state
                    .macro_groups
                    .iter_mut()
                    .find(|group| group.id == group_id)
                    .and_then(|group| {
                        group
                            .presets
                            .iter_mut()
                            .find(|preset| preset.id == preset_id)
                    })
                {
                    let key = binding.key.trim().to_owned();
                    let existing = preset
                        .trigger_keys
                        .split(',')
                        .map(str::trim)
                        .filter(|part| !part.is_empty())
                        .map(str::to_owned)
                        .collect::<Vec<_>>();
                    if existing.iter().any(|part| part.eq_ignore_ascii_case(&key)) {
                        self.status = format!("Key {key} is already in that trigger list.");
                    } else if existing.is_empty() {
                        preset.trigger_keys = key.clone();
                        self.status = format!("Captured trigger key for macro {preset_id}.");
                    } else {
                        preset.trigger_keys = format!("{},{}", preset.trigger_keys.trim(), key);
                        self.status = format!("Added trigger key {key} for macro {preset_id}.");
                    }
                    preset.hotkey = None;
                }
                self.sync_macro_presets();
            }
            (
                CaptureRequest::MacroPresetReleaseWaitKey(group_id, preset_id),
                CapturedInput::Binding(binding),
            ) => {
                if let Some(preset) = self
                    .state
                    .macro_groups
                    .iter_mut()
                    .find(|group| group.id == group_id)
                    .and_then(|group| {
                        group
                            .presets
                            .iter_mut()
                            .find(|preset| preset.id == preset_id)
                    })
                {
                    let key = binding.key.trim().to_owned();
                    let existing = preset
                        .release_wait_key
                        .split(',')
                        .map(str::trim)
                        .filter(|part| !part.is_empty())
                        .map(str::to_owned)
                        .collect::<Vec<_>>();
                    if existing.iter().any(|part| part.eq_ignore_ascii_case(&key)) {
                        self.status = format!("Key {key} is already in that release wait list.");
                    } else if existing.is_empty() {
                        preset.release_wait_key = key.clone();
                        self.status = format!("Captured release wait key for macro {preset_id}.");
                    } else {
                        preset.release_wait_key =
                            format!("{},{}", preset.release_wait_key.trim(), key);
                        self.status =
                            format!("Added release wait key {key} for macro {preset_id}.");
                    }
                }
                self.sync_macro_presets();
            }
            (
                CaptureRequest::MacroPresetHoldStopInput(group_id, preset_id),
                CapturedInput::Binding(binding),
            ) => {
                if let Some(preset) = self
                    .state
                    .macro_groups
                    .iter_mut()
                    .find(|group| group.id == group_id)
                    .and_then(|group| {
                        group
                            .presets
                            .iter_mut()
                            .find(|preset| preset.id == preset_id)
                    })
                {
                    if matches!(
                        preset.hold_stop_step.action,
                        MacroAction::LockKeys | MacroAction::UnlockKeys
                    ) {
                        let key = binding.key;
                        let existing = preset
                            .hold_stop_step
                            .key
                            .split(',')
                            .map(str::trim)
                            .filter(|part| !part.is_empty())
                            .map(str::to_owned)
                            .collect::<Vec<_>>();
                        if existing.iter().any(|part| part.eq_ignore_ascii_case(&key)) {
                            self.status =
                                format!("Key {key} is already in that hold-stop lock list.");
                        } else if existing.is_empty() {
                            preset.hold_stop_step.key = key.clone();
                            self.status =
                                format!("Captured hold-stop key {key} for macro {preset_id}.");
                        } else {
                            preset.hold_stop_step.key =
                                format!("{},{}", preset.hold_stop_step.key.trim(), key);
                            self.status =
                                format!("Added hold-stop key {key} for macro {preset_id}.");
                        }
                    } else {
                        preset.hold_stop_step.key = binding.key.clone();
                        self.status = format!(
                            "Captured hold-stop input {} for macro {preset_id}.",
                            binding.key
                        );
                    }
                }
                self.sync_macro_presets();
            }
            (
                CaptureRequest::MacroStepInput {
                    group_id,
                    preset_id,
                    step_index,
                },
                CapturedInput::Binding(binding),
            ) => {
                if let Some(step) = self
                    .state
                    .macro_groups
                    .iter_mut()
                    .find(|group| group.id == group_id)
                    .and_then(|group| {
                        group
                            .presets
                            .iter_mut()
                            .find(|preset| preset.id == preset_id)
                    })
                    .and_then(|preset| preset.steps.get_mut(step_index))
                {
                    if matches!(step.action, MacroAction::LockKeys | MacroAction::UnlockKeys) {
                        let key = binding.key;
                        let existing = step
                            .key
                            .split(',')
                            .map(str::trim)
                            .filter(|part| !part.is_empty())
                            .map(str::to_owned)
                            .collect::<Vec<_>>();
                        if existing.iter().any(|part| part.eq_ignore_ascii_case(&key)) {
                            self.status = format!("Key {key} is already in that lock list.");
                        } else if existing.is_empty() {
                            step.key = key.clone();
                            self.status =
                                format!("Captured lock key {key} for preset {preset_id}.");
                        } else {
                            step.key = format!("{},{}", step.key.trim(), key);
                            self.status = format!("Added lock key {key} for preset {preset_id}.");
                        }
                    } else {
                        step.key = binding.key;
                        if step.action == MacroAction::MouseMoveAbsolute
                            || step.action == MacroAction::MouseMoveRelative
                        {
                            step.action = MacroAction::KeyPress;
                        }
                        self.status = format!("Captured step input for preset {preset_id}.");
                    }
                }
                self.sync_macro_presets();
            }
            (
                CaptureRequest::MacroStepInput {
                    group_id,
                    preset_id,
                    step_index,
                },
                CapturedInput::Step(mut captured_step),
            ) => {
                captured_step.delay_ms = 0;
                if let Some(step) = self
                    .state
                    .macro_groups
                    .iter_mut()
                    .find(|group| group.id == group_id)
                    .and_then(|group| {
                        group
                            .presets
                            .iter_mut()
                            .find(|preset| preset.id == preset_id)
                    })
                    .and_then(|preset| preset.steps.get_mut(step_index))
                {
                    step.key = captured_step.key;
                    step.action = captured_step.action;
                    step.x = captured_step.x;
                    step.y = captured_step.y;
                    self.status = format!("Captured step input for preset {preset_id}.");
                }
                self.sync_macro_presets();
            }
            _ => {
                self.status = "Capture type mismatch.".to_owned();
            }
        }
        self.persist();
        keep_capture_open
    }

    fn poll_capture_input(&mut self, ctx: &egui::Context) {
        if self
            .capture_mouse_guard_until
            .is_some_and(|until| Instant::now() < until)
        {
            return;
        }
        self.capture_mouse_guard_until = None;
        if self.capture_suppress_polls_remaining > 0 {
            self.capture_suppress_polls_remaining -= 1;
            return;
        }
        if self.capture_suppress_next_poll {
            self.capture_suppress_next_poll = false;
            return;
        }
        if self.capture_ignore_mouse_until_release {
            if Self::is_vk_down(0x01)
                || Self::is_vk_down(0x02)
                || Self::is_vk_down(0x04)
                || Self::is_vk_down(0x05)
                || Self::is_vk_down(0x06)
            {
                return;
            }
            self.capture_ignore_mouse_until_release = false;
            return;
        }
        let Some(target) = self.capture_target.clone() else {
            self.capture_ignored_keys.clear();
            return;
        };
        let Some(captured) = self.capture_next_input(ctx) else {
            return;
        };
        let keep_capture_open = self.apply_captured_input(target, CapturedInput::Binding(captured));
        if !keep_capture_open {
            self.capture_target = None;
            self.capture_ignored_keys.clear();
        }
    }

    #[cfg(windows)]
    fn capture_next_input(&mut self, ctx: &egui::Context) -> Option<crate::model::HotkeyBinding> {
        let accepts_mouse = self
            .capture_target
            .as_ref()
            .is_none_or(|target| self.capture_request_accepts_mouse(target));
        if self.capture_wait_for_mouse_release {
            if Self::is_vk_down(0x01)
                || Self::is_vk_down(0x02)
                || Self::is_vk_down(0x04)
                || Self::is_vk_down(0x05)
                || Self::is_vk_down(0x06)
            {
                return None;
            }
            if self
                .capture_target
                .as_ref()
                .is_some_and(|target| self.capture_request_accepts_mouse(target))
            {
                for mouse_vk in [0x01, 0x02, 0x04, 0x05, 0x06] {
                    self.capture_ignored_keys.remove(&mouse_vk);
                }
            }
            self.capture_wait_for_mouse_release = false;
            return None;
        }
        if accepts_mouse && let Some(binding) = self.capture_scroll_binding(ctx) {
            return Some(binding);
        }
        for vk in Self::capture_scan_keys() {
            if !accepts_mouse && Self::capture_mouse_vk(vk) {
                continue;
            }
            let pressed = unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 };
            if pressed {
                if self.capture_ignored_keys.contains(&vk) {
                    continue;
                }
                let key_name = hotkey::vk_to_key_name(vk)?.to_owned();
                self.capture_ignored_keys.insert(vk);
                let ctrl =
                    Self::is_vk_down(0x11) || Self::is_vk_down(0xA2) || Self::is_vk_down(0xA3);
                let alt =
                    Self::is_vk_down(0x12) || Self::is_vk_down(0xA4) || Self::is_vk_down(0xA5);
                let shift =
                    Self::is_vk_down(0x10) || Self::is_vk_down(0xA0) || Self::is_vk_down(0xA1);
                let win = Self::is_vk_down(0x5B) || Self::is_vk_down(0x5C);
                return Some(crate::model::HotkeyBinding {
                    ctrl: ctrl && !key_name.eq_ignore_ascii_case("Ctrl"),
                    alt: alt && !key_name.eq_ignore_ascii_case("Alt"),
                    shift: shift && !key_name.eq_ignore_ascii_case("Shift"),
                    win: win && !key_name.eq_ignore_ascii_case("Win"),
                    key: key_name,
                });
            }
            self.capture_ignored_keys.remove(&vk);
        }
        None
    }

    #[cfg(not(windows))]
    fn capture_next_input(&mut self, _ctx: &egui::Context) -> Option<crate::model::HotkeyBinding> {
        None
    }

    #[cfg(windows)]
    fn capture_scroll_binding(&self, ctx: &egui::Context) -> Option<crate::model::HotkeyBinding> {
        let scroll_y = ctx.input(|input| input.raw_scroll_delta.y);
        if scroll_y.abs() < 0.01 {
            return None;
        }
        Some(crate::model::HotkeyBinding {
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
            key: if scroll_y > 0.0 {
                "MouseWheelUp".to_owned()
            } else {
                "MouseWheelDown".to_owned()
            },
        })
    }

    #[cfg(windows)]
    fn capture_mouse_vk(vk: u32) -> bool {
        matches!(vk, 0x01 | 0x02 | 0x04 | 0x05 | 0x06)
    }

    #[cfg(not(windows))]
    fn capture_scroll_binding(&self, _ctx: &egui::Context) -> Option<crate::model::HotkeyBinding> {
        None
    }

    #[cfg(not(windows))]
    fn capture_mouse_vk(_vk: u32) -> bool {
        false
    }

    #[cfg(windows)]
    fn is_vk_down(vk: u32) -> bool {
        unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
    }

    #[cfg(windows)]
    fn snapshot_pressed_capture_keys(&self) -> HashSet<u32> {
        Self::capture_scan_keys()
            .into_iter()
            .filter(|vk| Self::is_vk_down(*vk))
            .collect()
    }

    #[cfg(not(windows))]
    fn snapshot_pressed_capture_keys(&self) -> HashSet<u32> {
        HashSet::new()
    }

    fn capture_scan_keys() -> Vec<u32> {
        let mut keys = Vec::new();
        keys.extend(0x08..=0x0D);
        keys.extend([0x01, 0x02, 0x04, 0x05, 0x06]);
        keys.extend(0x10..=0x14);
        keys.extend(0x1B..=0x28);
        keys.extend(0x2C..=0x2E);
        keys.extend(0x30..=0x39);
        keys.extend(0x41..=0x5D);
        keys.extend(0x60..=0x6F);
        keys.extend(0x70..=0x87);
        keys.extend([
            0x90, 0x91, 0xBA, 0xBB, 0xBC, 0xBD, 0xBE, 0xBF, 0xC0, 0xDB, 0xDC, 0xDD, 0xDE,
        ]);
        keys
    }

    fn persist_window_presets(&mut self) {
        self.sync_window_presets();
        self.persist();
    }

    fn persist_macro_presets(&mut self) {
        self.sync_macro_presets();
        self.sync_macro_master_enabled();
        self.persist();
    }

    fn persist_toolbox_presets(&mut self) {
        self.sync_toolbox_presets();
        self.persist();
    }

    fn persist_mouse_path_presets(&mut self) {
        self.sync_window_presets();
        self.persist();
    }

    fn persist_mouse_sensitivity_presets(&mut self) {
        self.sync_mouse_sensitivity_presets();
        self.persist();
    }

    #[allow(unreachable_code)]
    fn render_crosshair_panel(&mut self, ui: &mut egui::Ui) {
        self.render_crosshair_presets_panel(ui);
        return;
        ui.spacing_mut().slider_width = 260.0;
        let mut changed = false;
        Self::show_preset_card(ui, self.state.active_style.enabled, |ui| {
            ui.horizontal(|ui| {
                changed |= ui
                    .checkbox(&mut self.state.active_style.enabled, "Enabled")
                    .changed();
                if ui
                    .button(if self.crosshair_panel_collapsed {
                        "Show"
                    } else {
                        "Hide"
                    })
                    .clicked()
                {
                    self.crosshair_panel_collapsed = !self.crosshair_panel_collapsed;
                }
            });
        });
        if self.crosshair_panel_collapsed {
            if changed {
                self.sync_crosshair();
                self.persist();
            }
            return;
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.heading("Quick Controls");
                egui::Grid::new("crosshair-quick-controls")
                    .num_columns(2)
                    .spacing([14.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Actions");
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Center on screen").clicked() {
                                self.state.active_style.x_offset = 0;
                                self.state.active_style.y_offset = 0;
                                changed = true;
                            }
                        });
                        ui.end_row();
                    });

                ui.separator();
                ui.heading("Crosshair Presets");
                egui::Grid::new("crosshair-profiles")
                    .num_columns(2)
                    .spacing([14.0, 8.0])
                    .show(ui, |ui| {
                        let selected = self
                            .state
                            .selected_profile
                            .clone()
                            .unwrap_or_else(|| "Default".to_owned());
                        ui.label("Selected preset");
                        egui::ComboBox::from_id_salt("saved-crosshair-profiles")
                            .width(260.0)
                            .selected_text(selected)
                            .show_ui(ui, |ui| {
                                for profile in self.state.profiles.clone() {
                                    if ui
                                        .selectable_label(
                                            self.state.selected_profile.as_deref()
                                                == Some(&profile.name),
                                            &profile.name,
                                        )
                                        .clicked()
                                    {
                                        self.state.selected_profile = Some(profile.name.clone());
                                        self.state.active_style = profile.style.clone();
                                        self.state.active_style.enabled = profile.enabled;
                                        self.save_name = profile.name;
                                        changed = true;
                                    }
                                }
                            });
                        ui.end_row();

                        ui.label("Preset name");
                        ui.horizontal_wrapped(|ui| {
                            ui.add_sized([220.0, 24.0], TextEdit::singleline(&mut self.save_name));
                            if ui.button("+ New Preset").clicked() {
                                self.add_profile();
                            }
                            if ui.button("Save").clicked() {
                                self.save_profile();
                            }
                            if ui.button("Delete").clicked() {
                                self.delete_profile();
                            }
                        });
                        ui.end_row();
                    });

                ui.add_space(6.0);
                let dark_mode = self.state.ui_theme == UiThemeMode::Dark;
                for index in 0..self.state.profiles.len() {
                    let is_selected = self.state.selected_profile.as_deref()
                        == Some(self.state.profiles[index].name.as_str());
                    let mut activate = false;
                    let mut remove = false;
                    {
                        let preset = &mut self.state.profiles[index];
                        Self::show_preset_card(ui, preset.enabled, |ui| {
                            ui.horizontal(|ui| {
                                changed |= ui.checkbox(&mut preset.enabled, "").changed();
                                ui.label(Self::preset_title_text(
                                    dark_mode,
                                    &preset.name,
                                    preset.enabled,
                                ));
                                if is_selected {
                                    ui.label(RichText::new("Active").strong());
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("Delete").clicked() {
                                            remove = true;
                                        }
                                        if ui
                                            .button(if is_selected { "Current" } else { "Apply" })
                                            .clicked()
                                        {
                                            activate = true;
                                        }
                                    },
                                );
                            });
                        });
                    }
                    if activate {
                        let preset = self.state.profiles[index].clone();
                        self.state.selected_profile = Some(preset.name.clone());
                        self.state.active_style = preset.style;
                        self.state.active_style.enabled = preset.enabled;
                        self.save_name = preset.name;
                        changed = true;
                    }
                    if remove {
                        let remove_name = self.state.profiles[index].name.clone();
                        self.state
                            .profiles
                            .retain(|profile| profile.name != remove_name);
                        if self.state.profiles.is_empty() {
                            self.state.profiles.push(ProfileRecord::default());
                        }
                        let next = self.state.profiles[0].clone();
                        self.state.selected_profile = Some(next.name.clone());
                        self.state.active_style = next.style;
                        self.state.active_style.enabled = next.enabled;
                        self.save_name = next.name;
                        changed = true;
                        break;
                    }
                    ui.add_space(4.0);
                }

                ui.separator();
                ui.heading("Crosshair Settings");
                egui::Grid::new("crosshair-settings-grid")
                    .num_columns(2)
                    .spacing([14.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Horizontal length");
                        changed |= ui
                            .add_sized(
                                [340.0, 20.0],
                                Slider::new(
                                    &mut self.state.active_style.horizontal_length,
                                    1.0..=80.0,
                                ),
                            )
                            .changed();
                        ui.end_row();

                        ui.label("Vertical length");
                        changed |= ui
                            .add_sized(
                                [340.0, 20.0],
                                Slider::new(
                                    &mut self.state.active_style.vertical_length,
                                    1.0..=80.0,
                                ),
                            )
                            .changed();
                        ui.end_row();

                        ui.label("Thickness");
                        changed |= ui
                            .add_sized(
                                [340.0, 20.0],
                                Slider::new(&mut self.state.active_style.thickness, 1.0..=20.0),
                            )
                            .changed();
                        ui.end_row();

                        ui.label("Gap");
                        changed |= ui
                            .add_sized(
                                [340.0, 20.0],
                                Slider::new(&mut self.state.active_style.gap, 0.0..=48.0),
                            )
                            .changed();
                        ui.end_row();

                        ui.label("Horizontal offset");
                        changed |= ui
                            .add_sized(
                                [340.0, 20.0],
                                Slider::new(&mut self.state.active_style.x_offset, -1000..=1000),
                            )
                            .changed();
                        ui.end_row();

                        ui.label("Vertical offset");
                        changed |= ui
                            .add_sized(
                                [340.0, 20.0],
                                Slider::new(&mut self.state.active_style.y_offset, -1000..=1000),
                            )
                            .changed();
                        ui.end_row();

                        ui.label("Opacity");
                        changed |= ui
                            .add_sized(
                                [340.0, 20.0],
                                Slider::new(&mut self.state.active_style.opacity, 0.05..=1.0),
                            )
                            .changed();
                        ui.end_row();
                    });

                ui.separator();
                ui.heading("Outline and Center Dot");
                egui::Grid::new("crosshair-outline-grid")
                    .num_columns(2)
                    .spacing([14.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Outline");
                        changed |= ui
                            .checkbox(&mut self.state.active_style.outline_enabled, "Enabled")
                            .changed();
                        ui.end_row();

                        ui.label("Outline thickness");
                        changed |= ui
                            .add_sized(
                                [340.0, 20.0],
                                Slider::new(
                                    &mut self.state.active_style.outline_thickness,
                                    0.0..=8.0,
                                ),
                            )
                            .changed();
                        ui.end_row();

                        ui.label("Center dot");
                        changed |= ui
                            .checkbox(&mut self.state.active_style.center_dot, "Enabled")
                            .changed();
                        ui.end_row();

                        ui.label("Center dot size");
                        changed |= ui
                            .add_sized(
                                [340.0, 20.0],
                                Slider::new(
                                    &mut self.state.active_style.center_dot_size,
                                    1.0..=24.0,
                                ),
                            )
                            .changed();
                        ui.end_row();
                    });

                ui.separator();
                ui.heading("Colors");
                egui::Grid::new("crosshair-colors-grid")
                    .num_columns(2)
                    .spacing([14.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Crosshair color");
                        let mut crosshair_rgba = [
                            self.state.active_style.color.r,
                            self.state.active_style.color.g,
                            self.state.active_style.color.b,
                            self.state.active_style.color.a,
                        ];
                        if ui
                            .color_edit_button_srgba_unmultiplied(&mut crosshair_rgba)
                            .changed()
                        {
                            self.state.active_style.color.r = crosshair_rgba[0];
                            self.state.active_style.color.g = crosshair_rgba[1];
                            self.state.active_style.color.b = crosshair_rgba[2];
                            self.state.active_style.color.a = crosshair_rgba[3];
                            changed = true;
                        }
                        ui.end_row();

                        ui.label("Outline color");
                        let mut outline_rgba = [
                            self.state.active_style.outline_color.r,
                            self.state.active_style.outline_color.g,
                            self.state.active_style.outline_color.b,
                            self.state.active_style.outline_color.a,
                        ];
                        if ui
                            .color_edit_button_srgba_unmultiplied(&mut outline_rgba)
                            .changed()
                        {
                            self.state.active_style.outline_color.r = outline_rgba[0];
                            self.state.active_style.outline_color.g = outline_rgba[1];
                            self.state.active_style.outline_color.b = outline_rgba[2];
                            self.state.active_style.outline_color.a = outline_rgba[3];
                            changed = true;
                        }
                        ui.end_row();
                    });
            });

        if changed {
            self.sync_crosshair();
            self.persist();
        }
    }

    fn startup_splash_progress(&mut self, ctx: &egui::Context) -> Option<f32> {
        if self.startup_splash.duration_sec <= 0.0 {
            return None;
        }
        let now = ctx.input(|input| input.time);
        let started_at = self.startup_splash.started_at.get_or_insert(now);
        let progress =
            ((now - *started_at) / self.startup_splash.duration_sec).clamp(0.0, 1.0) as f32;
        if progress >= 1.0 {
            self.startup_splash.duration_sec = 0.0;
            return None;
        }
        ctx.request_repaint();
        Some(progress)
    }

    fn render_startup_splash(&self, ctx: &egui::Context, progress: f32) {
        let time = ctx.input(|input| input.time) as f32;
        egui::CentralPanel::default()
            .frame(Frame::new().fill(Color32::TRANSPARENT).inner_margin(0.0))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter_at(rect);
                let fade = (1.0 - progress).clamp(0.0, 1.0);
                let alpha = (fade * 255.0) as u8;
                let pulse = (time * 2.6).sin() * 0.5 + 0.5;
                let scale = 0.94 + progress * 0.06 + pulse * 0.015;
                let title_font = egui::FontId::proportional(32.0 * scale);
                let subtitle_font = egui::FontId::proportional(15.0 * scale);
                let panel_size = vec2(rect.width().min(420.0), 168.0);
                let panel_rect = egui::Rect::from_center_size(rect.center(), panel_size);
                let panel_alpha = alpha.saturating_div(3);
                painter.rect_filled(
                    rect,
                    0.0,
                    Color32::from_rgba_premultiplied(8, 10, 14, alpha.saturating_div(2)),
                );
                painter.rect_filled(
                    panel_rect,
                    16.0,
                    Color32::from_rgba_premultiplied(18, 22, 30, panel_alpha),
                );
                painter.rect_stroke(
                    panel_rect,
                    16.0,
                    Stroke::new(
                        1.0,
                        Color32::from_rgba_premultiplied(88, 132, 198, panel_alpha),
                    ),
                    StrokeKind::Outside,
                );
                let bar_w = 18.0 * scale;
                let bar_h = 36.0 * scale;
                let bar_gap = 10.0 * scale;
                let bars_total_w = bar_w * 3.0 + bar_gap * 2.0;
                let bars_left = panel_rect.center().x - bars_total_w * 0.5;
                let bars_top = panel_rect.top() + 22.0 * scale;
                for i in 0..3 {
                    let n = i as f32;
                    let bar_phase = (time * 4.0 + n * 0.7).sin() * 0.5 + 0.5;
                    let bar_alpha = (90.0 + bar_phase * 140.0) as u8;
                    let bar_rect = egui::Rect::from_min_size(
                        pos2(
                            bars_left + n * (bar_w + bar_gap),
                            bars_top + (1.0 - bar_phase) * 10.0 * scale,
                        ),
                        vec2(bar_w, bar_h - (1.0 - bar_phase) * 10.0 * scale),
                    );
                    painter.rect_filled(
                        bar_rect,
                        8.0,
                        Color32::from_rgba_premultiplied(94, 220, 176, bar_alpha),
                    );
                }
                painter.text(
                    panel_rect.center() - vec2(0.0, 8.0 * scale),
                    egui::Align2::CENTER_CENTER,
                    self.app_brand_title(),
                    title_font,
                    Color32::from_rgba_premultiplied(240, 244, 248, alpha),
                );
                painter.text(
                    panel_rect.center() + vec2(0.0, 22.0 * scale),
                    egui::Align2::CENTER_CENTER,
                    self.startup_loading_text(),
                    subtitle_font,
                    Color32::from_rgba_premultiplied(208, 220, 255, alpha),
                );
                let track_rect = egui::Rect::from_center_size(
                    pos2(panel_rect.center().x, panel_rect.bottom() - 28.0 * scale),
                    vec2(panel_rect.width() - 56.0 * scale, 6.0 * scale),
                );
                painter.rect_filled(
                    track_rect,
                    999.0,
                    Color32::from_rgba_premultiplied(44, 52, 64, panel_alpha),
                );
                let fill_w = track_rect.width() * progress.clamp(0.0, 1.0);
                let fill_rect =
                    egui::Rect::from_min_size(track_rect.min, vec2(fill_w, track_rect.height()));
                painter.rect_filled(
                    fill_rect,
                    999.0,
                    Color32::from_rgba_premultiplied(94, 220, 176, alpha),
                );
            });
    }

    fn render_tray_blob_transition(&self, ctx: &egui::Context, progress: f32, opening: bool) {
        let rect = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new(if opening {
                "open-tray-blob"
            } else {
                "close-tray-blob"
            }),
        ));
        let eased = if opening {
            let p = progress.clamp(0.0, 1.0);
            p * p * (3.0 - 2.0 * p)
        } else {
            1.0 - (1.0 - progress).clamp(0.0, 1.0).powi(2)
        };
        let alpha = ((1.0 - eased).clamp(0.0, 1.0) * 180.0) as u8;
        painter.rect_filled(rect, 0.0, Color32::from_rgba_premultiplied(6, 8, 12, alpha));
    }

    fn render_crosshair_style_editor<H: std::hash::Hash>(
        ui: &mut egui::Ui,
        language: UiLanguage,
        grid_id: H,
        style: &mut CrosshairStyle,
    ) -> bool {
        let mut changed = false;
        egui::Grid::new(grid_id)
            .num_columns(2)
            .spacing([14.0, 8.0])
            .show(ui, |ui| {
                ui.label(Self::tr_lang(language, "Horizontal length", "Horizontal length"));
                changed |= ui
                    .add_sized(
                        [340.0, 20.0],
                        Slider::new(&mut style.horizontal_length, 1.0..=80.0),
                    )
                    .changed();
                ui.end_row();

                ui.label(Self::tr_lang(language, "Vertical length", "Vertical length"));
                changed |= ui
                    .add_sized(
                        [340.0, 20.0],
                        Slider::new(&mut style.vertical_length, 1.0..=80.0),
                    )
                    .changed();
                ui.end_row();

                ui.label(Self::tr_lang(language, "Thickness", "Thickness"));
                changed |= ui
                    .add_sized([340.0, 20.0], Slider::new(&mut style.thickness, 1.0..=20.0))
                    .changed();
                ui.end_row();

                ui.label(Self::tr_lang(language, "Gap", "Gap"));
                changed |= ui
                    .add_sized([340.0, 20.0], Slider::new(&mut style.gap, 0.0..=48.0))
                    .changed();
                ui.end_row();

                ui.label(Self::tr_lang(
                    language,
                    "Horizontal offset",
                    "Độ lệch ngang",
                ));
                changed |= ui
                    .add_sized(
                        [340.0, 20.0],
                        Slider::new(&mut style.x_offset, -1000..=1000),
                    )
                    .changed();
                ui.end_row();

                ui.label(Self::tr_lang(language, "Vertical offset", "Vertical offset"));
                changed |= ui
                    .add_sized(
                        [340.0, 20.0],
                        Slider::new(&mut style.y_offset, -1000..=1000),
                    )
                    .changed();
                ui.end_row();

                ui.label(Self::tr_lang(language, "Opacity", "Opacity"));
                changed |= ui
                    .add_sized([340.0, 20.0], Slider::new(&mut style.opacity, 0.05..=1.0))
                    .changed();
                ui.end_row();

                ui.label(Self::tr_lang(language, "Outline", "Outline"));
                changed |= ui
                    .checkbox(
                        &mut style.outline_enabled,
                        Self::tr_lang(language, "Enabled", "Enabled"),
                    )
                    .changed();
                ui.end_row();

                ui.label(Self::tr_lang(language, "Outline thickness", "Outline thickness"));
                changed |= ui
                    .add_sized(
                        [340.0, 20.0],
                        Slider::new(&mut style.outline_thickness, 0.0..=8.0),
                    )
                    .changed();
                ui.end_row();

                ui.label(Self::tr_lang(language, "Center dot", "Center dot"));
                changed |= ui
                    .checkbox(
                        &mut style.center_dot,
                        Self::tr_lang(language, "Enabled", "Enabled"),
                    )
                    .changed();
                ui.end_row();

                ui.label(Self::tr_lang(
                    language,
                    "Center dot size",
                    "Kích thước chấm giữa",
                ));
                changed |= ui
                    .add_sized(
                        [340.0, 20.0],
                        Slider::new(&mut style.center_dot_size, 1.0..=24.0),
                    )
                    .changed();
                ui.end_row();
            });
        changed
    }

    fn render_crosshair_presets_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        ui.spacing_mut().slider_width = 260.0;
        ui.horizontal_wrapped(|ui| {
        ui.add_space(2.0);
            if ui
                .button(Self::tr_lang(language, "+ Add preset", "+ Add preset"))
                .clicked()
            {
                self.add_profile();
            }
        });
        ui.separator();

        let mut changed = false;
        let mut remove_index = None;
        for index in 0..self.state.profiles.len() {
            let mut remove = false;
            let mut preset_changed = false;
            {
                let preset = &mut self.state.profiles[index];
                Self::show_preset_card(ui, preset.enabled, |ui| {
                    ui.horizontal(|ui| {
                        preset_changed |= ui
                            .add_sized([220.0, 24.0], TextEdit::singleline(&mut preset.name))
                            .changed();
                        if ui
                            .button(if preset.collapsed {
                                Self::tr_lang(language, "Show", "Show")
                            } else {
                                Self::tr_lang(language, "Hide", "Hide")
                            })
                            .clicked()
                        {
                            preset.collapsed = !preset.collapsed;
                            preset_changed = true;
                        }
                        if ui
                            .button(if preset.enabled {
                                Self::tr_lang(language, "Unapply", "Unapply")
                            } else {
                                Self::tr_lang(language, "Apply", "Apply")
                            })
                            .clicked()
                        {
                            preset.enabled = !preset.enabled;
                            preset.style.enabled = preset.enabled;
                            preset_changed = true;
                        }
                        if ui
                            .button(Self::tr_lang(language, "Delete", "Delete"))
                            .clicked()
                        {
                            remove = true;
                        }
                    });
                    if !preset.collapsed {
                        ui.add_space(4.0);
                        ui.label(Self::tr_lang(
                            language,
                            "Crosshair Settings",
                            "Cài đặt tâm ngắm",
                        ));
                        preset_changed |= Self::render_crosshair_style_editor(
                            ui,
                            language,
                            (index, "crosshair-style-grid"),
                            &mut preset.style,
                        );
                    }
                });
            }

            if remove {
                remove_index = Some(index);
                break;
            }
            if preset_changed {
                changed = true;
            }
            ui.add_space(6.0);
        }

        if let Some(index) = remove_index {
            let remove_name = self.state.profiles[index].name.clone();
            self.state
                .profiles
                .retain(|profile| profile.name != remove_name);
            if self.state.profiles.is_empty() {
                self.state.profiles.push(ProfileRecord::default());
            }
            let next = self.state.profiles[0].clone();
            self.state.selected_profile = Some(next.name.clone());
            self.state.active_style = next.style;
            self.save_name = next.name;
            changed = true;
        }

        if changed {
            self.sync_profiles();
            self.persist();
        }
    }

    fn render_window_presets_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            if ui
                .button(self.tr("+ Add preset", "+ Add preset"))
                .clicked()
            {
                self.add_window_preset();
                self.persist();
            }
            if ui
                .button(self.tr(
                    "+ Add window focus preset",
                    "+ Thêm preset focus",
                ))
                .clicked()
            {
                self.add_window_focus_preset();
                self.persist();
            }
        });

        let mut remove_id = None;
        let mut live_sync = false;
        for index in 0..self.state.window_presets.len() {
            let mut next_capture_target = None;
            let language = self.state.ui_language;
            ui.separator();
            {
                let preset = &mut self.state.window_presets[index];
                Self::show_preset_card(ui, preset.enabled, |ui| {
                    egui::Grid::new((preset.id, "window-preset-header"))
                        .num_columns(2)
                        .spacing([14.0, 8.0])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                live_sync |= ui.checkbox(&mut preset.enabled, "").changed();
                                ui.label(Self::preset_title_text(
                                    self.state.ui_theme == UiThemeMode::Dark,
                                    &preset.name,
                                    preset.enabled,
                                ));
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("Remove").clicked() {
                                        remove_id = Some(preset.id);
                                    }
                                    if ui
                                        .button(if preset.collapsed {
                                            Self::tr_lang(
                                                language,
                                                "Show",
                                                "Hiện",
                                            )
                                        } else {
                                            Self::tr_lang(
                                                language,
                                                "Hide",
                                                "Ẩn",
                                            )
                                        })
                                        .clicked()
                                    {
                                        preset.collapsed = !preset.collapsed;
                                        live_sync = true;
                                    }
                                },
                            );
                            ui.end_row();
                        });
                    if preset.collapsed {
                        return;
                    }
                    if let Some((preview_x, preview_y)) =
                        Self::window_anchor_preview_position(preset)
                    {
                        if preset.x != preview_x {
                            preset.x = preview_x;
                            live_sync = true;
                        }
                        if preset.y != preview_y {
                            preset.y = preview_y;
                            live_sync = true;
                        }
                    }
                    egui::Grid::new((preset.id, "window-preset-grid"))
                        .num_columns(2)
                        .spacing([14.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(Self::tr_lang(language, "Preset Name", "Preset Name"));
                            live_sync |= ui
                                .add_sized([260.0, 24.0], TextEdit::singleline(&mut preset.name))
                                .changed();
                            ui.end_row();

                            ui.label(Self::tr_lang(language, "Size", "Size"));
                            ui.horizontal(|ui| {
                                ui.label(Self::tr_lang(language, "Width", "Width"));
                                live_sync |= ui
                                    .add(DragValue::new(&mut preset.width).range(1..=20000))
                                    .changed();
                                ui.label(Self::tr_lang(language, "Height", "Height"));
                                live_sync |= ui
                                    .add(DragValue::new(&mut preset.height).range(1..=20000))
                                    .changed();
                            });
                            ui.end_row();

                            ui.label(Self::tr_lang(language, "Anchor", "Anchor"));
                            live_sync |= Self::window_anchor_picker(ui, preset);
                            ui.end_row();

                            ui.label(Self::tr_lang(language, "Position", "Position"));
                            ui.horizontal(|ui| {
                                ui.add_enabled_ui(preset.anchor == WindowAnchor::Manual, |ui| {
                                    ui.label("X");
                                    live_sync |= ui
                                        .add(DragValue::new(&mut preset.x).range(-20000..=20000))
                                        .changed();
                                    ui.label("Y");
                                    live_sync |= ui
                                        .add(DragValue::new(&mut preset.y).range(-20000..=20000))
                                        .changed();
                                });
                                if preset.anchor != WindowAnchor::Manual {
                                    ui.label(
                                        RichText::new(format!("Auto X={} Y={}", preset.x, preset.y))
                                            .italics(),
                                    );
                                }
                            });
                            ui.end_row();

                            ui.label(Self::tr_lang(language, "Hotkey", "Hotkey"));
                            ui.horizontal_wrapped(|ui| {
                                ui.monospace(hotkey::format_binding(preset.hotkey.as_ref()));
                                if ui.button(Self::tr_lang(language, "Capture", "Capture")).clicked() {
                                    next_capture_target = Some((
                                        CaptureRequest::WindowPresetHotkey(preset.id),
                                        format!("Capturing preset hotkey for {}.", preset.name),
                                    ));
                                }
                                if ui.button(Self::tr_lang(language, "Clear", "Clear")).clicked() {
                                    preset.hotkey = None;
                                    live_sync = true;
                                }
                            });
                            ui.end_row();

                            ui.label(Self::tr_lang(language, "Title", "Title"));
                            live_sync |= ui
                                .checkbox(&mut preset.remove_title_bar, Self::tr_lang(language, "Remove bar", "Remove bar"))
                                .on_hover_text(
                                    Self::tr_lang(
                                        language,
                                        "Remove title bar before apply. Off restores it.",
                                        "Nếu bật, preset sẽ xóa thanh tiêu đề trước khi áp dụng kích thước và vị trí. Nếu tắt, thanh tiêu đề sẽ được giữ hoặc khôi phục.",
                                    ),
                                )
                                .changed();
                            ui.end_row();

                            ui.label(Self::tr_lang(language, "Animated Apply", "Animated Apply"));
                            ui.horizontal_wrapped(|ui| {
                                live_sync |= ui
                                    .checkbox(&mut preset.animate_enabled, Self::tr_lang(language, "Enabled", "Enabled"))
                                    .changed();
                                ui.label(Self::tr_lang(language, "Duration", "Duration"));
                                live_sync |= ui
                                    .add(
                                        DragValue::new(&mut preset.animate_duration_ms)
                                            .range(60..=10_000)
                                            .suffix(" ms"),
                                    )
                                    .changed();
                            });
                            ui.end_row();

                            ui.label(Self::tr_lang(language, "Animate Hotkey", "Animate Hotkey"));
                            ui.horizontal_wrapped(|ui| {
                                ui.add_enabled_ui(preset.animate_enabled, |ui| {
                                    ui.monospace(hotkey::format_binding(
                                        preset.animate_hotkey.as_ref(),
                                    ));
                                    if ui.button(Self::tr_lang(language, "Capture", "Capture")).clicked() {
                                        next_capture_target = Some((
                                            CaptureRequest::WindowPresetAnimateHotkey(preset.id),
                                            format!(
                                                "Capturing animated preset hotkey for {}.",
                                                preset.name
                                            ),
                                        ));
                                    }
                                    if ui.button(Self::tr_lang(language, "Clear", "Clear")).clicked() {
                                        preset.animate_hotkey = None;
                                        live_sync = true;
                                    }
                                });
                            });
                            ui.end_row();

                            ui.label(Self::tr_lang(language, "Restore", "Restore"));
                            live_sync |= ui
                                .checkbox(
                                    &mut preset.restore_titlebar_enabled,
                                    Self::tr_lang(language, "Separate hotkey", "Separate hotkey"),
                                )
                                .on_hover_text(
                                    Self::tr_lang(
                                        language,
                                        "Only adds a second restore hotkey.",
                                        "Tùy chọn này không đổi hành động Apply bình thường hay Animated Apply. Nó chỉ bật thêm một phím tắt để khôi phục thanh tiêu đề về sau.",
                                    ),
                                )
                                .changed();
                            ui.end_row();

                            ui.label(Self::tr_lang(language, "Restore Key", "Restore Key"));
                            ui.horizontal_wrapped(|ui| {
                                ui.add_enabled_ui(preset.restore_titlebar_enabled, |ui| {
                                    ui.monospace(hotkey::format_binding(
                                        preset.titlebar_hotkey.as_ref(),
                                    ));
                                    if ui.button(Self::tr_lang(language, "Capture", "Capture")).clicked() {
                                        next_capture_target = Some((
                                            CaptureRequest::WindowPresetTitlebarHotkey(preset.id),
                                            format!(
                                                "Capturing restore title bar hotkey for {}.",
                                                preset.name
                                            ),
                                        ));
                                    }
                                    if ui.button(Self::tr_lang(language, "Clear", "Clear")).clicked() {
                                        preset.titlebar_hotkey = None;
                                        live_sync = true;
                                    }
                                });
                            });
                            ui.end_row();

                            ui.label(Self::tr_lang(language, "Target Window", "Target Window"));
                            live_sync |= Self::render_multi_window_targets(
                                ui,
                                (preset.id, "window-target"),
                                "Focus",
                                &mut preset.target_window_title,
                                &mut preset.extra_target_window_titles,
                                &self.open_windows,
                            );
                            ui.end_row();
                        });
                });
            }
            if let Some((target, status)) = next_capture_target.take() {
                self.begin_capture(target, status);
            }
        }

        ui.separator();
        let language = self.state.ui_language;
        ui.label(
            RichText::new(Self::tr_lang(
                language,
                "Focus",
                "Preset focus cửa sổ",
            ))
            .strong(),
        );
        let mut remove_focus_id = None;
        for index in 0..self.state.window_focus_presets.len() {
            let mut next_capture_target = None;
            ui.add_space(6.0);
            let preset = &mut self.state.window_focus_presets[index];
            Self::show_preset_card(ui, preset.enabled, |ui| {
                ui.horizontal(|ui| {
                    live_sync |= ui.checkbox(&mut preset.enabled, "").changed();
                    ui.label(Self::preset_title_text(
                        self.state.ui_theme == UiThemeMode::Dark,
                        &preset.name,
                        preset.enabled,
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(Self::tr_lang(language, "Remove", "Remove"))
                            .clicked()
                        {
                            remove_focus_id = Some(preset.id);
                        }
                        if ui
                            .button(if preset.collapsed {
                                Self::tr_lang(
                                    language,
                                    "Show",
                                    "Hiện",
                                )
                            } else {
                                Self::tr_lang(language, "Hide", "Hide")
                            })
                            .clicked()
                        {
                            preset.collapsed = !preset.collapsed;
                            live_sync = true;
                        }
                    });
                });
                if preset.collapsed {
                    return;
                }
                egui::Grid::new((preset.id, "window-focus-grid"))
                    .num_columns(2)
                    .spacing([14.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(Self::tr_lang(language, "Preset Name", "Preset Name"));
                        live_sync |= ui
                            .add_sized([260.0, 24.0], TextEdit::singleline(&mut preset.name))
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Hotkey", "Hotkey"));
                        ui.horizontal_wrapped(|ui| {
                            ui.monospace(Self::format_binding_ui(language, preset.hotkey.as_ref()));
                            if ui
                                .button(Self::tr_lang(language, "Capture", "Capture"))
                                .clicked()
                            {
                                next_capture_target = Some((
                                    CaptureRequest::WindowFocusPresetHotkey(preset.id),
                                    format!("Capturing focus hotkey for {}.", preset.name),
                                ));
                            }
                            if ui
                                .button(Self::tr_lang(language, "Clear", "Clear"))
                                .clicked()
                            {
                                preset.hotkey = None;
                                live_sync = true;
                            }
                        });
                        ui.end_row();

                        ui.label(Self::tr_lang(
                            language,
                            "Target Window",
                            "Cửa sổ mục tiêu",
                        ));
                        live_sync |= Self::render_multi_window_targets(
                            ui,
                            (preset.id, "window-focus-target"),
                            Self::tr_lang(language, "Focus", "Focus"),
                            &mut preset.target_window_title,
                            &mut preset.extra_target_window_titles,
                            &self.open_windows,
                        );
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Titles", "Titles"));
                        live_sync |= ui
                            .checkbox(
                                &mut preset.match_duplicate_window_titles,
                                Self::tr_lang(
                                    language,
                                    "Same titles",
                                    "Coi các cửa sổ trùng tiêu đề cũng là khớp",
                                ),
                            )
                            .changed();
                        ui.end_row();
                    });
            });
            if let Some((target, status)) = next_capture_target.take() {
                self.begin_capture(target, status);
            }
        }

        if live_sync {
            self.persist_window_presets();
        }
        if let Some(id) = remove_id {
            self.state.window_presets.retain(|preset| preset.id != id);
            self.persist_window_presets();
        }
        if let Some(id) = remove_focus_id {
            self.state
                .window_focus_presets
                .retain(|preset| preset.id != id);
            self.reconcile_master_presets();
            self.persist_window_presets();
        }
    }

    fn render_zoom_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        ui.heading("Zoom");
        ui.label("Source -> target. Shift=ratio.");
        let screen_size = Self::screen_size();
        if ui.button("+ Add zoom preset").clicked() {
            self.add_zoom_preset();
            self.persist();
        }

        let mut remove_id = None;
        let mut live_sync = false;
        for index in 0..self.state.zoom_presets.len() {
            let mut next_capture_target = None;
            ui.separator();
            let preset_snapshot = self.state.zoom_presets[index].clone();
            let preview = if preset_snapshot.preview_enabled && !preset_snapshot.collapsed {
                self.zoom_preview_for_preset(ui.ctx(), &preset_snapshot)
            } else {
                self.zoom_preview_cache.remove(&preset_snapshot.id);
                None
            };
            let preset = &mut self.state.zoom_presets[index];
            Self::show_preset_card(ui, preset.enabled, |ui| {
                ui.horizontal(|ui| {
                    live_sync |= ui.checkbox(&mut preset.enabled, "").changed();
                    ui.label(Self::preset_title_text(
                        self.state.ui_theme == UiThemeMode::Dark,
                        &preset.name,
                        preset.enabled,
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Remove").clicked() {
                            remove_id = Some(preset.id);
                        }
                        if ui
                            .button(if preset.collapsed { "Show" } else { "Hide" })
                            .clicked()
                        {
                            preset.collapsed = !preset.collapsed;
                            if preset.collapsed {
                                self.zoom_preview_cache.remove(&preset.id);
                            }
                            live_sync = true;
                        }
                    });
                });
                if preset.collapsed {
                    return;
                }
                egui::Grid::new((preset.id, "zoom-grid"))
                    .num_columns(2)
                    .spacing([14.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Preset Name");
                        live_sync |= ui
                            .add_sized([260.0, 24.0], TextEdit::singleline(&mut preset.name))
                            .changed();
                        ui.end_row();

                        ui.label("Source");
                        ui.horizontal(|ui| {
                            ui.label("X");
                            live_sync |= ui.add(DragValue::new(&mut preset.source_x)).changed();
                            ui.label("Y");
                            live_sync |= ui.add(DragValue::new(&mut preset.source_y)).changed();
                            ui.label("W");
                            live_sync |= ui
                                .add(DragValue::new(&mut preset.source_width).range(1..=8000))
                                .changed();
                            ui.label("H");
                            live_sync |= ui
                                .add(DragValue::new(&mut preset.source_height).range(1..=8000))
                                .changed();
                        });
                        ui.end_row();

                        ui.label("Target");
                        ui.horizontal(|ui| {
                            ui.label("X");
                            live_sync |= ui.add(DragValue::new(&mut preset.target_x)).changed();
                            ui.label("Y");
                            live_sync |= ui.add(DragValue::new(&mut preset.target_y)).changed();
                            ui.label("W");
                            live_sync |= ui
                                .add(DragValue::new(&mut preset.target_width).range(1..=8000))
                                .changed();
                            ui.label("H");
                            live_sync |= ui
                                .add(DragValue::new(&mut preset.target_height).range(1..=8000))
                                .changed();
                        });
                        ui.end_row();

                        ui.label("FPS");
                        live_sync |= ui
                            .add(DragValue::new(&mut preset.fps).range(1..=240).suffix(" fps"))
                            .changed();
                        ui.end_row();

                        ui.label("Preview");
                        live_sync |= ui
                            .checkbox(&mut preset.preview_enabled, "Stream preview in editor")
                            .on_hover_text("Only stream the selected window into Source/Result when this is enabled.")
                            .changed();
                        if !preset.preview_enabled {
                            self.zoom_preview_cache.remove(&preset.id);
                        }
                        ui.end_row();

                        ui.label("Target Window");
                        live_sync |= Self::render_multi_window_targets(
                            ui,
                            (preset.id, "zoom-target-window"),
                            "Any focused window",
                            &mut preset.target_window_title,
                            &mut preset.extra_target_window_titles,
                            &self.open_windows,
                        );
                        ui.end_row();

                        ui.label("Hotkey");
                        ui.horizontal_wrapped(|ui| {
                            ui.monospace(Self::format_binding_ui(language, preset.hotkey.as_ref()));
                            if ui.button("Capture").clicked() {
                                next_capture_target = Some((
                                    CaptureRequest::ZoomPresetHotkey(preset.id),
                                    format!("Capturing zoom hotkey for {}.", preset.name),
                                ));
                            }
                            if ui.button("Clear").clicked() {
                                preset.hotkey = None;
                                live_sync = true;
                            }
                        });
                        ui.end_row();
                });
                ui.separator();
                live_sync |= Self::render_zoom_rect_editor(
                    ui,
                    (preset.id, "source"),
                    "Source Region",
                    &mut preset.source_x,
                    &mut preset.source_y,
                    &mut preset.source_width,
                    &mut preset.source_height,
                    screen_size,
                    preview.as_ref(),
                    None,
                    None,
                );
                ui.add_space(8.0);
                live_sync |= Self::render_zoom_rect_editor(
                    ui,
                    (preset.id, "target"),
                    "Result Region",
                    &mut preset.target_x,
                    &mut preset.target_y,
                    &mut preset.target_width,
                    &mut preset.target_height,
                    screen_size,
                    preview.as_ref(),
                    Some((
                        preset.source_x,
                        preset.source_y,
                        preset.source_width,
                        preset.source_height,
                    )),
                    Some(
                        (preset.source_width.max(1) as f32) / (preset.source_height.max(1) as f32),
                    ),
                );
            });
            if let Some((target, status)) = next_capture_target.take() {
                self.begin_capture(target, status);
            }
        }

        if live_sync {
            self.persist_window_presets();
        }
        if let Some(id) = remove_id {
            self.state.zoom_presets.retain(|preset| preset.id != id);
            self.zoom_preview_cache.remove(&id);
            self.reconcile_master_presets();
            self.persist_window_presets();
        }
    }

    fn render_pin_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        ui.add_space(2.0);
        if ui
            .button(Self::tr_lang(
                language,
                "+ Add pin preset",
                "+ Thêm preset ghim",
            ))
            .clicked()
        {
            self.add_pin_preset();
            self.persist_window_presets();
        }

        let screen_size = Self::screen_size();
        let mut remove_id = None;
        let mut live_sync = false;
        let pin_preview_allowed = self.state.active_panel == AppPanel::Pin
            && ui
                .ctx()
                .input(|input| input.viewport().focused != Some(false));
        for index in 0..self.state.pin_presets.len() {
            let mut next_capture_target = None;
            ui.separator();
            let preset_snapshot = self.state.pin_presets[index].clone();
            let preview = if pin_preview_allowed
                && preset_snapshot.preview_enabled
                && !preset_snapshot.collapsed
            {
                self.window_preview_for_target(
                    ui.ctx(),
                    100_000 + preset_snapshot.id,
                    preset_snapshot.target_window_title.as_ref(),
                    &preset_snapshot.extra_target_window_titles,
                    preset_snapshot.match_duplicate_window_titles,
                )
            } else {
                self.zoom_preview_cache
                    .remove(&(100_000 + preset_snapshot.id));
                None
            };
            let preset = &mut self.state.pin_presets[index];
            Self::show_preset_card(ui, preset.enabled, |ui| {
                ui.horizontal(|ui| {
                    live_sync |= ui.checkbox(&mut preset.enabled, "").changed();
                    ui.label(Self::preset_title_text(
                        self.state.ui_theme == UiThemeMode::Dark,
                        &preset.name,
                        preset.enabled,
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(Self::tr_lang(language, "Remove", "Remove"))
                            .clicked()
                        {
                            remove_id = Some(preset.id);
                        }
                        if ui
                            .button(if preset.collapsed {
                                Self::tr_lang(
                                    language,
                                    "Show",
                                    "Hiện",
                                )
                            } else {
                                Self::tr_lang(language, "Hide", "Hide")
                            })
                            .clicked()
                        {
                            preset.collapsed = !preset.collapsed;
                            live_sync = true;
                        }
                    });
                });
                if preset.collapsed {
                    return;
                }

                egui::Grid::new((preset.id, "pin-grid"))
                    .num_columns(2)
                    .spacing([14.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(Self::tr_lang(language, "Preset Name", "Preset Name"));
                        live_sync |= ui
                            .add_sized([260.0, 24.0], TextEdit::singleline(&mut preset.name))
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Hotkey", "Hotkey"));
                        ui.horizontal_wrapped(|ui| {
                            ui.monospace(hotkey::format_binding(preset.hotkey.as_ref()));
                            if ui
                                .button(Self::tr_lang(language, "Capture", "Capture"))
                                .clicked()
                            {
                                next_capture_target = Some((
                                    CaptureRequest::PinPresetHotkey(preset.id),
                                    format!("Capturing pin hotkey for {}.", preset.name),
                                ));
                            }
                            if ui
                                .button(Self::tr_lang(language, "Clear", "Clear"))
                                .clicked()
                            {
                                preset.hotkey = None;
                                live_sync = true;
                            }
                        });
                        ui.end_row();

                        ui.label(Self::tr_lang(
                            language,
                            "Target Window",
                            "Cửa sổ mục tiêu",
                        ));
                        let target_changed = Self::render_multi_window_targets(
                            ui,
                            (preset.id, "pin-target-window"),
                            Self::tr_lang(language, "Focus", "Focus"),
                            &mut preset.target_window_title,
                            &mut preset.extra_target_window_titles,
                            &self.open_windows,
                        );
                        if target_changed {
                            preset.source_crop_initialized = false;
                            preset.source_crop_fit_version = 0;
                        }
                        live_sync |= target_changed;
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Titles", "Titles"));
                        live_sync |= ui
                            .checkbox(
                                &mut preset.match_duplicate_window_titles,
                                Self::tr_lang(
                                    language,
                                    "Same titles",
                                    "Coi các cửa sổ trùng tiêu đề cũng là khớp",
                                ),
                            )
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(
                            language,
                            "Custom Bounds",
                            "Khung tùy chỉnh",
                        ));
                        live_sync |= ui
                            .checkbox(
                                &mut preset.use_custom_bounds,
                                Self::tr_lang(
                                    language,
                                    "Use custom position and size",
                                    "Dùng vị trí và kích thước tùy chỉnh",
                                ),
                            )
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Overlay Shape", "Overlay Shape"));
                        if preset.overlay_style != PinOverlayStyle::Rectangle {
                            preset.overlay_style = PinOverlayStyle::Rectangle;
                            live_sync = true;
                        }
                        ui.label(Self::tr_lang(language, "Rectangle", "Rectangle"));
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Preview", "Preview"));
                        live_sync |= ui
                            .checkbox(
                                &mut preset.preview_enabled,
                                Self::tr_lang(
                                    language,
                                    "Stream preview in editor",
                                    "Phát xem trước trong trình chỉnh",
                                ),
                            )
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(
                            language,
                            "Source Crop",
                            "Cắt vùng nguồn",
                        ));
                        let source_crop_changed = ui
                            .checkbox(
                                &mut preset.use_source_crop,
                                Self::tr_lang(
                                    language,
                                    "Crop one part of the source window",
                                    "Cắt một phần của cửa sổ nguồn",
                                ),
                            )
                            .changed();
                        if source_crop_changed
                            && preset.use_source_crop
                            && let Some(preview_frame) = preview.as_ref()
                        {
                            preset.source_x = 0;
                            preset.source_y = 0;
                            preset.source_width = preview_frame.logical_width.max(1);
                            preset.source_height = preview_frame.logical_height.max(1);
                            preset.source_crop_initialized = true;
                            preset.source_crop_fit_version = 1;
                            live_sync = true;
                        } else if source_crop_changed && !preset.use_source_crop {
                            preset.source_crop_initialized = false;
                            preset.source_crop_fit_version = 0;
                        }
                        live_sync |= source_crop_changed;
                        ui.end_row();
                    });

                if preset.use_custom_bounds {
                    let pin_aspect_ratio = if preset.use_source_crop {
                        Some(preset.source_width.max(1) as f32 / preset.source_height.max(1) as f32)
                    } else {
                        preview.as_ref().map(|preview_frame| {
                            preview_frame.logical_width.max(1) as f32
                                / preview_frame.logical_height.max(1) as f32
                        })
                    };
                    live_sync |= Self::render_zoom_rect_editor(
                        ui,
                        (preset.id, "pin-bounds"),
                        Self::tr_lang(language, "Pinned Region", "Pinned Region"),
                        &mut preset.x,
                        &mut preset.y,
                        &mut preset.width,
                        &mut preset.height,
                        screen_size,
                        preview.as_ref(),
                        if preset.use_source_crop {
                            Some((
                                preset.source_x,
                                preset.source_y,
                                preset.source_width,
                                preset.source_height,
                            ))
                        } else {
                            None
                        },
                        pin_aspect_ratio,
                    );
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .button(Self::tr_lang(
                                language,
                                "Center Pinned Region",
                                "Căn giữa vùng ghim",
                            ))
                            .clicked()
                        {
                            preset.x = ((screen_size.x as i32 - preset.width.max(1)) / 2).max(0);
                            preset.y = ((screen_size.y as i32 - preset.height.max(1)) / 2).max(0);
                            live_sync = true;
                        }
                    });
                } else {
                    ui.label(
                        RichText::new(Self::tr_lang(
                            language,
                            "Pinned view will keep the original window position and size.",
                            "Khung ghim sẽ giữ vị trí và kích thước gốc của cửa sổ.",
                        ))
                        .italics(),
                    );
                }

                if preset.use_source_crop {
                    if (!preset.source_crop_initialized || preset.source_crop_fit_version < 1)
                        && let Some(preview_frame) = preview.as_ref()
                    {
                        preset.source_x = 0;
                        preset.source_y = 0;
                        preset.source_width = preview_frame.logical_width.max(1);
                        preset.source_height = preview_frame.logical_height.max(1);
                        preset.source_crop_initialized = true;
                        preset.source_crop_fit_version = 1;
                        live_sync = true;
                    }
                    let crop_changed = Self::render_zoom_rect_editor(
                        ui,
                        (preset.id, "pin-source-crop"),
                        Self::tr_lang(
                            language,
                            "Source Crop",
                            "Cắt vùng nguồn",
                        ),
                        &mut preset.source_x,
                        &mut preset.source_y,
                        &mut preset.source_width,
                        &mut preset.source_height,
                        screen_size,
                        preview.as_ref(),
                        None,
                        None,
                    );
                    if crop_changed {
                        preset.source_crop_initialized = true;
                        preset.source_crop_fit_version = 1;
                    }
                    live_sync |= crop_changed;
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .button(Self::tr_lang(
                                language,
                                "Center Source Crop",
                                "Căn giữa vùng cắt nguồn",
                            ))
                            .clicked()
                            && let Some(preview_frame) = preview.as_ref()
                        {
                            let max_w = preview_frame.logical_width.max(1);
                            let max_h = preview_frame.logical_height.max(1);
                            preset.source_x = ((max_w - preset.source_width.max(1)) / 2).max(0);
                            preset.source_y = ((max_h - preset.source_height.max(1)) / 2).max(0);
                            preset.source_crop_initialized = true;
                            preset.source_crop_fit_version = 1;
                            live_sync = true;
                        }
                    });
                    ui.label(
                        RichText::new(Self::tr_lang(
                            language,
                            "The cropped source area will be stretched into the pinned window, so this works like a lighter crop + zoom.",
                            "Vùng nguồn đã cắt sẽ được kéo giãn vào khung ghim, nên nó hoạt động như một kiểu crop + zoom nhẹ hơn.",
                        ))
                        .italics(),
                    );
                }
            });
            if let Some((target, status)) = next_capture_target.take() {
                self.begin_capture(target, status);
            }
        }

        if let Some(id) = remove_id {
            self.state.pin_presets.retain(|preset| preset.id != id);
            live_sync = true;
        }
        if live_sync {
            self.persist_window_presets();
        }
    }

    fn render_modes_panel(&mut self, ui: &mut egui::Ui) {
        self.ensure_master_presets();
        self.reconcile_master_presets();
        ui.heading("Mode");
        ui.horizontal(|ui| {
            if ui.button("+ Capture").clicked() {
                self.add_master_preset_from_current();
            }
        });

        let mut remove_id = None;
        let mut apply_id = None;
        let mut update_from_current_id = None;
        let mut needs_persist = false;
        let selected_id = self.state.selected_master_preset_id;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for preset in &mut self.state.master_presets {
                    ui.separator();
                    let active = selected_id == Some(preset.id);
                    Self::show_preset_card(ui, active, |ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .radio(active, "")
                                .on_hover_text("Apply this mode right now.")
                                .clicked()
                            {
                                apply_id = Some(preset.id);
                            }
                            needs_persist |= ui
                                .add_sized([220.0, 24.0], TextEdit::singleline(&mut preset.name))
                                .changed();
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("Remove").clicked() {
                                        remove_id = Some(preset.id);
                                    }
                                    if ui.button("Update").clicked() {
                                        update_from_current_id = Some(preset.id);
                                    }
                                    if ui.button(if active { "Active" } else { "Apply" }).clicked()
                                    {
                                        apply_id = Some(preset.id);
                                    }
                                    if ui
                                        .button(if preset.collapsed { "Show" } else { "Hide" })
                                        .clicked()
                                    {
                                        preset.collapsed = !preset.collapsed;
                                        needs_persist = true;
                                    }
                                },
                            );
                        });

                        if preset.collapsed {
                            return;
                        }

                        needs_persist |= ui
                            .checkbox(
                                &mut preset.macros_master_enabled,
                                "Enable all macros globally",
                            )
                            .changed();
                        ui.separator();
                        ui.label(RichText::new("Window Control").strong());
                        egui::Grid::new((preset.id, "mode-window-grid"))
                            .num_columns(4)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| {
                                ui.strong("Preset");
                                ui.strong("Apply");
                                ui.strong("Animate");
                                ui.strong("Restore");
                                ui.end_row();
                                for item in &mut preset.window_presets {
                                    let label = self
                                        .state
                                        .window_presets
                                        .iter()
                                        .find(|window_preset| window_preset.id == item.id)
                                        .map(|window_preset| window_preset.name.as_str())
                                        .unwrap_or("Missing preset");
                                    ui.label(label);
                                    needs_persist |= ui.checkbox(&mut item.enabled, "").changed();
                                    needs_persist |=
                                        ui.checkbox(&mut item.animate_enabled, "").changed();
                                    needs_persist |= ui
                                        .checkbox(&mut item.restore_titlebar_enabled, "")
                                        .changed();
                                    ui.end_row();
                                }
                            });

                        ui.separator();
                        ui.label(RichText::new("Window Focus").strong());
                        for item in &mut preset.window_focus_presets {
                            let label = self
                                .state
                                .window_focus_presets
                                .iter()
                                .find(|focus_preset| focus_preset.id == item.id)
                                .map(|focus_preset| focus_preset.name.as_str())
                                .unwrap_or("Missing focus preset");
                            needs_persist |= ui.checkbox(&mut item.enabled, label).changed();
                        }

                        ui.separator();
                        ui.label(RichText::new("Zoom").strong());
                        for item in &mut preset.zoom_presets {
                            let label = self
                                .state
                                .zoom_presets
                                .iter()
                                .find(|zoom_preset| zoom_preset.id == item.id)
                                .map(|zoom_preset| zoom_preset.name.as_str())
                                .unwrap_or("Missing zoom");
                            needs_persist |= ui.checkbox(&mut item.enabled, label).changed();
                        }

                        ui.separator();
                        ui.label(RichText::new("Macro Groups").strong());
                        for group_state in &mut preset.macro_groups {
                            let Some(group) = self
                                .state
                                .macro_groups
                                .iter()
                                .find(|group| group.id == group_state.id)
                            else {
                                continue;
                            };
                            Frame::group(ui.style())
                                .inner_margin(egui::Margin::same(6))
                                .show(ui, |ui| {
                                    needs_persist |= ui
                                        .checkbox(&mut group_state.enabled, &group.name)
                                        .changed();
                                    ui.add_space(4.0);
                                    for preset_state in &mut group_state.presets {
                                        let label = group
                                            .presets
                                            .iter()
                                            .find(|macro_preset| macro_preset.id == preset_state.id)
                                            .map(|macro_preset| {
                                                hotkey::format_binding(macro_preset.hotkey.as_ref())
                                            })
                                            .unwrap_or_else(|| "Missing macro".to_owned());
                                        ui.indent(
                                            (group.id, preset_state.id, "mode-macro-indent"),
                                            |ui| {
                                                needs_persist |= ui
                                                    .checkbox(&mut preset_state.enabled, label)
                                                    .changed();
                                            },
                                        );
                                    }
                                });
                        }
                    });
                }
            });

        if let Some(id) = update_from_current_id {
            self.update_master_preset_from_current(id);
        }
        if let Some(id) = remove_id {
            self.state.master_presets.retain(|preset| preset.id != id);
            if self.state.selected_master_preset_id == Some(id) {
                self.state.selected_master_preset_id =
                    self.state.master_presets.first().map(|preset| preset.id);
            }
            self.ensure_master_presets();
            self.persist();
        }
        if let Some(id) = apply_id {
            self.apply_master_preset(id);
        } else if needs_persist {
            self.persist();
        }
    }

    fn render_macro_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(Self::material_icon_text(0xe8b6, 18.0));
            ui.label(Self::tr_lang(language, "Search", "Tìm"));
            ui.add_sized(
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
        });

        let mut release_folder_id = None;
        let mut delete_folder_id = None;
        let mut begin_mouse_move_absolute_capture_target = None;
        let mut cancel_mouse_move_absolute_capture = false;
        let capture_target_snapshot = self.capture_target.clone();
        let active_folder_name = self.active_macro_folder_view.and_then(|folder_id| {
            self.state
                .macro_folders
                .iter()
                .find(|folder| folder.id == folder_id)
                .map(|folder| folder.name.clone())
        });
        if self.active_macro_folder_view.is_some() && active_folder_name.is_none() {
            self.active_macro_folder_view = None;
        }
        ui.vertical(|ui| {
            if let Some(folder_name) = &active_folder_name {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(Self::tr_lang(language, "< Back", "< Back"))
                        .clicked()
                    {
                        self.set_active_macro_folder_view(None);
                    }
                    ui.label(Self::folder_icon_text(true, 18.0));
                    if let Some(folder) = self
                        .state
                        .macro_folders
                        .iter_mut()
                        .find(|folder| Some(folder.id) == self.active_macro_folder_view)
                    {
                        if ui
                            .add_sized([220.0, 24.0], TextEdit::singleline(&mut folder.name))
                            .changed()
                        {
                            self.persist();
                        }
                    } else {
                        ui.label(
                            RichText::new(format!(
                                "{}: {folder_name}",
                                Self::tr_lang(language, "Folder", "Folder")
                            ))
                            .strong()
                            .color(Color32::from_rgb(46, 76, 122)),
                        );
                    }
                    if ui
                        .button(Self::tr_lang(
                            language,
                            "+ Add group here",
                            "+ Thêm group vào đây",
                        ))
                        .clicked()
                    {
                        if let Some(folder_id) = self.active_macro_folder_view {
                            self.add_macro_group_to_folder(folder_id);
                            self.persist();
                        }
                    }
                    if ui
                        .button(Self::tr_lang(
                            language,
                            "Enable All Groups",
                            "Bật tất cả group",
                        ))
                        .clicked()
                    {
                        if let Some(folder_id) = self.active_macro_folder_view {
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
                    }
                    if ui
                        .button(Self::tr_lang(
                            language,
                            "Release Folder",
                            "Nhả thư mục",
                        ))
                        .clicked()
                    {
                        if let Some(folder_id) = self.active_macro_folder_view {
                            self.confirm_release_folder_id = Some(folder_id);
                        }
                    }
                    if ui
                        .button(Self::tr_lang(
                            language,
                            "Delete Folder",
                            "Xóa thư mục",
                        ))
                        .clicked()
                    {
                        delete_folder_id = self.active_macro_folder_view;
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(
                            !self.macro_group_clipboard.is_empty(),
                            Button::new(Self::tr_lang(language, "Paste", "Paste")),
                        )
                        .clicked()
                    {
                        self.paste_macro_groups_into_folder(self.active_macro_folder_view);
                    }
                    if ui
                        .add_enabled(
                            !self.selected_macro_groups.is_empty(),
                            Button::new(Self::tr_lang(language, "Copy", "Copy")),
                        )
                        .clicked()
                    {
                        self.copy_selected_macro_groups();
                    }
                    if ui
                        .add_enabled(
                            !self.selected_macro_groups.is_empty(),
                            Button::new(Self::tr_lang(language, "Cut", "Cut")),
                        )
                        .clicked()
                    {
                        self.cut_selected_macro_groups();
                    }
                    if ui
                        .add_enabled(
                            !self.selected_macro_groups.is_empty(),
                            Button::new(Self::tr_lang(language, "Remove", "Remove")),
                        )
                        .clicked()
                    {
                        self.remove_selected_macro_groups();
                    }
                });
            } else {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(Self::tr_lang(
                            language,
                            "+ Add folder",
                            "+ Thêm thư mục",
                        ))
                        .clicked()
                    {
                        self.add_macro_folder();
                        self.persist();
                    }
                    if ui
                        .button(Self::tr_lang(
                            language,
                            "+ Add macro group",
                            "+ Thêm macro group",
                        ))
                        .clicked()
                    {
                        self.add_macro_group();
                        self.persist();
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(
                            !self.macro_group_clipboard.is_empty(),
                            Button::new(Self::tr_lang(language, "Paste", "Paste")),
                        )
                        .clicked()
                    {
                        self.paste_macro_groups_into_folder(None);
                    }
                    if ui
                        .add_enabled(
                            !self.selected_macro_groups.is_empty(),
                            Button::new(Self::tr_lang(language, "Copy", "Copy")),
                        )
                        .clicked()
                    {
                        self.copy_selected_macro_groups();
                    }
                    if ui
                        .add_enabled(
                            !self.selected_macro_groups.is_empty(),
                            Button::new(Self::tr_lang(language, "Cut", "Cut")),
                        )
                        .clicked()
                    {
                        self.cut_selected_macro_groups();
                    }
                    if ui
                        .add_enabled(
                            !self.selected_macro_groups.is_empty(),
                            Button::new(Self::tr_lang(language, "Remove", "Remove")),
                        )
                        .clicked()
                    {
                        self.remove_selected_macro_groups();
                    }
                });
            }
        });
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
            let master_text = if self.state.macros_master_enabled {
                Color32::WHITE
            } else {
                Color32::WHITE
            };
            if ui
                .add_sized(
                    [120.0, 28.0],
                    Button::new(RichText::new(master_label).color(master_text))
                        .fill(master_fill)
                        .stroke(egui::Stroke::new(1.0, master_stroke)),
                )
                .clicked()
            {
                self.state.macros_master_enabled = !self.state.macros_master_enabled;
                self.sync_macro_master_enabled();
                self.persist();
            }
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
            Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!(
                        "{} {folder_name} {} {group_count} {}?",
                        Self::tr_lang(language, "Delete", "Delete"),
                        Self::tr_lang(
                            language,
                            "and all",
                            "và toàn bộ"
                        ),
                        Self::tr_lang(
                            language,
                            "macro group(s) inside it",
                            "macro group bên trong"
                        )
                    ));
                    if ui
                        .button(Self::tr_lang(
                            language,
                            "Yes, Delete All",
                            "Đồng ý, xóa hết",
                        ))
                        .clicked()
                    {
                        delete_folder_id = Some(folder_id);
                    }
                    if ui
                        .button(Self::tr_lang(language, "Cancel", "Cancel"))
                        .clicked()
                    {
                        self.confirm_delete_folder_id = None;
                    }
                });
            });
        }
        if let Some(folder_id) = self.confirm_release_folder_id {
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
            Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!(
                        "{} {folder_name} {} {group_count} {}?",
                        Self::tr_lang(language, "Release", "Release"),
                        Self::tr_lang(language, "and move", "và chuyển"),
                        Self::tr_lang(
                            language,
                            "macro group(s) out of it",
                            "macro group ra khỏi nó",
                        )
                    ));
                    if ui
                        .button(Self::tr_lang(language, "Yes, Release", "Yes, Release"))
                        .clicked()
                    {
                        release_folder_id = Some(folder_id);
                    }
                    if ui
                        .button(Self::tr_lang(language, "Cancel", "Cancel"))
                        .clicked()
                    {
                        self.confirm_release_folder_id = None;
                    }
                });
            });
        }
        if let Some(group_id) = self.confirm_delete_macro_group_id {
            if let Some(group_name) = self
                .state
                .macro_groups
                .iter()
                .find(|group| group.id == group_id)
                .map(|group| group.name.clone())
            {
                Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "{} {}?",
                            Self::tr_lang(language, "Delete macro group", "Delete macro group"),
                            group_name
                        ));
                        if ui
                            .button(Self::tr_lang(language, "Yes, Delete", "Yes, Delete"))
                            .clicked()
                        {
                            self.state.macro_groups.retain(|group| group.id != group_id);
                            self.selected_macro_groups.remove(&group_id);
                            self.macro_group_clipboard
                                .retain(|clipboard_group_id| *clipboard_group_id != group_id);
                            self.confirm_delete_macro_group_id = None;
                            self.persist_macro_presets();
                        }
                        if ui
                            .button(Self::tr_lang(language, "Cancel", "Cancel"))
                            .clicked()
                        {
                            self.confirm_delete_macro_group_id = None;
                        }
                    });
                });
            } else {
                self.confirm_delete_macro_group_id = None;
            }
        }

        let mut remove_group = None;
        let mut live_sync = false;
        let mut add_preset_to_group = None;
        let mut paste_preset_to_group: Option<u32> = None;

        ui.separator();
        if active_folder_name.is_none() {
            ui.label(
                RichText::new(Self::tr_lang(
                    language,
                    "Folders",
                    "Thư mục",
                ))
                .strong(),
            );
            if self.state.macro_folders.is_empty() {
                ui.label(Self::tr_lang(
                    language,
                    "No folders yet. Macro groups can stay outside folders if you want.",
                    "Chưa có thư mục nào. Nếu muốn, macro group có thể nằm ngoài thư mục.",
                ));
            }
            let mut open_folder_id = None;
            for folder in &self.state.macro_folders {
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
                let folder_id = folder.id;
                let folder_name = folder.name.clone();
                Self::show_preset_card(ui, folder_has_enabled_content, |ui| {
                    egui::Grid::new((folder_id, "macro-folder-row"))
                        .num_columns(6)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            if ui
                                .add_sized([28.0, 24.0], Button::new(Self::folder_icon_text(false, 18.0)))
                                .clicked()
                            {
                                open_folder_id = Some(folder_id);
                            }
                            if ui
                                .add_sized([220.0, 24.0], Button::new(folder_name.clone()))
                                .clicked()
                            {
                                open_folder_id = Some(folder_id);
                            }
                            ui.add_sized(
                                [96.0, 24.0],
                                egui::Label::new(match language {
                                    UiLanguage::Vietnamese => format!("{folder_group_count} nhóm"),
                                    _ => format!("{folder_group_count} group(s)"),
                                }),
                            );
                            if ui
                                .add_sized(
                                    [70.0, 24.0],
                                    Button::new(Self::tr_lang(language, "Open", "Mở")),
                                )
                                .clicked()
                            {
                                open_folder_id = Some(folder_id);
                            }
                            if ui
                                .add_sized(
                                    [82.0, 24.0],
                                    Button::new(Self::tr_lang(language, "Release", "Nhả")),
                                )
                                .clicked()
                            {
                                self.confirm_release_folder_id = Some(folder_id);
                            }
                            if ui
                                .add_sized(
                                    [70.0, 24.0],
                                    Button::new(Self::tr_lang(language, "Delete", "Delete")),
                                )
                                .clicked()
                            {
                                if folder_group_count > 0 {
                                    self.confirm_delete_folder_id = Some(folder_id);
                                } else {
                                    delete_folder_id = Some(folder_id);
                                }
                            }
                            ui.end_row();
                        });
                });
                ui.add_space(4.0);
            }
            if let Some(folder_id) = open_folder_id {
                self.set_active_macro_folder_view(Some(folder_id));
            }
        }
        let search_query = self.macro_preset_search_query.trim().to_owned();
        Self::sort_macro_groups(&mut self.state.macro_groups);
        let visible_group_indices: Vec<usize> = self
            .state
            .macro_groups
            .iter()
            .enumerate()
            .filter(|(_, group)| match self.active_macro_folder_view {
                Some(folder_id) => group.folder_id == Some(folder_id),
                None => group.folder_id.is_none(),
            })
            .filter(|(_, group)| Self::macro_group_matches_search_query(group, &search_query))
            .map(|(index, _)| index)
            .collect();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
        if visible_group_indices.is_empty() {
            let empty_text = if self.active_macro_folder_view.is_some() {
                Self::tr_lang(language, "This folder does not have any macro groups yet.", "This folder does not have any macro groups yet.")
            } else {
                Self::tr_lang(language, "No macro groups outside folders yet.", "No macro groups outside folders yet.")
            };
            ui.label(empty_text);
        }
        for group_index in visible_group_indices {
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
            let mut paste_step_after = None;
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

            let image_search_timing_preset_options = self.image_search_timing_preset_options();
            {
                let group = &mut self.state.macro_groups[group_index];
                Self::show_preset_card(ui, group.enabled, |ui| {
                    if group.collapsed {
                        egui::Grid::new((group.id, "group-collapsed-header"))
                            .num_columns(4)
                            .min_col_width(140.0)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| {
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
                                        [28.0, 22.0],
                                        Button::new(
                                            Self::material_icon_text(star_icon, 15.0).color(if group.favorite {
                                                Color32::from_rgb(255, 224, 110)
                                            } else {
                                                Color32::from_rgb(208, 214, 224)
                                            }),
                                        )
                                        .fill(star_fill)
                                        .stroke(egui::Stroke::new(1.0, star_stroke)),
                                    )
                                    .on_hover_text(Self::tr_lang(
                                        language,
                                        "Favorite group",
                                        "Nhom yeu thich",
                                    ))
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
                                live_sync |= ui
                                    .checkbox(&mut group.enabled, Self::tr_lang(language, "Enabled", "Enabled"))
                                    .changed();
                                let title = Self::preset_title_text(self.state.ui_theme == UiThemeMode::Dark, &group.name, group.enabled);
                                if ui.selectable_label(false, title).clicked() {
                                    group.collapsed = false;
                                    live_sync = true;
                                }
                                ui.end_row();
                            });
                        return;
                    }
                    egui::Grid::new((group.id, "group-toolbar"))
                        .num_columns(3)
                        .min_col_width(140.0)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            live_sync |= ui
                                .add_sized(
                                    [86.0, 22.0],
                                    egui::Checkbox::new(
                                        &mut group.enabled,
                                        Self::tr_lang(language, "Enabled", "Enabled"),
                                    ),
                                )
                                .changed();
                            ui.horizontal(|ui| {
                                ui.label(Self::preset_title_text(
                                    self.state.ui_theme == UiThemeMode::Dark,
                                    Self::tr_lang(language, "Group Name", "Group Name"),
                                    group.enabled,
                                ));
                            ui.add_sized([240.0, 24.0], TextEdit::singleline(&mut group.name));
                            });
                            ui.add_space(0.0);
                            ui.end_row();
                        });
                    ui.horizontal_wrapped(|ui| {
                        if Self::sized_button(ui, 74.0, Self::tr_lang(language, "Hide", "Hide")).clicked() {
                            group.collapsed = true;
                            live_sync = true;
                        }
                        if Self::sized_button(ui, 92.0, Self::tr_lang(language, "+ Preset", "+ Preset")).clicked() {
                            add_preset_to_group = Some(group.id);
                        }
                        if Self::sized_button(ui, 86.0, Self::tr_lang(language, "Remove", "Remove")).clicked() {
                            remove_group = Some(group.id);
                        }
                    });
                    egui::Grid::new((group.id, "group-folder-row"))
                        .num_columns(2)
                        .spacing([8.0, 8.0])
                        .show(ui, |ui| {
                                        ui.label(Self::tr_lang(language, "Folder", "Folder"));
                            egui::ComboBox::from_id_salt((group.id, "macro-group-folder"))
                                .width(220.0)
                                .selected_text(
                                    group.folder_id
                                        .and_then(|id| {
                                            self.state
                                                .macro_folders
                                                .iter()
                                                .find(|folder| folder.id == id)
                                                .map(|folder| folder.name.clone())
                                        })
                                        .unwrap_or_else(|| {
                                            Self::tr_lang(language, "No folder", "No folder").to_owned()
                                        }),
                                )
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(
                                            group.folder_id.is_none(),
                                            Self::tr_lang(language, "No folder", "No folder"),
                                        )
                                        .clicked()
                                    {
                                        group.folder_id = None;
                                        live_sync = true;
                                    }
                                    for folder in &self.state.macro_folders {
                                        if ui
                                            .selectable_label(group.folder_id == Some(folder.id), &folder.name)
                                            .clicked()
                                        {
                                            group.folder_id = Some(folder.id);
                                            live_sync = true;
                                        }
                                    }
                                });
                            ui.end_row();
                        });
                    ui.separator();
                    egui::Grid::new((group.id, "group-target-row"))
                        .num_columns(2)
                        .spacing([8.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(Self::tr_lang(language, "Target Window", "Target Window"));
                            live_sync |= Self::render_multi_window_targets(
                                ui,
                                (group.id, "macro-group-window-target"),
                                Self::tr_lang(language, "Any focused window", "Any focused window"),
                                &mut group.target_window_title,
                                &mut group.extra_target_window_titles,
                                &self.open_windows,
                            );
                            ui.end_row();

                            ui.label(Self::tr_lang(language, "Titles", "Titles"));
                            live_sync |= ui
                                .checkbox(
                                    &mut group.match_duplicate_window_titles,
                                Self::tr_lang(language, "Same titles", "Same titles"),
                            )
                                .changed();
                            ui.end_row();
                        });

                    let binding_labels = Self::macro_group_binding_labels(group);
                    let group_preset_options = group
                        .presets
                        .iter()
                        .map(|preset_option| {
                            (
                                preset_option.id,
                                binding_labels
                                    .get(&preset_option.id)
                                    .cloned()
                                    .unwrap_or_else(|| hotkey::format_binding(preset_option.hotkey.as_ref())),
                            )
                        })
                        .collect::<Vec<_>>();
                    let image_search_preset_options = self
                        .state
                        .image_search_presets
                        .iter()
                        .map(|preset_option| {
                            (
                                preset_option.id,
                                preset_option.name.clone(),
                            )
                        })
                        .collect::<Vec<_>>();
                    for preset_index in render_preset_indices.iter().copied() {
                        let preset = &mut group.presets[preset_index];
                        Self::show_preset_card(ui, group.enabled && preset.enabled, |ui| {
                            ui.horizontal_top(|ui| {
                                let available_width = ui.available_width();
                                let right_width = 540.0;
                                let left_width = (available_width - right_width - 8.0).max(140.0);

                                ui.allocate_ui_with_layout(
                                    vec2(left_width, 0.0),
                                    egui::Layout::top_down(egui::Align::LEFT),
                                    |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(Self::tr_lang(
                                                language,
                                                if preset.trigger_mode == MacroTriggerMode::Release {
                                                    "Release"
                                                } else {
                                                    "Trigger"
                                                },
                                                if preset.trigger_mode == MacroTriggerMode::Release {
                                                    "Thả"
                                                } else {
                                                    "Kích hoạt"
                                                },
                                            ));
                                            ui.add_space(6.0);
                                            if !preset.trigger_keys.trim().is_empty() {
                                                live_sync |= Self::render_key_list_chips(
                                                    ui,
                                                    language,
                                                    &mut preset.trigger_keys,
                                                    Self::tr_lang(
                                                        language,
                                                        "Not set",
                                                        "Chưa được đặt",
                                                    ),
                                                );
                                            } else {
                                                ui.add_sized(
                                                    [148.0, 22.0],
                                                    egui::Label::new(
                                                        RichText::new(
                                                            binding_labels
                                                                .get(&preset.id)
                                                                .cloned()
                                                                .unwrap_or_else(|| {
                                                                    Self::format_macro_trigger_ui(language, preset)
                                                                }),
                                                        )
                                                        .monospace(),
                                                    ),
                                                );
                                            }
                                        });
                                    },
                                );

                                let right_spacer = (ui.available_width() - right_width).max(0.0);
                                if right_spacer > 0.0 {
                                    ui.add_space(right_spacer);
                                }
                                ui.allocate_ui_with_layout(
                                    vec2(right_width, 0.0),
                                    egui::Layout::right_to_left(egui::Align::TOP),
                                    |ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        if Self::sized_button(
                                            ui,
                                            60.0,
                                            Self::tr_lang(language, "Remove", "Remove"),
                                        )
                                        .clicked()
                                        {
                                            remove_preset = Some(preset.id);
                                        }
                                        if ui
                                            .add_enabled(
                                                self.macro_preset_clipboard.is_some(),
                                                Button::new(
                                                    Self::tr_lang(language, "Paste", "Paste")
                                                )
                                                .min_size(egui::vec2(60.0, 24.0)),
                                            )
                                            .clicked()
                                        {
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
                                        if Self::sized_button(
                                            ui,
                                            60.0,
                                            Self::tr_lang(language, "Clear", "Clear"),
                                        )
                                        .clicked()
                                        {
                                            let mut changed = false;
                                            if !preset.trigger_keys.trim().is_empty() {
                                                changed |= Self::pop_key_list_entry(&mut preset.trigger_keys);
                                            }
                                            if preset.hotkey.is_some() {
                                                preset.hotkey = None;
                                                changed = true;
                                            }
                                            live_sync |= changed;
                                        }
                                        let mouse_trigger_options = [
                                            ("MouseLeft", Self::tr_lang(language, "LClick", "LClick")),
                                            ("MouseRight", Self::tr_lang(language, "RClick", "RClick")),
                                            ("MouseMiddle", Self::tr_lang(language, "MClick", "MClick")),
                                            ("MouseX1", Self::tr_lang(language, "X1", "X1")),
                                            ("MouseX2", Self::tr_lang(language, "X2", "X2")),
                                            ("MouseWheelUp", Self::tr_lang(language, "WhUp", "WhUp")),
                                            ("MouseWheelDown", Self::tr_lang(language, "WhDn", "WhDn")),
                                        ];
                                        let selected_mouse_key = hotkey::split_key_list(&preset.trigger_keys)
                                            .into_iter()
                                            .find(|key| hotkey::is_mouse_key_name(key));
                                        let selected_mouse_label = selected_mouse_key
                                            .as_deref()
                                            .and_then(|key| mouse_trigger_options.iter().find(|(option_key, _)| option_key.eq_ignore_ascii_case(key)))
                                            .map(|(_, label)| *label)
                                            .unwrap_or_else(|| Self::tr_lang(language, "Mouse", "Mouse"));
                                        let mouse_trigger_response = ui
                                            .scope(|ui| {
                                                ui.spacing_mut().interact_size.y = 24.0;
                                                egui::ComboBox::from_id_salt((
                                                    group.id,
                                                    preset.id,
                                                    "mouse-trigger-dropdown",
                                                ))
                                                .width(56.0)
                                                .selected_text(selected_mouse_label)
                                                .show_ui(ui, |ui| {
                                                    for (option_key, option_label) in mouse_trigger_options {
                                                        if ui
                                                            .selectable_label(
                                                                selected_mouse_key
                                                                    .as_ref()
                                                                    .is_some_and(|current| current.eq_ignore_ascii_case(option_key)),
                                                                option_label,
                                                            )
                                                            .clicked()
                                                        {
                                                            let mut trigger_keys =
                                                                hotkey::split_key_list(&preset.trigger_keys);
                                                            if !trigger_keys.iter().any(|key| {
                                                                key.eq_ignore_ascii_case(option_key)
                                                            }) {
                                                                trigger_keys.push(option_key.to_owned());
                                                            }
                                                            preset.trigger_keys = trigger_keys.join(", ");
                                                            preset.hotkey = None;
                                                            live_sync = true;
                                                        }
                                                    }
                                                })
                                            })
                                            .inner;
                                        mouse_trigger_response
                                            .response
                                            .on_hover_text(selected_mouse_label);
                                        let capture_target =
                                            CaptureRequest::MacroPresetHotkey(group.id, preset.id);
                                        if ui
                                            .add_sized(
                                                [64.0, 24.0],
                                                Button::new(Self::capture_button_text(
                                                    language,
                                                    capture_target_snapshot.as_ref() == Some(&capture_target),
                                                )),
                                            )
                                            .clicked()
                                        {
                                            if capture_target_snapshot.as_ref() == Some(&capture_target) {
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
                                        live_sync |= ui
                                            .add_sized(
                                                [80.0, 22.0],
                                                egui::Checkbox::new(
                                                    &mut preset.enabled,
                                                    Self::tr_lang(language, "Enabled", "Enabled"),
                                                ),
                                            )
                                            .changed();
                                    },
                                );
                            });
                        if !preset.collapsed {
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
                        });
                        if preset.trigger_mode == MacroTriggerMode::Release {
                            live_sync |= ui
                                .checkbox(
                                    &mut preset.release_requires_all_inputs_released,
                                    Self::tr_lang(
                                        language,
                                        "Wait until every other held input is released",
                                        "Wait until every other held input is released",
                                    ),
                                )
                                .on_hover_text(
                                    Self::tr_lang(
                                        language,
                                        "If enabled, releasing the trigger key or mouse button will not fire while any other key or mouse button is still held down.",
                                        "If enabled, releasing the trigger key or mouse button will not fire while any other key or mouse button is still held down.",
                                    ),
                                )
                                .changed();
                            if preset.release_requires_all_inputs_released {
                                ui.horizontal(|ui| {
                                    live_sync |= Self::render_key_list_chips(
                                        ui,
                                        language,
                                        &mut preset.release_wait_key,
                                        Self::tr_lang(language, "Not set", "Not set"),
                                    );
                                    let wait_capture_target =
                                        CaptureRequest::MacroPresetReleaseWaitKey(group.id, preset.id);
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
                                        if capture_target_snapshot.as_ref() == Some(&wait_capture_target) {
                                            cancel_active_capture = true;
                                        } else {
                                            next_capture_target = Some(wait_capture_target);
                                        }
                                    }
                                });
                            }
                        }
                        if preset.trigger_mode == MacroTriggerMode::Hold {
                            Frame::group(ui.style())
                                .inner_margin(egui::Margin::symmetric(6, 4))
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
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
                                    });
                                    if preset.hold_stop_step_enabled {
                                        let mut clear_hold_stop_step = false;
                                        let step = &mut preset.hold_stop_step;
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
                                                egui::Grid::new((group.id, preset.id, "hold-stop-action-grid"))
                                                    .num_columns(5)
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
                                                            MacroAction::EnableCrosshairProfile,
                                                            MacroAction::DisableCrosshair,
                                                            MacroAction::EnablePinPreset,
                                                            MacroAction::DisablePin,
                                                            MacroAction::PlaySoundPreset,
                                                            MacroAction::ApplyMouseSensitivityPreset,
                                                            MacroAction::StopImageSearchWait,
                                                            MacroAction::TriggerImageSearchTiming,
                                                            MacroAction::LoopStart,
                                                            MacroAction::LoopEnd,
                                                            MacroAction::StopIfKeyPressed,
                                                            MacroAction::ShowToolbox,
                                                            MacroAction::HideToolbox,
                                                            MacroAction::LockKeys,
                                                            MacroAction::UnlockKeys,
                                                             MacroAction::EnableMacroPreset,
                                                             MacroAction::DisableMacroPreset,
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
                                                            if (index + 1) % 5 == 0 {
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
                                });
                            });
                                            Self::show_instant_hover_tooltip(
                                                ui,
                                                &hold_stop_combo.response,
                                                Self::macro_action_tooltip(step.action),
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
                                                            Self::tr_lang(language, "Select window preset", "Select window preset").to_owned()
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
                                                            Self::tr_lang(language, "Select focus preset", "Select focus preset").to_owned()
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
                                                            Self::tr_lang(language, "Select pin preset", "Select pin preset").to_owned()
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
                                                            Self::tr_lang(language, "Select mouse path", "Select mouse path").to_owned()
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
                                                    MacroAction::StartImageSearch
                                                        | MacroAction::TriggerImageSearchMove
                                                        | MacroAction::StopImageSearch
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
                                                } else if step.action
                                                    == MacroAction::TriggerImageSearchTiming
                                                {
                                                    let selected_id =
                                                        step.key.trim().parse::<u32>().ok();
                                                    let selected_label =
                                                        Self::image_search_timing_preset_label(
                                                            &image_search_timing_preset_options,
                                                            selected_id,
                                                            "Select timing preset",
                                                        );
                                                    egui::ComboBox::from_id_salt((
                                                        group.id,
                                                        preset.id,
                                                        "hold-stop-image-search-timing",
                                                    ))
                                                    .width(180.0)
                                                    .selected_text(selected_label)
                                                    .show_ui(ui, |ui| {
                                                        for (preset_option_id, preset_option_label) in
                                                            &image_search_timing_preset_options
                                                        {
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
                                                            .width(260.0)
                                                            .selected_text(format!("{selected_label} â–¾"))
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
                                                            Self::tr_lang(language, "Select sound preset", "Select sound preset").to_owned()
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
                                                    live_sync |= ui
                                                        .add_sized(
                                                            [160.0, 22.0],
                                                            TextEdit::singleline(&mut step.key).hint_text("A,S,W,D"),
                                                        )
                                                        .changed();
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
                                                        let mut loop_count =
                                                            step.key.trim().parse::<u32>().unwrap_or(1).max(1);
                                                        if ui
                                                            .add_sized(
                                                                [96.0, 22.0],
                                                                DragValue::new(&mut loop_count).range(1..=1_000_000),
                                                            )
                                                            .changed()
                                                        {
                                                            step.key = loop_count.to_string();
                                                            live_sync = true;
                                                        }
                                                    }
                                                } else if step.action == MacroAction::StopIfKeyPressed {
                                                    live_sync |= ui
                                                        .add_sized(
                                                            [160.0, 22.0],
                                                            TextEdit::singleline(&mut step.key).hint_text(Self::tr_lang(
                                                                language,
                                                                "Stop key",
                                                                "Phím dừng vòng lặp",
                                                            )),
                                                        )
                                                        .changed();
                                                } else if step.action == MacroAction::ShowToolbox {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .toolbox_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            if step.key.trim().is_empty() {
                                                                Self::tr_lang(
                                                                    language,
                                                                    "Select toolbox preset",
                                                                    "Chọn preset hộp công cụ",
                                                                )
                                                                .to_owned()
                                                            } else {
                                                                format!("CÅ©: {}", step.key)
                                                            }
                                                        });
                                                    ui.horizontal(|ui| {
                                                        egui::ComboBox::from_id_salt((group.id, preset.id, "hold-stop-toolbox-preset"))
                                                            .width(112.0)
                                                            .selected_text(selected_label)
                                                            .show_ui(ui, |ui| {
                                                                for toolbox_preset in &self.state.toolbox_presets {
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
                                                        live_sync |= ui
                                                            .add_sized(
                                                                [120.0, 22.0],
                                                                TextEdit::singleline(&mut step.text_override)
                                                                    .hint_text("Text override"),
                                                            )
                                                            .changed();
                                                    });
                                                } else if step.action == MacroAction::TypeText {
                                                    live_sync |= ui
                                                        .add_sized(
                                                            [220.0, 22.0],
                                                            TextEdit::singleline(&mut step.key).hint_text("Text to type"),
                                                        )
                                                        .changed();
                                                } else if matches!(step.action, MacroAction::DisableCrosshair | MacroAction::DisableZoom) {
                                                    ui.add_sized(
                                                        [110.0, 22.0],
                                                        egui::Label::new(Self::tr_lang(language, "No input", "No input")),
                                                    );
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
                                            } else if step.action == MacroAction::ShowToolbox {
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
                                                if ui
                                                    .add_sized(
                                                        [28.0, 22.0],
                                                        Button::new(if capture_target_snapshot.as_ref()
                                                            == Some(&CaptureRequest::MacroPresetHoldStopInput(
                                                                group.id,
                                                                preset.id,
                                                            ))
                                                        {
                                                            Self::material_icon_text(0xe312, 18.0)
                                                                .strong()
                                                                .color(Color32::from_rgb(255, 232, 96))
                                                        } else {
                                                            Self::material_icon_text(0xe312, 18.0)
                                                        })
                                                            .min_size(vec2(28.0, 22.0)),
                                                    )
                                                    .on_hover_text(Self::tr_lang(
                                                        language,
                                                        "Bắt phím giữ",
                                                        "Bắt phím cho action khi dừng giữ",
                                                    ))
                                                    .clicked()
                                                {
                                                    let hold_stop_capture_target =
                                                        CaptureRequest::MacroPresetHoldStopInput(group.id, preset.id);
                                                    if capture_target_snapshot.as_ref() == Some(&hold_stop_capture_target) {
                                                        cancel_active_capture = true;
                                                    } else {
                                                        next_capture_target = Some(hold_stop_capture_target);
                                                    }
                                                }
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
                                    }
                                });
                        }
                        ui.scope(|ui| {
                            Frame::new()
                                .inner_margin(egui::Margin::symmetric(4, 2))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add_sized([22.0, 18.0], Button::new(RichText::new("+").strong()))
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
                                        ui.add_sized([24.0, 18.0], egui::Label::new(""));
                                        ui.add_sized([24.0, 18.0], egui::Label::new(""));
                                        ui.add_sized([30.0, 18.0], egui::Label::new(RichText::new("#").strong()));
                                        ui.add_sized([54.0, 18.0], egui::Label::new(RichText::new(Self::tr_lang(language, "Delay", "Delay")).strong()));
                                        ui.add_sized([154.0, 18.0], egui::Label::new(RichText::new(Self::tr_lang(language, "Action", "Action")).strong()));
                                        ui.add_sized([146.0, 18.0], egui::Label::new(""));
                                        if ui
                                            .add_sized(
                                                [64.0, 20.0],
                                                Button::new(Self::tr_lang(language, "Clear all", "Clear all")),
                                            )
                                            .on_hover_text(Self::tr_lang(language, "Clear all steps", "Clear all steps"))
                                            .clicked()
                                        {
                                            preset.steps.clear();
                                            live_sync = true;
                                        }
                                        if ui
                                            .add_sized(
                                                [56.0, 20.0],
                                                Button::new(Self::tr_lang(language, "Copy", "Copy")),
                                            )
                                            .on_hover_text(Self::tr_lang(
                                                language,
                                                "Copy the selected steps in this preset.",
                                                "Copy selected steps in this preset.",
                                            ))
                                            .clicked()
                                        {
                                            copy_selected_steps = Some((group.id, preset.id));
                                        }
                                    });
                                });

                            let loop_colors = Self::macro_loop_colors(&preset.steps);
                            let steps_len = preset.steps.len();
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
                                    Color32::from_rgba_premultiplied(88, 148, 220, 130)
                                } else if let Some(color) =
                                    loop_colors.get(step_index).and_then(|color| *color)
                                {
                                    color
                                } else {
                                    ui.visuals().faint_bg_color
                                };
                                if !step.enabled {
                                    row_fill = Color32::from_rgba_unmultiplied(62, 62, 62, 220);
                                }
                                let drag_payload = MacroStepDragPayload {
                                    group_id: group.id,
                                    preset_id: preset.id,
                                    indices: drag_indices,
                                };
                                let row_response = Frame::group(ui.style())
                                    .fill(row_fill)
                                    .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
                                    .inner_margin(egui::Margin::symmetric(4, 2))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let select_label = if is_selected { "[x]" } else { "[ ]" };
                                            if ui
                                                .add_sized([22.0, 18.0], Button::new("+"))
                                                .on_hover_text("Add a new step below this one")
                                                .clicked()
                                            {
                                                insert_step_after = Some((preset.id, step_index));
                                            }
                                            let select_response = ui
                                                .add_sized([24.0, 18.0], Button::new(select_label));
                                            if select_response.clicked() {
                                                pending_step_selection = Some((
                                                    group.id,
                                                    preset.id,
                                                    step_index,
                                                    ui.input(|input| input.modifiers.ctrl),
                                                ));
                                            }
                                            live_sync |= ui
                                                .add_sized(
                                                    [18.0, 18.0],
                                                    egui::Checkbox::new(&mut step.enabled, ""),
                                                )
                                                .on_hover_text(Self::tr_lang(
                                                    language,
                                                    "Enable this step",
                                                    "Bật step này",
                                                ))
                                                .changed();
                                            if ui
                                                .add_sized(
                                                    [28.0, 18.0],
                                                    Button::new(Self::material_icon_text(0xe872, 18.0)),
                                                )
                                                .on_hover_text(Self::tr_lang(
                                                    language,
                                                    "Remove this step",
                                                    "Xóa step này",
                                                ))
                                                .clicked()
                                            {
                                                remove_step = Some((preset.id, step_index));
                                            }
                                            let drag_handle = ui
                                                .add_sized(
                                                    [24.0, 18.0],
                                                    Button::new(RichText::new("::").monospace())
                                                        .sense(Sense::drag()),
                                                )
                                                .on_hover_cursor(egui::CursorIcon::Grab);
                                            drag_handle.dnd_set_drag_payload(drag_payload.clone());
                                            ui.add_sized(
                                                [30.0, 18.0],
                                                egui::Label::new(
                                                    RichText::new(format!("{}", step_index + 1)).monospace(),
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
                                                    egui::Grid::new((group.id, preset.id, step_index, "action-grid"))
                                                        .num_columns(5)
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
                                                                MacroAction::EnableCrosshairProfile,
                                                                MacroAction::DisableCrosshair,
                                                                MacroAction::EnablePinPreset,
                                                                MacroAction::DisablePin,
                                                                MacroAction::PlaySoundPreset,
                                                                MacroAction::ApplyMouseSensitivityPreset,
                                                                MacroAction::StopImageSearchWait,
                                                                MacroAction::TriggerImageSearchTiming,
                                                                MacroAction::LoopStart,
                                                                MacroAction::LoopEnd,
                                                                MacroAction::StopIfKeyPressed,
                                                            MacroAction::ShowToolbox,
                                                                MacroAction::HideToolbox,
                                                                MacroAction::LockKeys,
                                                                MacroAction::UnlockKeys,
                                                                 MacroAction::EnableMacroPreset,
                                                                 MacroAction::DisableMacroPreset,
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
                                                                if (index + 1) % 5 == 0 {
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
                                                        });
                                                });
                                            Self::show_instant_hover_tooltip(
                                                ui,
                                                &action_combo.response,
                                                Self::macro_action_tooltip(step.action),
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
                                                        .unwrap_or_else(|| "Select window preset".to_owned());
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
                                                        .unwrap_or_else(|| "Select focus preset".to_owned());
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
                                                        .unwrap_or_else(|| "Select pin preset".to_owned());
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
                                                        .unwrap_or_else(|| "Select mouse path".to_owned());
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
                                                    MacroAction::StartImageSearch
                                                        | MacroAction::TriggerImageSearchMove
                                                        | MacroAction::StopImageSearch
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
                                                    if step.action == MacroAction::TriggerImageSearchMove {
                                                        ui.add_space(4.0);
                                                        ui.horizontal(|ui| {
                                                            live_sync |= ui
                                                                .checkbox(
                                                                    &mut step.image_search_move_cursor_on_match,
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
                                                                    &mut step.image_search_wait_until_found,
                                                                    Self::tr_lang(language, "Wait", "Wait"),
                                                                )
                                                                .on_hover_text(Self::tr_lang(
                                                                    language,
                                                                    "Keep scanning until the image is found.",
                                                                    "Tiếp tục dò cho tới khi thấy ảnh.",
                                                                ))
                                                                .changed();
                                                            let mut trigger_macro_enabled = step.image_search_trigger_macro_enabled;
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
                                                                step.image_search_trigger_macro_enabled = trigger_macro_enabled;
                                                                if trigger_macro_enabled {
                                                                    if step
                                                                        .image_search_trigger_macro_preset_id
                                                                        .is_none()
                                                                    {
                                                                        step.image_search_trigger_macro_preset_id = group_preset_options
                                                                            .iter()
                                                                            .find(|(preset_option_id, _)| *preset_option_id != preset.id)
                                                                            .map(|(preset_option_id, _)| *preset_option_id);
                                                                    }
                                                                }
                                                                live_sync = true;
                                                            }
                                                            if step.image_search_trigger_macro_enabled {
                                                                let selected_id = step.image_search_trigger_macro_preset_id;
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
                                                                            step.image_search_trigger_macro_preset_id =
                                                                                Some(*preset_option_id);
                                                                            live_sync = true;
                                                                }
                                                            }
                                                        });
                                                    } else if step.action
                                                        == MacroAction::TriggerImageSearchTiming
                                                    {
                                                        let selected_id =
                                                            step.key.trim().parse::<u32>().ok();
                                                        let selected_label =
                                                            Self::image_search_timing_preset_label(
                                                                &image_search_timing_preset_options,
                                                                selected_id,
                                                                "Select timing preset",
                                                            );
                                                        egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "image-search-timing-preset-step"))
                                                            .width(180.0)
                                                            .selected_text(selected_label)
                                                            .show_ui(ui, |ui| {
                                                                for (preset_option_id, preset_option_label) in &image_search_timing_preset_options {
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
                                                        .unwrap_or_else(|| "Select sound preset".to_owned());
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
                                                        .width(260.0)
                                                        .selected_text(format!("{selected_label} â–¾"))
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
                                                } else if matches!(step.action, MacroAction::LockKeys | MacroAction::UnlockKeys) {
                                                    live_sync |= ui
                                                        .add_sized(
                                                            [146.0, 18.0],
                                                            TextEdit::singleline(&mut step.key)
                                                                .hint_text(Self::tr_lang(language, "A,S,W,D", "A,S,W,D")),
                                                        )
                                                        .changed();
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
                                                        let mut loop_count =
                                                            step.key.trim().parse::<u32>().unwrap_or(1).max(1);
                                                        if ui
                                                            .add_sized(
                                                                [80.0, 18.0],
                                                                DragValue::new(&mut loop_count).range(1..=1_000_000),
                                                            )
                                                            .changed()
                                                        {
                                                            step.key = loop_count.to_string();
                                                            live_sync = true;
                                                        }
                                                    }
                                                } else if step.action == MacroAction::StopIfKeyPressed {
                                                    live_sync |= ui
                                                        .add_sized(
                                                            [146.0, 18.0],
                                                            TextEdit::singleline(&mut step.key)
                                                                .hint_text(Self::tr_lang(language, "Stop key", "Stop key")),
                                                        )
                                                        .changed();
                                                } else if step.action == MacroAction::ShowToolbox {
                                                    let selected_id = step.key.trim().parse::<u32>().ok();
                                                    let selected_label = selected_id
                                                        .and_then(|id| {
                                                            self.state
                                                                .toolbox_presets
                                                                .iter()
                                                                .find(|preset| preset.id == id)
                                                                .map(|preset| preset.name.clone())
                                                        })
                                                        .unwrap_or_else(|| {
                                                            if step.key.trim().is_empty() {
                                                                Self::tr_lang(
                                                                    language,
                                                                    "Select toolbox preset",
                                                                    "Chọn preset hộp công cụ",
                                                                )
                                                                .to_owned()
                                                            } else {
                                                                match language {
                                                                    UiLanguage::Vietnamese => format!("CÅ©: {}", step.key),
                                                                    _ => format!("Legacy: {}", step.key),
                                                                }
                                                            }
                                                        });
                                                    ui.horizontal(|ui| {
                                                        egui::ComboBox::from_id_salt((group.id, preset.id, step_index, "toolbox-preset-step"))
                                                            .width(104.0)
                                                            .selected_text(selected_label)
                                                            .show_ui(ui, |ui| {
                                                                for toolbox_preset in &self.state.toolbox_presets {
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
                                                        live_sync |= ui
                                                            .add_sized(
                                                                [122.0, 18.0],
                                                                TextEdit::singleline(&mut step.text_override)
                                                                    .hint_text(Self::tr_lang(language, "Text override", "Text override")),
                                                            )
                                                            .changed();
                                                    });
                                                } else if step.action == MacroAction::TypeText {
                                                    live_sync |= ui
                                                        .add_sized(
                                                            [146.0, 18.0],
                                                            TextEdit::singleline(&mut step.key)
                                                                .hint_text(Self::tr_lang(language, "Text to type", "Text to type")),
                                                        )
                                                        .changed();
                                                } else if matches!(step.action, MacroAction::DisableCrosshair | MacroAction::DisableZoom) {
                                                    ui.add_sized(
                                                        [146.0, 18.0],
                                                        egui::Label::new(Self::tr_lang(language, "No input", "No input")),
                                                    );
                                                } else {
                                                    live_sync |= ui
                                                        .add_sized(
                                                            [146.0, 18.0],
                                                            TextEdit::singleline(&mut step.key),
                                                        )
                                                        .changed();
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
                                                        group_id: group.id,
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
                                            } else if step.action == MacroAction::ShowToolbox {
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
                                            } else {
                                                ui.add_sized([48.0, 18.0], egui::Label::new(""));
                                                ui.add_sized([48.0, 18.0], egui::Label::new(""));
                                            }

                                            if action_supports_capture
                                            {
                                                if ui
                                                    .add_enabled(
                                                        true,
                                                        Button::new(if capture_target_snapshot.as_ref()
                                                            == Some(&CaptureRequest::MacroStepInput {
                                                                group_id: group.id,
                                                                preset_id: preset.id,
                                                                step_index,
                                                            })
                                                        {
                                                            Self::material_icon_text(0xe312, 18.0)
                                                                .strong()
                                                                .color(Color32::from_rgb(255, 232, 96))
                                                        } else {
                                                            Self::material_icon_text(0xe312, 18.0)
                                                        })
                                                            .min_size(vec2(28.0, 18.0)),
                                                    )
                                                    .on_hover_text(Self::tr_lang(
                                                        language,
                                                        "Bắt input",
                                                        "Bắt phím cho bước này",
                                                    ))
                                                    .clicked()
                                                {
                                                    let step_capture_target = CaptureRequest::MacroStepInput {
                                                        group_id: group.id,
                                                        preset_id: preset.id,
                                                        step_index,
                                                    };
                                                    if capture_target_snapshot.as_ref() == Some(&step_capture_target) {
                                                        cancel_active_capture = true;
                                                    } else {
                                                        next_capture_target = Some(step_capture_target);
                                                    }
                                                }
                                            } else {
                                                ui.add_sized([28.0, 18.0], egui::Label::new(""));
                                            }
                                            let paste_button_width = 56.0;
                                            let right_gap = (ui.available_width() - paste_button_width).max(0.0);
                                            if right_gap > 0.0 {
                                                ui.add_space(right_gap);
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
                                        });
                                    })
                                    .response;
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
            if let Some((group_id, preset_id, step_index)) = paste_step_after
                && let Some(selection) =
                    self.paste_macro_steps_after(group_id, preset_id, step_index)
            {
                clear_step_selection = Some((group_id, preset_id));
                selection_after_paste = Some((group_id, preset_id, selection));
                live_sync = true;
            }
            if let Some((group_id, preset_id, step_index, additive)) = pending_step_selection {
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
                    additive,
                    currently_selected,
                    selected_count_in_preset,
                );
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
                                    });

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
    }

    fn render_mouse_path_preview(
        ui: &mut egui::Ui,
        language: UiLanguage,
        events: &[MousePathEvent],
        desired_height: f32,
    ) {
        let desired = vec2(ui.available_width().max(560.0), desired_height.max(180.0));
        let (canvas_rect, _) = ui.allocate_exact_size(desired, Sense::hover());
        let draw_rect = canvas_rect.shrink(8.0);
        ui.painter().rect_filled(
            draw_rect,
            8.0,
            Color32::from_rgba_premultiplied(18, 24, 22, 220),
        );
        ui.painter().rect_stroke(
            draw_rect,
            8.0,
            egui::Stroke::new(1.0, Color32::from_rgb(104, 148, 124)),
            egui::StrokeKind::Outside,
        );
        let moves = events
            .iter()
            .filter(|event| matches!(event.kind, MousePathEventKind::Move))
            .collect::<Vec<_>>();
        if moves.len() < 2 {
            ui.painter().text(
                draw_rect.center(),
                egui::Align2::CENTER_CENTER,
                Self::tr_lang(
                    language,
                    "Record a mouse path to preview it here",
                    "Ghi một đường chuột để xem trước tại đây",
                ),
                egui::FontId::proportional(16.0),
                Color32::from_rgb(210, 210, 210),
            );
            return;
        }

        let min_x = moves.iter().map(|event| event.x).min().unwrap_or(0) as f32;
        let max_x = moves.iter().map(|event| event.x).max().unwrap_or(1) as f32;
        let min_y = moves.iter().map(|event| event.y).min().unwrap_or(0) as f32;
        let max_y = moves.iter().map(|event| event.y).max().unwrap_or(1) as f32;
        let span_x = (max_x - min_x).max(1.0);
        let span_y = (max_y - min_y).max(1.0);
        let scale = ((draw_rect.width() - 20.0) / span_x)
            .min((draw_rect.height() - 20.0) / span_y)
            .max(0.01);
        let content_size = vec2(span_x * scale, span_y * scale);
        let offset = draw_rect.center().to_vec2() - content_size * 0.5;
        let to_pos = |event: &MousePathEvent| {
            egui::pos2(
                offset.x + (event.x as f32 - min_x) * scale,
                offset.y + (event.y as f32 - min_y) * scale,
            )
        };
        let mut last = None;
        for event in moves {
            let current = to_pos(event);
            if let Some(prev) = last {
                ui.painter().line_segment(
                    [prev, current],
                    egui::Stroke::new(2.0, Color32::from_rgb(255, 92, 92)),
                );
            }
            last = Some(current);
        }
    }

    fn render_mouse_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        ui.separator();
        ui.heading(self.tr("Mouse Driver", "Mouse Driver"));
        let driver_downloaded = self.mouse_interception_driver_downloaded();
        let driver_installed = self.mouse_interception_driver_installed();
        let driver_ready = driver_downloaded || driver_installed;
        ui.horizontal_wrapped(|ui| {
            let download_driver_clicked = ui
                .add_enabled(
                    !driver_ready,
                    egui::Button::new(Self::tr_lang(self.state.ui_language, "Download", "Tai")),
                )
                .clicked();
            if download_driver_clicked {
                match self.download_and_install_interception_driver() {
                    Ok(status) => self.status = status,
                    Err(error) => {
                        self.status = match self.state.ui_language {
                            UiLanguage::Vietnamese => {
                                format!("Khong the tai/cai Interception driver: {error}")
                            }
                            _ => format!(
                                "Failed to download/install the Interception driver: {error}"
                            ),
                        }
                    }
                }
            }
            if ui
                .add_enabled(
                    driver_ready,
                    egui::Button::new(Self::tr_lang(self.state.ui_language, "Delete", "Xoa")),
                )
                .clicked()
            {
                match self.uninstall_and_remove_interception_driver() {
                    Ok(status) => {
                        self.persist();
                        self.status = status;
                    }
                    Err(error) => {
                        self.status = match self.state.ui_language {
                            UiLanguage::Vietnamese => {
                                format!("Khong the go/xoa Interception driver: {error}")
                            }
                            _ => format!("Failed to remove the Interception driver: {error}"),
                        }
                    }
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!(
                    "Package: {}",
                    if driver_downloaded {
                        "ready"
                    } else {
                        "missing"
                    }
                ))
                .small(),
            );
            ui.label(
                RichText::new(format!(
                    "Driver: {}",
                    if driver_installed {
                        "installed"
                    } else {
                        "not installed"
                    }
                ))
                .small(),
            );
        });

        ui.add_space(6.0);
        Frame::group(ui.style()).show(ui, |ui| {
            ui.vertical(|ui| {
                ui.heading(self.tr("Mouse Sensitivity", "Mouse Sensitivity"));
                if ui
                    .button(self.tr("+ Add preset", "+ Add preset"))
                    .clicked()
                {
                    self.add_mouse_sensitivity_preset();
                    self.persist_mouse_sensitivity_presets();
                }

                let mut remove_mouse_sensitivity_id = None;
                let mut next_mouse_sensitivity_capture_target = None;
                let mut cancel_active_capture = false;
                let mut mouse_sensitivity_live_sync = false;
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(Self::tr_lang(
                            self.state.ui_language,
                            "Restore",
                            "Khoi phuc khi thoat",
                        ))
                        .strong(),
                    );
                    mouse_sensitivity_live_sync |= ui
                        .checkbox(&mut self.state.mouse_sensitivity_restore_on_exit, "")
                        .changed();
                    ui.label(Self::tr_lang(self.state.ui_language, "Speed", "Toc do"));
                    mouse_sensitivity_live_sync |= ui
                        .add(DragValue::new(&mut self.state.mouse_sensitivity_restore_speed).range(1..=20))
                        .changed();
                });
                for index in 0..self.state.mouse_sensitivity_presets.len() {
                    let language = self.state.ui_language;
                    let dark_mode = self.state.ui_theme == UiThemeMode::Dark;
                    ui.separator();
                    let preset = &mut self.state.mouse_sensitivity_presets[index];
                    Self::show_preset_card(ui, preset.enabled, |ui| {
                        ui.horizontal(|ui| {
                            let enabled_changed = ui.checkbox(&mut preset.enabled, "").changed();
                            mouse_sensitivity_live_sync |= enabled_changed;
                            ui.label(Self::preset_title_text(
                                dark_mode,
                                &preset.name,
                                preset.enabled,
                            ));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .button(Self::tr_lang(
                                        language,
                                        "Restore",
                                        "Khôi phục",
                                    ))
                                    .clicked()
                                {
                                    let _ = self
                                        .overlay_tx
                                        .send(OverlayCommand::RestoreMouseSensitivity);
                                }
                                if ui
                                    .button(Self::tr_lang(
                                        language,
                                        "Apply",
                                        "Áp dụng",
                                    ))
                                    .clicked()
                                {
                                    let _ = self
                                        .overlay_tx
                                        .send(OverlayCommand::ApplyMouseSensitivityPreset(preset.id));
                                }
                                if ui
                                    .button(Self::tr_lang(language, "Remove", "Remove"))
                                    .clicked()
                                {
                                    remove_mouse_sensitivity_id = Some(preset.id);
                                }
                                if ui
                                    .button(if preset.collapsed {
                                        Self::tr_lang(
                                            language,
                                            "Show",
                                            "Hiện",
                                        )
                                    } else {
                                        Self::tr_lang(language, "Hide", "Hide")
                                    })
                                    .clicked()
                                {
                                    preset.collapsed = !preset.collapsed;
                                    mouse_sensitivity_live_sync = true;
                                }
                            });
                            if enabled_changed && !preset.enabled {
                                let _ = self
                                    .overlay_tx
                                    .send(OverlayCommand::RestoreMouseSensitivity);
                            }
                        });
                        if preset.collapsed {
                            return;
                        }
                        egui::Grid::new((preset.id, "mouse-sensitivity-grid"))
                            .num_columns(2)
                            .spacing([14.0, 8.0])
                            .show(ui, |ui| {
                                ui.label(Self::tr_lang(language, "Preset Name", "Preset Name"));
                                mouse_sensitivity_live_sync |= ui
                                    .add_sized([260.0, 24.0], TextEdit::singleline(&mut preset.name))
                                    .changed();
                                ui.end_row();

                                ui.label(Self::tr_lang(language, "Hotkey", "Hotkey"));
                                ui.horizontal_wrapped(|ui| {
                                    ui.monospace(Self::format_binding_ui(language, preset.hotkey.as_ref()));
                                    let capture_target =
                                        CaptureRequest::MouseSensitivityPresetHotkey(preset.id);
                                    let hotkey_active = self.capture_target.as_ref() == Some(&capture_target);
                                    if ui
                                        .button(Self::capture_button_text(language, hotkey_active))
                                        .clicked()
                                    {
                                        if hotkey_active {
                                            cancel_active_capture = true;
                                        } else {
                                            next_mouse_sensitivity_capture_target = Some((
                                                capture_target,
                                                match language {
                                                    UiLanguage::Vietnamese => {
                                                        format!("Đang bật phím tắt cho {}.", preset.name)
                                                    }
                                                    _ => format!("Capturing hotkey for {}.", preset.name),
                                                },
                                            ));
                                        }
                                    }
                                    if ui
                                        .button(Self::tr_lang(language, "Clear", "Clear"))
                                        .clicked()
                                    {
                                        preset.hotkey = None;
                                        mouse_sensitivity_live_sync = true;
                                    }
                                });
                                ui.end_row();

                                ui.label(Self::tr_lang(
                                    language,
                                    "Target Window",
                                    "Cửa sổ mục tiêu",
                                ));
                                mouse_sensitivity_live_sync |= Self::render_multi_window_targets(
                                    ui,
                                    (preset.id, "mouse-sensitivity-target"),
                                    Self::tr_lang(language, "Any window", "Any window"),
                                    &mut preset.target_window_title,
                                    &mut preset.extra_target_window_titles,
                                    &self.open_windows,
                                );
                                ui.end_row();

                                ui.label(Self::tr_lang(
                                    language,
                                    "Titles",
                                    "Tiêu đề trùng",
                                ));
                                mouse_sensitivity_live_sync |= ui
                                    .checkbox(
                                        &mut preset.match_duplicate_window_titles,
                                        Self::tr_lang(language, "Same titles", "Same titles"),
                                    )
                                    .changed();
                                ui.end_row();

                                ui.label(Self::tr_lang(
                                    language,
                                    "Speed",
                                    "Tốc độ chuột",
                                ));
                                mouse_sensitivity_live_sync |= ui
                                    .add(Slider::new(&mut preset.speed, 1..=20).show_value(true))
                                    .changed();
                                ui.end_row();

                                ui.label(Self::tr_lang(
                                    language,
                                    "Live",
                                    "Tốc độ hiện tại",
                                ));
                                ui.horizontal_wrapped(|ui| match Self::current_mouse_speed() {
                                    Some(current_speed) => {
                                        ui.monospace(format!("{current_speed}"));
                                        if current_speed == preset.speed {
                                            ui.label(Self::tr_lang(
                                                language,
                                                "matches this preset",
                                                "khớp với preset này",
                                            ));
                                        }
                                    }
                                    None => {
                                        ui.label(Self::tr_lang(
                                            language,
                                            "Unavailable",
                                            "Không đọc được",
                                        ));
                                    }
                                });
                                ui.end_row();
                            });
                    });
                }
                if let Some(remove_mouse_sensitivity_id) = remove_mouse_sensitivity_id {
                    self.state
                        .mouse_sensitivity_presets
                        .retain(|preset| preset.id != remove_mouse_sensitivity_id);
                    mouse_sensitivity_live_sync = true;
                }
                if let Some((target, status)) = next_mouse_sensitivity_capture_target {
                    self.begin_capture(target, status);
                }
                if mouse_sensitivity_live_sync {
                    self.persist_mouse_sensitivity_presets();
                    self.sync_mouse_sensitivity_settings();
                    self.persist();
                }
                if cancel_active_capture {
                    self.cancel_capture();
                }
            });
        });

        ui.add_space(8.0);
        Frame::group(ui.style()).show(ui, |ui| {
            ui.vertical(|ui| {
                ui.heading(self.tr("Mouse Path", "Mouse Path"));
                ui.horizontal(|ui| {
                    if ui
                        .button(self.tr(
                            "+ Add mouse path",
                            "+ Thêm đường chuột",
                        ))
                        .clicked()
                    {
                        self.add_mouse_path_preset();
                        self.persist_mouse_path_presets();
                    }
                    if let Some(active_id) = self.active_mouse_record_preset_id {
                        ui.label(
                            RichText::new(match self.state.ui_language {
                                UiLanguage::Vietnamese => format!("Đang ghi preset #{active_id}"),
                                _ => format!("Recording preset #{active_id}"),
                            })
                            .strong()
                            .color(Color32::from_rgb(255, 96, 96)),
                        );
                    }
                });

                let mut remove_id = None;
                let mut next_capture_target = None;
                let mut live_sync = false;
                let mut cancel_active_capture = false;
                for index in 0..self.state.mouse_path_presets.len() {
            let language = self.state.ui_language;
            ui.separator();
            let preset = &mut self.state.mouse_path_presets[index];
            Self::show_preset_card(ui, preset.enabled, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut preset.enabled, "");
                    ui.label(Self::preset_title_text(
                        self.state.ui_theme == UiThemeMode::Dark,
                        &preset.name,
                        preset.enabled,
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(Self::tr_lang(language, "Remove", "Remove"))
                            .clicked()
                        {
                            remove_id = Some(preset.id);
                        }
                        if ui
                            .button(if preset.collapsed {
                                Self::tr_lang(
                                    language,
                                    "Show",
                                    "Hiện",
                                )
                            } else {
                                Self::tr_lang(language, "Hide", "Hide")
                            })
                            .clicked()
                        {
                            preset.collapsed = !preset.collapsed;
                            live_sync = true;
                        }
                    });
                });
                if preset.collapsed {
                    return;
                }
                egui::Grid::new((preset.id, "mouse-path-grid"))
                    .num_columns(2)
                    .spacing([14.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(Self::tr_lang(language, "Preset Name", "Preset Name"));
                        live_sync |= ui
                            .add_sized([260.0, 24.0], TextEdit::singleline(&mut preset.name))
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Record Hotkey", "Record Hotkey"));
                        ui.horizontal_wrapped(|ui| {
                            let capture_target = CaptureRequest::MousePathRecordHotkey(preset.id);
                            let capture_active = self.capture_target.as_ref() == Some(&capture_target);
                            if ui
                                .button(Self::capture_button_text(language, capture_active))
                                .clicked()
                            {
                                if capture_active {
                                    cancel_active_capture = true;
                                } else {
                                    next_capture_target = Some((
                                        capture_target,
                                        match language {
                                            UiLanguage::Vietnamese => {
                                                format!(
                                                    "Đang bật phím tắt ghi cho {}.",
                                                    preset.name
                                                )
                                            }
                                            _ => {
                                                format!("Capturing record hotkey for {}.", preset.name)
                                            }
                                        },
                                    ));
                                }
                            }
                            if ui
                                .button(Self::tr_lang(language, "Clear", "Clear"))
                                .clicked()
                            {
                                preset.record_hotkey = None;
                                live_sync = true;
                            }
                        });
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Status", "Status"));
                        ui.horizontal_wrapped(|ui| {
                            if self.active_mouse_record_preset_id == Some(preset.id) {
                                ui.label(
                                    RichText::new(Self::tr_lang(
                                        language,
                                        "Recording via hotkey...",
                                        "Đang ghi bằng phím tắt...",
                                    ))
                                    .color(Color32::from_rgb(255, 96, 96))
                                    .strong(),
                                );
                            } else {
                                ui.label(Self::tr_lang(language, "Ready", "Ready"));
                            }
                            if ui
                                .button(Self::tr_lang(
                                    language,
                                    "Clear path",
                                    "Xóa đường chuột",
                                ))
                                .clicked()
                            {
                                preset.events.clear();
                                live_sync = true;
                            }
                            ui.label(match self.state.ui_language {
                                UiLanguage::Vietnamese => {
                                    format!("{} sự kiện", preset.events.len())
                                }
                                _ => format!("{} events", preset.events.len()),
                            });
                        });
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Driver", "Driver"));
                        ui.horizontal_wrapped(|ui| {
                            live_sync |= ui
                                .checkbox(
                                    &mut preset.use_interception_driver,
                                    Self::tr_lang(
                                        language,
                                        "Use Interception",
                                        "Dung Interception",
                                    ),
                                )
                                .changed();
                            ui.label(
                                RichText::new(if preset.use_interception_driver {
                                    Self::tr_lang(language, "Interception", "Interception")
                                } else {
                                    Self::tr_lang(language, "SendInput", "SendInput")
                                })
                                .small(),
                            );
                        });
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Replay mode", "Replay mode"));
                        ui.horizontal_wrapped(|ui| {
                            live_sync |= ui
                                .checkbox(
                                    &mut preset.replay_relative_motion,
                                    Self::tr_lang(
                                        language,
                                        "Relative motion",
                                        "Di chuyen tuong doi",
                                    ),
                                )
                                .changed();
                            ui.label(
                                RichText::new(Self::tr_lang(
                                    language,
                                    "3D/game mode",
                                    "Che do 3D/game",
                                ))
                                .small(),
                            );
                        });
                        ui.end_row();
                    });
                ui.add_space(6.0);
                Self::render_mouse_path_preview(ui, language, &preset.events, 240.0);
            });
        }
                if let Some(remove_id) = remove_id {
                    self.state
                        .mouse_path_presets
                        .retain(|preset| preset.id != remove_id);
                    live_sync = true;
                }
                if let Some((target, status)) = next_capture_target {
                    self.begin_capture(target, status);
                }
                if cancel_active_capture {
                    self.cancel_capture();
                }
                if live_sync {
                    self.persist_mouse_path_presets();
                }
            });
        });
    }

    fn render_image_search_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let language = self.state.ui_language;
        ui.add_space(2.0);

        ui.horizontal(|ui| {
            if ui
                .button(self.tr("+ Add preset", "+ Add preset"))
                .clicked()
            {
                let id = self.state.next_image_search_preset_id.max(1);
                self.state.next_image_search_preset_id = id + 1;
                self.state
                    .image_search_presets
                    .push(ImageSearchPreset::new(id));
                self.sync_image_search_presets();
                self.persist();
            }
        });

        ui.add_space(4.0);
        let mut remove_id = None;
        let mut live_sync = false;
        let mut remove_timing_id = None;
        let mut timing_live_sync = false;

        for index in 0..self.state.image_search_presets.len() {
            let preset_snapshot = self.state.image_search_presets[index].clone();
            let preview = if preset_snapshot.collapsed {
                self.image_search_preview_cache.remove(&preset_snapshot.id);
                None
            } else {
                self.image_search_preview_for_preset(ctx, &preset_snapshot)
            };
            let mut next_capture = None;
            let mut cancel_active_capture = false;
            let mut start_image_search_capture = None;
            let mut start_search_region_capture = None;
            let mut start_color_pick_capture = None;
            let mut start_color_priority_anchor_capture = None;
            let template_file = self.image_search_template_file_for_preset(preset_snapshot.id);
            let template_ready = template_file.exists();
            let dark_mode = self.state.ui_theme == UiThemeMode::Dark;
            let open_windows = self.open_windows.clone();
            let hotkey_text = preset_snapshot
                .hotkey
                .as_ref()
                .map(|binding| hotkey::format_binding(Some(binding)))
                .unwrap_or_else(|| Self::tr_lang(language, "None", "None").to_owned());
            let hotkey_capture_target = CaptureRequest::ImageSearchPresetHotkey(preset_snapshot.id);
            let hotkey_capture_active =
                self.capture_target.as_ref() == Some(&hotkey_capture_target);
            let preset = &mut self.state.image_search_presets[index];

            Self::show_preset_card(ui, preset.enabled, |ui| {
                ui.horizontal(|ui| {
                    live_sync |= ui.checkbox(&mut preset.enabled, "").changed();
                    ui.label(Self::preset_title_text(
                        dark_mode,
                        &preset.name,
                        preset.enabled,
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(Self::tr_lang(language, "Delete", "Delete"))
                            .clicked()
                        {
                            remove_id = Some(preset.id);
                        }
                        if ui
                            .button(if preset.collapsed {
                                Self::tr_lang(language, "Show", "Show")
                            } else {
                                Self::tr_lang(language, "Hide", "Hide")
                            })
                            .clicked()
                        {
                            preset.collapsed = !preset.collapsed;
                            live_sync = true;
                        }
                    });
                });

                if preset.collapsed {
                    return;
                }

                egui::Grid::new((preset.id, "image-search-grid"))
                    .num_columns(2)
                    .spacing([14.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(Self::tr_lang(language, "Preset Name", "Preset Name"));
                        live_sync |= ui
                            .add_sized([260.0, 24.0], TextEdit::singleline(&mut preset.name))
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Hotkey", "Hotkey"));
                        ui.horizontal_wrapped(|ui| {
                            ui.monospace(hotkey_text.clone());
                            if ui
                                .button(Self::capture_button_text(language, hotkey_capture_active))
                                .clicked()
                            {
                                if hotkey_capture_active {
                                    cancel_active_capture = true;
                                } else {
                                    next_capture = Some((
                                        hotkey_capture_target.clone(),
                                        Self::tr_lang(
                                            language,
                                            "Press a hotkey for this preset.",
                                            "Bam phim tat cho preset nay.",
                                        )
                                        .to_owned(),
                                    ));
                                }
                            }
                            if ui.button(Self::tr_lang(language, "Clear", "Clear")).clicked() {
                                preset.hotkey = None;
                                live_sync = true;
                            }
                        });
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Template", "Template"));
                        ui.horizontal_wrapped(|ui| {
                            ui.label(if template_ready {
                                Self::tr_lang(language, "ready", "ready")
                            } else {
                                Self::tr_lang(language, "missing", "missing")
                            });
                            if ui
                                .button(Self::tr_lang(
                                    language,
                                    "Pick from screen",
                                    "Chon tren man hinh",
                                ))
                                .clicked()
                            {
                                start_image_search_capture = Some(preset.id);
                            }
                            if ui
                                .button(Self::tr_lang(language, "Clear template", "Clear template"))
                                .clicked()
                            {
                                let _ = fs::remove_file(&template_file);
                                self.image_search_preview_cache.remove(&preset.id);
                                preset.enabled = false;
                                live_sync = true;
                            }
                        });
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Area", "Area"));
                        ui.horizontal_wrapped(|ui| {
                            ui.monospace(Self::image_search_search_area_text(preset));
                            if ui
                                .button(Self::tr_lang(language, "Pick area", "Pick area"))
                                .clicked()
                            {
                                start_search_region_capture = Some(preset.id);
                            }
                            if ui
                                .button(Self::tr_lang(language, "Clear area", "Clear area"))
                                .clicked()
                            {
                                preset.search_region_screen_x = None;
                                preset.search_region_screen_y = None;
                                preset.search_region_width = None;
                                preset.search_region_height = None;
                                live_sync = true;
                            }
                        });
                        ui.end_row();
                        ui.horizontal_wrapped(|ui| {
                            live_sync |= ui
                                .checkbox(
                                    &mut preset.search_region_is_circle,
                                    Self::tr_lang(language, "Circle area", "Circle area"),
                                )
                                .on_hover_text(Self::tr_lang(
                                    language,
                                    "Use a circular search region inside the selected box.",
                                    "Dung vung tim hinh tron nam trong khung da chon.",
                                ))
                                .changed();
                            live_sync |= ui
                                .checkbox(
                                    &mut preset.show_search_region_overlay,
                                    Self::tr_lang(language, "Overlay", "Overlay"),
                                )
                                .changed();
                        });
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Offset", "Offset"));
                        ui.horizontal_wrapped(|ui| {
                            ui.label("X");
                            live_sync |= ui
                                .add(DragValue::new(&mut preset.move_offset_x).range(-5000..=5000))
                                .changed();
                            ui.label("Y");
                            live_sync |= ui
                                .add(DragValue::new(&mut preset.move_offset_y).range(-5000..=5000))
                                .changed();
                            ui.label(
                                RichText::new(Self::tr_lang(
                                    language,
                                    "Applied after match",
                                    "Ap dung sau khi khop",
                                ))
                                .small(),
                            );
                        });
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Repeat", "Repeat"));
                        ui.horizontal_wrapped(|ui| {
                            live_sync |= ui
                                .checkbox(
                                    &mut preset.repeat_until_triggered_again,
                                    Self::tr_lang(
                                        language,
                                        "Repeat until triggered again",
                                        "Lap cho den khi bam trigger lai",
                                    ),
                                )
                                .changed();
                            ui.label(
                                RichText::new(Self::tr_lang(
                                    language,
                                    "Move only while active",
                                    "Chi di chuot khi dang bat",
                                ))
                                .small(),
                            );
                        });
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Move", "Move"));
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .button(Self::tr_lang(
                                    language,
                                    if preset.image_search_move_advanced_open {
                                        "Hide advanced"
                                    } else {
                                        "Advanced"
                                    },
                                    if preset.image_search_move_advanced_open {
                                        "An nang cao"
                                    } else {
                                        "Nang cao"
                                    },
                                ))
                                .clicked()
                            {
                                preset.image_search_move_advanced_open =
                                    !preset.image_search_move_advanced_open;
                                live_sync = true;
                            }
                            ui.label(
                                RichText::new(if preset.image_search_move_advanced_open {
                                    Self::tr_lang(language, "Open", "Open")
                                } else {
                                    Self::tr_lang(language, "Closed", "Closed")
                                })
                                .small(),
                            );
                        });
                        ui.end_row();

                        if preset.image_search_move_advanced_open {
                            ui.label(Self::tr_lang(language, "Move behavior", "Move behavior"));
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(Self::tr_lang(
                                        language,
                                        "Uses repeated absolute move.",
                                        "Dung di chuot tuyet doi lap lai.",
                                    ))
                                    .small(),
                                );
                            });
                            ui.end_row();

                            ui.label(Self::tr_lang(language, "Move passes", "Move passes"));
                            ui.horizontal_wrapped(|ui| {
                                live_sync |= ui
                                    .add(
                                        Slider::new(
                                            &mut preset.non_interception_move_passes,
                                            1..=10,
                                        )
                                        .clamping(egui::SliderClamping::Always),
                                    )
                                    .changed();
                                ui.label(
                                    RichText::new(Self::tr_lang(
                                        language,
                                        "Only used when Interception is off",
                                        "Chi dung khi Interception tat",
                                    ))
                                    .small(),
                                );
                            });
                            ui.end_row();

                            ui.label(Self::tr_lang(language, "Move delay", "Move delay"));
                            ui.horizontal_wrapped(|ui| {
                                live_sync |= ui
                                    .add(
                                        Slider::new(
                                            &mut preset.non_interception_move_delay_ms,
                                            0..=100,
                                        )
                                        .clamping(egui::SliderClamping::Always),
                                    )
                                    .changed();
                                ui.label(
                                    RichText::new(Self::tr_lang(
                                        language,
                                        "Delay between extra move passes in ms",
                                        "Do tre giua cac lan di them (ms)",
                                    ))
                                    .small(),
                                );
                            });
                            ui.end_row();
                        }

                        ui.label(Self::tr_lang(language, "Color", "Color"));
                        ui.horizontal_wrapped(|ui| {
                            live_sync |= ui
                                .checkbox(
                                    &mut preset.use_color_matching,
                                    Self::tr_lang(language, "Match color", "Match color"),
                                )
                                .changed();
                            let colors = Self::image_search_target_colors(&preset);
                            let uses_legacy_single_color =
                                preset.target_colors.is_empty() && preset.target_color.is_some();
                            if colors.is_empty() {
                                ui.monospace("None");
                            } else {
                                for (index, color) in colors.iter().copied().enumerate() {
                                    Self::image_search_target_color_swatch(ui, Some(color));
                                    ui.monospace(format!(
                                        "#{:02X}{:02X}{:02X}",
                                        color.r, color.g, color.b
                                    ));
                                    if ui
                                        .small_button(Self::tr_lang(language, "x", "x"))
                                        .on_hover_text(Self::tr_lang(
                                            language,
                                            "Remove this color",
                                            "Xoa mau nay",
                                        ))
                                        .clicked()
                                    {
                                        if uses_legacy_single_color && index == 0 {
                                            preset.target_color = None;
                                            preset.use_color_matching = false;
                                            live_sync = true;
                                        } else if !preset.target_colors.is_empty() {
                                            preset.target_colors = preset
                                                .target_colors
                                                .iter()
                                                .copied()
                                                .enumerate()
                                                .filter_map(|(i, item)| {
                                                    (i != index).then_some(item)
                                                })
                                                .collect();
                                            preset.target_color =
                                                preset.target_colors.first().copied();
                                            live_sync = true;
                                        }
                                    }
                                }
                            }
                            if ui
                                .button(Self::tr_lang(language, "Pick color", "Pick color"))
                                .clicked()
                            {
                                start_color_pick_capture = Some(preset.id);
                            }
                            if preset.use_color_matching && preset.color_priority_from_anchor {
                                if ui
                                    .button(Self::tr_lang(
                                        language,
                                        "Pick priority point",
                                        "Chon diem uu tien",
                                    ))
                                    .clicked()
                                {
                                    start_color_priority_anchor_capture = Some(preset.id);
                                }
                            }
                            if ui.button(Self::tr_lang(language, "Clear", "Clear")).clicked() {
                                preset.target_color = None;
                                preset.target_colors.clear();
                                preset.use_color_matching = false;
                                preset.color_priority_from_anchor = false;
                                preset.color_priority_anchor_screen_x = None;
                                preset.color_priority_anchor_screen_y = None;
                                live_sync = true;
                            }
                        });
                        ui.end_row();

                        if preset.use_color_matching {
                            ui.horizontal_wrapped(|ui| {
                                if ui
                                    .button(Self::tr_lang(
                                        language,
                                        if preset.image_search_advanced_open {
                                            "Hide advanced"
                                        } else {
                                            "Advanced"
                                        },
                                        if preset.image_search_advanced_open {
                                            "An nang cao"
                                        } else {
                                            "Nang cao"
                                        },
                                    ))
                                    .clicked()
                                {
                                    preset.image_search_advanced_open =
                                        !preset.image_search_advanced_open;
                                    live_sync = true;
                                }
                                ui.label(
                                    RichText::new(if preset.image_search_advanced_open {
                                        Self::tr_lang(language, "Open", "Open")
                                    } else {
                                        Self::tr_lang(language, "Closed", "Closed")
                                    })
                                    .small(),
                                );
                            });
                            ui.end_row();

                            if preset.image_search_advanced_open {
                                ui.label(Self::tr_lang(language, "Tolerance", "Tolerance"));
                                ui.horizontal_wrapped(|ui| {
                                    live_sync |= ui
                                        .add(Slider::new(&mut preset.color_tolerance, 0..=96))
                                        .changed();
                                    ui.label(
                                        RichText::new(Self::tr_lang(
                                            language,
                                            "Higher = wider color range",
                                            "Cang cao = mau gan do deu khop",
                                        ))
                                        .small(),
                                    );
                                });
                                ui.end_row();

                                ui.label(Self::tr_lang(language, "Scan rate", "Scan rate"));
                                ui.horizontal_wrapped(|ui| {
                                    live_sync |= ui
                                        .add(
                                            Slider::new(&mut preset.color_scan_rate_hz, 1..=120)
                                                .clamping(egui::SliderClamping::Always),
                                        )
                                        .changed();
                                    ui.label(
                                        RichText::new(Self::tr_lang(
                                            language,
                                            "Scans per second while repeating",
                                            "So lan quet moi giay khi lap",
                                        ))
                                        .small(),
                                    );
                                });
                                ui.end_row();

                                ui.label(Self::tr_lang(language, "Dual midpoint", "Dual midpoint"));
                                ui.horizontal_wrapped(|ui| {
                                    live_sync |= ui
                                        .checkbox(
                                            &mut preset.dual_color_scan_midpoint,
                                            Self::tr_lang(
                                                language,
                                                "Use midpoint of two scans",
                                                "Lay trung diem cua hai luong",
                                            ),
                                        )
                                        .changed();
                                    ui.label(
                                        RichText::new(Self::tr_lang(
                                            language,
                                            "Useful when color shifts with lighting",
                                            "Huu ich khi mau thay doi theo anh sang",
                                        ))
                                        .small(),
                                    );
                                });
                                ui.end_row();

                                ui.label(Self::tr_lang(language, "Priority point", "Priority point"));
                                ui.horizontal_wrapped(|ui| {
                                    live_sync |= ui
                                        .checkbox(
                                            &mut preset.color_priority_from_anchor,
                                            Self::tr_lang(
                                                language,
                                                "Prioritize from point",
                                                "Uu tien tu diem",
                                            ),
                                        )
                                        .changed();
                                    let anchor = preset
                                        .color_priority_anchor_screen_x
                                        .zip(preset.color_priority_anchor_screen_y);
                                    if let Some((x, y)) = anchor {
                                        ui.monospace(format!("{x}, {y}"));
                                        if ui
                                            .small_button(Self::tr_lang(language, "x", "x"))
                                            .on_hover_text(Self::tr_lang(
                                                language,
                                                "Clear priority point",
                                                "Xoa diem uu tien",
                                            ))
                                            .clicked()
                                        {
                                            preset.color_priority_anchor_screen_x = None;
                                            preset.color_priority_anchor_screen_y = None;
                                            live_sync = true;
                                        }
                                    } else {
                                        ui.monospace(Self::tr_lang(language, "None", "None"));
                                    }
                                    if preset.color_priority_from_anchor
                                        && ui
                                            .button(Self::tr_lang(
                                                language,
                                                "Pick point",
                                                "Chon diem",
                                            ))
                                            .clicked()
                                    {
                                        start_color_priority_anchor_capture = Some(preset.id);
                                    }
                                    ui.label(
                                        RichText::new(Self::tr_lang(
                                            language,
                                            "Search starts here and expands outward",
                                            "Tim tu diem nay va lan ra xung quanh",
                                        ))
                                        .small(),
                                    );
                                });
                                ui.end_row();
                            }
                        } else {
                            ui.label(Self::tr_lang(language, "Accuracy", "Accuracy"));
                            ui.horizontal_wrapped(|ui| {
                                live_sync |= ui
                                    .add(
                                        Slider::new(&mut preset.confidence_threshold, 0.35..=0.99)
                                            .fixed_decimals(2)
                                            .show_value(true),
                                    )
                                    .changed();
                                ui.label(
                                    RichText::new(Self::tr_lang(
                                        language,
                                        "Higher = stricter match",
                                        "Cang cao = can khop sat hon",
                                    ))
                                    .small(),
                                );
                            });
                            ui.end_row();
                        }

                        ui.label(Self::tr_lang(language, "Target window", "Target window"));
                        live_sync |= Self::render_multi_window_targets(
                            ui,
                            (preset.id, "image-search-target"),
                            Self::tr_lang(language, "Any screen", "Any screen"),
                            &mut preset.target_window_title,
                            &mut preset.extra_target_window_titles,
                            &open_windows,
                        );
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Duplicate titles", "Duplicate titles"));
                        live_sync |= ui
                            .checkbox(
                                &mut preset.match_duplicate_window_titles,
                                Self::tr_lang(language, "Same titles", "Same titles"),
                            )
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Mouse", "Mouse"));
                        ui.horizontal_wrapped(|ui| {
                            live_sync |= ui
                                .checkbox(
                                    &mut preset.use_interception_driver,
                                    Self::tr_lang(language, "Interception", "Interception"),
                                )
                                .changed();
                            live_sync |= ui
                                .checkbox(
                                    &mut preset.click_after_move,
                                    Self::tr_lang(language, "Click after move", "Click after move"),
                                )
                                .changed();
                        });
                        ui.end_row();
                    });

                if let Some(preview) = preview.as_ref() {
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} {}x{}",
                                preview.file_name, preview.width, preview.height
                            ))
                            .small(),
                        );
                        let base_scale = (320.0 / preview.width.max(1) as f32)
                            .min(180.0 / preview.height.max(1) as f32)
                            .min(1.0);
                        let scale = base_scale / ctx.pixels_per_point().max(1.0);
                        let size =
                            vec2(preview.width as f32 * scale, preview.height as f32 * scale);
                        ui.image((preview.texture.id(), size));
                    });
                }
            });

            if let Some((target, status)) = next_capture {
                self.begin_capture(target, status);
            }
            if let Some(preset_id) = start_image_search_capture {
                self.begin_image_search_capture(
                    ctx,
                    ImageSearchCaptureTarget::Preset(preset_id),
                    ImageSearchCaptureMode::Template,
                );
            }
            if let Some(preset_id) = start_search_region_capture {
                self.begin_image_search_capture(
                    ctx,
                    ImageSearchCaptureTarget::Preset(preset_id),
                    ImageSearchCaptureMode::SearchRegion,
                );
            }
            if let Some(preset_id) = start_color_pick_capture {
                self.begin_image_search_capture(
                    ctx,
                    ImageSearchCaptureTarget::Preset(preset_id),
                    ImageSearchCaptureMode::ColorSample,
                );
            }
            if let Some(preset_id) = start_color_priority_anchor_capture {
                self.begin_image_search_capture(
                    ctx,
                    ImageSearchCaptureTarget::Preset(preset_id),
                    ImageSearchCaptureMode::ColorPriorityAnchor,
                );
            }
            if cancel_active_capture {
                self.cancel_capture();
            }
        }

        ui.add_space(10.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.heading(self.tr("Timing Presets", "Timing Presets"));
            if ui
                .button(self.tr("+ Add timing preset", "+ Add timing preset"))
                .clicked()
            {
                let id = self.state.next_image_search_timing_preset_id.max(1);
                self.state.next_image_search_timing_preset_id = id + 1;
                self.state
                    .image_search_timing_presets
                    .push(ImageSearchTimingPreset::new(id));
                timing_live_sync = true;
            }
        });
        ui.label(
            RichText::new(self.tr(
                "Pick a region and a color. The macro step will infer timing from the color position inside that region.",
                "Chon vung va mau. Buoc macro se tu suy ra timing tu vi tri mau trong vung do.",
            ))
            .small(),
        );

        for index in 0..self.state.image_search_timing_presets.len() {
            let mut start_search_region_capture = None;
            let mut start_color_pick_capture = None;
            let dark_mode = self.state.ui_theme == UiThemeMode::Dark;
            let preset = &mut self.state.image_search_timing_presets[index];

            Self::show_preset_card(ui, preset.enabled, |ui| {
                ui.horizontal(|ui| {
                    timing_live_sync |= ui.checkbox(&mut preset.enabled, "").changed();
                    ui.label(Self::preset_title_text(
                        dark_mode,
                        &preset.name,
                        preset.enabled,
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(Self::tr_lang(language, "Delete", "Delete"))
                            .clicked()
                        {
                            remove_timing_id = Some(preset.id);
                        }
                        if ui
                            .button(if preset.collapsed {
                                Self::tr_lang(language, "Show", "Show")
                            } else {
                                Self::tr_lang(language, "Hide", "Hide")
                            })
                            .clicked()
                        {
                            preset.collapsed = !preset.collapsed;
                            timing_live_sync = true;
                        }
                    });
                });

                ui.label(RichText::new(Self::image_search_timing_preset_text(preset)).small());

                if preset.collapsed {
                    return;
                }

                egui::Grid::new((preset.id, "image-search-timing-grid"))
                    .num_columns(2)
                    .spacing([14.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(Self::tr_lang(language, "Preset Name", "Preset Name"));
                        timing_live_sync |= ui
                            .add_sized([260.0, 24.0], TextEdit::singleline(&mut preset.name))
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Area", "Area"));
                        ui.horizontal_wrapped(|ui| {
                            ui.monospace(Self::image_search_timing_area_text(preset));
                            if ui
                                .button(Self::tr_lang(language, "Pick area", "Pick area"))
                                .clicked()
                            {
                                start_search_region_capture = Some(preset.id);
                            }
                            if ui
                                .button(Self::tr_lang(language, "Clear area", "Clear area"))
                                .clicked()
                            {
                                preset.search_region_screen_x = None;
                                preset.search_region_screen_y = None;
                                preset.search_region_width = None;
                                preset.search_region_height = None;
                                timing_live_sync = true;
                            }
                        });
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Shape", "Shape"));
                        ui.horizontal_wrapped(|ui| {
                            timing_live_sync |= ui
                                .checkbox(
                                    &mut preset.search_region_is_circle,
                                    Self::tr_lang(language, "Circle area", "Circle area"),
                                )
                                .on_hover_text(Self::tr_lang(
                                    language,
                                    "Use a circular search region inside the selected box.",
                                    "Dung vung tim hinh tron nam trong khung da chon.",
                                ))
                                .changed();
                            timing_live_sync |= ui
                                .checkbox(
                                    &mut preset.show_search_region_overlay,
                                    Self::tr_lang(language, "Overlay", "Overlay"),
                                )
                                .changed();
                        });
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Color", "Color"));
                        ui.horizontal_wrapped(|ui| {
                            Self::image_search_target_color_swatch(
                                ui,
                                preset.target_color.or_else(|| preset.target_colors.first().copied()),
                            );
                            ui.monospace(Self::image_search_timing_color_text(preset));
                            if ui
                                .button(Self::tr_lang(language, "Pick color", "Pick color"))
                                .clicked()
                            {
                                start_color_pick_capture = Some(preset.id);
                            }
                            if ui
                                .button(Self::tr_lang(language, "Clear color", "Clear color"))
                                .clicked()
                            {
                                preset.target_color = None;
                                preset.target_colors.clear();
                                timing_live_sync = true;
                            }
                        });
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Tolerance", "Tolerance"));
                        timing_live_sync |= ui
                            .add(
                                DragValue::new(&mut preset.color_tolerance)
                                    .range(0..=255)
                                    .suffix(" / 255"),
                            )
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Scan rate", "Scan rate"));
                        timing_live_sync |= ui
                            .add(
                                DragValue::new(&mut preset.color_scan_rate_hz)
                                    .range(1..=240)
                                    .suffix(" Hz"),
                            )
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Cycle", "Cycle"));
                        timing_live_sync |= ui
                            .add(
                                DragValue::new(&mut preset.timing_cycle_ms)
                                    .range(100..=60_000)
                                    .suffix(" ms"),
                            )
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Loop", "Loop"));
                        ui.horizontal_wrapped(|ui| {
                            timing_live_sync |= ui
                                .checkbox(
                                    &mut preset.loop_enabled,
                                    Self::tr_lang(language, "Enable loop", "Enable loop"),
                                )
                                .changed();
                            if preset.loop_enabled {
                                timing_live_sync |= ui
                                    .checkbox(
                                        &mut preset.loop_forever,
                                        Self::tr_lang(language, "Infinite", "Infinite"),
                                    )
                                    .changed();
                                if !preset.loop_forever {
                                    ui.label(Self::tr_lang(language, "Seconds", "Seconds"));
                                    timing_live_sync |= ui
                                        .add(
                                            DragValue::new(&mut preset.loop_duration_secs)
                                                .range(1..=86400)
                                                .suffix(" s"),
                                        )
                                        .changed();
                                }
                            }
                        });
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Match", "Match"));
                        ui.label(
                            RichText::new(Self::tr_lang(
                                language,
                                "The macro step toggles this timing loop on or off. A loop can run forever or stop after the chosen duration.",
                                "Buoc macro se bat/tat loop timing nay. Loop co the chay vo han hoac dung sau thoi gian da chon.",
                            ))
                            .small(),
                        );
                        ui.end_row();
                    });
            });

            if let Some(preset_id) = start_search_region_capture {
                self.begin_image_search_capture(
                    ctx,
                    ImageSearchCaptureTarget::TimingPreset(preset_id),
                    ImageSearchCaptureMode::SearchRegion,
                );
            }
            if let Some(preset_id) = start_color_pick_capture {
                self.begin_image_search_capture(
                    ctx,
                    ImageSearchCaptureTarget::TimingPreset(preset_id),
                    ImageSearchCaptureMode::ColorSample,
                );
            }
        }

        if let Some(remove_id) = remove_id {
            if let Some(preset) = self
                .state
                .image_search_presets
                .iter()
                .find(|preset| preset.id == remove_id)
            {
                let template_file = self.image_search_template_file_for_preset(preset.id);
                let _ = fs::remove_file(&template_file);
            }
            self.image_search_preview_cache.remove(&remove_id);
            self.state
                .image_search_presets
                .retain(|preset| preset.id != remove_id);
            live_sync = true;
        }

        if let Some(remove_timing_id) = remove_timing_id {
            self.state
                .image_search_timing_presets
                .retain(|preset| preset.id != remove_timing_id);
            timing_live_sync = true;
        }

        if live_sync || timing_live_sync {
            self.sync_image_search_presets();
            self.sync_image_search_timing_presets();
            self.persist();
        }
    }

    fn render_image_search_capture_overlay(&mut self, ctx: &egui::Context) -> bool {
        if !self.image_search_capture_active {
            return false;
        }

        ctx.request_repaint();
        egui::CentralPanel::default()
            .frame(Frame::new().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let response = ui.allocate_rect(rect, Sense::click_and_drag());
                let painter = ui.painter_at(rect);
                let capture_mode = self
                    .image_search_capture_mode
                    .unwrap_or(ImageSearchCaptureMode::Template);
                let instruction = match capture_mode {
                    ImageSearchCaptureMode::Template => self.tr(
                        "Drag to capture an image template. Press Esc to cancel.",
                        "Keo de chup mau. Bam Esc de huy.",
                    ),
                    ImageSearchCaptureMode::SearchRegion => self.tr(
                        "Drag to pick the search area. Press Esc to cancel.",
                        "Keo de chon vung tim. Bam Esc de huy.",
                    ),
                    ImageSearchCaptureMode::ColorSample => self.tr(
                        "Click a pixel to pick the target color. Press Esc to cancel.",
                        "Bam vao diem anh de lay mau muc tieu. Bam Esc de huy.",
                    ),
                    ImageSearchCaptureMode::ColorPriorityAnchor => self.tr(
                        "Click a point to set the color priority anchor. Press Esc to cancel.",
                        "Bam vao diem de dat moc uu tien mau. Bam Esc de huy.",
                    ),
                };

                painter.text(
                    rect.left_top() + vec2(18.0, 18.0),
                    egui::Align2::LEFT_TOP,
                    instruction,
                    egui::FontId::proportional(18.0),
                    Color32::WHITE,
                );
                if let Some(target) = self.image_search_capture_target
                    && let Some(name) = self.image_search_capture_target_name(target)
                {
                    let target_label = match target {
                        ImageSearchCaptureTarget::Preset(_) => self.tr("Preset", "Preset"),
                        ImageSearchCaptureTarget::TimingPreset(_) => {
                            self.tr("Timing preset", "Timing preset")
                        }
                    };
                    painter.text(
                        rect.left_top() + vec2(18.0, 44.0),
                        egui::Align2::LEFT_TOP,
                        format!("{target_label}: {name}"),
                        egui::FontId::proportional(14.0),
                        Color32::from_rgb(210, 228, 255),
                    );
                }

                let precise_pointer = self
                    .precise_image_search_capture_pointer(ctx)
                    .filter(|pointer| rect.contains(*pointer));
                let preview_pointer = precise_pointer
                    .or(response.interact_pointer_pos())
                    .or(response.hover_pos());
                if let Some(pointer) = preview_pointer {
                    let preview_sample_size = if capture_mode == ImageSearchCaptureMode::Template {
                        29
                    } else {
                        17
                    };
                    let sampled_color =
                        self.update_image_search_cursor_preview(ctx, pointer, preview_sample_size);
                    let screen_point =
                        self.screen_point_from_pos(ctx, pointer, ctx.pixels_per_point());
                    self.render_image_search_cursor_preview_panel(
                        &painter,
                        rect,
                        pointer,
                        sampled_color,
                        screen_point,
                    );
                    if matches!(
                        capture_mode,
                        ImageSearchCaptureMode::ColorSample
                            | ImageSearchCaptureMode::ColorPriorityAnchor
                    ) {
                        painter.circle_stroke(
                            pointer,
                            9.0,
                            egui::Stroke::new(2.0, Color32::from_rgb(120, 220, 255)),
                        );
                        painter.line_segment(
                            [pointer + vec2(-14.0, 0.0), pointer + vec2(-4.0, 0.0)],
                            egui::Stroke::new(1.0, Color32::from_rgb(120, 220, 255)),
                        );
                        painter.line_segment(
                            [pointer + vec2(4.0, 0.0), pointer + vec2(14.0, 0.0)],
                            egui::Stroke::new(1.0, Color32::from_rgb(120, 220, 255)),
                        );
                        painter.line_segment(
                            [pointer + vec2(0.0, -14.0), pointer + vec2(0.0, -4.0)],
                            egui::Stroke::new(1.0, Color32::from_rgb(120, 220, 255)),
                        );
                        painter.line_segment(
                            [pointer + vec2(0.0, 4.0), pointer + vec2(0.0, 14.0)],
                            egui::Stroke::new(1.0, Color32::from_rgb(120, 220, 255)),
                        );
                    }
                }
                if capture_mode == ImageSearchCaptureMode::ColorSample
                    || capture_mode == ImageSearchCaptureMode::ColorPriorityAnchor
                {
                    if response.clicked()
                        && let Some(pointer) = precise_pointer
                            .or(response.interact_pointer_pos())
                            .or(response.hover_pos())
                    {
                        if capture_mode == ImageSearchCaptureMode::ColorSample {
                            self.finish_image_search_color_pick(ctx, pointer);
                        } else {
                            self.finish_image_search_color_priority_anchor_pick(ctx, pointer);
                        }
                        return;
                    }
                } else {
                    let pointer_down = ui.input(|input| input.pointer.primary_down());
                    if pointer_down
                        && self.image_search_capture_anchor.is_none()
                        && let Some(origin) =
                            precise_pointer.or(ui.input(|input| input.pointer.press_origin()))
                        && rect.contains(origin)
                    {
                        self.image_search_capture_anchor = Some(origin);
                        self.image_search_capture_current = Some(origin);
                    }
                    if pointer_down
                        && self.image_search_capture_anchor.is_some()
                        && let Some(pointer) = precise_pointer
                            .or(response.interact_pointer_pos())
                            .or(response.hover_pos())
                    {
                        self.image_search_capture_current = Some(pointer);
                    }

                    let pointer_released = ui.input(|input| input.pointer.any_released());
                    if pointer_released
                        && let (Some(anchor), Some(current)) = (
                            self.image_search_capture_anchor,
                            self.image_search_capture_current,
                        )
                    {
                        let selection = egui::Rect::from_two_pos(anchor, current);
                        if selection.width() >= 2.0 && selection.height() >= 2.0 {
                            self.finish_image_search_capture(ctx, selection);
                        } else {
                            self.cancel_image_search_capture(ctx);
                        }
                        return;
                    }
                }

                if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
                    self.cancel_image_search_capture(ctx);
                    return;
                }

                if let (Some(anchor), Some(current)) = (
                    self.image_search_capture_anchor,
                    self.image_search_capture_current,
                ) {
                    let selection = egui::Rect::from_two_pos(anchor, current);
                    let use_circle = capture_mode == ImageSearchCaptureMode::SearchRegion
                        && self.image_search_capture_target.is_some_and(|target| {
                            self.image_search_capture_target_is_circle(target)
                        });
                    if use_circle {
                        painter.circle_stroke(
                            selection.center(),
                            selection.width().min(selection.height()) * 0.5,
                            egui::Stroke::new(2.0, Color32::from_rgb(120, 220, 255)),
                        );
                    } else {
                        painter.rect_stroke(
                            selection,
                            0.0,
                            egui::Stroke::new(2.0, Color32::from_rgb(120, 220, 255)),
                            egui::StrokeKind::Middle,
                        );
                    }
                }
            });
        true
    }

    fn render_mouse_move_absolute_capture_overlay(&mut self, ctx: &egui::Context) -> bool {
        if self.mouse_move_absolute_capture_target.is_none() {
            return false;
        }

        ctx.request_repaint();
        egui::CentralPanel::default()
            .frame(Frame::new().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let _response = ui.allocate_rect(rect, Sense::click_and_drag());
                let painter = ui.painter_at(rect);
                let instruction = self.tr(
                    "Click a point to capture the mouse X/Y. Press Esc to cancel.",
                    "Bấm vào điểm muốn lấy tọa độ chuột X/Y. Nhấn Esc để hủy.",
                );
                painter.text(
                    rect.left_top() + vec2(18.0, 18.0),
                    egui::Align2::LEFT_TOP,
                    instruction,
                    egui::FontId::proportional(18.0),
                    Color32::WHITE,
                );
                if let Some(pointer) = self.precise_image_search_capture_pointer(ctx) {
                    let sampled_color = self.update_image_search_cursor_preview(ctx, pointer, 21);
                    let screen_point =
                        self.screen_point_from_pos(ctx, pointer, ctx.pixels_per_point());
                    self.render_image_search_cursor_preview_panel(
                        &painter,
                        rect,
                        pointer,
                        sampled_color,
                        screen_point,
                    );
                    painter.circle_stroke(
                        pointer,
                        9.0,
                        egui::Stroke::new(2.0, Color32::from_rgb(120, 220, 255)),
                    );
                    painter.line_segment(
                        [pointer + vec2(-14.0, 0.0), pointer + vec2(-4.0, 0.0)],
                        egui::Stroke::new(1.0, Color32::from_rgb(120, 220, 255)),
                    );
                    painter.line_segment(
                        [pointer + vec2(4.0, 0.0), pointer + vec2(14.0, 0.0)],
                        egui::Stroke::new(1.0, Color32::from_rgb(120, 220, 255)),
                    );
                    painter.line_segment(
                        [pointer + vec2(0.0, -14.0), pointer + vec2(0.0, -4.0)],
                        egui::Stroke::new(1.0, Color32::from_rgb(120, 220, 255)),
                    );
                    painter.line_segment(
                        [pointer + vec2(0.0, 4.0), pointer + vec2(0.0, 14.0)],
                        egui::Stroke::new(1.0, Color32::from_rgb(120, 220, 255)),
                    );
                }
                if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
                    self.cancel_mouse_move_absolute_capture(ctx);
                }
            });
        true
    }

    fn render_sound_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        ui.add_space(2.0);
        let mut changed = false;

        ui.separator();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(self.tr("Sound Presets", "Sound Presets")).strong(),
            );
            if ui
                .button(self.tr(
                    "+ Add Sound Preset",
                    "+ Thêm preset âm thanh",
                ))
                .clicked()
            {
                let id = self.state.audio_settings.next_preset_id;
                self.state.audio_settings.next_preset_id += 1;
                self.state.audio_settings.presets.push(SoundPreset::new(id));
                self.show_sound_preset_audio_editor.insert(id);
                changed = true;
            }
        });

        let mut remove_sound_preset = None;
        for index in 0..self.state.audio_settings.presets.len() {
            let mut choose_file_for = None;
            let mut open_editor_target = None;
            let preset = &mut self.state.audio_settings.presets[index];
            let waveform_path = preset.clip.file_path.trim().to_owned();
            let waveform = self.audio_waveforms.get(&waveform_path).cloned();
            let mut duration = self
                .sound_preset_clip_duration_ms
                .get(&preset.id)
                .copied()
                .flatten()
                .or_else(|| audio_duration(&preset.clip));
            let mut show_editor = self.show_sound_preset_audio_editor.contains(&preset.id);

            ui.add_space(6.0);
            Self::show_preset_card(ui, preset.clip.enabled, |ui| {
                ui.horizontal(|ui| {
                    changed |= ui
                        .checkbox(&mut preset.clip.enabled, "")
                        .on_hover_text(Self::tr_lang(
                            language,
                            "Enable this sound preset",
                            "Bật preset âm thanh này",
                        ))
                        .changed();
                    changed |= ui
                        .add_sized([220.0, 24.0], TextEdit::singleline(&mut preset.name))
                        .changed();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(Self::tr_lang(language, "Remove", "Remove"))
                            .clicked()
                        {
                            remove_sound_preset = Some(preset.id);
                        }
                        if ui
                            .button(if preset.collapsed {
                                Self::tr_lang(language, "Show", "Show")
                            } else {
                                Self::tr_lang(language, "Hide", "Hide")
                            })
                            .clicked()
                        {
                            preset.collapsed = !preset.collapsed;
                            changed = true;
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
            self.state
                .audio_settings
                .presets
                .retain(|preset| preset.id != preset_id);
            self.sound_preset_clip_duration_ms.remove(&preset_id);
            self.show_sound_preset_audio_editor.remove(&preset_id);
            changed = true;
        }

        ui.separator();
        if changed {
            self.sync_audio_settings();
            self.persist();
        }
    }

    fn render_audio_media_editor(
        ui: &mut egui::Ui,
        language: UiLanguage,
        id_source: impl std::hash::Hash + Copy,
        title: &str,
        clip: &mut AudioClipSettings,
        duration_ms: &mut Option<u64>,
        waveform: Option<&[f32]>,
    ) -> AudioCardOutcome {
        let mut outcome = AudioCardOutcome::default();
        let previewing = audio::is_previewing(clip);

        ui.heading(Self::tr_lang(language, "Media", "Media"));
        ui.label(RichText::new(title).strong());
        ui.add_space(6.0);

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
                    Button::new(if previewing {
                        Self::material_icon_text(0xe034, 18.0)
                    } else {
                        Self::material_icon_text(0xe037, 18.0)
                    }),
                )
                .on_hover_text(if previewing {
                    Self::tr_lang(
                        language,
                        "Stop preview",
                        "Dừng nghe thử",
                    )
                } else {
                    Self::tr_lang(
                        language,
                        "Preview audio",
                        "Nghe thử âm thanh",
                    )
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
                            UiLanguage::Vietnamese => format!(
                                "Đã dừng nghe thử {title}."
                            ),
                            _ => format!("Stopped {title} preview."),
                        })
                    }
                    Err(error) => {
                        outcome.status = Some(match language {
                            UiLanguage::Vietnamese => format!(
                                "Nghe thử thất bại: {error}"
                            ),
                            _ => format!("Preview failed: {error}"),
                        })
                    }
                }
            }
            if ui
                .add_enabled(
                    !clip.file_path.trim().is_empty(),
                    Button::new(Self::material_icon_text(0xe15b, 18.0)),
                )
                .on_hover_text(Self::tr_lang(
                    language,
                    "Clear audio file",
                    "Xóa file âm thanh",
                ))
                .clicked()
            {
                audio::stop_preview();
                clip.file_path.clear();
                clip.start_ms = 0;
                clip.end_ms = 0;
                clip.volume = 1.0;
                clip.speed = 1.0;
                *duration_ms = None;
                outcome.changed = true;
                outcome.status = Some(match language {
                    UiLanguage::Vietnamese => format!("Đã xóa {title}."),
                    _ => format!("Cleared {title}."),
                });
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
                Self::tr_lang(language, "Slice", "Slice"),
                Self::format_ms(clip.end_ms.saturating_sub(clip.start_ms))
            ));
            ui.add_space(8.0);
            outcome.changed |= Self::render_audio_trim_bar(
                ui,
                (id_source, "trim"),
                clip,
                total_ms,
                waveform,
                180.0,
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(Self::tr_lang(
                    language,
                    "Start",
                    "Bắt đầu",
                ));
                outcome.changed |= ui
                    .add(DragValue::new(&mut clip.start_ms).range(0..=total_ms))
                    .changed();
                ui.label(Self::tr_lang(language, "End", "End"));
                outcome.changed |= ui
                    .add(DragValue::new(&mut clip.end_ms).range(0..=total_ms))
                    .changed();
            });
            Self::trim_audio_bounds(clip, total_ms);
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(Self::tr_lang(
                language,
                "Volume",
                "Âm lượng",
            ));
            outcome.changed |= ui
                .add(
                    Slider::new(&mut clip.volume, 0.0..=2.0)
                        .text("x")
                        .clamping(egui::SliderClamping::Always),
                )
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label(Self::tr_lang(
                language,
                "Speed",
                "Tốc độ",
            ));
            outcome.changed |= ui
                .add(
                    Slider::new(&mut clip.speed, 0.25..=3.0)
                        .text("x")
                        .clamping(egui::SliderClamping::Always),
                )
                .changed();
        });

        outcome
    }

    fn render_media_panel(&mut self, ui: &mut egui::Ui) {
        let language = self.state.ui_language;
        let Some(target) = self.active_audio_editor else {
            self.state.active_panel = AppPanel::Sound;
            self.render_sound_panel(ui);
            return;
        };

        ui.horizontal(|ui| {
            if ui
                .button(self.tr("Back", "Back"))
                .clicked()
            {
                self.close_audio_editor();
            }
        });
        ui.separator();

        match target {
            AudioEditorTarget::Preset(preset_id) => {
                let space_pressed = ui.input(|input| input.key_pressed(egui::Key::Space));
                let waveform_path = self
                    .state
                    .audio_settings
                    .presets
                    .iter()
                    .find(|preset| preset.id == preset_id)
                    .map(|preset| preset.clip.file_path.trim().to_owned())
                    .unwrap_or_default();
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
                        .flatten()
                        .or_else(|| audio_duration(&preset.clip));
                    let outcome = Self::render_audio_media_editor(
                        ui,
                        language,
                        ("preset", preset.id),
                        &format!(
                            "{}: {}",
                            Self::tr_lang(
                                language,
                                "Sound Preset",
                                "Preset âm thanh"
                            ),
                            preset.name
                        ),
                        &mut preset.clip,
                        &mut duration,
                        waveform.as_deref(),
                    );
                    self.sound_preset_clip_duration_ms
                        .insert(preset.id, duration);
                    if outcome.choose_file {
                        choose_file_for = Some(preset.id);
                    }
                    let preview_clip = preset.clip.clone();
                    let preview_label = preset.name.clone();
                    if space_pressed && !preview_clip.file_path.trim().is_empty() {
                        match audio::toggle_preview(preview_clip) {
                            Ok(true) => {
                                self.status = match language {
                                    UiLanguage::Vietnamese => format!(
                                        "Đang nghe thử {}.",
                                        preview_label
                                    ),
                                    _ => format!("Previewing {}.", preview_label),
                                }
                            }
                            Ok(false) => {
                                self.status = match language {
                                    UiLanguage::Vietnamese => format!(
                                        "Đã dừng nghe thử {}.",
                                        preview_label
                                    ),
                                    _ => format!("Stopped {} preview.", preview_label),
                                }
                            }
                            Err(error) => {
                                self.status = match language {
                                    UiLanguage::Vietnamese => {
                                        format!("Nghe thử thất bại: {error}")
                                    }
                                    _ => format!("Preview failed: {error}"),
                                }
                            }
                        }
                    } else if let Some(status) = outcome.status {
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
    }

    fn render_settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        if ui
            .button(self.tr(
                "+ Add toolbox preset",
                "+ Thêm preset toolbox",
            ))
            .clicked()
        {
            self.add_toolbox_preset();
            self.persist_toolbox_presets();
        }

        let mut remove_id = None;
        let mut changed = false;
        let mut active_preview: Option<ToolboxPreset> = None;
        for index in 0..self.state.toolbox_presets.len() {
            let language = self.state.ui_language;
            ui.add_space(6.0);
            let preset = &mut self.state.toolbox_presets[index];
            Self::show_preset_card(ui, true, |ui| {
                ui.horizontal(|ui| {
                    changed |= ui
                        .add_sized([220.0, 24.0], TextEdit::singleline(&mut preset.name))
                        .changed();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(Self::tr_lang(language, "Remove", "Remove"))
                            .clicked()
                        {
                            remove_id = Some(preset.id);
                        }
                        if ui
                            .button(if preset.collapsed {
                                Self::tr_lang(
                                    language,
                                    "Show",
                                    "Hiện",
                                )
                            } else {
                                Self::tr_lang(language, "Hide", "Hide")
                            })
                            .clicked()
                        {
                            preset.collapsed = !preset.collapsed;
                            changed = true;
                        }
                    });
                });
                if preset.collapsed {
                    if preset.preview_enabled {
                        preset.preview_enabled = false;
                        changed = true;
                    }
                    return;
                }

                egui::Grid::new((preset.id, "toolbox-preset-grid"))
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(Self::tr_lang(language, "Text", "Text"));
                        changed |= ui
                            .add_sized([360.0, 24.0], TextEdit::singleline(&mut preset.text))
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(
                            language,
                            "Font Size",
                            "Cỡ chữ",
                        ));
                        changed |= ui
                            .add(
                                Slider::new(&mut preset.font_size, 1.0..=200.0)
                                    .text("px")
                                    .clamping(egui::SliderClamping::Always),
                            )
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(
                            language,
                            "Text Color",
                            "Màu chữ",
                        ));
                        changed |= Self::edit_rgba_color(ui, &mut preset.text_color);
                        ui.end_row();

                        ui.label(Self::tr_lang(
                            language,
                            "Background Color",
                            "Màu nền",
                        ));
                        changed |= Self::edit_rgba_color(ui, &mut preset.background_color);
                        ui.end_row();

                        ui.label(Self::tr_lang(
                            language,
                            "Background Opacity",
                            "Độ mờ nền",
                        ));
                        changed |= ui
                            .add(
                                Slider::new(&mut preset.background_opacity, 0.0..=1.0)
                                    .text("")
                                    .clamping(egui::SliderClamping::Always),
                            )
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(
                            language,
                            "Rounded Background",
                            "Nền bo góc",
                        ));
                        changed |= ui
                            .checkbox(
                                &mut preset.rounded_background,
                                Self::tr_lang(language, "Rounded corners", "Rounded corners"),
                            )
                            .changed();
                        ui.end_row();

                        ui.label(Self::tr_lang(language, "Preview", "Preview"));
                        changed |= ui
                            .checkbox(
                                &mut preset.preview_enabled,
                                Self::tr_lang(
                                    language,
                                    "Stream preview in editor",
                                    "Stream preview trong editor",
                                ),
                            )
                            .changed();
                        ui.end_row();
                    });

                ui.add_space(6.0);
                ui.label(
                    RichText::new(Self::tr_lang(
                        language,
                        "Position Preview",
                        "Preview vị trí",
                    ))
                    .strong(),
                );
                changed |=
                    Self::render_toolbox_rect_editor(ui, (preset.id, "toolbox-editor"), preset);

                if preset.preview_enabled {
                    active_preview = Some(preset.clone());
                }
            });
        }

        if let Some(id) = remove_id {
            self.state.toolbox_presets.retain(|preset| preset.id != id);
            changed = true;
        }
        self.sync_toolbox_preview(active_preview.as_ref());
        if changed {
            self.persist_toolbox_presets();
        }
    }

    fn animation_min_size() -> egui::Vec2 {
        vec2(96.0, 56.0)
    }

    fn render_custom_window_resize_handles(&self, ctx: &egui::Context) {
        if ctx.input(|input| input.viewport().maximized.unwrap_or(false)) {
            return;
        }

        let rect = ctx.content_rect();
        let edge = 8.0;
        let corner = 22.0;
        let handles = [
            (
                "resize-n",
                egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.min.y + edge)),
                egui::viewport::ResizeDirection::North,
                egui::CursorIcon::ResizeVertical,
            ),
            (
                "resize-s",
                egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - edge), rect.max),
                egui::viewport::ResizeDirection::South,
                egui::CursorIcon::ResizeVertical,
            ),
            (
                "resize-w",
                egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + edge, rect.max.y)),
                egui::viewport::ResizeDirection::West,
                egui::CursorIcon::ResizeHorizontal,
            ),
            (
                "resize-e",
                egui::Rect::from_min_max(egui::pos2(rect.max.x - edge, rect.min.y), rect.max),
                egui::viewport::ResizeDirection::East,
                egui::CursorIcon::ResizeHorizontal,
            ),
            (
                "resize-nw",
                egui::Rect::from_min_size(rect.min, vec2(corner, corner)),
                egui::viewport::ResizeDirection::NorthWest,
                egui::CursorIcon::ResizeNwSe,
            ),
            (
                "resize-ne",
                egui::Rect::from_min_max(
                    egui::pos2(rect.max.x - corner, rect.min.y),
                    egui::pos2(rect.max.x, rect.min.y + corner),
                ),
                egui::viewport::ResizeDirection::NorthEast,
                egui::CursorIcon::ResizeNeSw,
            ),
            (
                "resize-sw",
                egui::Rect::from_min_max(
                    egui::pos2(rect.min.x, rect.max.y - corner),
                    egui::pos2(rect.min.x + corner, rect.max.y),
                ),
                egui::viewport::ResizeDirection::SouthWest,
                egui::CursorIcon::ResizeNeSw,
            ),
            (
                "resize-se",
                egui::Rect::from_min_max(
                    egui::pos2(rect.max.x - corner, rect.max.y - corner),
                    rect.max,
                ),
                egui::viewport::ResizeDirection::SouthEast,
                egui::CursorIcon::ResizeNwSe,
            ),
        ];

        for (id, handle_rect, direction, cursor) in handles {
            egui::Area::new(egui::Id::new(id))
                .order(egui::Order::Foreground)
                .fixed_pos(handle_rect.min)
                .interactable(true)
                .show(ctx, |ui| {
                    let (_, response) =
                        ui.allocate_exact_size(handle_rect.size(), Sense::click_and_drag());
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(cursor);
                    }
                    if response.drag_started() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
                    }
                });
        }
    }

    fn render_custom_window_border(&self, ctx: &egui::Context) {
        let rect = ctx.content_rect().shrink(0.5);

        let stroke = if self.state.ui_theme == UiThemeMode::Dark {
            egui::Stroke::new(1.4, Color32::from_rgb(64, 84, 108))
        } else {
            egui::Stroke::new(1.4, Color32::from_rgb(184, 198, 214))
        };
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("window-border"),
        ));
        painter.rect_stroke(rect, 16.0, stroke, egui::StrokeKind::Outside);
    }

    fn begin_close_to_tray_animation(&mut self, ctx: &egui::Context) {
        let viewport = ctx.input(|input| input.viewport().clone());
        if let Some(outer_rect) = viewport.outer_rect {
            let inner_size = viewport
                .inner_rect
                .map(|rect| rect.size())
                .unwrap_or_else(|| outer_rect.size());
            self.hidden_window_inner_size = Some(inner_size);
            self.hidden_window_outer_pos = Some(outer_rect.min);
        }
        self.finish_close_to_tray_hide(ctx);
    }

    fn update_close_to_tray_animation(&mut self, ctx: &egui::Context) {
        let Some(animation) = &self.close_to_tray_animation else {
            return;
        };

        let elapsed = (ctx.input(|input| input.time) - animation.started_at).max(0.0);
        let progress = (elapsed / animation.duration_sec).clamp(0.0, 1.0) as f32;

        if progress >= 1.0 {
            self.finish_close_to_tray_hide(ctx);
        } else {
            ctx.request_repaint();
        }
    }

    fn begin_open_from_tray_animation(
        &mut self,
        ctx: &egui::Context,
        end_outer_pos: egui::Pos2,
        end_inner_size: egui::Vec2,
    ) {
        let end_inner_size = Self::square_window_size(end_inner_size);
        self.close_to_tray_animation = None;
        self.open_from_tray_animation = None;
        self.state.show_window = true;
        self.enforce_square_window_frames = 8;
        self.center_window_next_frame = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(end_inner_size));
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(end_outer_pos));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
            egui::UserAttentionType::Informational,
        ));
        let _ = self.overlay_tx.send(OverlayCommand::SetUiVisible(true));
        crate::overlay::wake_command_queue();
    }

    fn update_open_from_tray_animation(&mut self, ctx: &egui::Context) {
        let Some(animation) = &self.open_from_tray_animation else {
            return;
        };

        let elapsed = (ctx.input(|input| input.time) - animation.started_at).max(0.0);
        let progress = (elapsed / animation.duration_sec).clamp(0.0, 1.0) as f32;
        let eased = 1.0 - (1.0 - progress).powi(3);
        let new_size = vec2(
            animation.start_inner_size.x
                + (animation.end_inner_size.x - animation.start_inner_size.x) * eased,
            animation.start_inner_size.y
                + (animation.end_inner_size.y - animation.start_inner_size.y) * eased,
        );
        let new_pos = egui::pos2(
            animation.start_outer_pos.x
                + (animation.end_outer_pos.x - animation.start_outer_pos.x) * eased,
            animation.start_outer_pos.y
                + (animation.end_outer_pos.y - animation.start_outer_pos.y) * eased,
        );

        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(new_size));
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(new_pos));

        if progress >= 1.0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(animation.end_inner_size));
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                animation.end_outer_pos,
            ));
            self.open_from_tray_animation = None;
        } else {
            ctx.request_repaint();
        }
    }

    fn finish_close_to_tray_hide(&mut self, ctx: &egui::Context) {
        self.close_to_tray_animation = None;
        self.state.show_window = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        let _ = self.overlay_tx.send(OverlayCommand::SetUiVisible(false));
        crate::overlay::wake_command_queue();
        self.persist();
    }
}

impl eframe::App for CrosshairApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if matches!(self.state.active_panel, AppPanel::Zoom | AppPanel::Modes) {
            self.state.active_panel = AppPanel::Pin;
        }
        crate::overlay::set_ui_context(ctx.clone());
        self.apply_theme(ctx);
        let wants_native_shadow = self.state.show_window
            && self.startup_splash.duration_sec <= 0.0
            && self.close_to_tray_animation.is_none()
            && self.open_from_tray_animation.is_none();
        if self.native_shadow_applied != wants_native_shadow {
            crate::platform::set_native_window_shadow(frame, wants_native_shadow);
            self.native_shadow_applied = wants_native_shadow;
        }
        while let Ok(command) = self.ui_rx.try_recv() {
            match command {
                UiCommand::ShowWindow => {
                    if self.state.show_window {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                            egui::UserAttentionType::Informational,
                        ));
                        continue;
                    }
                    self.close_to_tray_animation = None;
                    self.open_from_tray_animation = None;
                    self.state.show_window = true;
                    self.enforce_square_window_frames = 8;
                    let target_size = Self::square_window_size(
                        self.hidden_window_inner_size
                            .take()
                            .unwrap_or_else(Self::desired_window_size),
                    );
                    let target_pos = self
                        .hidden_window_outer_pos
                        .take()
                        .unwrap_or_else(|| Self::centered_outer_position_for_size(target_size));
                    crate::platform::set_native_window_shadow(frame, false);
                    self.native_shadow_applied = false;
                    self.center_window_next_frame = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(target_size));
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(target_pos));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                        egui::UserAttentionType::Informational,
                    ));
                    let _ = self.overlay_tx.send(OverlayCommand::SetUiVisible(true));
                    crate::overlay::wake_command_queue();
                }
                UiCommand::Exit => {
                    self.quit_requested = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                UiCommand::SyncMacroGroups(groups, status) => {
                    self.state.macro_groups = groups;
                    self.persist_macro_presets();
                    self.status = status;
                }
                UiCommand::SyncCrosshairProfiles(profiles, status) => {
                    self.state.profiles = profiles;
                    if self.state.profiles.is_empty() {
                        self.state.profiles.push(ProfileRecord::default());
                    }
                    self.persist();
                    self.status = status;
                }
                UiCommand::SetMacrosMasterEnabled(enabled, status) => {
                    self.state.macros_master_enabled = enabled;
                    self.persist();
                    self.status = status;
                    ctx.request_repaint();
                }
                UiCommand::MousePathRecordingStarted(preset_id, status) => {
                    self.active_mouse_record_preset_id = Some(preset_id);
                    self.status = status;
                }
                UiCommand::MousePathRecordingFinished(preset_id, events, status) => {
                    if let Some(preset) = self
                        .state
                        .mouse_path_presets
                        .iter_mut()
                        .find(|preset| preset.id == preset_id)
                    {
                        preset.events = events;
                    }
                    self.active_mouse_record_preset_id = None;
                    self.persist_mouse_path_presets();
                    self.status = status;
                }
                UiCommand::ImageSearchFinished(status) => {
                    self.status = status;
                }
            }
        }

        if self.state.active_panel != self.last_active_panel {
            if self.last_active_panel == AppPanel::Settings
                && self.state.active_panel != AppPanel::Settings
            {
                self.clear_toolbox_preview();
            }
            if matches!(
                self.state.active_panel,
                AppPanel::WindowPresets | AppPanel::Pin | AppPanel::Macros | AppPanel::ImageSearch
            ) {
                self.refresh_open_windows_now();
            }
            self.last_active_panel = self.state.active_panel;
        }

        if self.state.show_window
            && self.last_window_refresh_at.elapsed() >= Duration::from_millis(250)
            && matches!(
                self.state.active_panel,
                AppPanel::WindowPresets
                    | AppPanel::Pin
                    | AppPanel::Macros
                    | AppPanel::ImageSearch
                    | AppPanel::Mouse
                    | AppPanel::Sound
            )
        {
            self.refresh_open_windows_now();
        }

        if self.close_to_tray_animation.is_some() {
            self.update_close_to_tray_animation(ctx);
        }

        if self.open_from_tray_animation.is_some() {
            self.update_open_from_tray_animation(ctx);
        }

        if !self.state.show_window {
            return;
        }

        if self.center_window_next_frame && self.state.show_window {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Self::desired_window_size()));
            if let Some(center_cmd) = egui::ViewportCommand::center_on_screen(ctx) {
                ctx.send_viewport_cmd(center_cmd);
                self.center_window_next_frame = false;
            }
        }

        if self.enforce_square_window_frames > 0 && self.state.show_window {
            let current_size = ctx
                .input(|input| input.viewport().inner_rect.map(|rect| rect.size()))
                .unwrap_or_else(Self::desired_window_size);
            let squared = Self::square_window_size(current_size);
            if (current_size.x - squared.x).abs() > 1.0 || (current_size.y - squared.y).abs() > 1.0
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(squared));
                ctx.request_repaint();
            }
            self.enforce_square_window_frames = self.enforce_square_window_frames.saturating_sub(1);
        }

        if self.capture_target.is_some() && ctx.input(|input| input.key_pressed(egui::Key::Escape))
        {
            self.cancel_capture();
        } else if self.capture_target.is_some()
            && ctx.input(|input| input.viewport().focused == Some(false))
        {
            self.cancel_capture();
        }
        if self.image_search_capture_active
            && ctx.input(|input| input.viewport().focused == Some(false))
        {
            self.cancel_image_search_capture(ctx);
        }
        if self.mouse_move_absolute_capture_target.is_some() {
            self.poll_mouse_move_absolute_capture(ctx);
        }

        if self.mouse_move_absolute_capture_raise_window {
            self.mouse_move_absolute_capture_raise_window = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                egui::UserAttentionType::Informational,
            ));
            crate::platform::bring_native_window_to_front(frame);
        }

        if ctx.input(|input| input.viewport().close_requested()) && !self.quit_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            crate::platform::set_native_window_shadow(frame, false);
            self.native_shadow_applied = false;
            self.begin_close_to_tray_animation(ctx);
        }

        if let Some(progress) = self.startup_splash_progress(ctx) {
            self.render_startup_splash(ctx, progress);
            return;
        }

        if let Some(animation) = &self.close_to_tray_animation {
            let elapsed = (ctx.input(|input| input.time) - animation.started_at).max(0.0);
            let progress = (elapsed / animation.duration_sec).clamp(0.0, 1.0) as f32;
            self.render_tray_blob_transition(ctx, progress, false);
            return;
        }

        if let Some(animation) = &self.open_from_tray_animation {
            let elapsed = (ctx.input(|input| input.time) - animation.started_at).max(0.0);
            let progress = (elapsed / animation.duration_sec).clamp(0.0, 1.0) as f32;
            self.render_tray_blob_transition(ctx, progress, true);
            return;
        }

        if self.render_image_search_capture_overlay(ctx) {
            return;
        }
        if self.render_mouse_move_absolute_capture_overlay(ctx) {
            return;
        }

        egui::TopBottomPanel::top("top")
            .frame(
                Frame::new()
                    .fill(if self.state.ui_theme == UiThemeMode::Dark {
                        Color32::from_rgb(16, 20, 26)
                    } else {
                        Color32::from_rgb(246, 248, 251)
                    })
                    .stroke(egui::Stroke::new(
                        1.0,
                        if self.state.ui_theme == UiThemeMode::Dark {
                            Color32::from_rgb(34, 42, 56)
                        } else {
                            Color32::from_rgb(210, 219, 230)
                        },
                    ))
                    .inner_margin(egui::Margin::same(10)),
            )
            .show(ctx, |ui| {
                let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
                let show_icon_tooltips = true;
                let hide_window_controls = self.close_to_tray_animation.is_some();
                ui.allocate_ui_with_layout(
                    vec2(ui.available_width(), 42.0),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        let button_fill = if self.state.ui_theme == UiThemeMode::Dark {
                            Color32::from_rgba_premultiplied(54, 67, 88, 78)
                        } else {
                            Color32::from_rgba_premultiplied(214, 223, 235, 110)
                        };

                        if !hide_window_controls {
                            let hide_response = Self::hover_if(
                                ui.add_sized(
                                    [38.0, 30.0],
                                    self.titlebar_button(
                                        Self::material_icon_text(0xe5cd, 18.0),
                                        false,
                                        true,
                                    ),
                                ),
                                show_icon_tooltips,
                                self.titlebar_hide_tooltip(),
                            );
                            if hide_response.clicked() {
                                self.begin_close_to_tray_animation(ctx);
                            }
                            let maximize_response = Self::hover_if(
                                ui.add_sized(
                                    [38.0, 30.0],
                                    self.titlebar_button(
                                        if maximized {
                                            Self::material_icon_text(0xe5cf, 18.0)
                                        } else {
                                            Self::material_icon_text(0xe5d0, 18.0)
                                        },
                                        maximized,
                                        false,
                                    ),
                                ),
                                show_icon_tooltips,
                                self.titlebar_maximize_tooltip(maximized),
                            );
                            if maximize_response.clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                            }
                            let minimize_response = Self::hover_if(
                                ui.add_sized(
                                    [38.0, 30.0],
                                    self.titlebar_button(
                                        Self::material_icon_text(0xe15b, 18.0),
                                        false,
                                        false,
                                    ),
                                ),
                                show_icon_tooltips,
                                self.titlebar_minimize_tooltip(),
                            );
                            if minimize_response.clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                            }
                            let theme_response = Self::hover_if(
                                ui.add_sized(
                                    [38.0, 30.0],
                                    self.titlebar_button(self.theme_button_text(), true, false),
                                ),
                                show_icon_tooltips,
                                self.titlebar_theme_tooltip(),
                            );
                            if theme_response.clicked() {
                                self.toggle_theme_mode();
                            }
                            let language_response = Self::hover_if(
                                ui.add_sized(
                                    [38.0, 30.0],
                                    self.titlebar_button(self.language_button_text(), false, false),
                                ),
                                show_icon_tooltips,
                                self.titlebar_language_tooltip(),
                            );
                            if language_response.clicked() {
                                self.cycle_language();
                            }
                        }

                        ui.add_space(8.0);

                        let drag_width = ui.available_width().max(120.0);
                        let drag_response = ui
                            .allocate_ui_with_layout(
                                vec2(drag_width, 42.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    let accent = if self.state.ui_theme == UiThemeMode::Dark {
                                        Color32::from_rgb(126, 214, 178)
                                    } else {
                                        Color32::from_rgb(34, 122, 88)
                                    };
                                    egui::Frame::new()
                                        .fill(button_fill)
                                        .stroke(egui::Stroke::new(1.0, accent.gamma_multiply(0.45)))
                                        .corner_radius(10.0)
                                        .inner_margin(egui::Margin::symmetric(12, 8))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(self.app_brand_title())
                                                        .strong()
                                                        .size(20.0),
                                                );
                                                ui.add_space(6.0);
                                                ui.label(
                                                    RichText::new(format!(
                                                        "v{}",
                                                        self.app_version_label()
                                                    ))
                                                    .size(11.0)
                                                    .color(
                                                        if self.state.ui_theme
                                                            == UiThemeMode::Dark
                                                        {
                                                            Color32::from_rgb(175, 194, 221)
                                                        } else {
                                                            Color32::from_rgb(80, 96, 128)
                                                        },
                                                    ),
                                                );
                                            });
                                        });
                                    ui.interact(
                                        ui.max_rect(),
                                        ui.id().with("titlebar-drag"),
                                        Sense::click_and_drag(),
                                    )
                                },
                            )
                            .inner;

                        if drag_response.double_clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        } else if drag_response.drag_started() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                    },
                );

                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    let panels = [
                        AppPanel::Macros,
                        AppPanel::Crosshair,
                        AppPanel::WindowPresets,
                        AppPanel::Pin,
                        AppPanel::Mouse,
                        AppPanel::ImageSearch,
                        AppPanel::Sound,
                    ];
                    for panel in panels {
                        let selected = self.state.active_panel == panel;
                        let emphasized = panel == AppPanel::Macros;
                        let text = RichText::new(self.panel_label(panel));
                        let response = Self::hover_if(
                            ui.add(self.top_tab_button(text, selected, emphasized)),
                            show_icon_tooltips,
                            self.panel_label(panel),
                        );
                        if response.clicked() {
                            self.state.active_panel = panel;
                        }
                    }
                    if self.active_audio_editor.is_some() {
                        let text = RichText::new(self.panel_label(AppPanel::Media));
                        let response = Self::hover_if(
                            ui.add(self.top_tab_button(
                                text,
                                self.state.active_panel == AppPanel::Media,
                                false,
                            )),
                            show_icon_tooltips,
                            self.panel_label(AppPanel::Media),
                        );
                        if response.clicked() {
                            self.state.active_panel = AppPanel::Media;
                        }
                    }
                    let text = RichText::new(self.panel_label(AppPanel::Settings));
                    let response = Self::hover_if(
                        ui.add(self.top_tab_button(
                            text,
                            self.state.active_panel == AppPanel::Settings,
                            false,
                        )),
                        show_icon_tooltips,
                        self.panel_label(AppPanel::Settings),
                    );
                    if response.clicked() {
                        self.state.active_panel = AppPanel::Settings;
                    }
                });
            });

        if !self.image_search_capture_active {
            self.render_custom_window_resize_handles(ctx);
            self.render_custom_window_border(ctx);
        }

        if self.state.active_panel != AppPanel::Pin
            || ctx.input(|input| input.viewport().focused == Some(false))
        {
            self.clear_pin_preview_cache();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    match self.state.active_panel {
                        AppPanel::Crosshair => self.render_crosshair_panel(ui),
                        AppPanel::WindowPresets => self.render_window_presets_panel(ui),
                        AppPanel::Pin => self.render_pin_panel(ui),
                        AppPanel::Mouse => self.render_mouse_panel(ui),
                        AppPanel::ImageSearch => self.render_image_search_panel(ui, ctx),
                        AppPanel::Zoom => self.render_pin_panel(ui),
                        AppPanel::Modes => self.render_macro_panel(ui),
                        AppPanel::Macros => self.render_macro_panel(ui),
                        AppPanel::Sound => self.render_sound_panel(ui),
                        AppPanel::Media => self.render_media_panel(ui),
                        AppPanel::Settings => self.render_settings_panel(ui),
                    }
                    ui.separator();
                    if self.capture_target.is_some() {
                        ui.label(self.capture_hint_text());
                    }
                    ui.label(RichText::new(&self.status).strong());
                });
        });

        self.poll_capture_input(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.sync_window_presets();
        self.sync_macro_presets();
        self.sync_macro_master_enabled();
        self.sync_audio_settings();
        self.sync_toolbox_presets();
        let _ = self.overlay_tx.send(OverlayCommand::Exit);
        self.persist();
    }
}

fn audio_duration(clip: &AudioClipSettings) -> Option<u64> {
    if clip.file_path.trim().is_empty() {
        None
    } else {
        audio::load_duration_ms(&clip.file_path).ok()
    }
}
