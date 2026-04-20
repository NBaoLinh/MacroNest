use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RgbaColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RgbaColor {
    pub const WHITE: Self = Self {
        r: 0,
        g: 255,
        b: 170,
        a: 255,
    };

    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };

    pub fn with_alpha(self, alpha: f32) -> Self {
        let mut next = self;
        next.a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
        next
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrosshairStyle {
    pub enabled: bool,
    pub x_offset: i32,
    pub y_offset: i32,
    pub arm_length: f32,
    pub thickness: f32,
    pub gap: f32,
    pub outline_enabled: bool,
    pub outline_thickness: f32,
    pub outline_color: RgbaColor,
    pub center_dot: bool,
    pub center_dot_size: f32,
    pub opacity: f32,
    pub color: RgbaColor,
    pub custom_asset: Option<String>,
    pub custom_scale: f32,
}

impl Default for CrosshairStyle {
    fn default() -> Self {
        Self {
            enabled: true,
            x_offset: 0,
            y_offset: 0,
            arm_length: 10.0,
            thickness: 3.0,
            gap: 5.0,
            outline_enabled: true,
            outline_thickness: 2.0,
            outline_color: RgbaColor::BLACK,
            center_dot: false,
            center_dot_size: 4.0,
            opacity: 0.95,
            color: RgbaColor::WHITE,
            custom_asset: None,
            custom_scale: 96.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileRecord {
    pub name: String,
    pub style: CrosshairStyle,
    pub target_window_title: Option<String>,
    pub extra_target_window_titles: Vec<String>,
}

impl Default for ProfileRecord {
    fn default() -> Self {
        Self {
            name: "Default".to_owned(),
            style: CrosshairStyle::default(),
            target_window_title: None,
            extra_target_window_titles: Vec::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AppPanel {
    #[default]
    Crosshair,
    WindowPresets,
    Pin,
    Mouse,
    ImageSearch,
    Zoom,
    Modes,
    Macros,
    #[serde(alias = "Bindings")]
    Sound,
    Media,
    #[serde(alias = "Toolbox")]
    Settings,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum UiLanguage {
    #[default]
    English,
    Icon,
    Vietnamese,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum UiThemeMode {
    Dark,
    #[default]
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct HotkeyBinding {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct WindowPreset {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub collapsed: bool,
    pub width: i32,
    pub height: i32,
    pub anchor: WindowAnchor,
    pub x: i32,
    pub y: i32,
    pub hotkey: Option<HotkeyBinding>,
    pub remove_title_bar: bool,
    pub animate_enabled: bool,
    pub animate_duration_ms: u64,
    pub animate_hotkey: Option<HotkeyBinding>,
    pub restore_titlebar_enabled: bool,
    pub titlebar_hotkey: Option<HotkeyBinding>,
    pub target_window_title: Option<String>,
    pub extra_target_window_titles: Vec<String>,
}

impl WindowPreset {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Preset {id}"),
            enabled: true,
            collapsed: true,
            width: 1920,
            height: 1080,
            anchor: WindowAnchor::Manual,
            x: 0,
            y: 0,
            hotkey: None,
            remove_title_bar: true,
            animate_enabled: false,
            animate_duration_ms: 260,
            animate_hotkey: None,
            restore_titlebar_enabled: false,
            titlebar_hotkey: None,
            target_window_title: None,
            extra_target_window_titles: Vec::new(),
        }
    }
}

impl Default for WindowPreset {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct WindowFocusPreset {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub collapsed: bool,
    pub target_window_title: Option<String>,
    pub extra_target_window_titles: Vec<String>,
    #[serde(default = "default_true")]
    pub match_duplicate_window_titles: bool,
    pub hotkey: Option<HotkeyBinding>,
}

impl WindowFocusPreset {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Focus {id}"),
            enabled: true,
            collapsed: true,
            target_window_title: None,
            extra_target_window_titles: Vec::new(),
            match_duplicate_window_titles: true,
            hotkey: None,
        }
    }
}

impl Default for WindowFocusPreset {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum WindowAnchor {
    #[default]
    Manual,
    Center,
    TopLeft,
    Top,
    TopRight,
    Left,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum MacroAction {
    #[default]
    KeyPress,
    KeyDown,
    KeyUp,
    TypeText,
    ApplyWindowPreset,
    FocusWindowPreset,
    TriggerMacroPreset,
    EnableCrosshairProfile,
    DisableCrosshair,
    EnablePinPreset,
    DisablePin,
    PlayMousePathPreset,
    ApplyMouseSensitivityPreset,
    EnableZoomPreset,
    DisableZoom,
    PlaySoundPreset,
    LoopStart,
    LoopEnd,
    StopIfTriggerPressedAgain,
    StopIfKeyPressed,
    ShowToolbox,
    HideToolbox,
    LockKeys,
    UnlockKeys,
    LockMouse,
    UnlockMouse,
    EnableMacroPreset,
    DisableMacroPreset,
    MouseLeftClick,
    MouseLeftDown,
    MouseLeftUp,
    MouseRightClick,
    MouseRightDown,
    MouseRightUp,
    MouseMiddleClick,
    MouseMiddleDown,
    MouseMiddleUp,
    MouseX1Click,
    MouseX1Down,
    MouseX1Up,
    MouseX2Click,
    MouseX2Down,
    MouseX2Up,
    MouseWheelUp,
    MouseWheelDown,
    MouseMoveAbsolute,
    MouseMoveRelative,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct MacroStep {
    pub key: String,
    pub action: MacroAction,
    pub delay_ms: u64,
    pub x: i32,
    pub y: i32,
    pub text_override: String,
    pub timed_override: bool,
    pub duration_override_ms: u64,
    pub smooth_mouse_path: bool,
    pub mouse_speed_percent: u32,
}

impl Default for MacroStep {
    fn default() -> Self {
        Self {
            key: String::new(),
            action: MacroAction::KeyPress,
            delay_ms: 0,
            x: 0,
            y: 0,
            text_override: String::new(),
            timed_override: false,
            duration_override_ms: 1500,
            smooth_mouse_path: false,
            mouse_speed_percent: 100,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum MacroTriggerMode {
    #[default]
    Press,
    Hold,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CaptureRequest {
    WindowPresetHotkey(u32),
    WindowFocusPresetHotkey(u32),
    WindowPresetAnimateHotkey(u32),
    WindowPresetTitlebarHotkey(u32),
    WindowExpandHotkey(WindowExpandDirection),
    PinPresetHotkey(u32),
    MousePathRecordHotkey(u32),
    MouseSensitivityPresetHotkey(u32),
    ZoomPresetHotkey(u32),
    ImageSearchTriggerHotkey,
    MacroPresetHotkey(u32, u32),
    MacroSelectorHotkey(u32, u32),
    MacroSelectorOptionKey(u32, u32, u32),
    MacroPresetHoldStopInput(u32, u32),
    MacroStepInput {
        group_id: u32,
        preset_id: u32,
        step_index: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CapturedInput {
    Binding(HotkeyBinding),
    Step(MacroStep),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WindowExpandDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct WindowExpandControls {
    pub enabled: bool,
    pub amount_px: i32,
    pub up: Option<HotkeyBinding>,
    pub down: Option<HotkeyBinding>,
    pub left: Option<HotkeyBinding>,
    pub right: Option<HotkeyBinding>,
}

impl Default for WindowExpandControls {
    fn default() -> Self {
        Self {
            enabled: false,
            amount_px: 48,
            up: Some(HotkeyBinding {
                ctrl: false,
                alt: false,
                shift: false,
                win: false,
                key: "Up".to_owned(),
            }),
            down: Some(HotkeyBinding {
                ctrl: false,
                alt: false,
                shift: false,
                win: false,
                key: "Down".to_owned(),
            }),
            left: Some(HotkeyBinding {
                ctrl: false,
                alt: false,
                shift: false,
                win: false,
                key: "Left".to_owned(),
            }),
            right: Some(HotkeyBinding {
                ctrl: false,
                alt: false,
                shift: false,
                win: false,
                key: "Right".to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ZoomPreset {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub collapsed: bool,
    pub preview_enabled: bool,
    pub source_x: i32,
    pub source_y: i32,
    pub source_width: i32,
    pub source_height: i32,
    pub target_x: i32,
    pub target_y: i32,
    pub target_width: i32,
    pub target_height: i32,
    pub fps: u32,
    pub target_window_title: Option<String>,
    pub extra_target_window_titles: Vec<String>,
    pub hotkey: Option<HotkeyBinding>,
}

impl ZoomPreset {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Zoom {id}"),
            enabled: true,
            collapsed: true,
            preview_enabled: false,
            source_x: 0,
            source_y: 0,
            source_width: 320,
            source_height: 180,
            target_x: 100,
            target_y: 100,
            target_width: 640,
            target_height: 360,
            fps: 30,
            target_window_title: None,
            extra_target_window_titles: Vec::new(),
            hotkey: None,
        }
    }
}

impl Default for ZoomPreset {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct PinPreset {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub collapsed: bool,
    pub preview_enabled: bool,
    pub target_window_title: Option<String>,
    pub extra_target_window_titles: Vec<String>,
    #[serde(default = "default_true")]
    pub match_duplicate_window_titles: bool,
    pub hotkey: Option<HotkeyBinding>,
    pub use_custom_bounds: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub use_source_crop: bool,
    pub source_crop_initialized: bool,
    pub source_crop_fit_version: u8,
    pub source_x: i32,
    pub source_y: i32,
    pub source_width: i32,
    pub source_height: i32,
}

impl PinPreset {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Pin {id}"),
            enabled: true,
            collapsed: true,
            preview_enabled: false,
            target_window_title: None,
            extra_target_window_titles: Vec::new(),
            match_duplicate_window_titles: true,
            hotkey: None,
            use_custom_bounds: false,
            x: 100,
            y: 100,
            width: 640,
            height: 360,
            use_source_crop: false,
            source_crop_initialized: false,
            source_crop_fit_version: 0,
            source_x: 0,
            source_y: 0,
            source_width: 320,
            source_height: 180,
        }
    }
}

impl Default for PinPreset {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum MousePathEventKind {
    #[default]
    Move,
    LeftDown,
    LeftUp,
    RightDown,
    RightUp,
    MiddleDown,
    MiddleUp,
    WheelUp,
    WheelDown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct MousePathEvent {
    pub kind: MousePathEventKind,
    pub x: i32,
    pub y: i32,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct MousePathPreset {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub collapsed: bool,
    pub record_hotkey: Option<HotkeyBinding>,
    pub events: Vec<MousePathEvent>,
}

impl MousePathPreset {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Mouse Path {id}"),
            enabled: true,
            collapsed: true,
            record_hotkey: None,
            events: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct MouseSensitivityPreset {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub collapsed: bool,
    pub target_window_title: Option<String>,
    pub extra_target_window_titles: Vec<String>,
    #[serde(default = "default_true")]
    pub match_duplicate_window_titles: bool,
    pub speed: u32,
    #[serde(default)]
    pub restore_on_exit: bool,
    #[serde(default = "default_mouse_sensitivity_restore_speed")]
    pub restore_speed: u32,
    pub hotkey: Option<HotkeyBinding>,
}

fn default_mouse_sensitivity_restore_speed() -> u32 {
    6
}

impl MouseSensitivityPreset {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Mouse Sensitivity {id}"),
            enabled: true,
            collapsed: true,
            target_window_title: None,
            extra_target_window_titles: Vec::new(),
            match_duplicate_window_titles: true,
            speed: 15,
            restore_on_exit: false,
            restore_speed: default_mouse_sensitivity_restore_speed(),
            hotkey: None,
        }
    }
}

impl Default for MouseSensitivityPreset {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ToolboxPreset {
    pub id: u32,
    pub name: String,
    pub collapsed: bool,
    pub preview_enabled: bool,
    pub text: String,
    pub font_size: f32,
    pub background_opacity: f32,
    pub rounded_background: bool,
    pub text_color: RgbaColor,
    pub background_color: RgbaColor,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl ToolboxPreset {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Toolbox {id}"),
            collapsed: true,
            preview_enabled: false,
            text: "Toolbox text".to_owned(),
            font_size: 28.0,
            background_opacity: 0.72,
            rounded_background: true,
            text_color: RgbaColor {
                r: 244,
                g: 244,
                b: 244,
                a: 255,
            },
            background_color: RgbaColor {
                r: 34,
                g: 34,
                b: 34,
                a: 255,
            },
            x: 660,
            y: 36,
            width: 600,
            height: 80,
        }
    }
}

impl Default for ToolboxPreset {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct MasterWindowPresetState {
    pub id: u32,
    pub enabled: bool,
    pub animate_enabled: bool,
    pub restore_titlebar_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct MasterWindowFocusPresetState {
    pub id: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct MasterZoomPresetState {
    pub id: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct MasterMacroPresetState {
    pub id: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct MasterMacroGroupState {
    pub id: u32,
    pub enabled: bool,
    pub presets: Vec<MasterMacroPresetState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct MasterPreset {
    pub id: u32,
    pub name: String,
    pub collapsed: bool,
    pub macros_master_enabled: bool,
    pub window_expand_controls_enabled: bool,
    pub window_presets: Vec<MasterWindowPresetState>,
    pub window_focus_presets: Vec<MasterWindowFocusPresetState>,
    pub zoom_presets: Vec<MasterZoomPresetState>,
    pub macro_groups: Vec<MasterMacroGroupState>,
}

impl MasterPreset {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Mode {id}"),
            collapsed: true,
            macros_master_enabled: true,
            window_expand_controls_enabled: false,
            window_presets: Vec::new(),
            window_focus_presets: Vec::new(),
            zoom_presets: Vec::new(),
            macro_groups: Vec::new(),
        }
    }
}

impl Default for MasterPreset {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct MacroPreset {
    pub id: u32,
    pub enabled: bool,
    pub collapsed: bool,
    pub trigger_mode: MacroTriggerMode,
    pub stop_on_retrigger_immediate: bool,
    pub hotkey: Option<HotkeyBinding>,
    pub hold_stop_step_enabled: bool,
    pub hold_stop_step: MacroStep,
    pub steps: Vec<MacroStep>,
}

impl MacroPreset {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            enabled: true,
            collapsed: true,
            trigger_mode: MacroTriggerMode::Press,
            stop_on_retrigger_immediate: false,
            hotkey: None,
            hold_stop_step_enabled: false,
            hold_stop_step: MacroStep::default(),
            steps: vec![MacroStep::default()],
        }
    }
}

impl Default for MacroPreset {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct MacroSelectorOption {
    pub id: u32,
    pub choice_key: String,
    pub enable_preset_ids: Vec<u32>,
    pub disable_preset_ids: Vec<u32>,
    #[serde(default, alias = "target_preset_id", skip_serializing)]
    pub legacy_target_preset_id: Option<u32>,
    pub toolbox_text: String,
}

impl MacroSelectorOption {
    pub fn new(id: u32, key: &str) -> Self {
        Self {
            id,
            choice_key: key.to_owned(),
            enable_preset_ids: Vec::new(),
            disable_preset_ids: Vec::new(),
            legacy_target_preset_id: None,
            toolbox_text: String::new(),
        }
    }
}

impl Default for MacroSelectorOption {
    fn default() -> Self {
        Self::new(1, "1")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct MacroSelectorPreset {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub collapsed: bool,
    pub hotkey: Option<HotkeyBinding>,
    pub prompt_text: String,
    pub active_option_id: Option<u32>,
    pub options: Vec<MacroSelectorOption>,
}

impl MacroSelectorPreset {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Selector {id}"),
            enabled: true,
            collapsed: true,
            hotkey: None,
            prompt_text: "Choose an option".to_owned(),
            active_option_id: None,
            options: Vec::new(),
        }
    }
}

impl Default for MacroSelectorPreset {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct MacroFolder {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub collapsed: bool,
}

impl MacroFolder {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Folder {id}"),
            enabled: true,
            collapsed: false,
        }
    }
}

impl Default for MacroFolder {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct MacroGroup {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub collapsed: bool,
    pub folder_id: Option<u32>,
    pub target_window_title: Option<String>,
    pub extra_target_window_titles: Vec<String>,
    #[serde(default = "default_true")]
    pub match_duplicate_window_titles: bool,
    pub selector_presets: Vec<MacroSelectorPreset>,
    pub presets: Vec<MacroPreset>,
}

impl MacroGroup {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Macro Group {id}"),
            enabled: true,
            collapsed: false,
            folder_id: None,
            target_window_title: None,
            extra_target_window_titles: Vec::new(),
            match_duplicate_window_titles: true,
            selector_presets: Vec::new(),
            presets: vec![MacroPreset::new(1)],
        }
    }
}

impl Default for MacroGroup {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AiSettings {
    pub api_key: String,
    pub show_api_key: bool,
    pub system_instruction: String,
    pub prompt: String,
    pub model: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ImageSearchSettings {
    pub enabled: bool,
    pub trigger_hotkey: Option<HotkeyBinding>,
    pub click_after_move: bool,
}

impl Default for ImageSearchSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            trigger_hotkey: None,
            click_after_move: false,
        }
    }
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            show_api_key: false,
            system_instruction: "Convert the user's request into a JSON array of macro steps. Each step must have: key, action, delay_ms. Action must be KeyDown or KeyUp. Return JSON only.".to_owned(),
            prompt: String::new(),
            model: "gemini-2.5-flash-lite".to_owned(),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AudioClipSettings {
    pub enabled: bool,
    pub file_path: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub volume: f32,
    pub speed: f32,
}

impl Default for AudioClipSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            file_path: String::new(),
            start_ms: 0,
            end_ms: 0,
            volume: 1.0,
            speed: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AudioSettings {
    pub startup: AudioClipSettings,
    pub exit: AudioClipSettings,
    pub library: Vec<SoundLibraryItem>,
    pub next_library_item_id: u32,
    pub presets: Vec<SoundPreset>,
    pub next_preset_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SoundLibraryItem {
    pub id: u32,
    pub name: String,
    pub collapsed: bool,
    pub clip: AudioClipSettings,
}

impl SoundLibraryItem {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Library Sound {id}"),
            collapsed: true,
            clip: AudioClipSettings::default(),
        }
    }
}

impl Default for SoundLibraryItem {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SoundPreset {
    pub id: u32,
    pub name: String,
    pub collapsed: bool,
    pub clip: AudioClipSettings,
    pub sequence_library_ids: Vec<u32>,
}

impl SoundPreset {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Sound {id}"),
            collapsed: true,
            clip: AudioClipSettings::default(),
            sequence_library_ids: Vec::new(),
        }
    }
}

impl Default for SoundPreset {
    fn default() -> Self {
        Self::new(1)
    }
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            startup: AudioClipSettings::default(),
            exit: AudioClipSettings::default(),
            library: Vec::new(),
            next_library_item_id: 1,
            presets: Vec::new(),
            next_preset_id: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    pub active_style: CrosshairStyle,
    pub profiles: Vec<ProfileRecord>,
    pub selected_profile: Option<String>,
    pub show_window: bool,
    pub active_panel: AppPanel,
    pub ui_language: UiLanguage,
    pub ui_theme: UiThemeMode,
    pub window_presets: Vec<WindowPreset>,
    pub next_preset_id: u32,
    pub window_expand_controls: WindowExpandControls,
    pub window_focus_presets: Vec<WindowFocusPreset>,
    pub next_window_focus_preset_id: u32,
    pub pin_presets: Vec<PinPreset>,
    pub next_pin_preset_id: u32,
    pub mouse_path_presets: Vec<MousePathPreset>,
    pub next_mouse_path_preset_id: u32,
    pub mouse_sensitivity_presets: Vec<MouseSensitivityPreset>,
    pub next_mouse_sensitivity_preset_id: u32,
    pub mouse_use_interception_driver: bool,
    pub keyboard_arrow_mouse_enabled: bool,
    pub keyboard_arrow_mouse_step_px: u32,
    pub mouse_sensitivity_restore_on_exit: bool,
    pub mouse_sensitivity_restore_speed: u32,
    pub zoom_presets: Vec<ZoomPreset>,
    pub next_zoom_preset_id: u32,
    pub toolbox_presets: Vec<ToolboxPreset>,
    pub next_toolbox_preset_id: u32,
    pub master_presets: Vec<MasterPreset>,
    pub selected_master_preset_id: Option<u32>,
    pub next_master_preset_id: u32,
    pub macro_folders: Vec<MacroFolder>,
    pub next_macro_folder_id: u32,
    pub macro_groups: Vec<MacroGroup>,
    pub next_macro_group_id: u32,
    pub macro_presets: Vec<MacroPreset>,
    pub next_macro_preset_id: u32,
    pub next_macro_selector_preset_id: u32,
    pub next_macro_selector_option_id: u32,
    pub macros_master_enabled: bool,
    pub ai_settings: AiSettings,
    pub audio_settings: AudioSettings,
    pub image_search_settings: ImageSearchSettings,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_style: CrosshairStyle::default(),
            profiles: vec![ProfileRecord {
                name: "Default".to_owned(),
                style: CrosshairStyle::default(),
                target_window_title: None,
                extra_target_window_titles: Vec::new(),
            }],
            selected_profile: Some("Default".to_owned()),
            show_window: true,
            active_panel: AppPanel::Crosshair,
            ui_language: UiLanguage::English,
            ui_theme: UiThemeMode::Light,
            window_presets: Vec::new(),
            next_preset_id: 1,
            window_expand_controls: WindowExpandControls::default(),
            window_focus_presets: Vec::new(),
            next_window_focus_preset_id: 1,
            pin_presets: Vec::new(),
            next_pin_preset_id: 1,
            mouse_path_presets: Vec::new(),
            next_mouse_path_preset_id: 1,
            mouse_sensitivity_presets: Vec::new(),
            next_mouse_sensitivity_preset_id: 1,
            mouse_use_interception_driver: false,
            keyboard_arrow_mouse_enabled: false,
            keyboard_arrow_mouse_step_px: 4,
            mouse_sensitivity_restore_on_exit: false,
            mouse_sensitivity_restore_speed: 6,
            zoom_presets: Vec::new(),
            next_zoom_preset_id: 1,
            toolbox_presets: vec![ToolboxPreset::new(1)],
            next_toolbox_preset_id: 2,
            master_presets: Vec::new(),
            selected_master_preset_id: None,
            next_master_preset_id: 1,
            macro_folders: Vec::new(),
            next_macro_folder_id: 1,
            macro_groups: Vec::new(),
            next_macro_group_id: 1,
            macro_presets: Vec::new(),
            next_macro_preset_id: 1,
            next_macro_selector_preset_id: 1,
            next_macro_selector_option_id: 1,
            macros_master_enabled: true,
            ai_settings: AiSettings::default(),
            audio_settings: AudioSettings::default(),
            image_search_settings: ImageSearchSettings::default(),
        }
    }
}
