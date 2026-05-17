#![allow(unsafe_op_in_unsafe_fn)]

#[derive(Debug, Clone)]
pub struct MacroRecordingEvent {
    pub key: Option<String>,
    pub action: crate::model::MacroAction,
    pub delay_ms: u64,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone)]
pub struct MacroRecordingSession {
    pub group_id: u32,
    pub preset_id: u32,
    pub last_event_at: std::time::Instant,
    pub events: Vec<MacroRecordingEvent>,
}

#[cfg(windows)]
mod windows_overlay {
    use super::{MacroRecordingEvent, MacroRecordingSession};
    fn log_to_file(msg: &str) {
        static START: once_cell::sync::Lazy<std::time::Instant> = once_cell::sync::Lazy::new(std::time::Instant::now);
        let elapsed = START.elapsed().as_millis();
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(r"C:\Users\ngbal\.gemini\antigravity\macronest_hook_log.txt")
        {
            use std::io::Write;
            let _ = writeln!(file, "[{:08}ms] {}", elapsed, msg);
        }
    }

    use std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        ffi::c_void,
        mem::size_of,
        os::windows::process::CommandExt,
        path::{Path, PathBuf},
        process::Command,
        ptr::null_mut,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicIsize, Ordering},
        },
        thread,
        time::{Duration, Instant},
    };

    use anyhow::{Context, Result, bail};
    use crossbeam_channel::{Receiver, Sender};
    use eframe::egui;
    use libloading::Library;
    use once_cell::sync::Lazy;
    use opencv::{
        core::{self as cv, Mat, Size},
        imgproc,
        prelude::*,
    };
    use parking_lot::Mutex;
    use windows::{
        Win32::{
            Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
            Graphics::{
                Dwm::{
                    DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION,
                    DWM_TNP_RECTSOURCE, DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE,
                    DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute, DwmRegisterThumbnail,
                    DwmUnregisterThumbnail, DwmUpdateThumbnailProperties,
                },
                Gdi::{
                    AC_SRC_ALPHA, AC_SRC_OVER, ANTIALIASED_QUALITY, BI_RGB, BITMAPINFO,
                    BITMAPINFOHEADER, BLENDFUNCTION, BeginPaint, CLIP_DEFAULT_PRECIS,
                    CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateRectRgn,
                    DEFAULT_CHARSET, DIB_RGB_COLORS, DT_CENTER, DT_SINGLELINE, DT_VCENTER,
                    DeleteDC, DeleteObject, DrawTextW, EndPaint, FF_DONTCARE, FW_MEDIUM, GetDC,
                    GetMonitorInfoW, HGDIOBJ, MONITOR_DEFAULTTONEAREST, MONITORINFO,
                    MonitorFromWindow, OUT_DEFAULT_PRECIS, PAINTSTRUCT, ReleaseDC, SelectObject,
                    SetBkMode, SetTextColor, SetWindowRgn, TRANSPARENT,
                },
            },
            System::{
                LibraryLoader::GetModuleHandleW,
                Threading::{
                    AttachThreadInput, CREATE_NO_WINDOW, GetCurrentProcessId, GetCurrentThreadId,
                },
            },
            UI::{
                Input::KeyboardAndMouse::{
                    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
                    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, KEYEVENTF_UNICODE,
                    MAPVK_VK_TO_VSC, MOD_ALT, MOD_CONTROL, MOUSE_EVENT_FLAGS, MOUSEEVENTF_ABSOLUTE,
                    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
                    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
                    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP,
                    MOUSEINPUT, MapVirtualKeyW, RegisterHotKey, SendInput, SetActiveWindow,
                    SetFocus, UnregisterHotKey, VIRTUAL_KEY,
                },
                Shell::{
                    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
                    NOTIFYICONDATAW, Shell_NotifyIconW,
                },
                WindowsAndMessaging::{
                    AppendMenuW, BringWindowToTop, CREATESTRUCTW, CallNextHookEx, CreatePopupMenu,
                    CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyMenu, DispatchMessageW,
                    GA_ROOT, GW_OWNER, GWLP_USERDATA, GetAncestor, GetClassNameW, GetClientRect,
                    GetCursorPos, GetForegroundWindow, GetMessageW, GetSystemMetrics, GetWindow,
                    GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, HC_ACTION, HHOOK,
                    WindowFromPoint,
                    HMENU, HTTRANSPARENT, HWND_NOTOPMOST, HWND_TOPMOST, IDC_ARROW, IMAGE_ICON,
                    IsIconic, IsZoomed, KBDLLHOOKSTRUCT, KillTimer, LR_LOADFROMFILE, LoadCursorW,
                    LoadImageW, MA_NOACTIVATE, MF_SEPARATOR, MF_STRING, MSG, MSLLHOOKSTRUCT,
                    PostMessageW, PostQuitMessage, RegisterClassW, SM_CXSCREEN, SM_CYSCREEN,
                    SPI_GETMOUSESPEED, SPI_SETMOUSESPEED, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE,
                    SW_HIDE, SW_RESTORE, SW_SHOWNA, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOMOVE,
                    SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SetCursorPos, SetForegroundWindow,
                    SetTimer, SetWindowLongPtrW, SetWindowPos, SetWindowsHookExW, ShowWindow,
                    SystemParametersInfoW, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TrackPopupMenu,
                    TranslateMessage, ULW_ALPHA, UnhookWindowsHookEx, UpdateLayeredWindow,
                    WH_KEYBOARD_LL, WH_MOUSE_LL, WINDOW_EX_STYLE, WINDOW_LONG_PTR_INDEX, WM_APP,
                    WM_COMMAND, WM_CREATE, WM_DESTROY, WM_HOTKEY, WM_KEYDOWN, WM_KEYUP,
                    WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
                    WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCREATE, WM_NCHITTEST,
                    WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER,
                    WM_XBUTTONDOWN, WM_XBUTTONUP, WNDCLASSW, WS_CAPTION, WS_EX_LAYERED,
                    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
                    WS_OVERLAPPEDWINDOW, WS_POPUP,
                },
            },
        },
        core::{PCWSTR, w},
    };

    use crate::{
        ai, audio, hotkey,
        model::{
            AudioSettings, CrosshairStyle, CommandPreset, HotkeyBinding, VisionPreset,
            MacroAction, MacroGroup, MacroPreset, MacroStep,
            MacroTriggerMode, MousePathEvent, MousePathEventKind, MousePathPreset,
            MouseSensitivityPreset, PinOverlayStyle, PinPreset, ProfileRecord, RgbaColor,
            SoundLibraryItem, SoundPreset, HudPreset, WindowAnchor, WindowExpandControls,
            WindowExpandDirection, WindowFocusPreset, WindowPreset,
        },
        render::{RenderedCrosshair, render_crosshair},
        storage::AppPaths,
        window_list,
    };
    use image::{RgbaImage, imageops::FilterType};

    #[path = "../window_preset.rs"]
    mod window_preset;

    const HOTKEY_ID: i32 = 1001;
    const TIMER_ID: usize = 1;
    const TRAY_UID: u32 = 7001;
    const XBUTTON1_DATA: u16 = 0x0001;
    const XBUTTON2_DATA: u16 = 0x0002;
    const WMAPP_TRAYICON: u32 = WM_APP + 1;
    const WMAPP_PROCESS_QUEUE: u32 = WM_APP + 2;
    const MACRO_PRESET_BASE_ID: i32 = 10000;

    #[derive(Debug, Clone)]
    struct VisionRunOutcome {
        matched: bool,
        status: String,
    }


    const MENU_SHOW: usize = 2002;
    const MENU_EXIT: usize = 2003;

    static SUPPRESSED_MACRO_HOTKEYS: Lazy<Mutex<HashSet<i32>>> =
        Lazy::new(|| Mutex::new(HashSet::new()));
    static STOP_REQUESTED_MACRO_PRESETS: Lazy<Mutex<HashSet<u32>>> =
        Lazy::new(|| Mutex::new(HashSet::new()));
    static IMAGE_SEARCH_WAIT_GENERATIONS: Lazy<Mutex<HashMap<u32, u64>>> =
        Lazy::new(|| Mutex::new(HashMap::new()));
    static HUD_DISPLAY: Lazy<Mutex<Option<HudDisplayState>>> =
        Lazy::new(|| Mutex::new(None));
    static HUD_PREVIEW_DISPLAY: Lazy<Mutex<Option<HudDisplayState>>> =
        Lazy::new(|| Mutex::new(None));
    static MOUSE_RECORDING: Lazy<Mutex<Option<MouseRecordingSession>>> =
        Lazy::new(|| Mutex::new(None));
    static MACRO_RECORDING: Lazy<Mutex<Option<MacroRecordingSession>>> =
        Lazy::new(|| Mutex::new(None));
    static HOOK_STATE: Lazy<Mutex<HookState>> = Lazy::new(|| Mutex::new(HookState::default()));
    
    static OVERLAY_COMMAND_TX: Lazy<Mutex<Option<Sender<OverlayCommand>>>> =
        Lazy::new(|| Mutex::new(None));
    static UI_CONTEXT: Lazy<Mutex<Option<egui::Context>>> = Lazy::new(|| Mutex::new(None));
    static CONTROLLER_HWND: AtomicIsize = AtomicIsize::new(0);
    #[derive(Debug, Clone)]
    pub enum OverlayCommand {
        Update(CrosshairStyle),
        UpdateProfiles(Vec<ProfileRecord>),
        UpdateCrosshairProfile {
            index: usize,
            profile: ProfileRecord,
        },
        UpdateWindowPresets(Vec<WindowPreset>),
        UpdateWindowFocusPresets(Vec<WindowFocusPreset>),
        #[allow(dead_code)]
        UpdateWindowExpandControls(WindowExpandControls),
        UpdatePinPresets(Vec<PinPreset>),
        UpdateMousePathPresets(Vec<MousePathPreset>),
        UpdateMouseSensitivityPresets(Vec<MouseSensitivityPreset>),
        UpdateMouseSensitivitySettings {
            restore_on_exit: bool,
            restore_speed: u32,
        },

        UpdateKeyboardArrowMouseSettings {
            enabled: bool,
            step_px: u32,
        },
        UpdateVisionPresets(Vec<VisionPreset>),
        InvalidateVisionWaits(Vec<u32>),
        ApplyMouseSensitivityPreset(u32),
        RestoreMouseSensitivity,
        UpdateHudPresets(Vec<HudPreset>),
        UpdateCommandPresets(Vec<CommandPreset>),
        PreviewHudPreset(Option<HudPreset>),
        UpdateMacroPresets(Vec<MacroGroup>),
        UpdateAudioSettings(AudioSettings),
        SetMacrosMasterEnabled(bool),
        SetVietnameseInputEnabled(bool),
        UpdateMacrosMasterHotkey(Option<HotkeyBinding>),
        RefreshPinOverlay,
        SetVisionCaptureMouseBlocked(bool),
        SetUiVisible(bool),
        SetTrayIconVisible(bool),
        Exit,
        ToggleMacroRecording(u32, u32, String),
    }

    #[derive(Debug, Clone)]
    pub enum UiCommand {
        ShowWindow,
        Exit,
        SyncMacroGroups(Vec<MacroGroup>, String),
        SyncCrosshairProfiles(Vec<ProfileRecord>, String),
        SetMacrosMasterEnabled(bool, String),
        SetVietnameseInputEnabled(bool, String),
        MousePathRecordingStarted(u32, String),
        MousePathRecordingFinished(u32, Vec<MousePathEvent>, String),
        VisionFinished(String),
        VisionCaptureMouseDown {
            screen_x: i32,
            screen_y: i32,
        },
        VisionCaptureMouseMove {
            screen_x: i32,
            screen_y: i32,
        },
        VisionCaptureMouseUp {
            screen_x: i32,
            screen_y: i32,
        },
        VisionPointCaptured {
            preset_id: u32,
            priority_anchor: bool,
            screen_x: i32,
            screen_y: i32,
            color: Option<RgbaColor>,
        },
        VisionRegionPreview {
            screen_x: i32,
            screen_y: i32,
            width: i32,
            height: i32,
        },
        VisionRegionCaptured {
            preset_id: u32,
            template_mode: bool,
            screen_x: i32,
            screen_y: i32,
            width: i32,
            height: i32,
        },
        VisionPointCaptureCancelled(String),
        UpdateCheckStarted,
        UpdateAvailable(String, String, String), // version, body, download_url
        MacroRecordingStarted(u32, String),
        MacroRecordingFinished(u32, u32, Vec<MacroRecordingEvent>, String),
        MacroRealtimeStepAdded(u32, u32, crate::model::MacroStep),
        MacroRealtimeStepRemoved(u32, u32),
        UpdateDownloadStarted,
        UpdateDownloadFinished(String), // new_exe_path
        UpdateError(String),
        UpdateUpToDate,
    }

    pub struct OverlayHandle {
        tx: Sender<OverlayCommand>,
    }

    impl OverlayHandle {
        pub fn send(&self, command: OverlayCommand) {
            let _ = self.tx.send(command);
        }
    }

    pub fn wake_command_queue() {
        unsafe {
            let hwnd = HWND(CONTROLLER_HWND.load(Ordering::Relaxed) as *mut c_void);
            if !hwnd.0.is_null() {
                let _ = PostMessageW(Some(hwnd), WMAPP_PROCESS_QUEUE, WPARAM(0), LPARAM(0));
            }
        }
    }

    pub fn set_ui_context(ctx: egui::Context) {
        *UI_CONTEXT.lock() = Some(ctx);
    }

    pub fn request_ui_repaint() {
        if let Some(ctx) = UI_CONTEXT.lock().as_ref() {
            ctx.request_repaint();
        }
    }

    struct HookState {
        ui_tx: Option<Sender<UiCommand>>,
        window_presets: Vec<WindowPreset>,
        window_focus_presets: Vec<WindowFocusPreset>,
        window_expand_controls: WindowExpandControls,
        pin_presets: Vec<PinPreset>,
        mouse_path_presets: Vec<MousePathPreset>,
        mouse_sensitivity_presets: Vec<MouseSensitivityPreset>,
        active_mouse_sensitivity_preset_id: Option<u32>,
        mouse_sensitivity_restore_speed: Option<u32>,

        keyboard_arrow_mouse_enabled: bool,
        keyboard_arrow_mouse_step_px: u32,
        vision_presets: Vec<VisionPreset>,
        vision_following_presets: HashSet<u32>,
        vision_dir: PathBuf,
        opencv_dll_path: PathBuf,

        mouse_sensitivity_restore_on_exit: bool,
        mouse_sensitivity_exit_restore_speed: u32,
        active_pin_preset_id: Option<u32>,
        vision_capture_mouse_blocked: bool,
        vision_capture_anchor: Option<(i32, i32)>,
        vision_capture_preview_region: Option<VisionRegion>,
        hud_presets: Vec<HudPreset>,
        command_presets: Vec<CommandPreset>,
        macro_groups: Vec<MacroGroup>,
        macros_master_enabled: bool,
        macros_master_hotkey: Option<HotkeyBinding>,
        vietnamese_input_enabled: bool,
        locked_inputs: HashMap<String, usize>,
        locked_mouse_count: usize,
        current_style: CrosshairStyle,
        profiles: Vec<ProfileRecord>,
        sound_presets: Vec<SoundPreset>,
        sound_library: Vec<SoundLibraryItem>,
        active_hold_macros: HashMap<u32, ActiveHoldMacro>,
        next_hold_run_token: u64,
        pending_tray_toggle: Option<bool>,
        tray_double_click_suppress_next_up: bool,
        active_crosshair_profile_name: Option<String>,
        stop_ignore_keys: HashMap<u32, String>,
        press_trigger_suppression: HashMap<String, usize>,
        pending_press_trigger_keys: HashSet<String>,
        ctrl: bool,
        alt: bool,
        shift: bool,
        win: bool,
        held_inputs: HashSet<String>,
        pressed_inputs: HashSet<String>,
        held_mouse_buttons: HashSet<String>,
    }

    impl Default for HookState {
        fn default() -> Self {
            Self {
                ui_tx: None,
                window_presets: Vec::new(),
                window_focus_presets: Vec::new(),
                window_expand_controls: WindowExpandControls::default(),
                pin_presets: Vec::new(),
                mouse_path_presets: Vec::new(),
                mouse_sensitivity_presets: Vec::new(),
                active_mouse_sensitivity_preset_id: None,
                mouse_sensitivity_restore_speed: None,

                keyboard_arrow_mouse_enabled: false,
                keyboard_arrow_mouse_step_px: 12,
                vision_presets: Vec::new(),
                vision_following_presets: HashSet::new(),
                vision_dir: PathBuf::new(),
                opencv_dll_path: PathBuf::new(),

                mouse_sensitivity_restore_on_exit: false,
                mouse_sensitivity_exit_restore_speed: 6,
                active_pin_preset_id: None,
                vision_capture_mouse_blocked: false,
                vision_capture_anchor: None,
                vision_capture_preview_region: None,
                hud_presets: Vec::new(),
                command_presets: Vec::new(),
                macro_groups: Vec::new(),
                macros_master_enabled: true,
                macros_master_hotkey: None,
                vietnamese_input_enabled: false,
                locked_inputs: HashMap::new(),
                locked_mouse_count: 0,
                current_style: CrosshairStyle::default(),
                profiles: Vec::new(),
                sound_presets: Vec::new(),
                sound_library: Vec::new(),
                active_hold_macros: HashMap::new(),
                next_hold_run_token: 1,
                pending_tray_toggle: None,
                tray_double_click_suppress_next_up: false,
                active_crosshair_profile_name: None,
                stop_ignore_keys: HashMap::new(),
                press_trigger_suppression: HashMap::new(),
                pending_press_trigger_keys: HashSet::new(),
                ctrl: false,
                alt: false,
                shift: false,
                win: false,
                held_inputs: HashSet::new(),
                pressed_inputs: HashSet::new(),
                held_mouse_buttons: HashSet::new(),
            }
        }
    }


    struct Runtime {
        rx: Receiver<OverlayCommand>,
        ui_tx: Sender<UiCommand>,
        paths: AppPaths,
        style: CrosshairStyle,
        window_presets: Vec<WindowPreset>,
        window_focus_presets: Vec<WindowFocusPreset>,
        pin_presets: Vec<PinPreset>,
        mouse_path_presets: Vec<MousePathPreset>,
        macro_groups: Vec<MacroGroup>,
        audio_settings: AudioSettings,
        registered_window_hotkeys: HashMap<i32, WindowHotkeyAction>,
        registered_macro_hotkeys: HashMap<i32, MacroPreset>,
        overlay_hwnd: HWND,
        mouse_trail_hwnd: HWND,
        search_area_hwnd: HWND,
        hud_hwnd: HWND,
        pin_hwnd: HWND,
        last_pin_update: Instant,
        hud_display: Option<HudDisplayState>,
        tray_menu: HMENU,
        keyboard_hook: HHOOK,
        mouse_hook: HHOOK,
        running: Arc<AtomicBool>,
        active_pin_thumbnail: Option<ActivePinThumbnail>,
        timer_interval_ms: u32,
        ui_visible: bool,
        ui_foreground: bool,
    }

    struct MouseRecordingSession {
        preset_id: u32,
        last_event_at: Instant,
        events: Vec<MousePathEvent>,
        dirty: bool,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum MacroRunFlow {
        Continue,
        BreakLoop,
        StopExecution,
    }

    #[derive(Clone)]
    struct ActiveHoldMacro {
        trigger: HotkeyBinding,
        release_steps: Vec<MacroStep>,
        hold_stop_step: Option<MacroStep>,
        image_search_preset_ids: Vec<u32>,
        locked_keys: Vec<String>,
        locked_mouse_count: usize,
        run_token: u64,
        completed: bool,
    }

    #[derive(Clone, PartialEq)]
    struct HudDisplayState {
        owner_preset_id: Option<u32>,
        preset_id: Option<u32>,
        text: String,
        text_color: RgbaColor,
        background_color: RgbaColor,
        background_opacity: f32,
        rounded_background: bool,
        font_size: f32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        auto_hide_on_owner_completion: bool,
        expires_at: Option<Instant>,
    }

    struct ActivePinThumbnail {
        preset_id: u32,
        source_hwnd: HWND,
        thumbnail_id: Option<isize>,
        overlay_style: PinOverlayStyle,
        last_target_bounds: (i32, i32, i32, i32),
        last_source_crop: Option<(i32, i32, i32, i32)>,
    }

    #[allow(dead_code)]
    enum WindowHotkeyAction {
        Apply(WindowPreset),
        Focus(WindowFocusPreset),
        Animate(WindowPreset),
        RestoreTitleBar(WindowPreset),
    }

    pub fn start(
        paths: AppPaths,
        initial_style: CrosshairStyle,
        ui_tx: Sender<UiCommand>,
    ) -> Result<OverlayHandle> {
        let (tx, rx) = crossbeam_channel::unbounded();
        *OVERLAY_COMMAND_TX.lock() = Some(tx.clone());
        let running = Arc::new(AtomicBool::new(true));
        let worker_running = running.clone();

        thread::spawn(move || {
            let result = run_thread(paths, initial_style, rx, ui_tx, worker_running.clone());
            if let Err(error) = result {
                eprintln!("overlay error: {error:#}");
            }
            worker_running.store(false, Ordering::Relaxed);
        });

        Ok(OverlayHandle { tx })
    }

    fn run_thread(
        paths: AppPaths,
        initial_style: CrosshairStyle,
        rx: Receiver<OverlayCommand>,
        ui_tx: Sender<UiCommand>,
        running: Arc<AtomicBool>,
    ) -> Result<()> {
        {
            let mut hook_state = HOOK_STATE.lock();
            
            hook_state.vision_dir = paths.vision_dir.clone();
            hook_state.opencv_dll_path = paths.opencv_dll.clone();
        }
        unsafe {
            let instance = HINSTANCE(GetModuleHandleW(None)?.0);
            register_class(
                instance,
                w!("CrosshairController"),
                Some(controller_wnd_proc),
            )?;
            register_class(instance, w!("CrosshairOverlay"), Some(overlay_wnd_proc))?;
            register_class(instance, w!("CrosshairToolbox"), Some(hud_wnd_proc))?;
            let overlay_hwnd = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOOLWINDOW
                    | WS_EX_TOPMOST
                    | WS_EX_NOACTIVATE,
                w!("CrosshairOverlay"),
                w!("CrosshairOverlay"),
                WS_POPUP,
                0,
                0,
                32,
                32,
                None,
                None,
                Some(instance),
                None,
            )?;

            let mouse_trail_hwnd = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOOLWINDOW
                    | WS_EX_TOPMOST
                    | WS_EX_NOACTIVATE,
                w!("CrosshairOverlay"),
                w!("CrosshairMouseTrail"),
                WS_POPUP,
                0,
                0,
                32,
                32,
                None,
                None,
                Some(instance),
                None,
            )?;

            let search_area_hwnd = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOOLWINDOW
                    | WS_EX_TOPMOST
                    | WS_EX_NOACTIVATE,
                w!("CrosshairOverlay"),
                w!("CrosshairSearchArea"),
                WS_POPUP,
                0,
                0,
                32,
                32,
                None,
                None,
                Some(instance),
                None,
            )?;

            let hud_hwnd = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TOOLWINDOW
                    | WS_EX_TOPMOST
                    | WS_EX_NOACTIVATE
                    | WS_EX_TRANSPARENT,
                w!("CrosshairToolbox"),
                w!("CrosshairToolbox"),
                WS_POPUP,
                0,
                0,
                360,
                44,
                None,
                None,
                Some(instance),
                None,
            )?;

            let pin_hwnd = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TOOLWINDOW
                    | WS_EX_TOPMOST
                    | WS_EX_NOACTIVATE
                    | WS_EX_TRANSPARENT,
                w!("CrosshairOverlay"),
                w!("CrosshairPinHost"),
                WS_POPUP,
                0,
                0,
                320,
                180,
                None,
                None,
                Some(instance),
                None,
            )?;

            let tray_menu = CreatePopupMenu()?;
            let _ = AppendMenuW(tray_menu, MF_STRING, MENU_SHOW, w!("Open settings"));
            let _ = AppendMenuW(tray_menu, MF_SEPARATOR, 0, PCWSTR::null());
            let _ = AppendMenuW(tray_menu, MF_STRING, MENU_EXIT, w!("Exit"));

            {
                let mut hook_state = HOOK_STATE.lock();
                hook_state.ui_tx = Some(ui_tx.clone());
            }

            let runtime = Box::new(Runtime {
                rx,
                ui_tx,
                paths,
                style: initial_style,
                window_presets: Vec::new(),
                window_focus_presets: Vec::new(),
                pin_presets: Vec::new(),
                mouse_path_presets: Vec::new(),
                macro_groups: Vec::new(),
                audio_settings: AudioSettings::default(),
                registered_window_hotkeys: HashMap::new(),
                registered_macro_hotkeys: HashMap::new(),
                overlay_hwnd,
                mouse_trail_hwnd,
                search_area_hwnd,
                hud_hwnd,
                pin_hwnd,
                last_pin_update: Instant::now() - Duration::from_secs(1),
                hud_display: None,
                tray_menu,
                keyboard_hook: HHOOK::default(),
                mouse_hook: HHOOK::default(),
                running,
                active_pin_thumbnail: None,
                timer_interval_ms: 500,
                ui_visible: true,
                ui_foreground: true,
            });

            let _controller_hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("CrosshairController"),
                w!("CrosshairController"),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(instance),
                Some(Box::into_raw(runtime) as *const c_void),
            )?;

            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).into() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }

        Ok(())
    }

    unsafe fn register_class(
        instance: HINSTANCE,
        name: PCWSTR,
        proc: Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT>,
    ) -> Result<()> {
        let cursor = LoadCursorW(None, IDC_ARROW)?;
        let class = WNDCLASSW {
            lpfnWndProc: proc,
            hInstance: instance,
            lpszClassName: name,
            hCursor: cursor,
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            bail!("Failed to register the window class");
        }
        Ok(())
    }

    unsafe extern "system" fn overlay_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_NCHITTEST {
            return LRESULT(HTTRANSPARENT as isize);
        }
        if msg == WM_MOUSEACTIVATE {
            return LRESULT(MA_NOACTIVATE as isize);
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    unsafe extern "system" fn controller_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_NCCREATE => {
                let create = lparam.0 as *const CREATESTRUCTW;
                let runtime = (*create).lpCreateParams as *mut Runtime;
                SetWindowLongPtrW(
                    hwnd,
                    WINDOW_LONG_PTR_INDEX(GWLP_USERDATA.0),
                    runtime as isize,
                );
                LRESULT(1)
            }
            WM_CREATE => {
                CONTROLLER_HWND.store(hwnd.0 as isize, Ordering::Relaxed);
                if let Some(runtime) = runtime_mut(hwnd) {
                    // let _ = add_tray_icon(hwnd); // Removed: Tray icon only appears when hidden
                    let _ =
                        RegisterHotKey(Some(hwnd), HOTKEY_ID, MOD_CONTROL | MOD_ALT, b'X' as u32);
                    let _ = SetTimer(Some(hwnd), TIMER_ID, 500, None);
                    let _ = set_input_hooks_enabled(runtime, false);
                    let _ = refresh_overlay(runtime);
                }
                LRESULT(0)
            }
            WM_TIMER => {
                if let Some(runtime) = runtime_mut(hwnd) {
                    process_pending_commands(hwnd, runtime);
                    let ui_foreground = is_ui_in_foreground();
                    if ui_foreground != runtime.ui_foreground {
                        runtime.ui_foreground = ui_foreground;
                        let _ = set_input_hooks_enabled(runtime, desired_hooks_enabled(runtime));
                        let _ = refresh_overlay(runtime);
                        if ui_foreground {
                            let _ = ShowWindow(runtime.pin_hwnd, SW_HIDE);
                            let _ = ShowWindow(runtime.hud_hwnd, SW_HIDE);
                            let _ = ShowWindow(runtime.mouse_trail_hwnd, SW_HIDE);
                        } else {
                            clear_transient_input_state();
                            let _ = refresh_pin_overlay(runtime);
                            let _ = refresh_hud(runtime);
                            let _ = refresh_mouse_record_trail(runtime);
                        }
                    }
                    if !is_ui_in_foreground() {
                        apply_keyboard_arrow_mouse_movement();

                        let pin_active = runtime.active_pin_thumbnail.is_some()
                            || HOOK_STATE.lock().active_pin_preset_id.is_some();
                        if pin_active {
                            let _ = refresh_pin_overlay(runtime);
                        }

                        let toolbox_active = HUD_DISPLAY.lock().is_some()
                            || HUD_PREVIEW_DISPLAY.lock().is_some()
                            || runtime.hud_display.is_some();
                        if toolbox_active {
                            let _ = refresh_hud(runtime);
                        }

                        let mouse_recording_active = MOUSE_RECORDING.lock().is_some();
                        let mouse_trail_visible =
                            windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(
                                runtime.mouse_trail_hwnd,
                            )
                            .as_bool();
                        if mouse_recording_active || mouse_trail_visible {
                            let _ = refresh_mouse_record_trail(runtime);
                        }
                    }
                    let _ = refresh_search_area_overlay(runtime);

                    refresh_overlay_timer(hwnd, runtime);
                }
                LRESULT(0)
            }
            WMAPP_PROCESS_QUEUE => {
                if let Some(runtime) = runtime_mut(hwnd) {
                    process_pending_commands(hwnd, runtime);
                    let _ = refresh_search_area_overlay(runtime);
                    refresh_overlay_timer(hwnd, runtime);
                }
                LRESULT(0)
            }
            WM_HOTKEY => {
                if let Some(runtime) = runtime_mut(hwnd) {
                    if is_ui_in_foreground() {
                        return LRESULT(0);
                    }

                    let hotkey_id = wparam.0 as i32;
                    if hotkey_id == HOTKEY_ID {
                        runtime.style.enabled = !runtime.style.enabled;
                        let _ = refresh_overlay(runtime);
                    } else if let Some(action) = runtime.registered_window_hotkeys.get(&hotkey_id) {
                        match action {
                            WindowHotkeyAction::Apply(preset) => {
                                let _ = apply_window_preset(preset);
                            }
                            WindowHotkeyAction::Focus(preset) => {
                                let _ = focus_window_for_preset(preset);
                            }
                            WindowHotkeyAction::Animate(preset) => {
                                let preset = preset.clone();
                                thread::spawn(move || {
                                    let _ = apply_window_preset_animated(&preset);
                                });
                            }
                            WindowHotkeyAction::RestoreTitleBar(preset) => {
                                let _ = restore_window_title_bar_for_preset(preset);
                            }
                        }
                    } else if let Some(preset) = runtime.registered_macro_hotkeys.get(&hotkey_id) {
                        if !SUPPRESSED_MACRO_HOTKEYS.lock().contains(&hotkey_id) {
                            let trigger_key = preset
                                .hotkey
                                .as_ref()
                                .map(|binding| binding.key.clone())
                                .unwrap_or_default();
                            let _ = play_macro_preset(
                                hotkey_id,
                                preset.clone(),
                                None,
                                Vec::new(),
                                false,
                                trigger_key,
                            );
                        }
                    }
                }
                LRESULT(0)
            }
            WM_COMMAND => {
                if let Some(runtime) = runtime_mut(hwnd) {
                    match wparam.0 {
                        MENU_SHOW => {
                            mark_ui_visible(runtime, true);
                            refresh_overlay_timer(hwnd, runtime);
                            show_ui_window_native();
                            let _ = runtime.ui_tx.send(UiCommand::ShowWindow);
                        }
                        MENU_EXIT => {
                            let _ = runtime.ui_tx.send(UiCommand::Exit);
                            let _ = shutdown_application(hwnd, runtime);
                        }
                        _ => {}
                    }
                }
                LRESULT(0)
            }
            WMAPP_TRAYICON => {
                match lparam.0 as u32 {
                    WM_RBUTTONUP => {
                        if let Some(runtime) = runtime_mut(hwnd) {
                            let mut point = POINT::default();
                            let _ = GetCursorPos(&mut point);
                            let _ = SetForegroundWindow(hwnd);
                            let _ = TrackPopupMenu(
                                runtime.tray_menu,
                                TPM_LEFTALIGN | TPM_BOTTOMALIGN,
                                point.x,
                                point.y,
                                Some(0),
                                hwnd,
                                None,
                            );
                        }
                    }
                    WM_LBUTTONUP => {
                        if let Some(runtime) = runtime_mut(hwnd) {
                            let suppress_next_up = {
                                let mut hook_state = HOOK_STATE.lock();
                                if hook_state.tray_double_click_suppress_next_up {
                                    hook_state.tray_double_click_suppress_next_up = false;
                                    true
                                } else {
                                    false
                                }
                            };
                            if suppress_next_up {
                                return LRESULT(0);
                            }
                            if runtime.ui_visible {
                                let (enabled, previous) = {
                                    let mut hook_state = HOOK_STATE.lock();
                                    let previous = hook_state.macros_master_enabled;
                                    hook_state.macros_master_enabled =
                                        !hook_state.macros_master_enabled;
                                    (hook_state.macros_master_enabled, previous)
                                };
                                let _ = previous;
                                let _ = update_tray_icon(hwnd, enabled);
                                let status = if enabled {
                                    "Enabled all macros globally.".to_owned()
                                } else {
                                    "Disabled all macros globally.".to_owned()
                                };
                                let _ = runtime
                                    .ui_tx
                                    .send(UiCommand::SetMacrosMasterEnabled(enabled, status));
                                request_ui_repaint();
                            } else {
                                let (enabled, previous) = {
                                    let mut hook_state = HOOK_STATE.lock();
                                    let previous = hook_state.macros_master_enabled;
                                    hook_state.macros_master_enabled =
                                        !hook_state.macros_master_enabled;
                                    hook_state.pending_tray_toggle = Some(previous);
                                    (hook_state.macros_master_enabled, previous)
                                };
                                let _ = previous;
                                let _ = unsafe { update_tray_icon(hwnd, enabled) };
                                let status = if enabled {
                                    "Enabled all macros globally.".to_owned()
                                } else {
                                    "Disabled all macros globally.".to_owned()
                                };
                                let _ = runtime
                                    .ui_tx
                                    .send(UiCommand::SetMacrosMasterEnabled(enabled, status));
                                request_ui_repaint();
                            }
                        }
                    }
                    WM_LBUTTONDBLCLK => {
                        if let Some(runtime) = runtime_mut(hwnd) {
                            {
                                let mut hook_state = HOOK_STATE.lock();
                                if let Some(previous) = hook_state.pending_tray_toggle.take() {
                                    hook_state.macros_master_enabled = previous;
                                    let _ = unsafe { update_tray_icon(hwnd, previous) };
                                    let status = if previous {
                                        "Enabled all macros globally.".to_owned()
                                    } else {
                                        "Disabled all macros globally.".to_owned()
                                    };
                                    let _ = runtime
                                        .ui_tx
                                        .send(UiCommand::SetMacrosMasterEnabled(previous, status));
                                }
                                hook_state.tray_double_click_suppress_next_up = true;
                            }
                            show_ui_window_native();
                            mark_ui_visible(runtime, true);
                            refresh_overlay_timer(hwnd, runtime);
                            let _ = runtime.ui_tx.send(UiCommand::ShowWindow);
                            request_ui_repaint();
                            wake_command_queue();
                        }
                    }
                    _ => {}
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                CONTROLLER_HWND.store(0, Ordering::Relaxed);
                let _ = KillTimer(Some(hwnd), TIMER_ID);
                unregister_all_hotkeys(hwnd, runtime_mut(hwnd));
                let _ = Shell_NotifyIconW(NIM_DELETE, &notify_icon(hwnd));
                if let Some(runtime) = runtime_mut(hwnd) {
                    runtime.running.store(false, Ordering::Relaxed);
                    let _ = DestroyMenu(runtime.tray_menu);
                    let _ = ShowWindow(runtime.overlay_hwnd, SW_HIDE);
                    let _ = ShowWindow(runtime.hud_hwnd, SW_HIDE);
                    let _ = set_input_hooks_enabled(runtime, false);
                }
                let mut hook_state = HOOK_STATE.lock();
                hook_state.ui_tx = None;
                hook_state.window_presets.clear();
                hook_state.window_expand_controls = WindowExpandControls::default();
                hook_state.macro_groups.clear();
                hook_state.locked_inputs.clear();
                hook_state.locked_mouse_count = 0;
                hook_state.profiles.clear();
                hook_state.sound_presets.clear();
                hook_state.active_hold_macros.clear();
                hook_state.held_mouse_buttons.clear();
                *OVERLAY_COMMAND_TX.lock() = None;
                let ptr = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(GWLP_USERDATA.0));
                if ptr != 0 {
                    let _runtime = Box::from_raw(ptr as *mut Runtime);
                }
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    unsafe extern "system" fn hud_wnd_proc(
        hwnd: HWND,
        msg: u32,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            windows::Win32::UI::WindowsAndMessaging::WM_PAINT => {
                let mut paint = PAINTSTRUCT::default();
                let _ = BeginPaint(hwnd, &mut paint);
                let _ = EndPaint(hwnd, &paint);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, _wparam, _lparam),
        }
    }

    unsafe extern "system" fn low_level_keyboard_proc(
        code: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if code == HC_ACTION as i32 {
            let info = *(lparam.0 as *const KBDLLHOOKSTRUCT);
            let msg = wparam.0 as u32;
            let is_key_event = matches!(msg, WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP);
            let injected = info.flags.0 & 0x10 != 0;
            if is_key_event && !injected {
                let is_key_down = matches!(msg, WM_KEYDOWN | WM_SYSKEYDOWN);
                let is_key_up = matches!(msg, WM_KEYUP | WM_SYSKEYUP);
                let key_name = hotkey::vk_to_key_name(info.vkCode).map(str::to_owned);
                
                log_to_file(&format!(
                    "KB Hook: key_name={:?}, is_down={}, is_up={}, ui_foreground={}, is_recording={}",
                    key_name,
                    is_key_down,
                    is_key_up,
                    is_ui_in_foreground(),
                    MACRO_RECORDING.lock().is_some()
                ));

                // Realtime keyboard recording
                if is_key_down {
                    let mut rec_guard = MACRO_RECORDING.lock();
                    if let Some(session) = rec_guard.as_mut() {
                        let now = std::time::Instant::now();
                        let delay_ms = now
                            .saturating_duration_since(session.last_event_at)
                            .as_millis()
                            .min(u64::MAX as u128) as u64;
                        if let Some(k_name) = key_name.clone() {
                            session.last_event_at = now;
                            session.events.push(MacroRecordingEvent {
                                key: Some(k_name.clone()),
                                action: crate::model::MacroAction::KeyPress,
                                delay_ms,
                                x: 0,
                                y: 0,
                            });

                            if let Some(tx) = &HOOK_STATE.lock().ui_tx {
                                let mut step = crate::model::MacroStep::default();
                                step.action = crate::model::MacroAction::KeyPress;
                                step.delay_ms = delay_ms;
                                step.key = k_name;
                                let _ = tx.send(UiCommand::MacroRealtimeStepAdded(
                                    session.group_id,
                                    session.preset_id,
                                    step,
                                ));
                            }
                        }
                    }
                }

                // Global record toggle hotkey processing
                if let Some(key_name) = key_name.clone() {
                    let binding = binding_from_trigger_event(&key_name);
                    if is_key_down {
                        let repeat = is_repeat_key(&key_name);
                        if let Some(swallow) = process_macro_record_hotkey(&binding, repeat) {
                            update_modifier_state(info.vkCode, is_key_down);
                            if swallow {
                                return LRESULT(1);
                            }
                        }
                        if let Some(swallow) = process_mouse_path_record_hotkey(&binding, repeat) {
                            update_modifier_state(info.vkCode, is_key_down);
                            if swallow {
                                return LRESULT(1);
                            }
                        }
                    }
                }

                // Skip normal hotkeys if UI is focused
                if is_ui_in_foreground() {
                    if let Some(key_name) = key_name.clone() {
                        update_held_key(&key_name, is_key_down, is_key_up);
                    }
                    update_modifier_state(info.vkCode, is_key_down);
                    return CallNextHookEx(None, code, wparam, lparam);
                }

                if let Some(key_name) = key_name.clone() {
                    let binding = binding_from_trigger_event(&key_name);
                    if key_name.eq_ignore_ascii_case("Tab") && binding.alt {
                        update_held_key(&key_name, is_key_down, is_key_up);
                        update_modifier_state(info.vkCode, is_key_down);
                        return CallNextHookEx(None, code, wparam, lparam);
                    }
                    let mut swallow = false;
                    if is_key_down {
                        let repeat = is_repeat_key(&key_name);
                        if let Some(binding_swallow) = process_binding_press(&binding, repeat) {
                            swallow |= binding_swallow;
                        }
                    }
                    update_held_key(&key_name, is_key_down, is_key_up);
                    if is_key_up {
                        swallow |= process_binding_release(&binding);
                    }

                    let macros_master_enabled = {
                        let hook_state = HOOK_STATE.lock();
                        hook_state.macros_master_enabled
                    };

                    if macros_master_enabled {
                        swallow |= binding_matches_any_hold_macro(&binding);
                        swallow |= is_locked_input(&key_name);
                    }

                    swallow |= keyboard_arrow_mouse_should_swallow(&key_name);
                    update_modifier_state(info.vkCode, is_key_down);
                    return if swallow {
                        LRESULT(1)
                    } else {
                        CallNextHookEx(None, code, wparam, lparam)
                    };
                }
                update_modifier_state(info.vkCode, is_key_down);
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    unsafe extern "system" fn low_level_mouse_proc(
        code: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if code == HC_ACTION as i32 {
            let info = *(lparam.0 as *const MSLLHOOKSTRUCT);
            let injected = info.flags & 0x01 != 0;
            if injected {
                return CallNextHookEx(None, code, wparam, lparam);
            }
            let message = wparam.0 as u32;
            
            log_to_file(&format!(
                "Mouse Hook: msg=0x{:X}, x={}, y={}, ui_foreground={}, is_recording={}",
                message,
                info.pt.x,
                info.pt.y,
                is_ui_in_foreground(),
                MACRO_RECORDING.lock().is_some() || MOUSE_RECORDING.lock().is_some()
            ));

            record_mouse_event(message, &info);
            record_macro_mouse_event(message, &info);
            if is_mouse_locked() {
                match message {
                    WM_MOUSEMOVE
                    | WM_MOUSEWHEEL
                    | WM_LBUTTONDOWN
                    | WM_LBUTTONUP
                    | WM_RBUTTONDOWN
                    | WM_RBUTTONUP
                    | WM_MBUTTONDOWN
                    | windows::Win32::UI::WindowsAndMessaging::WM_MBUTTONUP
                    | WM_XBUTTONDOWN
                    | WM_XBUTTONUP => {
                        update_held_mouse_button(message, ((info.mouseData >> 16) & 0xFFFF) as u16);
                        return LRESULT(1);
                    }
                    _ => {}
                }
            }
            if is_vision_capture_mouse_blocked() {
                    match message {
                        WM_MOUSEMOVE => {
                        let mut hook_state = HOOK_STATE.lock();
                        let left_held = hook_state.held_mouse_buttons.contains("MouseLeft");
                        if left_held {
                            if let Some((start_x, start_y)) = hook_state.vision_capture_anchor
                            {
                                let left = start_x.min(info.pt.x);
                                let top = start_y.min(info.pt.y);
                                let width = (start_x - info.pt.x).abs().max(1);
                                let height = (start_y - info.pt.y).abs().max(1);
                                let region = VisionRegion {
                                    left,
                                    top,
                                    width,
                                    height,
                                    is_circle: false,
                                    angle_offset_deg: None, angle_span_deg: None,
                                };
                                if hook_state.vision_capture_preview_region != Some(region) {
                                    hook_state.vision_capture_preview_region = Some(region);
                                    let ui_tx = hook_state.ui_tx.clone();
                                    drop(hook_state);
                                    if let Some(ui_tx) = ui_tx {
                                        let _ =
                                            ui_tx.send(UiCommand::VisionCaptureMouseMove {
                                                screen_x: info.pt.x,
                                                screen_y: info.pt.y,
                                            });
                                    }
                                    wake_command_queue();
                                    return CallNextHookEx(None, code, wparam, lparam);
                                }
                            }
                            let ui_tx = hook_state.ui_tx.clone();
                            drop(hook_state);
                            if let Some(ui_tx) = ui_tx {
                                let _ = ui_tx.send(UiCommand::VisionCaptureMouseMove {
                                    screen_x: info.pt.x,
                                    screen_y: info.pt.y,
                                });
                            }
                        }
                        return CallNextHookEx(None, code, wparam, lparam);
                    }
                    WM_LBUTTONDOWN => {
                        update_held_mouse_button(message, ((info.mouseData >> 16) & 0xFFFF) as u16);
                        let mut hook_state = HOOK_STATE.lock();
                        hook_state.vision_capture_anchor = Some((info.pt.x, info.pt.y));
                        hook_state.vision_capture_preview_region = Some(VisionRegion {
                            left: info.pt.x,
                            top: info.pt.y,
                            width: 1,
                            height: 1,
                            is_circle: false,
                            angle_offset_deg: None, angle_span_deg: None,
                        });
                        let ui_tx = hook_state.ui_tx.clone();
                        drop(hook_state);
                        if let Some(ui_tx) = ui_tx {
                            let _ = ui_tx.send(UiCommand::VisionCaptureMouseDown {
                                screen_x: info.pt.x,
                                screen_y: info.pt.y,
                            });
                        }
                        wake_command_queue();
                        return LRESULT(1);
                    }
                    WM_LBUTTONUP => {
                        update_held_mouse_button(message, ((info.mouseData >> 16) & 0xFFFF) as u16);
                        let mut hook_state = HOOK_STATE.lock();
                        hook_state.vision_capture_anchor = None;
                        hook_state.vision_capture_preview_region = None;
                        let ui_tx = hook_state.ui_tx.clone();
                        drop(hook_state);
                        if let Some(ui_tx) = ui_tx {
                            let _ = ui_tx.send(UiCommand::VisionCaptureMouseUp {
                                screen_x: info.pt.x,
                                screen_y: info.pt.y,
                            });
                        }
                        wake_command_queue();
                        return LRESULT(1);
                    }
                    WM_MOUSEWHEEL
                    | WM_RBUTTONDOWN
                    | WM_RBUTTONUP
                    | WM_MBUTTONDOWN
                    | windows::Win32::UI::WindowsAndMessaging::WM_MBUTTONUP
                    | WM_XBUTTONDOWN
                    | WM_XBUTTONUP => {
                        update_held_mouse_button(message, ((info.mouseData >> 16) & 0xFFFF) as u16);
                        return LRESULT(1);
                    }
                    _ => {}
                }
            }
            let is_recording = MACRO_RECORDING.lock().is_some() || MOUSE_RECORDING.lock().is_some();
            if is_ui_in_foreground() && !is_recording {
                return CallNextHookEx(None, code, wparam, lparam);
            }
            let event = match (wparam.0 as u32, ((info.mouseData >> 16) & 0xFFFF) as u16) {
                (WM_LBUTTONDOWN, _) => Some((binding_from_trigger_event("MouseLeft"), true)),
                (WM_LBUTTONUP, _) => Some((binding_from_trigger_event("MouseLeft"), false)),
                (WM_RBUTTONDOWN, _) => Some((binding_from_trigger_event("MouseRight"), true)),
                (WM_RBUTTONUP, _) => Some((binding_from_trigger_event("MouseRight"), false)),
                (WM_MBUTTONDOWN, _) => Some((binding_from_trigger_event("MouseMiddle"), true)),
                (windows::Win32::UI::WindowsAndMessaging::WM_MBUTTONUP, _) => {
                    Some((binding_from_trigger_event("MouseMiddle"), false))
                }
                (WM_XBUTTONDOWN, XBUTTON1_DATA) => Some((binding_from_trigger_event("MouseX1"), true)),
                (WM_XBUTTONUP, XBUTTON1_DATA) => Some((binding_from_trigger_event("MouseX1"), false)),
                (WM_XBUTTONDOWN, XBUTTON2_DATA) => Some((binding_from_trigger_event("MouseX2"), true)),
                (WM_XBUTTONUP, XBUTTON2_DATA) => Some((binding_from_trigger_event("MouseX2"), false)),
                _ => None,
            };
            if let Some((binding, is_down)) = event {
                update_held_mouse_button(message, ((info.mouseData >> 16) & 0xFFFF) as u16);
                let mut swallow = if is_down {
                    process_binding_press(&binding, false).unwrap_or(false)
                } else {
                    process_binding_release(&binding)
                };

                let macros_master_enabled = {
                    let hook_state = HOOK_STATE.lock();
                    hook_state.macros_master_enabled
                };

                if macros_master_enabled {
                    swallow |= binding_matches_any_hold_macro(&binding);
                }

                return if swallow {
                    LRESULT(1)
                } else {
                    CallNextHookEx(None, code, wparam, lparam)
                };
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    fn binding_from_event(key_name: &str) -> HotkeyBinding {
        let ctrl_down = unsafe { GetAsyncKeyState(0x11) } < 0;
        let alt_down = unsafe { GetAsyncKeyState(0x12) } < 0;
        let shift_down = unsafe { GetAsyncKeyState(0x10) } < 0;
        let win_down =
            unsafe { GetAsyncKeyState(0x5B) } < 0 || unsafe { GetAsyncKeyState(0x5C) } < 0;
        let mut combo_keys = {
            let hook_state = HOOK_STATE.lock();
            let mut keys = hook_state
                .held_inputs
                .iter()
                .cloned()
                .chain(hook_state.held_mouse_buttons.iter().cloned())
                .collect::<Vec<_>>();
            keys.push(key_name.to_owned());
            keys
        };
        combo_keys.retain(|key| !key.trim().is_empty());
        combo_keys.sort_by(|a, b| {
            let rank_a = hotkey_binding_rank(a);
            let rank_b = hotkey_binding_rank(b);
            rank_a
                .cmp(&rank_b)
                .then_with(|| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()))
        });
        combo_keys.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        HotkeyBinding {
            ctrl: ctrl_down && !key_name.eq_ignore_ascii_case("Ctrl"),
            alt: alt_down && !key_name.eq_ignore_ascii_case("Alt"),
            shift: shift_down && !key_name.eq_ignore_ascii_case("Shift"),
            win: win_down && !key_name.eq_ignore_ascii_case("Win"),
            key: key_name.to_owned(),
            combo_keys,
        }
    }

    fn binding_from_trigger_event(key_name: &str) -> HotkeyBinding {
        let ctrl_down = unsafe { GetAsyncKeyState(0x11) } < 0;
        let alt_down = unsafe { GetAsyncKeyState(0x12) } < 0;
        let shift_down = unsafe { GetAsyncKeyState(0x10) } < 0;
        let win_down =
            unsafe { GetAsyncKeyState(0x5B) } < 0 || unsafe { GetAsyncKeyState(0x5C) } < 0;
        let mut combo_keys = vec![key_name.to_owned()];
        if ctrl_down {
            combo_keys.push("Ctrl".to_owned());
        }
        if alt_down {
            combo_keys.push("Alt".to_owned());
        }
        if shift_down {
            combo_keys.push("Shift".to_owned());
        }
        if win_down {
            combo_keys.push("Win".to_owned());
        }
        combo_keys.sort_by(|a, b| {
            let rank_a = hotkey_binding_rank(a);
            let rank_b = hotkey_binding_rank(b);
            rank_a
                .cmp(&rank_b)
                .then_with(|| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()))
        });
        combo_keys.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        HotkeyBinding {
            ctrl: ctrl_down && !key_name.eq_ignore_ascii_case("Ctrl"),
            alt: alt_down && !key_name.eq_ignore_ascii_case("Alt"),
            shift: shift_down && !key_name.eq_ignore_ascii_case("Shift"),
            win: win_down && !key_name.eq_ignore_ascii_case("Win"),
            key: key_name.to_owned(),
            combo_keys,
        }
    }

    fn hotkey_binding_rank(name: &str) -> (u8, String) {
        let normalized = name.trim().to_ascii_lowercase();
        let rank = match normalized.as_str() {
            "ctrl" | "control" => 0,
            "alt" => 1,
            "shift" => 2,
            "win" | "meta" => 3,
            _ => 4,
        };
        (rank, normalized)
    }

    fn process_mouse_path_record_hotkey(binding: &HotkeyBinding, is_repeat: bool) -> Option<bool> {
        if is_repeat {
            return None;
        }
        let matched = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .mouse_path_presets
                .iter()
                .find(|preset| {
                    preset.enabled
                        && preset
                            .record_hotkey
                            .as_ref()
                            .is_some_and(|hotkey| hotkey::binding_matches(hotkey, binding))
                })
                .cloned()
        };
        let Some(preset) = matched else {
            return None;
        };
        toggle_mouse_recording(preset.id, preset.name);
        Some(true)
    }

    fn image_search_following_is_active(preset_id: u32) -> bool {
        HOOK_STATE
            .lock()
            .vision_following_presets
            .contains(&preset_id)
    }


    fn image_search_wait_generation(preset_id: u32) -> u64 {
        IMAGE_SEARCH_WAIT_GENERATIONS
            .lock()
            .get(&preset_id)
            .copied()
            .unwrap_or(0)
    }


    fn set_image_search_following_active(preset_id: u32, active: bool) {
        let mut hook_state = HOOK_STATE.lock();
        if active {
            hook_state.vision_following_presets.insert(preset_id);
        } else {
            hook_state.vision_following_presets.remove(&preset_id);
        }
    }


    fn bump_image_search_wait_generation(preset_id: u32) {
        let mut guard = IMAGE_SEARCH_WAIT_GENERATIONS.lock();
        let generation = guard.entry(preset_id).or_insert(0);
        *generation = generation.saturating_add(1);
    }


    fn run_image_search_follow_loop(preset: VisionPreset, ui_tx: Option<Sender<UiCommand>>) {
        if let Some(tx) = ui_tx.as_ref() {
            let _ = tx.send(UiCommand::VisionFinished(format!(
                "{}: repeat mode started. Press the hotkey again to stop.",
                preset.name
            )));
        }

        while image_search_following_is_active(preset.id) {
            match run_vision_once_with_options(&preset, true, false) {
                Ok(_) => {}
                Err(error) => {
                    if let Some(tx) = ui_tx.as_ref() {
                        let _ = tx.send(UiCommand::VisionFinished(format!(
                            "{}: Vision search failed: {error}",
                            preset.name
                        )));
                    }
                    break;
                }
            }
            let rate_hz = preset.color_scan_rate_hz.max(1);
            let sleep_ms = (1000 / rate_hz).max(1);
            thread::sleep(Duration::from_millis(sleep_ms as u64));
        }

        set_image_search_following_active(preset.id, false);
        if let Some(tx) = ui_tx {
            let _ = tx.send(UiCommand::VisionFinished(format!(
                "{}: repeat mode stopped.",
                preset.name
            )));
        }
    }

    fn process_image_search_hotkey(binding: &HotkeyBinding, is_repeat: bool) -> Option<bool> {
        if is_repeat {
            return None;
        }

        let (matched, ui_tx) = {
            let hook_state = HOOK_STATE.lock();
            let matched = hook_state
                .vision_presets
                .iter()
                .filter(|preset| {
                    preset.enabled
                        && window_focus_matches(
                            preset.target_window_title.as_deref(),
                            &preset.extra_target_window_titles,
                            preset.match_duplicate_window_titles,
                        )
                        && preset
                            .hotkey
                            .as_ref()
                            .is_some_and(|hotkey| hotkey::binding_matches(hotkey, binding))
                })
                .cloned()
                .collect::<Vec<_>>();
            (matched, hook_state.ui_tx.clone())
        };

        if matched.is_empty() {
            return None;
        }

        for preset in matched {
            if preset.repeat_until_triggered_again {
                let active = {
                    let mut hook_state = HOOK_STATE.lock();
                    if hook_state
                        .vision_following_presets
                        .contains(&preset.id)
                    {
                        hook_state.vision_following_presets.remove(&preset.id);
                        false
                    } else {
                        hook_state.vision_following_presets.insert(preset.id);
                        true
                    }
                };

                if !active {
                    if let Some(tx) = ui_tx.as_ref() {
                        let _ = tx.send(UiCommand::VisionFinished(format!(
                            "{}: repeat mode stopped.",
                            preset.name
                        )));
                    }
                    continue;
                }

                let ui_tx = ui_tx.clone();
                set_image_search_following_active(preset.id, true);
                thread::spawn(move || run_image_search_follow_loop(preset, ui_tx));
                continue;
            }

            let ui_tx = ui_tx.clone();
            thread::spawn(move || {
                let status = match run_vision_once(&preset) {
                    Ok(status) => status,
                    Err(error) => format!("Vision search failed: {error}"),
                };
                if let Some(tx) = ui_tx {
                    let _ = tx.send(UiCommand::VisionFinished(format!(
                        "{}: {status}",
                        preset.name
                    )));
                }
            });
        }

        Some(true)
    }

    fn toggle_mouse_recording(preset_id: u32, preset_name: String) {
        let finished = {
            let mut guard = MOUSE_RECORDING.lock();
            if guard
                .as_ref()
                .is_some_and(|session| session.preset_id == preset_id)
            {
                guard
                    .take()
                    .map(|session| (session.preset_id, session.events))
            } else {
                *guard = Some(MouseRecordingSession {
                    preset_id,
                    last_event_at: Instant::now(),
                    events: Vec::new(),
                    dirty: true,
                });
                None
            }
        };

        let ui_tx = HOOK_STATE.lock().ui_tx.clone();
        if let Some((finished_id, events)) = finished {
            if let Some(tx) = ui_tx {
                let _ = tx.send(UiCommand::MousePathRecordingFinished(
                    finished_id,
                    events,
                    format!("Saved mouse record for {preset_name}."),
                ));
            }
        } else if let Some(tx) = ui_tx {
            let _ = tx.send(UiCommand::MousePathRecordingStarted(
                preset_id,
                format!("Recording mouse path for {preset_name}. Press the hotkey again to stop."),
            ));
        }
    }

    fn process_mouse_sensitivity_hotkey(binding: &HotkeyBinding, is_repeat: bool) -> Option<bool> {
        if is_repeat {
            return None;
        }
        let matched = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .mouse_sensitivity_presets
                .iter()
                .find(|preset| {
                    preset.enabled
                        && window_focus_matches(
                            preset.target_window_title.as_deref(),
                            &preset.extra_target_window_titles,
                            preset.match_duplicate_window_titles,
                        )
                        && preset
                            .hotkey
                            .as_ref()
                            .is_some_and(|hotkey| hotkey::binding_matches(hotkey, binding))
                })
                .cloned()
        };
        let Some(preset) = matched else {
            return None;
        };
        let _ = toggle_mouse_sensitivity_preset(&preset);
        Some(true)
    }

    fn record_macro_mouse_event(message: u32, info: &MSLLHOOKSTRUCT) {
        if is_click_inside_ui(info.pt) {
            return;
        }
        let mut guard = MACRO_RECORDING.lock();
        let Some(session) = guard.as_mut() else {
            return;
        };
        let now = std::time::Instant::now();
        let delay_ms = now
            .saturating_duration_since(session.last_event_at)
            .as_millis()
            .min(u64::MAX as u128) as u64;

        let kind = match message {
            WM_LBUTTONDOWN => Some(crate::model::MacroAction::MouseLeftClick),
            WM_RBUTTONDOWN => Some(crate::model::MacroAction::MouseRightClick),
            WM_MBUTTONDOWN => Some(crate::model::MacroAction::MouseMiddleClick),
            WM_XBUTTONDOWN => {
                let xbutton = ((info.mouseData >> 16) & 0xFFFF) as u16;
                if xbutton == 1 {
                    Some(crate::model::MacroAction::MouseX1Click)
                } else {
                    Some(crate::model::MacroAction::MouseX2Click)
                }
            }
            WM_MOUSEWHEEL => {
                let data = ((info.mouseData >> 16) & 0xFFFF) as i16;
                if data > 0 {
                    Some(crate::model::MacroAction::MouseWheelUp)
                } else {
                    Some(crate::model::MacroAction::MouseWheelDown)
                }
            }
            _ => None,
        };

        if let Some(action) = kind {
            session.last_event_at = now;
            session.events.push(MacroRecordingEvent {
                key: None,
                action,
                delay_ms,
                x: info.pt.x,
                y: info.pt.y,
            });

            if let Some(tx) = &HOOK_STATE.lock().ui_tx {
                let mut step = crate::model::MacroStep::default();
                step.action = action;
                step.delay_ms = delay_ms;
                step.x = info.pt.x;
                step.y = info.pt.y;
                let _ = tx.send(UiCommand::MacroRealtimeStepAdded(
                    session.group_id,
                    session.preset_id,
                    step,
                ));
            }
        }
    }

    fn toggle_macro_recording(group_id: u32, preset_id: u32, preset_name: String) {
        let finished = {
            let mut guard = MACRO_RECORDING.lock();
            if guard.is_some() {
                let session = guard.take().unwrap();
                if session.preset_id == preset_id {
                    Some((session.group_id, session.preset_id, session.events, true))
                } else {
                    *guard = Some(MacroRecordingSession {
                        group_id,
                        preset_id,
                        last_event_at: std::time::Instant::now(),
                        events: Vec::new(),
                    });
                    Some((session.group_id, session.preset_id, session.events, false))
                }
            } else {
                *guard = Some(MacroRecordingSession {
                    group_id,
                    preset_id,
                    last_event_at: std::time::Instant::now(),
                    events: Vec::new(),
                });
                None
            }
        };

        let ui_tx = HOOK_STATE.lock().ui_tx.clone();
        if let Some((finished_group_id, finished_preset_id, mut events, is_same)) = finished {
            if is_ui_in_foreground() {
                while let Some(last) = events.last() {
                    if let Some(k) = &last.key {
                        if k.eq_ignore_ascii_case("Alt") || k.eq_ignore_ascii_case("Tab") {
                            events.pop();
                            continue;
                        }
                    }
                    break;
                }
            }
            if let Some(tx) = &ui_tx {
                let _ = tx.send(UiCommand::MacroRecordingFinished(
                    finished_group_id,
                    finished_preset_id,
                    events,
                    format!("Saved macro record."),
                ));
            }
            if !is_same {
                if let Some(tx) = &ui_tx {
                    let _ = tx.send(UiCommand::MacroRecordingStarted(
                        preset_id,
                        format!("Recording macro for {preset_name}. Press Stop in the UI to finish."),
                    ));
                }
            }
        } else if let Some(tx) = ui_tx {
            let _ = tx.send(UiCommand::MacroRecordingStarted(
                preset_id,
                format!("Recording macro for {preset_name}. Press Stop in the UI to finish."),
            ));
        }
    }

    fn process_macro_record_hotkey(binding: &HotkeyBinding, is_repeat: bool) -> Option<bool> {
        if is_repeat {
            return None;
        }
        let matched = {
            let hook_state = HOOK_STATE.lock();
            let mut found = None;
            for group in &hook_state.macro_groups {
                for preset in &group.presets {
                    if let Some(record_hotkey) = &preset.record_hotkey {
                        if hotkey::binding_matches(record_hotkey, binding) {
                            found = Some((group.id, preset.id, group.name.clone()));
                            break;
                        }
                    }
                }
                if found.is_some() {
                    break;
                }
            }
            found
        };

        if let Some((group_id, preset_id, group_name)) = matched {
            toggle_macro_recording(group_id, preset_id, group_name);
            Some(true)
        } else {
            None
        }
    }

    fn record_mouse_event(message: u32, info: &MSLLHOOKSTRUCT) {
        let mut guard = MOUSE_RECORDING.lock();
        let Some(session) = guard.as_mut() else {
            return;
        };
        let now = Instant::now();
        let delay_ms = now
            .saturating_duration_since(session.last_event_at)
            .as_millis()
            .min(u64::MAX as u128) as u64;
        session.last_event_at = now;
        let point = info.pt;
        let kind = match (message, ((info.mouseData >> 16) & 0xFFFF) as u16) {
            (WM_MOUSEMOVE, _) => Some(MousePathEventKind::Move),
            (WM_LBUTTONDOWN, _) => Some(MousePathEventKind::LeftDown),
            (WM_LBUTTONUP, _) => Some(MousePathEventKind::LeftUp),
            (WM_RBUTTONDOWN, _) => Some(MousePathEventKind::RightDown),
            (WM_RBUTTONUP, _) => Some(MousePathEventKind::RightUp),
            (WM_MBUTTONDOWN, _) => Some(MousePathEventKind::MiddleDown),
            (windows::Win32::UI::WindowsAndMessaging::WM_MBUTTONUP, _) => {
                Some(MousePathEventKind::MiddleUp)
            }
            (WM_MOUSEWHEEL, data) if (data as i16) > 0 => Some(MousePathEventKind::WheelUp),
            (WM_MOUSEWHEEL, _) => Some(MousePathEventKind::WheelDown),
            _ => None,
        };
        let Some(kind) = kind else {
            return;
        };
        if matches!(kind, MousePathEventKind::Move)
            && session.events.last().is_some_and(|last| {
                matches!(last.kind, MousePathEventKind::Move)
                    && last.x == point.x
                    && last.y == point.y
            })
        {
            return;
        }
        session.events.push(MousePathEvent {
            kind,
            x: point.x,
            y: point.y,
            delay_ms,
        });
        session.dirty = true;
    }

    fn release_trigger_ready(
        wait_key_spec: &str,
        require_all_inputs_released: bool,
        _released_key: &str,
    ) -> bool {
        let wait_keys = parse_locked_keys(wait_key_spec);
        let hook_state = HOOK_STATE.lock();
        if wait_keys.iter().any(|wait_key| {
            hook_state
                .held_inputs
                .iter()
                .any(|held| held.eq_ignore_ascii_case(wait_key))
                || hook_state
                    .held_mouse_buttons
                    .iter()
                    .any(|held| held.eq_ignore_ascii_case(wait_key))
        }) {
            return false;
        }

        if !require_all_inputs_released {
            return true;
        }

        hook_state.held_inputs.is_empty() && hook_state.held_mouse_buttons.is_empty()
    }

    fn binding_is_single_key(binding: &HotkeyBinding) -> bool {
        hotkey::binding_key_names(binding).len() == 1
    }

    fn mouse_trigger_is_physically_down(trigger: &HotkeyBinding) -> bool {
        let Some(vk) = hotkey::key_name_to_vk(&trigger.key) else {
            return true;
        };
        if !hotkey::is_mouse_key_name(&trigger.key) {
            return true;
        }
        (unsafe { GetAsyncKeyState(vk as i32) }) < 0
    }

    fn reconcile_active_hold_mouse_macros() {
        let stale_ids = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .active_hold_macros
                .iter()
                .filter_map(|(preset_id, active)| {
                    (!mouse_trigger_is_physically_down(&active.trigger)).then_some(*preset_id)
                })
                .collect::<Vec<_>>()
        };

        for preset_id in stale_ids {
            deactivate_hold_macro(preset_id);
        }
    }

    fn hold_macro_release_matches(active: &ActiveHoldMacro, binding: &HotkeyBinding) -> bool {
        active.trigger.key.eq_ignore_ascii_case(&binding.key)
    }

    fn binding_matches_any_hold_macro(binding: &HotkeyBinding) -> bool {
        let hook_state = HOOK_STATE.lock();
        if !hook_state.macros_master_enabled {
            return false;
        }
        hook_state.macro_groups.iter().any(|group| {
            group.enabled
                && macro_target_matches(group)
                && group.presets.iter().any(|preset| {
                    preset.enabled
                        && preset.trigger_mode == MacroTriggerMode::Hold
                        && macro_preset_trigger_matches(preset, binding)
                })
        })
    }

    fn trigger_binding_matches(expected: &HotkeyBinding, observed: &HotkeyBinding) -> bool {
        let expected_keys = hotkey::binding_key_names(expected);
        if expected_keys.is_empty() {
            return false;
        }
        let observed_keys = hotkey::binding_key_names(observed)
            .into_iter()
            .map(|key| key.to_ascii_lowercase())
            .collect::<HashSet<_>>();
        expected_keys
            .into_iter()
            .map(|key| key.to_ascii_lowercase())
            .all(|key| observed_keys.contains(&key))
    }

    fn remove_pending_press_trigger_key(key_name: &str) -> Option<String> {
        let mut hook_state = HOOK_STATE.lock();
        let pending = hook_state
            .pending_press_trigger_keys
            .iter()
            .find(|pending| pending.eq_ignore_ascii_case(key_name))
            .cloned()?;
        hook_state.pending_press_trigger_keys.remove(&pending);
        Some(pending)
    }

    fn consume_pending_press_trigger_keys(binding: &HotkeyBinding) -> Vec<String> {
        let combo_keys = hotkey::binding_key_names(binding);
        let mut hook_state = HOOK_STATE.lock();
        let mut consumed = Vec::new();
        for key in combo_keys {
            if let Some(pending) = hook_state
                .pending_press_trigger_keys
                .iter()
                .find(|pending| pending.eq_ignore_ascii_case(&key))
                .cloned()
            {
                hook_state.pending_press_trigger_keys.remove(&pending);
                consumed.push(pending);
            }
        }
        consumed
    }

    fn fire_pending_press_triggers(binding: &HotkeyBinding) -> bool {
        let Some(_) = remove_pending_press_trigger_key(&binding.key) else {
            return false;
        };

        let press_matches = {
            let hook_state = HOOK_STATE.lock();
            let mut press_matches: Vec<(MacroPreset, Option<String>, Vec<String>, bool, String)> =
                Vec::new();
            for group in &hook_state.macro_groups {
                if !group.enabled {
                    continue;
                }
                if !macro_target_matches(group) {
                    continue;
                }
                for preset in &group.presets {
                    if !preset.enabled
                        || preset.trigger_mode != MacroTriggerMode::Press
                        || !macro_preset_trigger_matches(preset, binding)
                    {
                        continue;
                    }
                    press_matches.push((
                        preset.clone(),
                        group.target_window_title.clone(),
                        group.extra_target_window_titles.clone(),
                        group.match_duplicate_window_titles,
                        binding.key.clone(),
                    ));
                }
            }
            press_matches
        };

        for (
            preset,
            target_window_title,
            extra_target_window_titles,
            match_duplicate_window_titles,
            trigger_key,
        ) in press_matches
        {
            let hotkey_id = MACRO_PRESET_BASE_ID + preset.id as i32;
            if !SUPPRESSED_MACRO_HOTKEYS.lock().contains(&hotkey_id) {
                let _ = play_macro_preset(
                    hotkey_id,
                    preset,
                    target_window_title,
                    extra_target_window_titles,
                    match_duplicate_window_titles,
                    trigger_key,
                );
            } else {
                STOP_REQUESTED_MACRO_PRESETS.lock().insert(preset.id);
            }
        }

        true
    }

    fn process_binding_press(binding: &HotkeyBinding, is_repeat: bool) -> Option<bool> {
        if let Some(swallow) = process_mouse_sensitivity_hotkey(binding, is_repeat) {
            return Some(swallow);
        }
        if let Some(swallow) = process_image_search_hotkey(binding, is_repeat) {
            return Some(swallow);
        }
        let master_toggle = {
            let mut hook_state = HOOK_STATE.lock();
            let matches_master_hotkey = hook_state
                .macros_master_hotkey
                .as_ref()
                .is_some_and(|hotkey| hotkey::binding_matches(hotkey, binding));
            if matches_master_hotkey {
                hook_state.macros_master_enabled = !hook_state.macros_master_enabled;
                let enabled = hook_state.macros_master_enabled;
                let status = if enabled {
                    "Enabled macros globally.".to_owned()
                } else {
                    "Disabled macros globally.".to_owned()
                };
                Some((enabled, status))
            } else {
                None
            }
        };
        if let Some((enabled, status)) = master_toggle {
            send_ui_command(UiCommand::SetMacrosMasterEnabled(enabled, status));
            send_overlay_command(OverlayCommand::SetMacrosMasterEnabled(enabled));
            return Some(true);
        }
        let is_record_hotkey = {
            let hook_state = HOOK_STATE.lock();
            hook_state.macro_groups.iter().any(|g| {
                g.presets.iter().any(|p| {
                    p.record_hotkey.as_ref().is_some_and(|h| hotkey::binding_matches(h, binding))
                })
            }) || hook_state.mouse_path_presets.iter().any(|p| {
                p.record_hotkey.as_ref().is_some_and(|h| hotkey::binding_matches(h, binding))
            })
        };

        if is_ui_in_foreground() && !is_record_hotkey {
            return Some(false);
        }



        let hook_state = HOOK_STATE.lock();
        let mut matched_any_window = false;
        let mut window_actions = Vec::new();
        for preset in &hook_state.window_presets {
            if !preset.enabled {
                continue;
            }
            if !window_focus_matches(
                preset.target_window_title.as_deref(),
                &preset.extra_target_window_titles,
                false,
            ) {
                continue;
            }
            if let Some(hotkey) = preset.hotkey.as_ref()
                && hotkey::binding_matches(hotkey, binding)
                && !is_repeat
            {
                matched_any_window = true;
                if preset.animate_enabled {
                    window_actions.push(WindowHotkeyAction::Animate(preset.clone()));
                } else {
                    window_actions.push(WindowHotkeyAction::Apply(preset.clone()));
                }
            }
        }

        for preset in &hook_state.window_focus_presets {
            if !preset.enabled {
                continue;
            }
            if let Some(hotkey) = preset.hotkey.as_ref()
                && hotkey::binding_matches(hotkey, binding)
                && !is_repeat
            {
                matched_any_window = true;
                window_actions.push(WindowHotkeyAction::Focus(preset.clone()));
            }
        }

        let mut pin_toggle_id = None;
        for preset in &hook_state.pin_presets {
            if !preset.enabled {
                continue;
            }
            if let Some(hotkey) = preset.hotkey.as_ref()
                && hotkey::binding_matches(hotkey, binding)
                && !is_repeat
            {
                pin_toggle_id = Some(preset.id);
                break;
            }
        }
        if let Some(preset_id) = pin_toggle_id {
            drop(hook_state);
            let mut hook_state = HOOK_STATE.lock();
            if hook_state.active_pin_preset_id == Some(preset_id) {
                hook_state.active_pin_preset_id = None;
            } else {
                hook_state.active_pin_preset_id = Some(preset_id);
            }
            return Some(false);
        }



        if !hook_state.macros_master_enabled {
            drop(hook_state);
            for action in window_actions {
                match action {
                    WindowHotkeyAction::Apply(preset) => {
                        let _ = apply_window_preset(&preset);
                    }
                    WindowHotkeyAction::Focus(preset) => {
                        let _ = focus_window_for_preset(&preset);
                    }
                    WindowHotkeyAction::Animate(preset) => {
                        thread::spawn(move || {
                            let _ = apply_window_preset_animated(&preset);
                        });
                    }
                    WindowHotkeyAction::RestoreTitleBar(preset) => {
                        let _ = restore_window_title_bar_for_preset(&preset);
                    }
                }
            }
            return Some(false);
        }

        let mut matched_any_macro = false;
        let mut hold_matches: Vec<(
            MacroPreset,
            HotkeyBinding,
            Option<String>,
            Vec<String>,
            bool,
            String,
        )> = Vec::new();
        let mut press_matches: Vec<(MacroPreset, Option<String>, Vec<String>, bool, String)> =
            Vec::new();
        let mut matched_any_press = false;
        for group in &hook_state.macro_groups {
            if !group.enabled {
                continue;
            }
            if !macro_target_matches(group) {
                continue;
            }
            for preset in &group.presets {
                if !preset.enabled {
                    continue;
                }
                if !macro_preset_trigger_matches(preset, &binding) {
                    continue;
                }
                if preset.trigger_mode == MacroTriggerMode::Hold {
                    matched_any_macro = true;
                    if !hook_state.active_hold_macros.contains_key(&preset.id) {
                        hold_matches.push((
                            preset.clone(),
                            binding.clone(),
                            group.target_window_title.clone(),
                            group.extra_target_window_titles.clone(),
                            group.match_duplicate_window_titles,
                            binding.key.clone(),
                        ));
                    }
                    continue;
                }

                if preset.trigger_mode == MacroTriggerMode::Release {
                    continue;
                }

                matched_any_macro = true;
                matched_any_press = true;

                if is_repeat {
                    continue;
                }

                press_matches.push((
                    preset.clone(),
                    group.target_window_title.clone(),
                    group.extra_target_window_titles.clone(),
                    group.match_duplicate_window_titles,
                    binding.key.clone(),
                ));
            }
        }

        drop(hook_state);

        if matched_any_press {
            for key_name in consume_pending_press_trigger_keys(binding) {
                increment_press_trigger_suppression(&key_name);
            }
        }

        for action in window_actions {
            match action {
                WindowHotkeyAction::Apply(preset) => {
                    let _ = apply_window_preset(&preset);
                }
                WindowHotkeyAction::Focus(preset) => {
                    let _ = focus_window_for_preset(&preset);
                }
                WindowHotkeyAction::Animate(preset) => {
                    thread::spawn(move || {
                        let _ = apply_window_preset_animated(&preset);
                    });
                }
                WindowHotkeyAction::RestoreTitleBar(preset) => {
                    let _ = restore_window_title_bar_for_preset(&preset);
                }
            }
        }

        for (
            preset,
            trigger,
            target_window_title,
            extra_target_window_titles,
            match_duplicate_window_titles,
            trigger_key,
        ) in hold_matches
        {
            activate_hold_macro(
                preset,
                trigger,
                target_window_title,
                extra_target_window_titles,
                match_duplicate_window_titles,
                trigger_key,
            );
        }

        for (
            preset,
            target_window_title,
            extra_target_window_titles,
            match_duplicate_window_titles,
            trigger_key,
        ) in press_matches
        {
            let hotkey_id = MACRO_PRESET_BASE_ID + preset.id as i32;
            if !SUPPRESSED_MACRO_HOTKEYS.lock().contains(&hotkey_id) {
                let _ = play_macro_preset(
                    hotkey_id,
                    preset,
                    target_window_title,
                    extra_target_window_titles,
                    match_duplicate_window_titles,
                    trigger_key,
                );
            } else {
                STOP_REQUESTED_MACRO_PRESETS.lock().insert(preset.id);
            }
        }

        if matched_any_macro {
            return Some(true);
        }

        Some(matched_any_window)
    }

    fn process_binding_release(binding: &HotkeyBinding) -> bool {
        let suppressed_press_release = is_press_trigger_suppressed(&binding.key);
        if suppressed_press_release {
            decrement_press_trigger_suppression(&binding.key);
        }

        let mut release_matches: Vec<(MacroPreset, Option<String>, Vec<String>, bool)> = Vec::new();
        let preset_ids = {
            let hook_state = HOOK_STATE.lock();
            for group in &hook_state.macro_groups {
                if !group.enabled {
                    continue;
                }
                if !macro_target_matches(group) {
                    continue;
                }
                for preset in &group.presets {
                    if !preset.enabled {
                        continue;
                    }
                    if preset.trigger_mode != MacroTriggerMode::Release {
                        continue;
                    }
                    if !macro_preset_trigger_matches(preset, binding) {
                        continue;
                    }
                    release_matches.push((
                        preset.clone(),
                        group.target_window_title.clone(),
                        group.extra_target_window_titles.clone(),
                        group.match_duplicate_window_titles,
                    ));
                }
            }

            hook_state
                .active_hold_macros
                .iter()
                .filter(|(_, active)| hold_macro_release_matches(active, binding))
                .map(|(preset_id, _)| *preset_id)
                .collect::<Vec<_>>()
        };

        for (
            preset,
            target_window_title,
            extra_target_window_titles,
            match_duplicate_window_titles,
        ) in release_matches
        {
            if !release_trigger_ready(
                &preset.release_wait_key,
                preset.release_requires_all_inputs_released,
                &binding.key,
            ) {
                continue;
            }
            let hotkey_id = MACRO_PRESET_BASE_ID + preset.id as i32;
            if STOP_REQUESTED_MACRO_PRESETS.lock().contains(&preset.id) {
                continue;
            }
            let _ = play_macro_preset(
                hotkey_id,
                preset,
                target_window_title,
                extra_target_window_titles,
                match_duplicate_window_titles,
                binding.key.clone(),
            );
        }

        let had_hold_matches = !preset_ids.is_empty();
        if had_hold_matches {
            for preset_id in preset_ids {
                deactivate_hold_macro(preset_id);
            }
        }

        // If the key press was already suppressed as a hotkey trigger, also
        // swallow the matching key-up so games and apps do not see a leaked tap.
        if suppressed_press_release {
            return true;
        }

        // Release triggers should not swallow the key-up event. They are meant to
        // observe the release and run actions, not to lock the source key.
        let _ = had_hold_matches;
        false
    }

    fn increment_press_trigger_suppression(key_name: &str) {
        let mut hook_state = HOOK_STATE.lock();
        *hook_state
            .press_trigger_suppression
            .entry(key_name.to_owned())
            .or_insert(0) += 1;
    }

    fn decrement_press_trigger_suppression(key_name: &str) {
        let mut hook_state = HOOK_STATE.lock();
        if let Some(count) = hook_state.press_trigger_suppression.get_mut(key_name) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                hook_state.press_trigger_suppression.remove(key_name);
            }
        }
    }

    fn is_press_trigger_suppressed(key_name: &str) -> bool {
        HOOK_STATE
            .lock()
            .press_trigger_suppression
            .get(key_name)
            .copied()
            .unwrap_or_default()
            > 0
    }

    fn is_locked_input(key_name: &str) -> bool {
        HOOK_STATE
            .lock()
            .locked_inputs
            .get(key_name)
            .copied()
            .unwrap_or_default()
            > 0
    }

    fn current_mouse_speed() -> Result<u32> {
        let mut speed = 10u32;
        unsafe {
            SystemParametersInfoW(
                SPI_GETMOUSESPEED,
                0,
                Some((&mut speed as *mut u32).cast()),
                Default::default(),
            )
            .context("Failed to read mouse speed")?;
        }
        Ok(speed.clamp(1, 20))
    }

    fn set_mouse_speed(speed: u32) -> Result<()> {
        let speed = speed.clamp(1, 20);
        unsafe {
            SystemParametersInfoW(
                SPI_SETMOUSESPEED,
                0,
                Some(speed as usize as *mut c_void),
                SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
            )
            .context("Failed to set mouse speed")?;
        }
        Ok(())
    }

    fn apply_mouse_sensitivity_preset(preset: &MouseSensitivityPreset) -> Result<()> {
        let mut hook_state = HOOK_STATE.lock();
        if hook_state.mouse_sensitivity_restore_speed.is_none() {
            hook_state.mouse_sensitivity_restore_speed = Some(current_mouse_speed()?);
        }
        hook_state.active_mouse_sensitivity_preset_id = Some(preset.id);
        drop(hook_state);
        set_mouse_speed(preset.speed)?;
        Ok(())
    }

    fn restore_mouse_sensitivity() -> Result<()> {
        let restore_speed = {
            let mut hook_state = HOOK_STATE.lock();
            let restore_speed = hook_state.mouse_sensitivity_restore_speed.take();
            hook_state.active_mouse_sensitivity_preset_id = None;
            restore_speed
        };
        if let Some(speed) = restore_speed {
            set_mouse_speed(speed)?;
        }
        Ok(())
    }

    fn restore_mouse_sensitivity_on_exit() -> Result<()> {
        let (enabled, speed) = {
            let hook_state = HOOK_STATE.lock();
            (
                hook_state.mouse_sensitivity_restore_on_exit,
                hook_state.mouse_sensitivity_exit_restore_speed,
            )
        };
        if enabled {
            set_mouse_speed(speed)?;
        }
        Ok(())
    }

    fn toggle_mouse_sensitivity_preset(preset: &MouseSensitivityPreset) -> Result<()> {
        let should_restore = {
            let hook_state = HOOK_STATE.lock();
            hook_state.active_mouse_sensitivity_preset_id == Some(preset.id)
        };
        if should_restore {
            restore_mouse_sensitivity()
        } else {
            apply_mouse_sensitivity_preset(preset)
        }
    }

    fn parse_mouse_sensitivity_preset_id(key: &str) -> Option<u32> {
        key.trim().parse::<u32>().ok()
    }

    fn update_modifier_state(vk: u32, is_key_down: bool) {
        let mut hook_state = HOOK_STATE.lock();
        match vk {
            0x10 | 0xA0 | 0xA1 => hook_state.shift = is_key_down,
            0x11 | 0xA2 | 0xA3 => hook_state.ctrl = is_key_down,
            0x12 | 0xA4 | 0xA5 => hook_state.alt = is_key_down,
            0x5B | 0x5C => hook_state.win = is_key_down,
            _ => {}
        }
    }

    fn update_held_key(key_name: &str, is_key_down: bool, is_key_up: bool) {
        let mut hook_state = HOOK_STATE.lock();
        if is_key_down {
            hook_state.held_inputs.insert(key_name.to_owned());
            let ignored_for_stop = hook_state
                .stop_ignore_keys
                .values()
                .any(|ignored| ignored.eq_ignore_ascii_case(key_name));
            if !ignored_for_stop {
                hook_state.pressed_inputs.insert(key_name.to_owned());
            }
        } else if is_key_up {
            hook_state.held_inputs.remove(key_name);
            hook_state
                .stop_ignore_keys
                .retain(|_, ignored| !ignored.eq_ignore_ascii_case(key_name));
        }
    }

    fn update_held_mouse_button(message: u32, mouse_data: u16) {
        let key_name = match (message, mouse_data) {
            (WM_LBUTTONDOWN | WM_LBUTTONUP, _) => Some("MouseLeft"),
            (WM_RBUTTONDOWN | WM_RBUTTONUP, _) => Some("MouseRight"),
            (WM_MBUTTONDOWN | windows::Win32::UI::WindowsAndMessaging::WM_MBUTTONUP, _) => {
                Some("MouseMiddle")
            }
            (WM_XBUTTONDOWN | WM_XBUTTONUP, XBUTTON1_DATA) => Some("MouseX1"),
            (WM_XBUTTONDOWN | WM_XBUTTONUP, XBUTTON2_DATA) => Some("MouseX2"),
            _ => None,
        };
        let Some(key_name) = key_name else {
            return;
        };
        let is_down = matches!(
            message,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        );
        let mut hook_state = HOOK_STATE.lock();
        if is_down {
            hook_state.held_mouse_buttons.insert(key_name.to_owned());
        } else {
            hook_state.held_mouse_buttons.remove(key_name);
        }
    }

    fn clear_transient_input_state() {
        let mut hook_state = HOOK_STATE.lock();
        hook_state.ctrl = false;
        hook_state.alt = false;
        hook_state.shift = false;
        hook_state.win = false;
        hook_state.held_inputs.clear();
        hook_state.held_mouse_buttons.clear();
    }

    fn cancel_pending_tray_toggle() {
        let mut hook_state = HOOK_STATE.lock();
        hook_state.pending_tray_toggle = None;
    }

    fn stop_key_triggered(preset_id: u32, key_name: &str) -> bool {
        let mut hook_state = HOOK_STATE.lock();
        if hook_state
            .stop_ignore_keys
            .get(&preset_id)
            .is_some_and(|ignored| ignored.eq_ignore_ascii_case(key_name))
        {
            return false;
        }
        if let Some(pressed) = hook_state
            .pressed_inputs
            .iter()
            .find(|pressed| pressed.eq_ignore_ascii_case(key_name))
            .cloned()
        {
            hook_state.pressed_inputs.remove(&pressed);
            return true;
        }
        hook_state
            .held_inputs
            .iter()
            .any(|held| held.eq_ignore_ascii_case(key_name))
    }

    fn is_repeat_key(key_name: &str) -> bool {
        HOOK_STATE.lock().held_inputs.contains(key_name)
    }

    fn is_mouse_locked() -> bool {
        HOOK_STATE.lock().locked_mouse_count > 0
    }

    fn is_vision_capture_mouse_blocked() -> bool {
        HOOK_STATE.lock().vision_capture_mouse_blocked
    }

    fn physical_mouse_buttons_down() -> bool {
        unsafe {
            [0x01, 0x02, 0x04, 0x05, 0x06]
                .into_iter()
                .any(|vk| GetAsyncKeyState(vk) < 0)
        }
    }

    fn clear_stuck_mouse_lock() {
        if physical_mouse_buttons_down() {
            return;
        }
        let mut hook_state = HOOK_STATE.lock();
        if hook_state.locked_mouse_count == 0 {
            return;
        }
        hook_state.locked_mouse_count = 0;
        for active in hook_state.active_hold_macros.values_mut() {
            active.locked_mouse_count = 0;
        }
    }

    fn is_keyboard_arrow_mouse_key(key_name: &str) -> bool {
        matches!(key_name, "Left" | "Right" | "Up" | "Down")
    }

    fn keyboard_arrow_mouse_delta() -> Option<(i32, i32)> {
        let hook_state = HOOK_STATE.lock();
        if !hook_state.keyboard_arrow_mouse_enabled {
            return None;
        }
        let step = hook_state.keyboard_arrow_mouse_step_px as i32;
        let mut dx = 0i32;
        let mut dy = 0i32;
        if hook_state.held_inputs.contains("Left") {
            dx -= step;
        }
        if hook_state.held_inputs.contains("Right") {
            dx += step;
        }
        if hook_state.held_inputs.contains("Up") {
            dy -= step;
        }
        if hook_state.held_inputs.contains("Down") {
            dy += step;
        }
        if dx == 0 && dy == 0 {
            None
        } else {
            Some((dx, dy))
        }
    }

    fn keyboard_arrow_mouse_should_swallow(key_name: &str) -> bool {
        let hook_state = HOOK_STATE.lock();
        hook_state.keyboard_arrow_mouse_enabled && is_keyboard_arrow_mouse_key(key_name)
    }

    fn keyboard_arrow_mouse_is_active() -> bool {
        let hook_state = HOOK_STATE.lock();
        hook_state.keyboard_arrow_mouse_enabled
            && hook_state
                .held_inputs
                .iter()
                .any(|key_name| is_keyboard_arrow_mouse_key(key_name))
    }

    fn apply_keyboard_arrow_mouse_movement() {
        if let Some((dx, dy)) = keyboard_arrow_mouse_delta() {
            let _ = send_mouse_move_relative(dx, dy);
        }
    }

    unsafe fn runtime_mut(hwnd: HWND) -> Option<&'static mut Runtime> {
        let ptr = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(GWLP_USERDATA.0));
        if ptr == 0 {
            None
        } else {
            Some(&mut *(ptr as *mut Runtime))
        }
    }

    unsafe fn process_pending_commands(hwnd: HWND, runtime: &mut Runtime) {
        while let Ok(command) = runtime.rx.try_recv() {
            match command {
                OverlayCommand::Update(style) => {
                    runtime.style = style.clone();
                    HOOK_STATE.lock().current_style = style;
                    let _ = refresh_overlay(runtime);
                }
                OverlayCommand::UpdateProfiles(profiles) => {
                    HOOK_STATE.lock().profiles = profiles;
                    let _ = refresh_overlay(runtime);
                }
                OverlayCommand::UpdateCrosshairProfile { index, profile } => {
                    let mut hook_state = HOOK_STATE.lock();
                    if let Some(existing) = hook_state.profiles.get_mut(index) {
                        *existing = profile;
                    } else {
                        hook_state.profiles.push(profile);
                    }
                    drop(hook_state);
                    let _ = refresh_overlay(runtime);
                }
                OverlayCommand::UpdateWindowPresets(presets) => {
                    runtime.window_presets = presets;
                    let _ = sync_window_hotkeys(hwnd, runtime);
                }
                OverlayCommand::UpdateWindowFocusPresets(presets) => {
                    runtime.window_focus_presets = presets;
                    let _ = sync_window_hotkeys(hwnd, runtime);
                }
                OverlayCommand::UpdateWindowExpandControls(controls) => {
                    HOOK_STATE.lock().window_expand_controls = controls;
                }
                OverlayCommand::UpdatePinPresets(presets) => {
                    let mut hook_state = HOOK_STATE.lock();
                    hook_state.pin_presets = presets.clone();
                    runtime.pin_presets = presets;
                    if let Some(active_id) = hook_state.active_pin_preset_id
                        && !hook_state
                            .pin_presets
                            .iter()
                            .any(|preset| preset.id == active_id)
                    {
                        hook_state.active_pin_preset_id = None;
                    }
                }
                OverlayCommand::UpdateMousePathPresets(presets) => {
                    HOOK_STATE.lock().mouse_path_presets = presets.clone();
                    runtime.mouse_path_presets = presets;
                }
                OverlayCommand::UpdateMouseSensitivityPresets(presets) => {
                    let mut hook_state = HOOK_STATE.lock();
                    hook_state.mouse_sensitivity_presets = presets.clone();
                    if let Some(active_id) = hook_state.active_mouse_sensitivity_preset_id
                        && !hook_state
                            .mouse_sensitivity_presets
                            .iter()
                            .any(|preset| preset.id == active_id)
                    {
                        hook_state.active_mouse_sensitivity_preset_id = None;
                        hook_state.mouse_sensitivity_restore_speed = None;
                    }
                }
                OverlayCommand::UpdateMouseSensitivitySettings {
                    restore_on_exit,
                    restore_speed,
                } => {
                    let mut hook_state = HOOK_STATE.lock();
                    hook_state.mouse_sensitivity_restore_on_exit = restore_on_exit;
                    hook_state.mouse_sensitivity_exit_restore_speed = restore_speed.clamp(1, 20);
                }
                
                OverlayCommand::UpdateKeyboardArrowMouseSettings { enabled, step_px } => {
                    let mut hook_state = HOOK_STATE.lock();
                    hook_state.keyboard_arrow_mouse_enabled = enabled;
                    hook_state.keyboard_arrow_mouse_step_px = step_px.clamp(1, 100) as u32;
                }
                OverlayCommand::UpdateVisionPresets(presets) => {
                    {
                        let mut hook_state = HOOK_STATE.lock();
                        hook_state.vision_presets = presets;
                        let valid_ids: HashSet<u32> = hook_state
                            .vision_presets
                            .iter()
                            .map(|preset| preset.id)
                            .collect();
                        hook_state
                            .vision_following_presets
                            .retain(|preset_id| valid_ids.contains(preset_id));
                    }
                    let _ = refresh_search_area_overlay(runtime);
                }
                OverlayCommand::InvalidateVisionWaits(preset_ids) => {
                    let mut guard = IMAGE_SEARCH_WAIT_GENERATIONS.lock();
                    for preset_id in preset_ids {
                        let generation = guard.entry(preset_id).or_insert(0);
                        *generation = generation.saturating_add(1);
                    }
                }
                OverlayCommand::ApplyMouseSensitivityPreset(preset_id) => {
                    if let Some(preset) = HOOK_STATE
                        .lock()
                        .mouse_sensitivity_presets
                        .iter()
                        .find(|preset| preset.id == preset_id)
                        .cloned()
                    {
                        let _ = apply_mouse_sensitivity_preset(&preset);
                    }
                }
                OverlayCommand::RestoreMouseSensitivity => {
                    let _ = restore_mouse_sensitivity();
                }
                OverlayCommand::UpdateHudPresets(presets) => {
                    HOOK_STATE.lock().hud_presets = presets;
                }
                OverlayCommand::UpdateCommandPresets(presets) => {
                    HOOK_STATE.lock().command_presets = presets;
                }
                OverlayCommand::PreviewHudPreset(preset) => {
                    *HUD_PREVIEW_DISPLAY.lock() =
                        preset.map(toolbox_preview_display_from_preset);
                    let _ = refresh_hud(runtime);
                }
                OverlayCommand::UpdateMacroPresets(presets) => {
                    runtime.macro_groups = presets;
                    let _ = sync_macro_hotkeys(hwnd, runtime);
                }
                OverlayCommand::UpdateAudioSettings(settings) => {
                    let mut hook_state = HOOK_STATE.lock();
                    hook_state.sound_presets = settings.presets.clone();
                    runtime.audio_settings = settings;
                }
                OverlayCommand::SetMacrosMasterEnabled(enabled) => {
                    let mut hook_state = HOOK_STATE.lock();
                    hook_state.macros_master_enabled = enabled;
                    if !enabled {
                        hook_state.locked_inputs.clear();
                        hook_state.press_trigger_suppression.clear();
                        hook_state.active_hold_macros.clear();
                    }
                    drop(hook_state);
                    let _ = update_tray_icon(hwnd, enabled);
                }
                OverlayCommand::SetTrayIconVisible(visible) => {
                    if visible {
                        let _ = add_tray_icon(hwnd);
                    } else {
                        let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &notify_icon(hwnd)) };
                    }
                }
                OverlayCommand::SetVietnameseInputEnabled(enabled) => {
                    HOOK_STATE.lock().vietnamese_input_enabled = enabled;
                }
                OverlayCommand::UpdateMacrosMasterHotkey(binding) => {
                    HOOK_STATE.lock().macros_master_hotkey = binding;
                }
                OverlayCommand::RefreshPinOverlay => {
                    let _ = refresh_pin_overlay(runtime);
                }
                OverlayCommand::SetVisionCaptureMouseBlocked(blocked) => {
                    let mut hook_state = HOOK_STATE.lock();
                    hook_state.vision_capture_mouse_blocked = blocked;
                    if !blocked {
                        hook_state.vision_capture_anchor = None;
                        hook_state.vision_capture_preview_region = None;
                    }
                }
                OverlayCommand::SetUiVisible(visible) => {
                    runtime.ui_visible = visible;
                    if visible {
                        cancel_pending_tray_toggle();
                        let _ = set_input_hooks_enabled(runtime, desired_hooks_enabled(runtime));
                        let _ = ShowWindow(runtime.pin_hwnd, SW_HIDE);
                        let _ = ShowWindow(runtime.hud_hwnd, SW_HIDE);
                        let _ = ShowWindow(runtime.mouse_trail_hwnd, SW_HIDE);
                    } else {
                        *HUD_PREVIEW_DISPLAY.lock() = None;
                        let _ = set_input_hooks_enabled(runtime, desired_hooks_enabled(runtime));
                        let _ = refresh_overlay(runtime);
                        let _ = refresh_pin_overlay(runtime);
                        let _ = refresh_hud(runtime);
                    }
                }
                OverlayCommand::ToggleMacroRecording(group_id, preset_id, preset_name) => {
                    toggle_macro_recording(group_id, preset_id, preset_name);
                }
                OverlayCommand::Exit => {
                    let _ = runtime.ui_tx.send(UiCommand::Exit);
                    let _ = shutdown_application(hwnd, runtime);
                }
            }
        }
    }

    unsafe fn mark_ui_visible(runtime: &mut Runtime, visible: bool) {
        runtime.ui_visible = visible;
        let _ = set_input_hooks_enabled(runtime, desired_hooks_enabled(runtime));
        if visible {
            let _ = ShowWindow(runtime.pin_hwnd, SW_HIDE);
            let _ = ShowWindow(runtime.hud_hwnd, SW_HIDE);
            let _ = ShowWindow(runtime.mouse_trail_hwnd, SW_HIDE);
        }
    }

    unsafe fn refresh_overlay(runtime: &mut Runtime) -> Result<()> {
        let visible_profiles = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .profiles
                .iter()
                .filter(|profile| profile.enabled)
                .cloned()
                .collect::<Vec<_>>()
        };

        if visible_profiles.is_empty() {
            let _ = ShowWindow(runtime.overlay_hwnd, SW_HIDE);
            return Ok(());
        }

        let screen_width = GetSystemMetrics(SM_CXSCREEN).max(1) as u32;
        let screen_height = GetSystemMetrics(SM_CYSCREEN).max(1) as u32;
        let mut canvas =
            RgbaImage::from_pixel(screen_width, screen_height, image::Rgba([0, 0, 0, 0]));

        for profile in visible_profiles {
            let custom_path = profile
                .style
                .custom_asset
                .as_ref()
                .map(|name| runtime.paths.asset_path(name));
            let rendered = render_crosshair(&profile.style, custom_path.as_deref())?;
            let layer = RgbaImage::from_raw(rendered.width, rendered.height, rendered.rgba)
                .context("Failed to build crosshair layer")?;
            let left = (profile.style.x_offset - rendered.center_x) as i64;
            let top = (profile.style.y_offset - rendered.center_y) as i64;
            image::imageops::overlay(&mut canvas, &layer, left, top);
        }

        paint_crosshair_canvas(runtime.overlay_hwnd, canvas)?;
        let _ = ShowWindow(runtime.overlay_hwnd, SW_SHOWNA);
        Ok(())
    }

    unsafe fn paint_crosshair_canvas(hwnd: HWND, canvas: RgbaImage) -> Result<()> {
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);

        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            width as i32,
            height as i32,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );

        let screen_dc = GetDC(None);
        if screen_dc.0.is_null() {
            bail!("Failed to acquire the screen DC");
        }
        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        if mem_dc.0.is_null() {
            let _ = ReleaseDC(None, screen_dc);
            bail!("Failed to create a memory DC");
        }

        let mut bitmap_info = BITMAPINFO::default();
        bitmap_info.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };

        let mut bits: *mut c_void = null_mut();
        let bitmap = CreateDIBSection(
            Some(screen_dc),
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        )
        .context("Failed to create a DIB section")?;
        if bits.is_null() {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(None, screen_dc);
            bail!("Failed to map the DIB section");
        }

        let _previous = SelectObject(mem_dc, HGDIOBJ(bitmap.0));
        std::ptr::copy_nonoverlapping(
            canvas.as_raw().as_ptr(),
            bits as *mut u8,
            canvas.as_raw().len(),
        );

        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let _ = UpdateLayeredWindow(
            hwnd,
            Some(screen_dc),
            None,
            Some(&SIZE {
                cx: width as i32,
                cy: height as i32,
            }),
            Some(mem_dc),
            Some(&POINT { x: 0, y: 0 }),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(None, screen_dc);
        Ok(())
    }

    fn refresh_hud(runtime: &mut Runtime) -> Result<()> {
        let display = {
            let mut preview_guard = HUD_PREVIEW_DISPLAY.lock();
            if let Some(active) = preview_guard.as_ref()
                && let Some(expires_at) = active.expires_at
                && Instant::now() >= expires_at
            {
                *preview_guard = None;
            }
            if let Some(preview) = preview_guard.clone() {
                Some(preview)
            } else {
                let mut guard = HUD_DISPLAY.lock();
                if let Some(active) = guard.as_ref()
                    && let Some(expires_at) = active.expires_at
                    && Instant::now() >= expires_at
                {
                    *guard = None;
                }
                guard.clone()
            }
        };
        if runtime.hud_display == display {
            return Ok(());
        }
        runtime.hud_display = display.clone();

        let Some(display) = display else {
            let _ = unsafe { ShowWindow(runtime.hud_hwnd, SW_HIDE) };
            return Ok(());
        };

        unsafe { paint_hud(runtime.hud_hwnd, &display) }
    }

    fn refresh_mouse_record_trail(runtime: &mut Runtime) -> Result<()> {
        let points = {
            let mut guard = MOUSE_RECORDING.lock();
            let Some(session) = guard.as_mut() else {
                unsafe {
                    let _ = ShowWindow(runtime.mouse_trail_hwnd, SW_HIDE);
                }
                return Ok(());
            };
            if !session.dirty {
                return Ok(());
            }
            session.dirty = false;
            session
                .events
                .iter()
                .filter(|event| matches!(event.kind, MousePathEventKind::Move))
                .map(|event| POINT {
                    x: event.x,
                    y: event.y,
                })
                .collect::<Vec<_>>()
        };

        if points.len() < 2 {
            unsafe {
                let _ = ShowWindow(runtime.mouse_trail_hwnd, SW_HIDE);
            }
            return Ok(());
        }

        unsafe { paint_mouse_trail(runtime.mouse_trail_hwnd, &points) }
    }

    fn refresh_search_area_overlay(runtime: &mut Runtime) -> Result<()> {
        let (regions, preview_region) = {
            let hook_state = HOOK_STATE.lock();
            let mut regions = hook_state
                .vision_presets
                .iter()
                .filter(|preset| preset.show_search_region_overlay)
                .filter_map(|preset| configured_image_search_region(preset))
                .collect::<Vec<_>>();
            (regions, hook_state.vision_capture_preview_region)
        };

        if regions.is_empty() && preview_region.is_none() {
            unsafe {
                let _ = ShowWindow(runtime.search_area_hwnd, SW_HIDE);
            }
            return Ok(());
        }

        unsafe { paint_search_area_overlay(runtime.search_area_hwnd, &regions, preview_region) }
    }

    fn desired_timer_interval_ms(runtime: &Runtime) -> u32 {
        let capture_active = {
            let hook_state = HOOK_STATE.lock();
            hook_state.vision_capture_preview_region.is_some()
                || hook_state.vision_capture_mouse_blocked
        };
        if capture_active {
            return 16;
        }

        if is_ui_in_foreground() {
            return 100;
        }

        let recording_active = MOUSE_RECORDING.lock().is_some() || MACRO_RECORDING.lock().is_some();
        if recording_active {
            return 33;
        }

        let toolbox_active = HUD_DISPLAY.lock().is_some()
            || HUD_PREVIEW_DISPLAY.lock().is_some()
            || runtime.hud_display.is_some();
        if toolbox_active {
            return 100;
        }

        let pin_active = runtime.active_pin_thumbnail.is_some()
            || HOOK_STATE.lock().active_pin_preset_id.is_some();
        if pin_active {
            return 33;
        }

        if keyboard_arrow_mouse_is_active() {
            return 12;
        }

        if HOOK_STATE.lock().keyboard_arrow_mouse_enabled {
            return 33;
        }

        750
    }

    fn desired_hooks_enabled(_runtime: &Runtime) -> bool {
        true
    }

    unsafe fn set_input_hooks_enabled(runtime: &mut Runtime, enabled: bool) -> Result<()> {
        let instance = GetModuleHandleW(None)?;
        if enabled {
            if runtime.keyboard_hook.0.is_null() {
                runtime.keyboard_hook = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(low_level_keyboard_proc),
                    Some(instance.into()),
                    0,
                )?;
            }
            if runtime.mouse_hook.0.is_null() {
                runtime.mouse_hook = SetWindowsHookExW(
                    WH_MOUSE_LL,
                    Some(low_level_mouse_proc),
                    Some(instance.into()),
                    0,
                )?;
            }
        } else {
            if !runtime.keyboard_hook.0.is_null() {
                let _ = UnhookWindowsHookEx(runtime.keyboard_hook);
                runtime.keyboard_hook = HHOOK::default();
            }
            if !runtime.mouse_hook.0.is_null() {
                let _ = UnhookWindowsHookEx(runtime.mouse_hook);
                runtime.mouse_hook = HHOOK::default();
            }
        }
        Ok(())
    }

    unsafe fn refresh_overlay_timer(hwnd: HWND, runtime: &mut Runtime) {
        let desired = desired_timer_interval_ms(runtime);
        if desired != runtime.timer_interval_ms {
            let _ = SetTimer(Some(hwnd), TIMER_ID, desired, None);
            runtime.timer_interval_ms = desired;
        }
    }

    fn refresh_pin_overlay(runtime: &mut Runtime) -> Result<()> {
        let active = {
            let hook_state = HOOK_STATE.lock();
            hook_state.active_pin_preset_id.and_then(|id| {
                hook_state
                    .pin_presets
                    .iter()
                    .find(|preset| preset.id == id)
                    .cloned()
            })
        };

        let Some(preset) = active else {
            unsafe {
                if let Some(active) = runtime.active_pin_thumbnail.take()
                    && let Some(thumbnail_id) = active.thumbnail_id
                {
                    let _ = DwmUnregisterThumbnail(thumbnail_id);
                }
                let _ = ShowWindow(runtime.pin_hwnd, SW_HIDE);
            }
            runtime.last_pin_update = Instant::now();
            return Ok(());
        };

        if runtime.active_pin_thumbnail.is_some()
            && runtime.last_pin_update.elapsed() < Duration::from_millis(33)
        {
            return Ok(());
        }

        let source = find_target_window_hwnd(
            preset.target_window_title.as_deref(),
            &preset.extra_target_window_titles,
            preset.match_duplicate_window_titles,
            false,
        )
        .context("Pin source window was not found")?;

        unsafe {
            let source_root = GetAncestor(source, GA_ROOT);
            if !source_root.0.is_null()
                && window_belongs_to_current_process(source_root)
                && !is_internal_app_window(source_root)
            {
                let _ = ShowWindow(runtime.pin_hwnd, SW_HIDE);
                runtime.last_pin_update = Instant::now();
                return Ok(());
            }

            let mut source_rect = RECT::default();
            GetWindowRect(source, &mut source_rect)?;

            let base_bounds = if preset.use_custom_bounds {
                (
                    preset.x,
                    preset.y,
                    preset.width.max(1),
                    preset.height.max(1),
                )
            } else {
                (
                    source_rect.left,
                    source_rect.top,
                    (source_rect.right - source_rect.left).max(1),
                    (source_rect.bottom - source_rect.top).max(1),
                )
            };

            let target_bounds = base_bounds;

            let source_width = (source_rect.right - source_rect.left).max(1);
            let source_height = (source_rect.bottom - source_rect.top).max(1);
            let source_crop_key = if preset.use_source_crop {
                let crop_x = preset.source_x.clamp(0, source_width.saturating_sub(1));
                let crop_y = preset.source_y.clamp(0, source_height.saturating_sub(1));
                let crop_w = preset
                    .source_width
                    .max(1)
                    .min(source_width.saturating_sub(crop_x).max(1));
                let crop_h = preset
                    .source_height
                    .max(1)
                    .min(source_height.saturating_sub(crop_y).max(1));
                Some((crop_x, crop_y, crop_w, crop_h))
            } else {
                None
            };

            let needs_register = runtime.active_pin_thumbnail.as_ref().is_none_or(|active| {
                active.preset_id != preset.id
                    || active.source_hwnd != source
                    || active.thumbnail_id.is_none()
            });
            if needs_register {
                if let Some(active) = runtime.active_pin_thumbnail.take()
                    && let Some(thumbnail_id) = active.thumbnail_id
                {
                    let _ = DwmUnregisterThumbnail(thumbnail_id);
                }
                let thumbnail_id = DwmRegisterThumbnail(runtime.pin_hwnd, source)?;
                runtime.active_pin_thumbnail = Some(ActivePinThumbnail {
                    preset_id: preset.id,
                    source_hwnd: source,
                    thumbnail_id: Some(thumbnail_id),
                    overlay_style: preset.overlay_style,
                    last_target_bounds: (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
                    last_source_crop: None,
                });
            }

            if let Some(active) = runtime.active_pin_thumbnail.as_ref() {
                let mut source_flags = DWM_TNP_SOURCECLIENTAREAONLY;
                let mut source_rect_crop = RECT::default();
                if let Some((crop_x, crop_y, crop_w, crop_h)) = source_crop_key {
                    source_rect_crop = RECT {
                        left: crop_x,
                        top: crop_y,
                        right: crop_x + crop_w,
                        bottom: crop_y + crop_h,
                    };
                    source_flags |= DWM_TNP_RECTSOURCE;
                }
                let needs_apply = active.last_target_bounds != target_bounds
                    || active.last_source_crop != source_crop_key
                    || active.overlay_style != preset.overlay_style;
                if needs_apply {
                    let _ = SetWindowPos(
                        runtime.pin_hwnd,
                        Some(HWND_TOPMOST),
                        target_bounds.0,
                        target_bounds.1,
                        target_bounds.2,
                        target_bounds.3,
                        SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    );
                    let properties = DWM_THUMBNAIL_PROPERTIES {
                        dwFlags: DWM_TNP_RECTDESTINATION
                            | DWM_TNP_VISIBLE
                            | DWM_TNP_OPACITY
                            | source_flags,
                        rcDestination: RECT {
                            left: 0,
                            top: 0,
                            right: target_bounds.2,
                            bottom: target_bounds.3,
                        },
                        rcSource: source_rect_crop,
                        opacity: 255,
                        fVisible: true.into(),
                        fSourceClientAreaOnly: false.into(),
                        ..Default::default()
                    };
                    if let Some(thumbnail_id) = active.thumbnail_id {
                        let _ = DwmUpdateThumbnailProperties(thumbnail_id, &properties);
                    }

                    let region = CreateRectRgn(0, 0, target_bounds.2, target_bounds.3);
                    if region.0.is_null() {
                        return Err(anyhow::anyhow!("Failed to create pin window region"));
                    }
                    if SetWindowRgn(runtime.pin_hwnd, Some(region), true) == 0 {
                        let _ = DeleteObject(HGDIOBJ(region.0));
                        return Err(anyhow::anyhow!("Failed to apply pin window region"));
                    }

                    if let Some(active_mut) = runtime.active_pin_thumbnail.as_mut() {
                        active_mut.last_target_bounds = target_bounds;
                        active_mut.last_source_crop = source_crop_key;
                        active_mut.overlay_style = preset.overlay_style;
                    }
                }
                let _ = ShowWindow(runtime.pin_hwnd, SW_SHOWNA);
            }
        }

        runtime.last_pin_update = Instant::now();
        Ok(())
    }

    fn pin_overlay_shape_rect(
        style: PinOverlayStyle,
        target_w: i32,
        target_h: i32,
    ) -> (i32, i32, i32, i32) {
        let target_w = target_w.max(1);
        let target_h = target_h.max(1);
        match style {
            PinOverlayStyle::Rectangle => (0, 0, target_w, target_h),
            PinOverlayStyle::Circle => {
                let padding = ((target_w.min(target_h) as f32 * 0.04).round() as i32).max(4);
                let size = (target_w.min(target_h) - padding * 2).max(1);
                ((target_w - size) / 2, (target_h - size) / 2, size, size)
            }
            PinOverlayStyle::HorizontalBar => {
                let width = target_w.max(1);
                let min_height = ((target_h as f32 * 0.12).round() as i32).clamp(18, target_h);
                let bar_height =
                    ((target_h as f32 * 0.24).round() as i32).clamp(min_height, target_h.max(1));
                (
                    (target_w - width) / 2,
                    (target_h - bar_height) / 2,
                    width,
                    bar_height,
                )
            }
        }
    }

    fn point_in_rounded_rect(
        x: i32,
        y: i32,
        left: i32,
        top: i32,
        width: i32,
        height: i32,
        radius: f32,
    ) -> bool {
        if width <= 0 || height <= 0 {
            return false;
        }
        let radius = radius
            .max(0.0)
            .min(width as f32 * 0.5)
            .min(height as f32 * 0.5);
        if radius <= 0.0 {
            return x >= left && x < left + width && y >= top && y < top + height;
        }

        let px = x as f32 + 0.5;
        let py = y as f32 + 0.5;
        let inner_left = left as f32 + radius;
        let inner_right = left as f32 + width as f32 - radius;
        let inner_top = top as f32 + radius;
        let inner_bottom = top as f32 + height as f32 - radius;
        if (px >= inner_left && px <= inner_right) || (py >= inner_top && py <= inner_bottom) {
            return true;
        }

        let corner_x = if px < inner_left {
            inner_left
        } else {
            inner_right
        };
        let corner_y = if py < inner_top {
            inner_top
        } else {
            inner_bottom
        };
        let dx = px - corner_x;
        let dy = py - corner_y;
        (dx * dx) + (dy * dy) <= radius * radius
    }

    fn render_pin_overlay_bitmap(
        capture: &window_list::ScreenCaptureFrame,
        target_w: i32,
        target_h: i32,
        style: PinOverlayStyle,
        source_crop: Option<(i32, i32, i32, i32)>,
    ) -> Result<Vec<u8>> {
        let target_w = target_w.max(1);
        let target_h = target_h.max(1);
        let source = RgbaImage::from_raw(
            capture.width as u32,
            capture.height as u32,
            capture.rgba.clone(),
        )
        .context("Failed to decode pin capture")?;
        let source = if let Some((crop_x, crop_y, crop_w, crop_h)) = source_crop {
            image::imageops::crop_imm(
                &source,
                crop_x.max(0) as u32,
                crop_y.max(0) as u32,
                crop_w.max(1) as u32,
                crop_h.max(1) as u32,
            )
            .to_image()
        } else {
            source
        };

        let (shape_left, shape_top, shape_w, shape_h) =
            pin_overlay_shape_rect(style, target_w, target_h);
        let mut output = vec![0u8; (target_w as usize) * (target_h as usize) * 4];

        let source_w = source.width().max(1);
        let source_h = source.height().max(1);
        let scale = (shape_w.max(1) as f32 / source_w as f32)
            .min(shape_h.max(1) as f32 / source_h as f32)
            .max(0.01);
        let fit_w = (source_w as f32 * scale).round().max(1.0) as u32;
        let fit_h = (source_h as f32 * scale).round().max(1.0) as u32;
        let fit_x = shape_left + ((shape_w - fit_w as i32) / 2).max(0);
        let fit_y = shape_top + ((shape_h - fit_h as i32) / 2).max(0);
        let resized = image::imageops::resize(&source, fit_w, fit_h, FilterType::CatmullRom);
        let resized_pixels = resized.as_raw();

        for y in 0..fit_h as i32 {
            for x in 0..fit_w as i32 {
                let dst_x = fit_x + x;
                let dst_y = fit_y + y;
                if dst_x < 0 || dst_y < 0 || dst_x >= target_w || dst_y >= target_h {
                    continue;
                }
                let inside = match style {
                    PinOverlayStyle::Rectangle => true,
                    PinOverlayStyle::Circle => {
                        point_in_ellipse(dst_x, dst_y, shape_left, shape_top, shape_w, shape_h)
                    }
                    PinOverlayStyle::HorizontalBar => point_in_rounded_rect(
                        dst_x,
                        dst_y,
                        shape_left,
                        shape_top,
                        shape_w,
                        shape_h,
                        shape_h as f32 * 0.5,
                    ),
                };
                if !inside {
                    continue;
                }
                let src_index = ((y as usize) * (fit_w as usize) + x as usize) * 4;
                let dst_index = ((dst_y as usize) * (target_w as usize) + dst_x as usize) * 4;
                output[dst_index..dst_index + 4]
                    .copy_from_slice(&resized_pixels[src_index..src_index + 4]);
            }
        }

        Ok(output)
    }

    unsafe fn paint_pin_overlay(
        hwnd: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        rgba: &[u8],
    ) -> Result<()> {
        let width = width.max(1);
        let height = height.max(1);

        let screen_dc = GetDC(None);
        if screen_dc.0.is_null() {
            bail!("Failed to acquire the screen DC");
        }
        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        if mem_dc.0.is_null() {
            let _ = ReleaseDC(None, screen_dc);
            bail!("Failed to create a memory DC");
        }

        let mut bitmap_info = BITMAPINFO::default();
        bitmap_info.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };

        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(
            Some(mem_dc),
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        )?;
        if bitmap.0.is_null() || bits.is_null() {
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(None, screen_dc);
            bail!("Failed to create pin DIB");
        }

        let old_bitmap = SelectObject(mem_dc, HGDIOBJ(bitmap.0));
        let bgra = rgba_to_bgra(rgba);
        std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, bgra.len());

        let destination = POINT { x, y };
        let source = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: width,
            cy: height,
        };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        let _ = UpdateLayeredWindow(
            hwnd,
            Some(screen_dc),
            Some(&destination),
            Some(&size),
            Some(mem_dc),
            Some(&source),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        let _ = SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(None, screen_dc);
        let _ = ShowWindow(hwnd, SW_SHOWNA);
        Ok(())
    }

    unsafe fn paint_hud(hwnd: HWND, display: &HudDisplayState) -> Result<()> {
        let window_x = display.x.max(0);
        let window_y = display.y.max(0);
        let width = display.width.max(1);
        let height = display.height.max(1);

        let screen_dc = GetDC(None);
        let mem_dc = CreateCompatibleDC(Some(screen_dc));

        let bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits_ptr: *mut c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(
            Some(mem_dc),
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bits_ptr,
            None,
            0,
        )?;
        let old_bitmap = SelectObject(mem_dc, HGDIOBJ(bitmap.0));

        let bg_alpha = (display.background_opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        let bytes_len = (width as usize) * (height as usize) * 4;
        let pixels = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, bytes_len);
        let radius = if display.rounded_background {
            16.0
        } else {
            0.0
        };
        let bg_b = ((display.background_color.b as u32 * bg_alpha as u32) / 255) as u8;
        let bg_g = ((display.background_color.g as u32 * bg_alpha as u32) / 255) as u8;
        let bg_r = ((display.background_color.r as u32 * bg_alpha as u32) / 255) as u8;
        for py in 0..height {
            for px in 0..width {
                let index = ((py as usize) * (width as usize) + (px as usize)) * 4;
                let inside = if radius <= 0.0 {
                    true
                } else {
                    let px_f = px as f32 + 0.5;
                    let py_f = py as f32 + 0.5;
                    let inner_left = radius;
                    let inner_right = width as f32 - radius;
                    let inner_top = radius;
                    let inner_bottom = height as f32 - radius;
                    if (px_f >= inner_left && px_f <= inner_right)
                        || (py_f >= inner_top && py_f <= inner_bottom)
                    {
                        true
                    } else {
                        let corner_x = if px_f < inner_left {
                            inner_left
                        } else {
                            inner_right
                        };
                        let corner_y = if py_f < inner_top {
                            inner_top
                        } else {
                            inner_bottom
                        };
                        let dx = px_f - corner_x;
                        let dy = py_f - corner_y;
                        (dx * dx) + (dy * dy) <= radius * radius
                    }
                };
                if inside && bg_alpha > 0 {
                    pixels[index] = bg_b;
                    pixels[index + 1] = bg_g;
                    pixels[index + 2] = bg_r;
                    pixels[index + 3] = bg_alpha;
                } else {
                    pixels[index] = 0;
                    pixels[index + 1] = 0;
                    pixels[index + 2] = 0;
                    pixels[index + 3] = 0;
                }
            }
        }

        let font_name = "Segoe UI"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let font = CreateFontW(
            -(display.font_size.round() as i32).max(1),
            0,
            0,
            0,
            FW_MEDIUM.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            ANTIALIASED_QUALITY,
            FF_DONTCARE.0 as u32,
            PCWSTR(font_name.as_ptr()),
        );
        let old_font = SelectObject(mem_dc, HGDIOBJ(font.0));
        let _ = SetBkMode(mem_dc, TRANSPARENT);
        let _ = SetTextColor(
            mem_dc,
            COLORREF(
                ((display.text_color.b as u32) << 16)
                    | ((display.text_color.g as u32) << 8)
                    | (display.text_color.r as u32),
            ),
        );
        let mut text_rect = RECT {
            left: 12,
            top: 4,
            right: width - 12,
            bottom: height - 4,
        };
        let mut wide = display
            .text
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let _ = DrawTextW(
            mem_dc,
            &mut wide,
            &mut text_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );

        let text_alpha = display.text_color.a.max(1);
        for chunk in pixels.chunks_exact_mut(4) {
            let looks_like_bg =
                chunk[0] == bg_b && chunk[1] == bg_g && chunk[2] == bg_r && chunk[3] == bg_alpha;
            let alpha = if looks_like_bg {
                bg_alpha
            } else if chunk[0] == 0 && chunk[1] == 0 && chunk[2] == 0 && chunk[3] == 0 {
                0
            } else {
                text_alpha
            };
            chunk[3] = alpha;
            chunk[0] = ((chunk[0] as u32 * alpha as u32) / 255) as u8;
            chunk[1] = ((chunk[1] as u32 * alpha as u32) / 255) as u8;
            chunk[2] = ((chunk[2] as u32 * alpha as u32) / 255) as u8;
        }

        let size = SIZE {
            cx: width,
            cy: height,
        };
        let src_pt = POINT { x: 0, y: 0 };
        let pos = POINT {
            x: window_x,
            y: window_y,
        };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let _ = UpdateLayeredWindow(
            hwnd,
            Some(screen_dc),
            Some(&pos),
            Some(&size),
            Some(mem_dc),
            Some(&src_pt),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        let _ = SelectObject(mem_dc, old_font);
        let _ = DeleteObject(HGDIOBJ(font.0));
        let _ = SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(None, screen_dc);
        let _ = ShowWindow(hwnd, SW_SHOWNA);
        Ok(())
    }

    fn sync_window_hotkeys(hwnd: HWND, runtime: &mut Runtime) -> Result<()> {
        for hotkey_id in runtime
            .registered_window_hotkeys
            .keys()
            .copied()
            .collect::<Vec<_>>()
        {
            let _ = unsafe { UnregisterHotKey(Some(hwnd), hotkey_id) };
        }
        runtime.registered_window_hotkeys.clear();
        let mut hook_state = HOOK_STATE.lock();
        hook_state.window_presets = runtime.window_presets.clone();
        hook_state.window_focus_presets = runtime.window_focus_presets.clone();
        hook_state.pin_presets = runtime.pin_presets.clone();
        Ok(())
    }

    fn sync_macro_hotkeys(hwnd: HWND, runtime: &mut Runtime) -> Result<()> {
        for hotkey_id in runtime
            .registered_macro_hotkeys
            .keys()
            .copied()
            .collect::<Vec<_>>()
        {
            let _ = unsafe { UnregisterHotKey(Some(hwnd), hotkey_id) };
        }
        runtime.registered_macro_hotkeys.clear();
        HOOK_STATE.lock().macro_groups = runtime.macro_groups.clone();
        Ok(())
    }

    fn unregister_all_hotkeys(hwnd: HWND, runtime: Option<&mut Runtime>) {
        let Some(runtime) = runtime else {
            return;
        };
        let _ = unsafe { UnregisterHotKey(Some(hwnd), HOTKEY_ID) };
        for hotkey_id in runtime
            .registered_window_hotkeys
            .keys()
            .copied()
            .collect::<Vec<_>>()
        {
            let _ = unsafe { UnregisterHotKey(Some(hwnd), hotkey_id) };
        }
        for hotkey_id in runtime
            .registered_macro_hotkeys
            .keys()
            .copied()
            .collect::<Vec<_>>()
        {
            let _ = unsafe { UnregisterHotKey(Some(hwnd), hotkey_id) };
        }
    }

    fn play_macro_preset(
        hotkey_id: i32,
        preset: MacroPreset,
        target_window_title: Option<String>,
        extra_target_window_titles: Vec<String>,
        match_duplicate_window_titles: bool,
        trigger_key: String,
    ) -> Result<()> {
        SUPPRESSED_MACRO_HOTKEYS.lock().insert(hotkey_id);
        STOP_REQUESTED_MACRO_PRESETS.lock().remove(&preset.id);
        let trigger_key_for_cleanup = trigger_key.clone();
        HOOK_STATE
            .lock()
            .stop_ignore_keys
            .insert(preset.id, trigger_key);
        increment_press_trigger_suppression(&trigger_key_for_cleanup);
        thread::spawn(move || {
            let cleanup_steps = collect_macro_release_steps(&preset.steps);
            let mut press_locked_keys: Vec<String> = Vec::new();
            let mut press_locked_mouse_count = 0usize;
            let _ = execute_macro_sequence(
                preset.id,
                &preset.steps,
                &mut press_locked_keys,
                &mut press_locked_mouse_count,
                preset.stop_on_retrigger_immediate,
                target_window_title.as_deref(),
                &extra_target_window_titles,
                match_duplicate_window_titles,
            );
            for step in cleanup_steps {
                let _ = send_key_event(&step);
            }
            if !press_locked_keys.is_empty() {
                apply_unlock_keys(&press_locked_keys, None);
            }
            for _ in 0..press_locked_mouse_count {
                apply_unlock_mouse(None);
            }
            let image_search_preset_ids = collect_macro_image_search_start_ids(&preset.steps);
            stop_vision_following_ids(&image_search_preset_ids);
            hide_toolbox_for_owner(preset.id);
            HOOK_STATE.lock().stop_ignore_keys.remove(&preset.id);
            decrement_press_trigger_suppression(&trigger_key_for_cleanup);
            STOP_REQUESTED_MACRO_PRESETS.lock().remove(&preset.id);
            SUPPRESSED_MACRO_HOTKEYS.lock().remove(&hotkey_id);
        });
        Ok(())
    }

    fn activate_hold_macro(
        preset: MacroPreset,
        trigger: HotkeyBinding,
        target_window_title: Option<String>,
        extra_target_window_titles: Vec<String>,
        match_duplicate_window_titles: bool,
        trigger_key: String,
    ) {
        let stale_run_exists = HOOK_STATE
            .lock()
            .active_hold_macros
            .contains_key(&preset.id);
        if stale_run_exists {
            deactivate_hold_macro(preset.id);
        }
        STOP_REQUESTED_MACRO_PRESETS.lock().remove(&preset.id);
        HOOK_STATE
            .lock()
            .stop_ignore_keys
            .insert(preset.id, trigger_key);
        let release_steps = collect_macro_release_steps(&preset.steps);
        let hold_stop_step = preset
            .hold_stop_step_enabled
            .then(|| preset.hold_stop_step.clone());
        let image_search_preset_ids = collect_macro_image_search_start_ids(&preset.steps);
        let run_token = {
            let mut hook_state = HOOK_STATE.lock();
            let run_token = hook_state.next_hold_run_token;
            hook_state.next_hold_run_token = hook_state.next_hold_run_token.saturating_add(1);
            hook_state.active_hold_macros.insert(
                preset.id,
                ActiveHoldMacro {
                    trigger,
                    release_steps,
                    hold_stop_step,
                    image_search_preset_ids,
                    locked_keys: Vec::new(),
                    locked_mouse_count: 0,
                    run_token,
                    completed: false,
                },
            );
            run_token
        };
        thread::spawn(move || {
            let flow = execute_hold_macro_sequence(
                preset.id,
                &preset.steps,
                preset.stop_on_retrigger_immediate,
                run_token,
                target_window_title.as_deref(),
                &extra_target_window_titles,
                match_duplicate_window_titles,
            );
            if matches!(flow, MacroRunFlow::Continue) {
                let mut hook_state = HOOK_STATE.lock();
                if let Some(active) = hook_state.active_hold_macros.get_mut(&preset.id)
                    && active.run_token == run_token
                {
                    active.completed = true;
                }
            }
        });
    }

    fn deactivate_hold_macro(preset_id: u32) {
        STOP_REQUESTED_MACRO_PRESETS.lock().insert(preset_id);
        let active = {
            let mut hook_state = HOOK_STATE.lock();
            let Some(active) = hook_state.active_hold_macros.remove(&preset_id) else {
                return;
            };
            active
        };

        let ActiveHoldMacro {
            trigger: _,
            release_steps,
            hold_stop_step,
            image_search_preset_ids,
            locked_keys,
            locked_mouse_count,
            run_token: _,
            completed,
        } = active;

        for step in release_steps {
            let _ = send_key_event(&step);
        }

        if !locked_keys.is_empty() {
            apply_unlock_keys(&locked_keys, Some(preset_id));
        }
        for _ in 0..locked_mouse_count {
            apply_unlock_mouse(Some(preset_id));
        }

        if !completed {
            if let Some(step) = hold_stop_step {
                execute_hold_abort_step(preset_id, &step);
            }
        }
        stop_vision_following_ids(&image_search_preset_ids);

        hide_toolbox_for_owner(preset_id);
        HOOK_STATE.lock().stop_ignore_keys.remove(&preset_id);
    }

    fn current_hold_run_matches(preset_id: u32, run_token: u64) -> bool {
        let hook_state = HOOK_STATE.lock();
        current_hold_run_matches_with_guard(preset_id, run_token, &hook_state)
    }

    fn current_hold_run_matches_with_guard(
        preset_id: u32,
        run_token: u64,
        hook_state: &HookState,
    ) -> bool {
        hook_state
            .active_hold_macros
            .get(&preset_id)
            .is_some_and(|active| active.run_token == run_token)
    }

    fn send_overlay_command(command: OverlayCommand) {
        if let Some(tx) = OVERLAY_COMMAND_TX.lock().clone() {
            let _ = tx.send(command);
            wake_command_queue();
        }
    }

    fn send_ui_command(command: UiCommand) {
        if let Some(tx) = HOOK_STATE.lock().ui_tx.clone() {
            let _ = tx.send(command);
        }
    }

    fn apply_window_preset_by_id(spec: &str) -> Result<()> {
        window_preset::apply_window_preset_by_id(spec)
    }

    fn spawn_custom_command(use_powershell: bool, command_text: String) {
        thread::spawn(move || {
            let mut command = if use_powershell {
                let mut cmd = Command::new("powershell.exe");
                cmd.args([
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-WindowStyle",
                    "Hidden",
                    "-Command",
                    &command_text,
                ]);
                cmd
            } else {
                let mut cmd = Command::new("cmd.exe");
                cmd.args(["/C", &command_text]);
                cmd
            };
            let _ = command.creation_flags(CREATE_NO_WINDOW.0).spawn();
        });
    }

    fn trigger_custom_preset_by_id(spec: &str) -> Result<()> {
        let spec = spec.trim();
        let preset = {
            let hook_state = HOOK_STATE.lock();
            let by_id = spec.parse::<u32>().ok().and_then(|preset_id| {
                hook_state
                    .command_presets
                    .iter()
                    .find(|preset| preset.id == preset_id)
                    .cloned()
            });
            by_id.or_else(|| {
                hook_state
                    .command_presets
                    .iter()
                    .find(|preset| preset.name.trim().eq_ignore_ascii_case(spec))
                    .cloned()
            })
        }
        .context("Custom preset was not found")?;

        if !preset.enabled {
            bail!("Custom preset is disabled");
        }

        if preset.target_window_title.is_some() || !preset.extra_target_window_titles.is_empty() {
            let foreground = unsafe { GetForegroundWindow() };
            let matches = unsafe {
                window_matches_any_selector(
                    foreground,
                    preset.target_window_title.as_deref(),
                    &preset.extra_target_window_titles,
                    preset.match_duplicate_window_titles,
                )
            };
            if !matches {
                return Ok(());
            }
        }

        let command_text = ai::normalize_command_text(&preset.command);
        if command_text.is_empty() {
            bail!("Custom preset command is empty");
        }

        spawn_custom_command(preset.use_powershell, command_text);
        Ok(())
    }

    fn trigger_command_preset_step(step: &MacroStep) -> Result<()> {
        let spec = step.key.trim();
        if spec.is_empty() {
            bail!("Custom preset key is empty");
        }

        let preset = {
            let hook_state = HOOK_STATE.lock();
            let by_id = spec.parse::<u32>().ok().and_then(|preset_id| {
                hook_state
                    .command_presets
                    .iter()
                    .find(|preset| preset.id == preset_id)
                    .cloned()
            });
            by_id.or_else(|| {
                hook_state
                    .command_presets
                    .iter()
                    .find(|preset| preset.name.trim().eq_ignore_ascii_case(spec))
                    .cloned()
            })
        };

        if let Some(preset) = preset {
            if !preset.enabled {
                bail!("Custom preset is disabled");
            }

            if preset.target_window_title.is_some() || !preset.extra_target_window_titles.is_empty()
            {
                let foreground = unsafe { GetForegroundWindow() };
                let matches = unsafe {
                    window_matches_any_selector(
                        foreground,
                        preset.target_window_title.as_deref(),
                        &preset.extra_target_window_titles,
                        preset.match_duplicate_window_titles,
                    )
                };
                if !matches {
                    return Ok(());
                }
            }

            let command_text = ai::normalize_command_text(&preset.command);
            if command_text.is_empty() {
                bail!("Custom preset command is empty");
            }

            spawn_custom_command(preset.use_powershell, command_text);
            return Ok(());
        }

        let command_text = ai::normalize_command_text(&step.command_preset_command);
        if command_text.is_empty() {
            bail!("Custom preset was not found");
        }
        spawn_custom_command(step.command_preset_use_powershell, command_text);
        Ok(())
    }

    fn focus_window_by_preset_id(spec: &str) -> Result<()> {
        window_preset::focus_window_by_preset_id(spec)
    }

    fn focus_window_for_preset(preset: &WindowFocusPreset) -> Result<()> {
        window_preset::focus_window_for_preset(preset)
    }

    fn replay_held_inputs_after_focus() {
        let held_keys = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .held_inputs
                .iter()
                .filter(|key| {
                    !matches!(
                        key.as_str(),
                        "Ctrl"
                            | "Alt"
                            | "Shift"
                            | "Win"
                            | "Tab"
                            | "MouseLeft"
                            | "MouseRight"
                            | "MouseMiddle"
                            | "MouseX1"
                            | "MouseX2"
                    )
                })
                .cloned()
                .collect::<Vec<_>>()
        };
        for key in held_keys {
            let _ = send_key_event(&MacroStep {
                action: MacroAction::KeyDown,
                key,
                ..MacroStep::default()
            });
        }
    }

    fn macro_stop_requested(preset_id: u32, stop_immediately_on_retrigger: bool) -> bool {
        if !STOP_REQUESTED_MACRO_PRESETS.lock().contains(&preset_id) {
            return false;
        }
        if stop_immediately_on_retrigger {
            return true;
        }
        HOOK_STATE
            .lock()
            .macro_groups
            .iter()
            .flat_map(|group| group.presets.iter())
            .find(|preset| preset.id == preset_id)
            .is_some_and(|preset| preset.stop_on_retrigger_immediate)
    }

    fn sleep_for_mouse_path_delay(
        preset_id: Option<u32>,
        delay_ms: u64,
        stop_immediately_on_retrigger: bool,
    ) -> bool {
        if delay_ms == 0 {
            return preset_id
                .is_some_and(|id| macro_stop_requested(id, stop_immediately_on_retrigger));
        }
        let mut remaining_ms = delay_ms;
        while remaining_ms > 0 {
            if preset_id.is_some_and(|id| macro_stop_requested(id, stop_immediately_on_retrigger)) {
                return true;
            }
            let chunk_ms = remaining_ms.min(10);
            thread::sleep(Duration::from_millis(chunk_ms));
            remaining_ms = remaining_ms.saturating_sub(chunk_ms);
        }
        preset_id.is_some_and(|id| macro_stop_requested(id, stop_immediately_on_retrigger))
    }

    fn enable_crosshair_profile(spec: &str) -> Result<()> {
        let profile_name = spec.trim();
        if profile_name.is_empty() {
            bail!("Crosshair profile name is empty");
        }
        let mut hook_state = HOOK_STATE.lock();
        let profile_index = hook_state
            .profiles
            .iter()
            .position(|profile| profile.name == profile_name)
            .context("Crosshair profile was not found")?;
        let profile_name_owned = hook_state.profiles[profile_index].name.clone();
        hook_state.profiles[profile_index].enabled = true;
        let mut style = hook_state.profiles[profile_index].style.clone();
        style.enabled = true;
        hook_state.current_style = style.clone();
        hook_state.active_crosshair_profile_name = Some(profile_name_owned.clone());
        let profiles = hook_state.profiles.clone();
        drop(hook_state);
        send_overlay_command(OverlayCommand::Update(style));
        send_ui_command(UiCommand::SyncCrosshairProfiles(
            profiles,
            format!("Enabled crosshair profile {}.", profile_name_owned),
        ));
        Ok(())
    }

    fn disable_crosshair_overlay() {
        let mut hook_state = HOOK_STATE.lock();
        let mut style = hook_state.current_style.clone();
        style.enabled = false;
        hook_state.current_style = style.clone();
        hook_state.active_crosshair_profile_name = None;
        for profile in &mut hook_state.profiles {
            profile.enabled = false;
        }
        let profiles = hook_state.profiles.clone();
        drop(hook_state);
        send_ui_command(UiCommand::SyncCrosshairProfiles(
            profiles,
            "Disabled crosshair overlay.".to_owned(),
        ));
        send_overlay_command(OverlayCommand::Update(style));
    }

    fn enable_pin_preset(spec: &str) -> Result<()> {
        let preset_id = spec
            .trim()
            .parse::<u32>()
            .context("Pin preset id is invalid")?;
        let mut hook_state = HOOK_STATE.lock();
        if !hook_state
            .pin_presets
            .iter()
            .any(|preset| preset.id == preset_id)
        {
            bail!("Pin preset was not found");
        }
        hook_state.active_pin_preset_id = Some(preset_id);
        send_overlay_command(OverlayCommand::RefreshPinOverlay);
        Ok(())
    }

    fn disable_pin_overlay() {
        HOOK_STATE.lock().active_pin_preset_id = None;
        send_overlay_command(OverlayCommand::RefreshPinOverlay);
    }

    fn play_sound_preset(spec: &str) -> Result<()> {
        let preset_id = spec
            .trim()
            .parse::<u32>()
            .context("Sound preset id is invalid")?;
        let clip = {
            let hook_state = HOOK_STATE.lock();
            let preset = hook_state
                .sound_presets
                .iter()
                .find(|preset| preset.id == preset_id)
                .cloned()
                .context("Sound preset was not found")?;
            let mut clip = preset.clip.clone();
            clip.enabled = true;
            clip
        };
        audio::play_clip_async(clip);
        Ok(())
    }

    fn play_mouse_path_preset(
        spec: &str,
        step: &MacroStep,
        preset_id: Option<u32>,
        stop_immediately_on_retrigger: bool,
    ) -> Result<()> {
        let mouse_path_preset_id = spec
            .trim()
            .parse::<u32>()
            .context("Mouse path preset id is invalid")?;
        let (events, _, replay_relative_motion) = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .mouse_path_presets
                .iter()
                .find(|preset| preset.id == mouse_path_preset_id)
                .map(|preset| {
                    (
                        preset.events.clone(),
                        false,
                        preset.replay_relative_motion,
                    )
                })
                .context("Mouse path preset was not found")?
        };
        if events.is_empty() {
            return Ok(());
        }

        if step.smooth_mouse_path {
            let speed = step.mouse_speed_percent.max(10) as f32 / 100.0;
            let mut last_move_pos: Option<(i32, i32)> = None;
            for event in &events {
                if preset_id
                    .is_some_and(|id| macro_stop_requested(id, stop_immediately_on_retrigger))
                {
                    return Ok(());
                }
                match event.kind {
                    MousePathEventKind::Move => {
                        if replay_relative_motion {
                            if let Some((from_x, from_y)) = last_move_pos {
                                settle_mouse_path_relative_segment(from_x, from_y, event.x, event.y, speed, preset_id, stop_immediately_on_retrigger,
                                )?;
                            }
                            last_move_pos = Some((event.x, event.y));
                        } else if let Some((from_x, from_y)) = last_move_pos {
                            let dx = event.x - from_x;
                            let dy = event.y - from_y;
                            let distance = (((dx * dx + dy * dy) as f32).sqrt()).max(1.0);
                            let duration_ms = ((distance / (900.0 * speed)) * 1000.0)
                                .round()
                                .clamp(1.0, 5_000.0)
                                as u64;
                            let steps = (duration_ms / 8).max(1);
                            for index in 1..=steps {
                                if preset_id.is_some_and(|id| {
                                    macro_stop_requested(id, stop_immediately_on_retrigger)
                                }) {
                                    return Ok(());
                                }
                                let t = index as f32 / steps as f32;
                                let x = from_x as f32 + dx as f32 * t;
                                let y = from_y as f32 + dy as f32 * t;
                                send_mouse_move_absolute(
                                    x.round() as i32,
                                    y.round() as i32,
                                )?;
                                if sleep_for_mouse_path_delay(
                                    preset_id,
                                    8,
                                    stop_immediately_on_retrigger,
                                ) {
                                    return Ok(());
                                }
                            }
                            last_move_pos = Some((event.x, event.y));
                        } else {
                            send_mouse_move_absolute(
                                event.x,
                                event.y,
                            )?;
                            last_move_pos = Some((event.x, event.y));
                        }
                    }
                    _ => {
                        if sleep_for_mouse_path_delay(
                            preset_id,
                            event.delay_ms,
                            stop_immediately_on_retrigger,
                        ) {
                            return Ok(());
                        }
                        let pseudo_step = MacroStep {
                            action: match event.kind {
                                MousePathEventKind::LeftDown => MacroAction::MouseLeftDown,
                                MousePathEventKind::LeftUp => MacroAction::MouseLeftUp,
                                MousePathEventKind::RightDown => MacroAction::MouseRightDown,
                                MousePathEventKind::RightUp => MacroAction::MouseRightUp,
                                MousePathEventKind::MiddleDown => MacroAction::MouseMiddleDown,
                                MousePathEventKind::MiddleUp => MacroAction::MouseMiddleUp,
                                MousePathEventKind::WheelUp => MacroAction::MouseWheelUp,
                                MousePathEventKind::WheelDown => MacroAction::MouseWheelDown,
                                MousePathEventKind::Move => MacroAction::MouseMoveAbsolute,
                            },
                            x: event.x,
                            y: event.y,
                            ..MacroStep::default()
                        };
                        send_mouse_event(&pseudo_step)?;
                    }
                }
            }
        } else {
            let mut last_move_pos: Option<(i32, i32)> = None;
            for event in &events {
                if sleep_for_mouse_path_delay(
                    preset_id,
                    event.delay_ms,
                    stop_immediately_on_retrigger,
                ) {
                    return Ok(());
                }
                match event.kind {
                    MousePathEventKind::Move if replay_relative_motion => {
                        if let Some((from_x, from_y)) = last_move_pos {
                            send_mouse_move_relative(
                                event.x - from_x,
                                event.y - from_y,
                            )?;
                        }
                        last_move_pos = Some((event.x, event.y));
                    }
                    MousePathEventKind::Move => {
                        let pseudo_step = MacroStep {
                            action: MacroAction::MouseMoveAbsolute,
                            x: event.x,
                            y: event.y,
                            ..MacroStep::default()
                        };
                        send_mouse_event(&pseudo_step)?;
                    }
                    _ => {
                        let pseudo_step = MacroStep {
                            action: match event.kind {
                                MousePathEventKind::LeftDown => MacroAction::MouseLeftDown,
                                MousePathEventKind::LeftUp => MacroAction::MouseLeftUp,
                                MousePathEventKind::RightDown => MacroAction::MouseRightDown,
                                MousePathEventKind::RightUp => MacroAction::MouseRightUp,
                                MousePathEventKind::MiddleDown => MacroAction::MouseMiddleDown,
                                MousePathEventKind::MiddleUp => MacroAction::MouseMiddleUp,
                                MousePathEventKind::WheelUp => MacroAction::MouseWheelUp,
                                MousePathEventKind::WheelDown => MacroAction::MouseWheelDown,
                                MousePathEventKind::Move => MacroAction::MouseMoveAbsolute,
                            },
                            x: event.x,
                            y: event.y,
                            ..MacroStep::default()
                        };
                        send_mouse_event(&pseudo_step)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn apply_mouse_sensitivity_preset_by_id(spec: &str) -> Result<()> {
        let preset_id = parse_mouse_sensitivity_preset_id(spec)
            .context("Mouse sensitivity preset id is invalid")?;
        let preset = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .mouse_sensitivity_presets
                .iter()
                .find(|preset| preset.id == preset_id)
                .cloned()
                .context("Mouse sensitivity preset was not found")?
        };
        apply_mouse_sensitivity_preset(&preset)
    }

    fn enable_zoom_preset(_spec: &str) -> Result<()> {
        bail!("Zoom was removed")
    }

    fn disable_zoom_overlay() {}

    fn set_macro_preset_enabled(spec: &str, enabled: bool) -> Result<()> {
        let preset_id = spec
            .trim()
            .parse::<u32>()
            .context("Macro preset id is invalid")?;
        let mut hook_state = HOOK_STATE.lock();
        for group in &mut hook_state.macro_groups {
            if let Some(preset) = group
                .presets
                .iter_mut()
                .find(|preset| preset.id == preset_id)
            {
                preset.enabled = enabled;
                let updated_groups = hook_state.macro_groups.clone();
                let status = format!(
                    "{} macro preset {}.",
                    if enabled { "Enabled" } else { "Disabled" },
                    preset_id
                );
                if let Some(tx) = hook_state.ui_tx.clone() {
                    let _ = tx.send(UiCommand::SyncMacroGroups(updated_groups, status));
                }
                return Ok(());
            }
        }
        bail!("Macro preset was not found")
    }

    fn execute_hold_abort_step(preset_id: u32, step: &MacroStep) {
        if !step.enabled {
            return;
        }
        match step.action {
            MacroAction::LoopStart
            | MacroAction::LoopEnd
            | MacroAction::StopIfTriggerPressedAgain
            | MacroAction::StopIfKeyPressed => {}
            MacroAction::ApplyWindowPreset => {
                let _ = apply_window_preset_by_id(&step.key);
            }
            MacroAction::FocusWindowPreset => {
                let _ = focus_window_by_preset_id(&step.key);
            }
            MacroAction::TriggerMacroPreset => {
                let mut no_locked_keys = Vec::new();
                let mut no_locked_mouse = 0usize;
                let _ = trigger_nested_macro_preset(
                    &step.key,
                    &mut no_locked_keys,
                    &mut no_locked_mouse,
                    false,
                    None,
                    &[],
                    false,
                );
            }
            MacroAction::TriggerCommandPreset => {
                let _ = trigger_command_preset_step(step);
            }
            MacroAction::EnableCrosshairProfile => {
                let _ = enable_crosshair_profile(&step.key);
            }
            MacroAction::DisableCrosshair => {
                disable_crosshair_overlay();
            }
            MacroAction::EnablePinPreset => {
                let _ = enable_pin_preset(&step.key);
            }
            MacroAction::DisablePin => {
                disable_pin_overlay();
            }
            MacroAction::PlayMousePathPreset => {
                let _ = play_mouse_path_preset(&step.key, step, Some(preset_id), false);
            }
            MacroAction::EnableZoomPreset => {
                let _ = enable_zoom_preset(&step.key);
            }
            MacroAction::DisableZoom => {
                disable_zoom_overlay();
            }
            MacroAction::PlaySoundPreset => {
                let _ = play_sound_preset(&step.key);
            }
            MacroAction::StartVisionSearch => {
                let _ = start_vision_following(&step.key);
            }
            MacroAction::TriggerVisionMove => {
                if let Ok(preset) = vision_preset_by_id(&step.key) {
                    let mut no_locked_keys = Vec::new();
                    let mut no_locked_mouse = 0usize;
                    let _ = trigger_vision_move_with_options(
                        &preset,
                        step.vision_move_cursor_on_match,
                        step.vision_wait_until_found,
                        step.vision_trigger_macro_enabled,
                        step.vision_trigger_macro_preset_id,
                        preset_id,
                        &mut no_locked_keys,
                        &mut no_locked_mouse,
                        false,
                        None,
                        &[],
                        false,
                    );
                }
            }
            MacroAction::StopVisionWait => {
                let _ = stop_vision_waiting(&step.key);
            }
            MacroAction::StopVision => {
                let _ = stop_vision_following(&step.key);
            }
            MacroAction::ShowHud => {
                trigger_hud_display(preset_id, step);
            }
            MacroAction::HideHud => {
                hide_hud_now();
            }
            MacroAction::LockKeys => {
                apply_lock_keys(&parse_locked_keys(&step.key), Some(preset_id));
            }
            MacroAction::UnlockKeys => {
                apply_unlock_keys(&parse_locked_keys(&step.key), Some(preset_id));
            }
            MacroAction::LockMouse => {
                apply_lock_mouse(Some(preset_id));
            }
            MacroAction::UnlockMouse => {
                apply_unlock_mouse(Some(preset_id));
            }
            MacroAction::EnableMacroPreset => {
                let _ = set_macro_preset_enabled(&step.key, true);
            }
            MacroAction::DisableMacroPreset => {
                let _ = set_macro_preset_enabled(&step.key, false);
            }
            _ => {
                let _ = send_key_event(step);
            }
        }
    }

    fn execute_macro_sequence(
        preset_id: u32,
        steps: &[MacroStep],
        press_locked_keys: &mut Vec<String>,
        press_locked_mouse_count: &mut usize,
        stop_immediately_on_retrigger: bool,
        target_window_title: Option<&str>,
        extra_target_window_titles: &[String],
        match_duplicate_window_titles: bool,
    ) -> MacroRunFlow {
        let mut index = 0usize;
        while index < steps.len() {
            if !macro_runtime_target_matches(
                target_window_title,
                extra_target_window_titles,
                match_duplicate_window_titles,
            ) {
                return MacroRunFlow::StopExecution;
            }
            if stop_immediately_on_retrigger
                && STOP_REQUESTED_MACRO_PRESETS.lock().contains(&preset_id)
            {
                return MacroRunFlow::StopExecution;
            }
            let step = &steps[index];
            if !step.enabled {
                index += 1;
                continue;
            }
            let hold_duration_ms = if step.action == MacroAction::KeyDown {
                step.delay_ms
            } else {
                0
            };
            if !step.enabled {
                index += 1;
                continue;
            }
            if step.action != MacroAction::KeyDown
                && sleep_for_macro_delay(
                    preset_id,
                    step.delay_ms,
                    stop_immediately_on_retrigger,
                    target_window_title,
                    extra_target_window_titles,
                    match_duplicate_window_titles,
                )
            {
                return MacroRunFlow::StopExecution;
            }
            match step.action {
                MacroAction::LoopStart => {
                    let Some(loop_end) = find_matching_loop_end(steps, index) else {
                        index += 1;
                        continue;
                    };
                    let loop_body = &steps[index + 1..loop_end];
                    let loop_end_delay_ms = steps[loop_end].delay_ms;
                    if is_infinite_loop_marker(&step.key) {
                        loop {
                            match execute_macro_sequence(
                                preset_id,
                                loop_body,
                                press_locked_keys,
                                press_locked_mouse_count,
                                stop_immediately_on_retrigger,
                                target_window_title,
                                extra_target_window_titles,
                                match_duplicate_window_titles,
                            ) {
                                MacroRunFlow::BreakLoop => break,
                                MacroRunFlow::StopExecution => return MacroRunFlow::StopExecution,
                                MacroRunFlow::Continue => {}
                            }
                            if loop_end_delay_ms > 0
                                && sleep_for_macro_delay(
                                    preset_id,
                                    loop_end_delay_ms,
                                    stop_immediately_on_retrigger,
                                    target_window_title,
                                    extra_target_window_titles,
                                    match_duplicate_window_titles,
                                )
                            {
                                return MacroRunFlow::StopExecution;
                            }
                        }
                    } else {
                        let loop_count = step.key.trim().parse::<u32>().unwrap_or(1).max(1);
                        for _ in 0..loop_count {
                            match execute_macro_sequence(
                                preset_id,
                                loop_body,
                                press_locked_keys,
                                press_locked_mouse_count,
                                stop_immediately_on_retrigger,
                                target_window_title,
                                extra_target_window_titles,
                                match_duplicate_window_titles,
                            ) {
                                MacroRunFlow::BreakLoop => break,
                                MacroRunFlow::StopExecution => return MacroRunFlow::StopExecution,
                                MacroRunFlow::Continue => {}
                            }
                            if loop_end_delay_ms > 0
                                && sleep_for_macro_delay(
                                    preset_id,
                                    loop_end_delay_ms,
                                    stop_immediately_on_retrigger,
                                    target_window_title,
                                    extra_target_window_titles,
                                    match_duplicate_window_titles,
                                )
                            {
                                return MacroRunFlow::StopExecution;
                            }
                        }
                    }
                    index = loop_end + 1;
                    continue;
                }
                MacroAction::LoopEnd => return MacroRunFlow::Continue,
                MacroAction::StopIfTriggerPressedAgain => {
                    if STOP_REQUESTED_MACRO_PRESETS.lock().remove(&preset_id) {
                        return MacroRunFlow::BreakLoop;
                    }
                }
                MacroAction::StopIfKeyPressed => {
                    let key = normalize_locked_key(&step.key);
                    if stop_key_triggered(preset_id, &key) {
                        return MacroRunFlow::BreakLoop;
                    }
                }
                MacroAction::Wait => {}
                MacroAction::ApplyWindowPreset => {
                    let _ = apply_window_preset_by_id(&step.key);
                }
                MacroAction::FocusWindowPreset => {
                    let _ = focus_window_by_preset_id(&step.key);
                }
                MacroAction::TriggerMacroPreset => {
                    let _ = trigger_nested_macro_preset(
                        &step.key,
                        press_locked_keys,
                        press_locked_mouse_count,
                        stop_immediately_on_retrigger,
                        target_window_title,
                        extra_target_window_titles,
                        match_duplicate_window_titles,
                    );
                }
                MacroAction::TriggerCommandPreset => {
                    let _ = trigger_command_preset_step(step);
                }
                MacroAction::EnableCrosshairProfile => {
                    let _ = enable_crosshair_profile(&step.key);
                }
                MacroAction::DisableCrosshair => {
                    disable_crosshair_overlay();
                }
                MacroAction::EnablePinPreset => {
                    let _ = enable_pin_preset(&step.key);
                }
                MacroAction::DisablePin => {
                    disable_pin_overlay();
                }
                MacroAction::PlayMousePathPreset => {
                    let _ = play_mouse_path_preset(
                        &step.key,
                        step,
                        Some(preset_id),
                        stop_immediately_on_retrigger,
                    );
                }
                MacroAction::ApplyMouseSensitivityPreset => {
                    let _ = apply_mouse_sensitivity_preset_by_id(&step.key);
                }
                MacroAction::EnableZoomPreset => {
                    let _ = enable_zoom_preset(&step.key);
                }
                MacroAction::DisableZoom => {
                    disable_zoom_overlay();
                }
                MacroAction::PlaySoundPreset => {
                    let _ = play_sound_preset(&step.key);
                }
                MacroAction::StartVisionSearch => {
                    let _ = start_vision_following(&step.key);
                }
                MacroAction::TriggerVisionMove => {
                    if let Some(preset) = vision_preset_by_id(&step.key).ok() {
                        let mut no_locked_keys = Vec::new();
                        let mut no_locked_mouse = 0usize;
                        if let MacroRunFlow::StopExecution = trigger_vision_move_with_options(
                            &preset,
                            step.vision_move_cursor_on_match,
                            step.vision_wait_until_found,
                            step.vision_trigger_macro_enabled,
                            step.vision_trigger_macro_preset_id,
                            preset_id,
                            &mut no_locked_keys,
                            &mut no_locked_mouse,
                            stop_immediately_on_retrigger,
                            target_window_title,
                            extra_target_window_titles,
                            match_duplicate_window_titles,
                        ) {
                            return MacroRunFlow::StopExecution;
                        }
                    }
                }
                MacroAction::StopVisionWait => {
                    let _ = stop_vision_waiting(&step.key);
                }
                MacroAction::StopVision => {
                    let _ = stop_vision_following(&step.key);
                }
                MacroAction::ShowHud => {
                    trigger_hud_display(preset_id, step);
                }
                MacroAction::HideHud => {
                    hide_hud_now();
                }
                MacroAction::LockKeys => {
                    let keys = parse_locked_keys(&step.key);
                    for key in &keys {
                        if !press_locked_keys
                            .iter()
                            .any(|existing| existing.eq_ignore_ascii_case(key))
                        {
                            press_locked_keys.push(key.clone());
                        }
                    }
                    apply_lock_keys(&keys, None);
                }
                MacroAction::UnlockKeys => {
                    let keys = parse_locked_keys(&step.key);
                    apply_unlock_keys(&keys, None);
                    press_locked_keys
                        .retain(|locked| !keys.iter().any(|key| key.eq_ignore_ascii_case(locked)));
                }
                MacroAction::LockMouse => {
                    apply_lock_mouse(None);
                    *press_locked_mouse_count = press_locked_mouse_count.saturating_add(1);
                }
                MacroAction::UnlockMouse => {
                    if *press_locked_mouse_count > 0 {
                        *press_locked_mouse_count -= 1;
                    }
                    apply_unlock_mouse(None);
                }
                MacroAction::EnableMacroPreset => {
                    let _ = set_macro_preset_enabled(&step.key, true);
                }
                MacroAction::DisableMacroPreset => {
                    let _ = set_macro_preset_enabled(&step.key, false);
                }
                MacroAction::KeyDown => {
                    let _ = send_key_event(step);
                    if hold_duration_ms > 0
                        && sleep_for_macro_delay(
                            preset_id,
                            hold_duration_ms,
                            stop_immediately_on_retrigger,
                            target_window_title,
                            extra_target_window_titles,
                            match_duplicate_window_titles,
                        )
                    {
                        return MacroRunFlow::StopExecution;
                    }
                }
                _ => {
                    let _ = send_key_event(step);
                }
            }
            index += 1;
        }
        MacroRunFlow::Continue
    }

    fn execute_hold_macro_sequence(
        preset_id: u32,
        steps: &[MacroStep],
        stop_immediately_on_retrigger: bool,
        run_token: u64,
        target_window_title: Option<&str>,
        extra_target_window_titles: &[String],
        match_duplicate_window_titles: bool,
    ) -> MacroRunFlow {
        let mut index = 0usize;
        while index < steps.len() {
            if !current_hold_run_matches(preset_id, run_token) {
                return MacroRunFlow::StopExecution;
            }
            if !macro_runtime_target_matches(
                target_window_title,
                extra_target_window_titles,
                match_duplicate_window_titles,
            ) {
                return MacroRunFlow::StopExecution;
            }
            if stop_immediately_on_retrigger
                && STOP_REQUESTED_MACRO_PRESETS.lock().contains(&preset_id)
            {
                return MacroRunFlow::StopExecution;
            }
            let step = &steps[index];
            if !step.enabled {
                index += 1;
                continue;
            }
            let hold_duration_ms = if step.action == MacroAction::KeyDown {
                step.delay_ms
            } else {
                0
            };
            if !step.enabled {
                index += 1;
                continue;
            }
            if step.action != MacroAction::KeyDown
                && sleep_for_hold_delay(
                    preset_id,
                    step.delay_ms,
                    stop_immediately_on_retrigger,
                    run_token,
                    target_window_title,
                    extra_target_window_titles,
                    match_duplicate_window_titles,
                )
            {
                return MacroRunFlow::StopExecution;
            }
            match step.action {
                MacroAction::LoopStart => {
                    let Some(loop_end) = find_matching_loop_end(steps, index) else {
                        index += 1;
                        continue;
                    };
                    let loop_body = &steps[index + 1..loop_end];
                    let loop_end_delay_ms = steps[loop_end].delay_ms;
                    if is_infinite_loop_marker(&step.key) {
                        loop {
                            match execute_hold_macro_sequence(
                                preset_id,
                                loop_body,
                                stop_immediately_on_retrigger,
                                run_token,
                                target_window_title,
                                extra_target_window_titles,
                                match_duplicate_window_titles,
                            ) {
                                MacroRunFlow::BreakLoop => break,
                                MacroRunFlow::StopExecution => return MacroRunFlow::StopExecution,
                                MacroRunFlow::Continue => {}
                            }
                            if loop_end_delay_ms > 0
                                && sleep_for_hold_delay(
                                    preset_id,
                                    loop_end_delay_ms,
                                    stop_immediately_on_retrigger,
                                    run_token,
                                    target_window_title,
                                    extra_target_window_titles,
                                    match_duplicate_window_titles,
                                )
                            {
                                return MacroRunFlow::StopExecution;
                            }
                        }
                    } else {
                        let loop_count = step.key.trim().parse::<u32>().unwrap_or(1).max(1);
                        for _ in 0..loop_count {
                            match execute_hold_macro_sequence(
                                preset_id,
                                loop_body,
                                stop_immediately_on_retrigger,
                                run_token,
                                target_window_title,
                                extra_target_window_titles,
                                match_duplicate_window_titles,
                            ) {
                                MacroRunFlow::BreakLoop => break,
                                MacroRunFlow::StopExecution => return MacroRunFlow::StopExecution,
                                MacroRunFlow::Continue => {}
                            }
                            if loop_end_delay_ms > 0
                                && sleep_for_hold_delay(
                                    preset_id,
                                    loop_end_delay_ms,
                                    stop_immediately_on_retrigger,
                                    run_token,
                                    target_window_title,
                                    extra_target_window_titles,
                                    match_duplicate_window_titles,
                                )
                            {
                                return MacroRunFlow::StopExecution;
                            }
                        }
                    }
                    index = loop_end + 1;
                    continue;
                }
                MacroAction::LoopEnd => return MacroRunFlow::Continue,
                MacroAction::StopIfTriggerPressedAgain => {
                    if STOP_REQUESTED_MACRO_PRESETS.lock().remove(&preset_id) {
                        return MacroRunFlow::BreakLoop;
                    }
                }
                MacroAction::StopIfKeyPressed => {
                    let key = normalize_locked_key(&step.key);
                    if stop_key_triggered(preset_id, &key) {
                        return MacroRunFlow::BreakLoop;
                    }
                }
                MacroAction::Wait => {}
                MacroAction::ApplyWindowPreset => {
                    let _ = apply_window_preset_by_id(&step.key);
                }
                MacroAction::FocusWindowPreset => {
                    let _ = focus_window_by_preset_id(&step.key);
                }
                MacroAction::TriggerMacroPreset => {
                    let mut no_locked_keys = Vec::new();
                    let mut no_locked_mouse = 0usize;
                    let _ = trigger_nested_macro_preset(
                        &step.key,
                        &mut no_locked_keys,
                        &mut no_locked_mouse,
                        stop_immediately_on_retrigger,
                        target_window_title,
                        extra_target_window_titles,
                        match_duplicate_window_titles,
                    );
                }
                MacroAction::TriggerCommandPreset => {
                    let _ = trigger_command_preset_step(step);
                }
                MacroAction::EnableCrosshairProfile => {
                    let _ = enable_crosshair_profile(&step.key);
                }
                MacroAction::DisableCrosshair => {
                    disable_crosshair_overlay();
                }
                MacroAction::EnablePinPreset => {
                    let _ = enable_pin_preset(&step.key);
                }
                MacroAction::DisablePin => {
                    disable_pin_overlay();
                }
                MacroAction::PlayMousePathPreset => {
                    let _ = play_mouse_path_preset(
                        &step.key,
                        step,
                        Some(preset_id),
                        stop_immediately_on_retrigger,
                    );
                }
                MacroAction::ApplyMouseSensitivityPreset => {
                    let _ = apply_mouse_sensitivity_preset_by_id(&step.key);
                }
                MacroAction::EnableZoomPreset => {
                    let _ = enable_zoom_preset(&step.key);
                }
                MacroAction::DisableZoom => {
                    disable_zoom_overlay();
                }
                MacroAction::PlaySoundPreset => {
                    let _ = play_sound_preset(&step.key);
                }
                MacroAction::StartVisionSearch => {
                    let _ = start_vision_following(&step.key);
                }
                MacroAction::TriggerVisionMove => {
                    if let Some(preset) = vision_preset_by_id(&step.key).ok() {
                        let mut no_locked_keys = Vec::new();
                        let mut no_locked_mouse = 0usize;
                        if let MacroRunFlow::StopExecution = trigger_vision_move_with_options(
                            &preset,
                            step.vision_move_cursor_on_match,
                            step.vision_wait_until_found,
                            step.vision_trigger_macro_enabled,
                            step.vision_trigger_macro_preset_id,
                            preset_id,
                            &mut no_locked_keys,
                            &mut no_locked_mouse,
                            stop_immediately_on_retrigger,
                            target_window_title,
                            extra_target_window_titles,
                            match_duplicate_window_titles,
                        ) {
                            return MacroRunFlow::StopExecution;
                        }
                    }
                }
                MacroAction::StopVisionWait => {
                    let _ = stop_vision_waiting(&step.key);
                }
                MacroAction::StopVision => {
                    let _ = stop_vision_following(&step.key);
                }
                MacroAction::ShowHud => {
                    trigger_hud_display(preset_id, step);
                }
                MacroAction::HideHud => {
                    hide_hud_now();
                }
                MacroAction::LockKeys => {
                    apply_lock_keys(&parse_locked_keys(&step.key), Some(preset_id));
                }
                MacroAction::UnlockKeys => {
                    apply_unlock_keys(&parse_locked_keys(&step.key), Some(preset_id));
                }
                MacroAction::LockMouse => {
                    apply_lock_mouse(Some(preset_id));
                }
                MacroAction::UnlockMouse => {
                    apply_unlock_mouse(Some(preset_id));
                }
                MacroAction::EnableMacroPreset => {
                    let _ = set_macro_preset_enabled(&step.key, true);
                }
                MacroAction::DisableMacroPreset => {
                    let _ = set_macro_preset_enabled(&step.key, false);
                }
                MacroAction::KeyDown => {
                    let _ = send_key_event(step);
                    if hold_duration_ms > 0
                        && sleep_for_hold_delay(
                            preset_id,
                            hold_duration_ms,
                            stop_immediately_on_retrigger,
                            run_token,
                            target_window_title,
                            extra_target_window_titles,
                            match_duplicate_window_titles,
                        )
                    {
                        return MacroRunFlow::StopExecution;
                    }
                }
                _ => {
                    let _ = send_key_event(step);
                }
            }
            index += 1;
        }
        MacroRunFlow::Continue
    }

    fn sleep_for_hold_delay(
        preset_id: u32,
        delay_ms: u64,
        stop_immediately_on_retrigger: bool,
        run_token: u64,
        target_window_title: Option<&str>,
        extra_target_window_titles: &[String],
        match_duplicate_window_titles: bool,
    ) -> bool {
        if delay_ms == 0 {
            return !macro_runtime_target_matches(
                target_window_title,
                extra_target_window_titles,
                match_duplicate_window_titles,
            ) || !current_hold_run_matches(preset_id, run_token)
                || (stop_immediately_on_retrigger
                    && STOP_REQUESTED_MACRO_PRESETS.lock().contains(&preset_id));
        }

        let mut remaining_ms = delay_ms;
        while remaining_ms > 0 {
            {
                let hook_state = HOOK_STATE.lock();
                if !hook_state.macros_master_enabled {
                    return true;
                }
                if !current_hold_run_matches_with_guard(preset_id, run_token, &hook_state) {
                    return true;
                }
                if !macro_runtime_target_matches_with_guard(
                    target_window_title,
                    extra_target_window_titles,
                    match_duplicate_window_titles,
                    &hook_state,
                ) {
                    return true;
                }
            }
            if stop_immediately_on_retrigger
                && STOP_REQUESTED_MACRO_PRESETS.lock().contains(&preset_id)
            {
                return true;
            }
            let chunk_ms = remaining_ms.min(10);
            thread::sleep(std::time::Duration::from_millis(chunk_ms));
            remaining_ms = remaining_ms.saturating_sub(chunk_ms);
        }
        !macro_runtime_target_matches(
            target_window_title,
            extra_target_window_titles,
            match_duplicate_window_titles,
        ) || !current_hold_run_matches(preset_id, run_token)
            || (stop_immediately_on_retrigger
                && STOP_REQUESTED_MACRO_PRESETS.lock().contains(&preset_id))
    }

    fn sleep_for_macro_delay(
        preset_id: u32,
        delay_ms: u64,
        stop_immediately_on_retrigger: bool,
        target_window_title: Option<&str>,
        extra_target_window_titles: &[String],
        match_duplicate_window_titles: bool,
    ) -> bool {
        if delay_ms == 0 {
            return !macro_runtime_target_matches(
                target_window_title,
                extra_target_window_titles,
                match_duplicate_window_titles,
            );
        }

        let mut remaining_ms = delay_ms;
        while remaining_ms > 0 {
            {
                let hook_state = HOOK_STATE.lock();
                if !hook_state.macros_master_enabled {
                    return true;
                }
                if !macro_runtime_target_matches_with_guard(
                    target_window_title,
                    extra_target_window_titles,
                    match_duplicate_window_titles,
                    &hook_state,
                ) {
                    return true;
                }
            }
            if stop_immediately_on_retrigger
                && STOP_REQUESTED_MACRO_PRESETS.lock().contains(&preset_id)
            {
                return true;
            }
            let chunk_ms = remaining_ms.min(10);
            thread::sleep(std::time::Duration::from_millis(chunk_ms));
            remaining_ms = remaining_ms.saturating_sub(chunk_ms);
        }
        !macro_runtime_target_matches(
            target_window_title,
            extra_target_window_titles,
            match_duplicate_window_titles,
        ) || (stop_immediately_on_retrigger
            && STOP_REQUESTED_MACRO_PRESETS.lock().contains(&preset_id))
    }

    fn find_matching_loop_end(steps: &[MacroStep], start_index: usize) -> Option<usize> {
        let mut depth = 0usize;
        for (index, step) in steps.iter().enumerate().skip(start_index) {
            match step.action {
                MacroAction::LoopStart => depth += 1,
                MacroAction::LoopEnd => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(index);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn is_infinite_loop_marker(value: &str) -> bool {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "infinite" | "inf" | "forever" | "-1"
        )
    }

    fn macro_runtime_target_matches(
        target_window_title: Option<&str>,
        extra_target_window_titles: &[String],
        match_duplicate_window_titles: bool,
    ) -> bool {
        let hook_state = HOOK_STATE.lock();
        macro_runtime_target_matches_with_guard(
            target_window_title,
            extra_target_window_titles,
            match_duplicate_window_titles,
            &hook_state,
        )
    }

    fn macro_runtime_target_matches_with_guard(
        target_window_title: Option<&str>,
        extra_target_window_titles: &[String],
        match_duplicate_window_titles: bool,
        _hook_state: &HookState,
    ) -> bool {
        window_focus_matches(
            target_window_title,
            extra_target_window_titles,
            match_duplicate_window_titles,
        )
    }

    fn trigger_nested_macro_preset(
        spec: &str,
        press_locked_keys: &mut Vec<String>,
        press_locked_mouse_count: &mut usize,
        stop_immediately_on_retrigger: bool,
        target_window_title: Option<&str>,
        extra_target_window_titles: &[String],
        match_duplicate_window_titles: bool,
    ) -> Result<()> {
        let preset_id = spec
            .trim()
            .parse::<u32>()
            .context("Macro preset id is invalid")?;
        let preset = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .macro_groups
                .iter()
                .flat_map(|group| group.presets.iter())
                .find(|preset| preset.id == preset_id)
                .cloned()
        }
        .context("Macro preset was not found")?;
        let _ = execute_macro_sequence(
            preset.id,
            &preset.steps,
            press_locked_keys,
            press_locked_mouse_count,
            stop_immediately_on_retrigger,
            target_window_title,
            extra_target_window_titles,
            match_duplicate_window_titles,
        );
        Ok(())
    }

    fn parse_locked_keys(spec: &str) -> Vec<String> {
        let trimmed = spec.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        let has_separator = trimmed
            .chars()
            .any(|ch| matches!(ch, ',' | ';' | '+' | ' ' | '\t' | '\n'));
        if has_separator {
            return trimmed
                .split(|ch: char| matches!(ch, ',' | ';' | '+' | ' ' | '\t' | '\n'))
                .filter_map(|part| {
                    let key = part.trim();
                    (!key.is_empty()).then(|| normalize_locked_key(key))
                })
                .collect();
        }

        if trimmed.len() > 1 && trimmed.chars().all(|ch| ch.is_ascii_alphanumeric()) {
            return trimmed
                .chars()
                .map(|ch| normalize_locked_key(&ch.to_string()))
                .collect();
        }

        vec![normalize_locked_key(trimmed)]
    }

    fn normalize_locked_key(key: &str) -> String {
        let trimmed = key.trim();
        if let Some(vk) = hotkey::key_name_to_vk(trimmed)
            && let Some(name) = hotkey::vk_to_key_name(vk)
        {
            return name.to_owned();
        }
        trimmed.to_owned()
    }

    fn show_hud_preset(owner_preset_id: u32, step: &MacroStep) -> Result<()> {
        let preset_id = step
            .key
            .trim()
            .parse::<u32>()
            .context("Toolbox preset id is invalid")?;
        let preset = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .hud_presets
                .iter()
                .find(|preset| preset.id == preset_id)
                .cloned()
        }
        .context("Toolbox preset was not found")?;
        let text = if step.text_override.trim().is_empty() {
            preset.text.trim().to_owned()
        } else {
            step.text_override.trim().to_owned()
        };
        if text.is_empty() {
            hide_hud_now();
            return Ok(());
        }

        let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) }.max(1);
        let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) }.max(1);
        let scale_x = screen_width as f32 / 1920.0;
        let scale_y = screen_height as f32 / 1080.0;
        let expires_at = if step.timed_override && step.duration_override_ms > 0 {
            Some(Instant::now() + Duration::from_millis(step.duration_override_ms))
        } else {
            None
        };

        *HUD_DISPLAY.lock() = Some(HudDisplayState {
            owner_preset_id: Some(owner_preset_id),
            preset_id: Some(preset.id),
            text,
            text_color: preset.text_color,
            background_color: preset.background_color,
            background_opacity: preset.background_opacity.clamp(0.0, 1.0),
            rounded_background: preset.rounded_background,
            font_size: preset.font_size.max(1.0),
            x: (preset.x as f32 * scale_x).round() as i32,
            y: (preset.y as f32 * scale_y).round() as i32,
            width: ((preset.width.max(1)) as f32 * scale_x).round().max(1.0) as i32,
            height: ((preset.height.max(1)) as f32 * scale_y).round().max(1.0) as i32,
            auto_hide_on_owner_completion: expires_at.is_none(),
            expires_at,
        });
        Ok(())
    }

    fn toolbox_preview_display_from_preset(preset: HudPreset) -> HudDisplayState {
        let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) }.max(1);
        let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) }.max(1);
        let scale_x = screen_width as f32 / 1920.0;
        let scale_y = screen_height as f32 / 1080.0;
        HudDisplayState {
            owner_preset_id: None,
            preset_id: Some(preset.id),
            text: preset.text,
            text_color: preset.text_color,
            background_color: preset.background_color,
            background_opacity: preset.background_opacity.clamp(0.0, 1.0),
            rounded_background: preset.rounded_background,
            font_size: preset.font_size.max(1.0),
            x: (preset.x as f32 * scale_x).round() as i32,
            y: (preset.y as f32 * scale_y).round() as i32,
            width: ((preset.width.max(1)) as f32 * scale_x).round().max(1.0) as i32,
            height: ((preset.height.max(1)) as f32 * scale_y).round().max(1.0) as i32,
            auto_hide_on_owner_completion: false,
            expires_at: None,
        }
    }

    fn show_legacy_hud_text(owner_preset_id: u32, step: &MacroStep) {
        let text = if step.text_override.trim().is_empty() {
            step.key.trim().to_owned()
        } else {
            step.text_override.trim().to_owned()
        };
        let trimmed = text.trim().to_owned();
        if trimmed.is_empty() {
            hide_hud_now();
            return;
        }
        *HUD_DISPLAY.lock() = Some(HudDisplayState {
            owner_preset_id: Some(owner_preset_id),
            preset_id: None,
            text: trimmed,
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
            background_opacity: 0.72,
            rounded_background: true,
            font_size: 28.0,
            x: 660,
            y: 36,
            width: 600,
            height: 80,
            auto_hide_on_owner_completion: true,
            expires_at: if step.timed_override && step.duration_override_ms > 0 {
                Some(Instant::now() + Duration::from_millis(step.duration_override_ms))
            } else {
                None
            },
        });
    }

    fn trigger_hud_display(owner_preset_id: u32, step: &MacroStep) {
        if show_hud_preset(owner_preset_id, step).is_err() {
            show_legacy_hud_text(owner_preset_id, step);
        }
        wake_command_queue();
    }

    fn hide_hud_now() {
        *HUD_DISPLAY.lock() = None;
        wake_command_queue();
    }

    fn hide_toolbox_for_owner(owner_preset_id: u32) {
        let mut guard = HUD_DISPLAY.lock();
        if let Some(active) = guard.as_ref()
            && active.owner_preset_id == Some(owner_preset_id)
            && active.auto_hide_on_owner_completion
        {
            *guard = None;
        }
    }

    fn apply_lock_keys(keys: &[String], preset_id: Option<u32>) {
        let keys_to_release = {
            let mut to_release = Vec::new();
            let mut hook_state = HOOK_STATE.lock();
            for key in keys {
                let already_locked = hook_state
                    .locked_inputs
                    .get(key)
                    .copied()
                    .unwrap_or_default()
                    > 0;
                if !already_locked && hook_state.held_inputs.contains(key.as_str()) {
                    to_release.push(key.clone());
                }
                *hook_state.locked_inputs.entry(key.clone()).or_insert(0) += 1;
                if let Some(preset_id) = preset_id
                    && let Some(active) = hook_state.active_hold_macros.get_mut(&preset_id)
                    && !active
                        .locked_keys
                        .iter()
                        .any(|existing| existing.eq_ignore_ascii_case(key))
                {
                    active.locked_keys.push(key.clone());
                }
            }
            to_release
        };

        for key in keys_to_release {
            let _ = send_key_event(&MacroStep {
                key,
                action: MacroAction::KeyUp,
                delay_ms: 0,
                x: 0,
                y: 0,
                ..MacroStep::default()
            });
        }
    }

    fn apply_unlock_keys(keys: &[String], preset_id: Option<u32>) {
        let keys_to_restore = {
            let mut to_restore = Vec::new();
            let mut hook_state = HOOK_STATE.lock();
            for key in keys {
                let mut should_restore = false;
                if let Some(preset_id) = preset_id
                    && let Some(active) = hook_state.active_hold_macros.get_mut(&preset_id)
                {
                    active
                        .locked_keys
                        .retain(|locked| !locked.eq_ignore_ascii_case(key));
                }
                if let Some(count) = hook_state.locked_inputs.get_mut(key) {
                    if *count > 1 {
                        *count -= 1;
                    } else {
                        hook_state.locked_inputs.remove(key);
                        should_restore = hook_state.held_inputs.contains(key.as_str());
                    }
                }
                if should_restore {
                    to_restore.push(key.clone());
                }
            }
            to_restore
        };

        for key in keys_to_restore {
            let _ = send_key_event(&MacroStep {
                key,
                action: MacroAction::KeyDown,
                delay_ms: 0,
                x: 0,
                y: 0,
                ..MacroStep::default()
            });
        }
    }

    fn apply_lock_mouse(preset_id: Option<u32>) {
        let buttons_to_release = {
            let mut hook_state = HOOK_STATE.lock();
            let first_lock = hook_state.locked_mouse_count == 0;
            hook_state.locked_mouse_count = hook_state.locked_mouse_count.saturating_add(1);
            if let Some(preset_id) = preset_id
                && let Some(active) = hook_state.active_hold_macros.get_mut(&preset_id)
            {
                active.locked_mouse_count = active.locked_mouse_count.saturating_add(1);
            }
            if first_lock {
                hook_state
                    .held_mouse_buttons
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        };

        for button in buttons_to_release {
            let action = match button.as_str() {
                "MouseLeft" => MacroAction::MouseLeftUp,
                "MouseRight" => MacroAction::MouseRightUp,
                "MouseMiddle" => MacroAction::MouseMiddleUp,
                "MouseX1" => MacroAction::MouseX1Up,
                "MouseX2" => MacroAction::MouseX2Up,
                _ => continue,
            };
            let _ = send_mouse_event(&MacroStep {
                action,
                ..MacroStep::default()
            });
        }
    }

    fn apply_unlock_mouse(preset_id: Option<u32>) {
        let should_restore = {
            let mut hook_state = HOOK_STATE.lock();
            if let Some(preset_id) = preset_id
                && let Some(active) = hook_state.active_hold_macros.get_mut(&preset_id)
                && active.locked_mouse_count > 0
            {
                active.locked_mouse_count -= 1;
            }
            if hook_state.locked_mouse_count > 0 {
                hook_state.locked_mouse_count -= 1;
            }
            hook_state.locked_mouse_count == 0
        };

        if should_restore {
            restore_physical_mouse_buttons();
        }
    }

    fn restore_physical_mouse_buttons() {
        for (vk, action) in [
            (0x01, MacroAction::MouseLeftDown),
            (0x02, MacroAction::MouseRightDown),
            (0x04, MacroAction::MouseMiddleDown),
            (0x05, MacroAction::MouseX1Down),
            (0x06, MacroAction::MouseX2Down),
        ] {
            let is_down = unsafe { GetAsyncKeyState(vk) } < 0;
            if is_down {
                let _ = send_mouse_event(&MacroStep {
                    action,
                    ..MacroStep::default()
                });
            }
        }
    }

    fn collect_macro_release_steps(steps: &[MacroStep]) -> Vec<MacroStep> {
        let mut held_keys = HashSet::new();
        let mut held_mouse = HashSet::new();

        for step in steps {
            if !step.enabled {
                continue;
            }
            match step.action {
                MacroAction::KeyDown => {
                    held_keys.insert(step.key.clone());
                }
                MacroAction::KeyUp | MacroAction::KeyPress => {
                    held_keys.remove(&step.key);
                }
                MacroAction::TypeText
                | MacroAction::Wait
                | MacroAction::ApplyWindowPreset
                | MacroAction::FocusWindowPreset
                | MacroAction::TriggerMacroPreset
                | MacroAction::TriggerCommandPreset
                | MacroAction::EnableCrosshairProfile
                | MacroAction::DisableCrosshair
                | MacroAction::EnablePinPreset
                | MacroAction::DisablePin
                | MacroAction::PlayMousePathPreset
                | MacroAction::ApplyMouseSensitivityPreset
                | MacroAction::EnableZoomPreset
                | MacroAction::DisableZoom
                | MacroAction::PlaySoundPreset
                | MacroAction::StartVisionSearch
                | MacroAction::TriggerVisionMove
                | MacroAction::StopVisionWait
                | MacroAction::StopVision => {}
                MacroAction::LoopStart
                | MacroAction::LoopEnd
                | MacroAction::StopIfTriggerPressedAgain
                | MacroAction::StopIfKeyPressed
                | MacroAction::ShowHud
                | MacroAction::HideHud
                | MacroAction::LockKeys
                | MacroAction::UnlockKeys
                | MacroAction::LockMouse
                | MacroAction::UnlockMouse
                | MacroAction::EnableMacroPreset
                | MacroAction::DisableMacroPreset => {}
                MacroAction::MouseLeftDown => {
                    held_mouse.insert(MacroAction::MouseLeftUp);
                }
                MacroAction::MouseLeftUp | MacroAction::MouseLeftClick => {
                    held_mouse.remove(&MacroAction::MouseLeftUp);
                }
                MacroAction::MouseRightDown => {
                    held_mouse.insert(MacroAction::MouseRightUp);
                }
                MacroAction::MouseRightUp | MacroAction::MouseRightClick => {
                    held_mouse.remove(&MacroAction::MouseRightUp);
                }
                MacroAction::MouseMiddleDown => {
                    held_mouse.insert(MacroAction::MouseMiddleUp);
                }
                MacroAction::MouseMiddleUp | MacroAction::MouseMiddleClick => {
                    held_mouse.remove(&MacroAction::MouseMiddleUp);
                }
                MacroAction::MouseX1Down => {
                    held_mouse.insert(MacroAction::MouseX1Up);
                }
                MacroAction::MouseX1Up | MacroAction::MouseX1Click => {
                    held_mouse.remove(&MacroAction::MouseX1Up);
                }
                MacroAction::MouseX2Down => {
                    held_mouse.insert(MacroAction::MouseX2Up);
                }
                MacroAction::MouseX2Up | MacroAction::MouseX2Click => {
                    held_mouse.remove(&MacroAction::MouseX2Up);
                }
                MacroAction::MouseWheelUp
                | MacroAction::MouseWheelDown
                | MacroAction::MouseMoveAbsolute
                | MacroAction::MouseMoveRelative => {}
                _ => {}
            }
        }

        let mut cleanup_steps = Vec::new();
        for key in held_keys {
            cleanup_steps.push(MacroStep {
                key,
                action: MacroAction::KeyUp,
                delay_ms: 0,
                x: 0,
                y: 0,
                ..MacroStep::default()
            });
        }
        for action in held_mouse {
            cleanup_steps.push(MacroStep {
                key: String::new(),
                action,
                delay_ms: 0,
                x: 0,
                y: 0,
                ..MacroStep::default()
            });
        }
        cleanup_steps
    }

    fn collect_macro_image_search_start_ids(steps: &[MacroStep]) -> Vec<u32> {
        let mut ids = HashSet::new();
        for step in steps {
            if !step.enabled {
                continue;
            }
            if step.action == MacroAction::StartVisionSearch
                && let Ok(preset_id) = step.key.trim().parse::<u32>()
            {
                ids.insert(preset_id);
            }
        }
        ids.into_iter().collect()
    }

    fn send_key_event(step: &MacroStep) -> Result<()> {
        match step.action {
            MacroAction::MouseLeftClick
            | MacroAction::MouseLeftDown
            | MacroAction::MouseLeftUp
            | MacroAction::MouseRightClick
            | MacroAction::MouseRightDown
            | MacroAction::MouseRightUp
            | MacroAction::MouseMiddleClick
            | MacroAction::MouseMiddleDown
            | MacroAction::MouseMiddleUp
            | MacroAction::MouseX1Click
            | MacroAction::MouseX1Down
            | MacroAction::MouseX1Up
            | MacroAction::MouseX2Click
            | MacroAction::MouseX2Down
            | MacroAction::MouseX2Up
            | MacroAction::MouseWheelUp
            | MacroAction::MouseWheelDown
            | MacroAction::MouseMoveAbsolute
            | MacroAction::MouseMoveRelative => return send_mouse_event(step),
            MacroAction::TypeText => return send_text_input(&step.key),
            MacroAction::Wait => return Ok(()),
            MacroAction::ApplyWindowPreset
            | MacroAction::FocusWindowPreset
            | MacroAction::TriggerMacroPreset
            | MacroAction::TriggerCommandPreset
            | MacroAction::EnableCrosshairProfile
            | MacroAction::DisableCrosshair
            | MacroAction::EnablePinPreset
            | MacroAction::DisablePin
            | MacroAction::PlayMousePathPreset
            | MacroAction::ApplyMouseSensitivityPreset
            | MacroAction::EnableZoomPreset
            | MacroAction::DisableZoom
            | MacroAction::PlaySoundPreset
            | MacroAction::StartVisionSearch
            | MacroAction::TriggerVisionMove
            | MacroAction::StopVisionWait
            | MacroAction::StopVision => return Ok(()),
            MacroAction::LoopStart
            | MacroAction::LoopEnd
            | MacroAction::StopIfTriggerPressedAgain
            | MacroAction::StopIfKeyPressed
            | MacroAction::ShowHud
            | MacroAction::HideHud
            | MacroAction::LockKeys
            | MacroAction::UnlockKeys
            | MacroAction::LockMouse
            | MacroAction::UnlockMouse
            | MacroAction::EnableMacroPreset
            | MacroAction::DisableMacroPreset => return Ok(()),
            MacroAction::KeyPress | MacroAction::KeyDown | MacroAction::KeyUp => {}
            _ => return Ok(()),
        }

        let Some(vk) = hotkey::key_name_to_vk(&step.key) else {
            bail!("Unsupported macro key: {}", step.key);
        };
        let scan = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) };
        if scan == 0 {
            bail!("Unsupported macro key scan code: {}", step.key);
        }
        let base_flags = KEYEVENTF_SCANCODE
            | if is_extended_key(vk) {
                KEYEVENTF_EXTENDEDKEY
            } else {
                Default::default()
            };
        let key_down = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: scan as u16,
                    dwFlags: base_flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let key_up = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: scan as u16,
                    dwFlags: base_flags | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            let inputs: Vec<INPUT> = match step.action {
                MacroAction::KeyPress => vec![key_down, key_up],
                MacroAction::KeyDown => vec![key_down],
                MacroAction::KeyUp => vec![key_up],
                _ => unreachable!("mouse actions are handled earlier"),
            };
            let sent = SendInput(&inputs, size_of::<INPUT>() as i32);
            if sent == 0 {
                bail!("SendInput failed");
            }
        }
        Ok(())
    }

    fn send_text_input(text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        let mut inputs = Vec::with_capacity(text.encode_utf16().count() * 2);
        for unit in text.encode_utf16() {
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: unit,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: unit,
                        dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
        }

        unsafe {
            let sent = SendInput(&inputs, size_of::<INPUT>() as i32);
            if sent == 0 {
                bail!("SendInput failed");
            }
        }

        Ok(())
    }

    

    fn send_mouse_input(dw_flags: MOUSE_EVENT_FLAGS, mouse_data: u32) -> Result<()> {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: mouse_data,
                    dwFlags: dw_flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            let sent = SendInput(&[input], size_of::<INPUT>() as i32);
            if sent == 0 {
                bail!("SendInput failed");
            }
        }
        Ok(())
    }

    fn send_mouse_event(step: &MacroStep) -> Result<()> {
        match step.action {
            MacroAction::MouseMoveAbsolute => {
                return send_mouse_move_absolute(step.x, step.y);
            }
            MacroAction::MouseMoveRelative => {
                return send_mouse_move_relative(step.x, step.y);
            }
            MacroAction::MouseLeftClick => {
                send_mouse_input(MOUSEEVENTF_LEFTDOWN, 0)?;
                thread::sleep(Duration::from_millis(16));
                return send_mouse_input(MOUSEEVENTF_LEFTUP, 0);
            }
            MacroAction::MouseRightClick => {
                send_mouse_input(MOUSEEVENTF_RIGHTDOWN, 0)?;
                thread::sleep(Duration::from_millis(16));
                return send_mouse_input(MOUSEEVENTF_RIGHTUP, 0);
            }
            MacroAction::MouseMiddleClick => {
                send_mouse_input(MOUSEEVENTF_MIDDLEDOWN, 0)?;
                thread::sleep(Duration::from_millis(16));
                return send_mouse_input(MOUSEEVENTF_MIDDLEUP, 0);
            }
            MacroAction::MouseX1Click => {
                send_mouse_input(MOUSEEVENTF_XDOWN, XBUTTON1_DATA as u32)?;
                thread::sleep(Duration::from_millis(16));
                return send_mouse_input(MOUSEEVENTF_XUP, XBUTTON1_DATA as u32);
            }
            MacroAction::MouseX2Click => {
                send_mouse_input(MOUSEEVENTF_XDOWN, XBUTTON2_DATA as u32)?;
                thread::sleep(Duration::from_millis(16));
                return send_mouse_input(MOUSEEVENTF_XUP, XBUTTON2_DATA as u32);
            }
            _ => {}
        }

        let (flags, mouse_data) = match step.action {
            MacroAction::MouseLeftDown => (MOUSEEVENTF_LEFTDOWN, 0),
            MacroAction::MouseLeftUp => (MOUSEEVENTF_LEFTUP, 0),
            MacroAction::MouseRightDown => (MOUSEEVENTF_RIGHTDOWN, 0),
            MacroAction::MouseRightUp => (MOUSEEVENTF_RIGHTUP, 0),
            MacroAction::MouseMiddleDown => (MOUSEEVENTF_MIDDLEDOWN, 0),
            MacroAction::MouseMiddleUp => (MOUSEEVENTF_MIDDLEUP, 0),
            MacroAction::MouseX1Down => (MOUSEEVENTF_XDOWN, XBUTTON1_DATA as u32),
            MacroAction::MouseX1Up => (MOUSEEVENTF_XUP, XBUTTON1_DATA as u32),
            MacroAction::MouseX2Down => (MOUSEEVENTF_XDOWN, XBUTTON2_DATA as u32),
            MacroAction::MouseX2Up => (MOUSEEVENTF_XUP, XBUTTON2_DATA as u32),
            MacroAction::MouseWheelUp => (MOUSEEVENTF_WHEEL, 120u32),
            MacroAction::MouseWheelDown => (MOUSEEVENTF_WHEEL, (-120i32) as u32),
            _ => bail!("Unsupported mouse action"),
        };

        send_mouse_input(flags, mouse_data)
    }

    fn send_mouse_move_absolute(x: i32, y: i32) -> Result<()> {
        let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) }.max(1);
        let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) }.max(1);
        let normalized_x = ((x.clamp(0, screen_w - 1) as i64) * 65535 / (screen_w - 1).max(1) as i64) as i32;
        let normalized_y = ((y.clamp(0, screen_h - 1) as i64) * 65535 / (screen_h - 1).max(1) as i64) as i32;
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: normalized_x,
                    dy: normalized_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            let sent = SendInput(&[input], size_of::<INPUT>() as i32);
            if sent == 0 {
                let _ = SetCursorPos(x, y);
            }
        }
        Ok(())
    }

    fn settle_image_search_mouse_move(
        x: i32,
        y: i32,
        move_passes: u8,
        move_delay_ms: u64,
    ) -> Result<()> {
        let attempts = move_passes.max(1) as usize;
        for attempt in 0..attempts {
            send_mouse_move_absolute(x, y)?;
            if attempt + 1 < attempts && move_delay_ms > 0 {
                thread::sleep(Duration::from_millis(move_delay_ms));
            }
        }
        Ok(())
    }

    fn settle_mouse_path_relative_segment(
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
        speed: f32,
        preset_id: Option<u32>,
        stop_immediately_on_retrigger: bool,
    ) -> Result<()> {
        let dx = to_x - from_x;
        let dy = to_y - from_y;
        let distance = (((dx * dx + dy * dy) as f32).sqrt()).max(1.0);
        let duration_ms = ((distance / (900.0 * speed)) * 1000.0)
            .round()
            .clamp(1.0, 5_000.0) as u64;
        let steps = (duration_ms / 8).max(1);
        let mut prev_x = from_x;
        let mut prev_y = from_y;
        for index in 1..=steps {
            if preset_id.is_some_and(|id| macro_stop_requested(id, stop_immediately_on_retrigger)) {
                return Ok(());
            }
            let t = index as f32 / steps as f32;
            let next_x = (from_x as f32 + dx as f32 * t).round() as i32;
            let next_y = (from_y as f32 + dy as f32 * t).round() as i32;
            send_mouse_move_relative(
                next_x - prev_x,
                next_y - prev_y,
            )?;
            prev_x = next_x;
            prev_y = next_y;
            if sleep_for_mouse_path_delay(preset_id, 8, stop_immediately_on_retrigger) {
                return Ok(());
            }
        }
        Ok(())
    }

    fn send_mouse_move_relative(dx: i32, dy: i32) -> Result<()> {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            let sent = SendInput(&[input], size_of::<INPUT>() as i32);
            if sent == 0 {
                let mut point = POINT::default();
                let _ = GetCursorPos(&mut point);
                let _ = SetCursorPos(point.x + dx, point.y + dy);
            }
        }
        Ok(())
    }

    fn send_mouse_left_click() -> Result<()> {
        send_mouse_input(MOUSEEVENTF_LEFTDOWN, 0)?;
        thread::sleep(Duration::from_millis(16));
        send_mouse_input(MOUSEEVENTF_LEFTUP, 0)
    }

    fn send_mouse_left_click_backend() -> Result<()> {
        send_mouse_left_click()
    }

    #[derive(Clone, Copy, Debug)]
    struct TemplateMatchHit {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        scale: f32,
        confidence: f32,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct VisionRegion {
        left: i32,
        top: i32,
        width: i32,
        height: i32,
        is_circle: bool,
        angle_offset_deg: Option<f32>,
        angle_span_deg: Option<f32>,
    }

    fn rgba_to_color_mat(rgba: &[u8], width: usize, height: usize) -> Result<Mat> {
        if !HOOK_STATE.lock().opencv_dll_path.exists() {
            bail!("OpenCV library not found. Please install it in Settings.");
        }
        let expected_len = width
            .checked_mul(height)
            .and_then(|value| value.checked_mul(4))
            .context("Image buffer is too large.")?;
        if rgba.len() != expected_len {
            bail!("Image buffer size does not match width/height.");
        }
        let flat = Mat::from_slice(rgba).context("Failed to create OpenCV Mat from RGBA slice.")?;
        let rgba_mat = flat
            .reshape(4, height as i32)
            .context("Failed to reshape RGBA buffer into OpenCV Mat.")?;
        let mut bgr = Mat::default();
        imgproc::cvt_color(&rgba_mat, &mut bgr, imgproc::COLOR_RGBA2BGR, 0)
            .context("Failed to convert RGBA image to BGR.")?;
        Ok(bgr)
    }

    fn rgba_to_gray_mat(rgba: &[u8], width: usize, height: usize) -> Result<Mat> {
        let rgba_mat = rgba_to_color_mat(rgba, width, height)?;
        let mut gray = Mat::default();
        imgproc::cvt_color(&rgba_mat, &mut gray, imgproc::COLOR_BGR2GRAY, 0)
            .context("Failed to convert BGR image to grayscale.")?;
        Ok(gray)
    }

    fn select_better_template_match(
        candidate: TemplateMatchHit,
        current: Option<TemplateMatchHit>,
        anchor_hint: Option<(i32, i32)>,
    ) -> bool {
        let Some(current) = current else {
            return true;
        };
        if candidate.confidence > current.confidence + 0.002 {
            return true;
        }
        if current.confidence > candidate.confidence + 0.002 {
            return false;
        }
        if let Some((anchor_x, anchor_y)) = anchor_hint {
            let candidate_center_x = candidate.x + candidate.width / 2;
            let candidate_center_y = candidate.y + candidate.height / 2;
            let current_center_x = current.x + current.width / 2;
            let current_center_y = current.y + current.height / 2;
            let candidate_distance =
                (candidate_center_x - anchor_x).pow(2) + (candidate_center_y - anchor_y).pow(2);
            let current_distance =
                (current_center_x - anchor_x).pow(2) + (current_center_y - anchor_y).pow(2);
            return candidate_distance < current_distance;
        }
        false
    }

    fn find_template_match_opencv(
        screen: &window_list::ScreenCaptureFrame,
        template_rgba: &[u8],
        template_width: usize,
        template_height: usize,
        scales: &[f32],
        anchor_hint_screen: Option<(i32, i32)>,
        use_color_matching: bool,
        search_region: Option<&VisionRegion>,
    ) -> Result<Option<TemplateMatchHit>> {
        let screen_mat = if use_color_matching {
            rgba_to_color_mat(&screen.rgba, screen.width, screen.height)?
        } else {
            rgba_to_gray_mat(&screen.rgba, screen.width, screen.height)?
        };
        let template_mat = if use_color_matching {
            rgba_to_color_mat(template_rgba, template_width, template_height)?
        } else {
            rgba_to_gray_mat(template_rgba, template_width, template_height)?
        };
        let anchor_hint = anchor_hint_screen
            .map(|(screen_x, screen_y)| (screen_x - screen.screen_x, screen_y - screen.screen_y));
        let mut best_hit: Option<TemplateMatchHit> = None;

        for &scale in scales {
            let scaled_width = ((template_width as f32) * scale).round().max(1.0) as i32;
            let scaled_height = ((template_height as f32) * scale).round().max(1.0) as i32;
            if scaled_width > screen.width as i32 || scaled_height > screen.height as i32 {
                continue;
            }

            let scaled_template = if (scale - 1.0).abs() < f32::EPSILON {
                template_mat
                    .try_clone()
                    .context("Failed to clone template Mat.")?
            } else {
                let mut resized = Mat::default();
                imgproc::resize(
                    &template_mat,
                    &mut resized,
                    Size::new(scaled_width, scaled_height),
                    0.0,
                    0.0,
                    imgproc::INTER_LINEAR,
                )
                .context("Failed to resize template for OpenCV matching.")?;
                resized
            };

            let result_cols = screen_mat.cols() - scaled_template.cols() + 1;
            let result_rows = screen_mat.rows() - scaled_template.rows() + 1;
            if result_cols <= 0 || result_rows <= 0 {
                continue;
            }

            let mut result = Mat::default();
            imgproc::match_template(
                &screen_mat,
                &scaled_template,
                &mut result,
                imgproc::TM_CCOEFF_NORMED,
                &cv::no_array(),
            )
            .context("OpenCV matchTemplate failed.")?;

            let result_data = result
                .data_typed::<f32>()
                .context("OpenCV result matrix was not readable.")?;
            let result_width = result.cols().max(0) as usize;
            let result_height = result.rows().max(0) as usize;
            for y in 0..result_height {
                for x in 0..result_width {
                    let confidence = result_data[y * result_width + x];
                    let center_x = screen.screen_x + x as i32 + scaled_width / 2;
                    let center_y = screen.screen_y + y as i32 + scaled_height / 2;
                    if !image_search_region_contains_point(search_region, center_x, center_y) {
                        continue;
                    }
                    let candidate = TemplateMatchHit {
                        x: x as i32,
                        y: y as i32,
                        width: scaled_width,
                        height: scaled_height,
                        scale,
                        confidence,
                    };
                    if select_better_template_match(candidate, best_hit, anchor_hint) {
                        best_hit = Some(candidate);
                    }
                }
            }
        }

        Ok(best_hit)
    }

    fn configured_image_search_region(preset: &VisionPreset) -> Option<VisionRegion> {
        let (Some(region_x), Some(region_y), Some(region_width), Some(region_height)) = (
            preset.search_region_screen_x,
            preset.search_region_screen_y,
            preset.search_region_width,
            preset.search_region_height,
        ) else {
            return None;
        };
        if region_width <= 0 || region_height <= 0 {
            return None;
        }

        let (virtual_left, virtual_top, virtual_width, virtual_height) =
            window_list::virtual_screen_bounds();
        let virtual_right = virtual_left + virtual_width;
        let virtual_bottom = virtual_top + virtual_height;
        let left = region_x.max(virtual_left);
        let top = region_y.max(virtual_top);
        let right = (region_x + region_width).min(virtual_right);
        let bottom = (region_y + region_height).min(virtual_bottom);
        let width = right - left;
        let height = bottom - top;
        if width <= 0 || height <= 0 {
            return None;
        }
        Some(VisionRegion {
            left,
            top,
            width,
            height,
            is_circle: preset.search_region_is_circle,
            angle_offset_deg: None, angle_span_deg: None,
        })
    }

    fn expand_search_region_to_fit(
        region: VisionRegion,
        min_width: i32,
        min_height: i32,
    ) -> VisionRegion {
        let VisionRegion {
            left,
            top,
            width,
            height,
            is_circle,
            angle_offset_deg,
            angle_span_deg,
        } = region;
        let target_width = width.max(min_width.max(1));
        let target_height = height.max(min_height.max(1));
        if target_width == width && target_height == height {
            return region;
        }

        let center_x = left + width / 2;
        let center_y = top + height / 2;
        let mut next_left = center_x - target_width / 2;
        let mut next_top = center_y - target_height / 2;
        let (virtual_left, virtual_top, virtual_width, virtual_height) =
            window_list::virtual_screen_bounds();
        let virtual_right = virtual_left + virtual_width;
        let virtual_bottom = virtual_top + virtual_height;

        if next_left < virtual_left {
            next_left = virtual_left;
        }
        if next_top < virtual_top {
            next_top = virtual_top;
        }

        let mut next_right = (next_left + target_width).min(virtual_right);
        let mut next_bottom = (next_top + target_height).min(virtual_bottom);
        if next_right - next_left < target_width {
            next_left = (next_right - target_width).max(virtual_left);
            next_right = (next_left + target_width).min(virtual_right);
        }
        if next_bottom - next_top < target_height {
            next_top = (next_bottom - target_height).max(virtual_top);
            next_bottom = (next_top + target_height).min(virtual_bottom);
        }

        VisionRegion {
            left: next_left,
            top: next_top,
            width: (next_right - next_left).max(1),
            height: (next_bottom - next_top).max(1),
            is_circle,
            angle_offset_deg,
            angle_span_deg,
        }
    }

    fn image_search_region_contains_point(
        region: Option<&VisionRegion>,
        x: i32,
        y: i32,
    ) -> bool {
        let Some(region) = region else {
            return true;
        };
        let inside_rect = x >= region.left
            && y >= region.top
            && x < region.left + region.width
            && y < region.top + region.height;
        if !inside_rect {
            return false;
        }
        if !region.is_circle {
            return true;
        }

        let center_x = region.left as f32 + region.width as f32 * 0.5;
        let center_y = region.top as f32 + region.height as f32 * 0.5;
        let radius_x = (region.width as f32 * 0.5).max(1.0);
        let radius_y = (region.height as f32 * 0.5).max(1.0);
        let dx = (x as f32 + 0.5 - center_x) / radius_x;
        let dy = (y as f32 + 0.5 - center_y) / radius_y;
        dx * dx + dy * dy <= 1.0
    }

    #[derive(Clone, Copy, Debug)]
    struct ColorMatchHit {
        x: i32,
        y: i32,
        score: u32,
        distance_sq: i32,
        matched_color: RgbaColor,
    }

    fn find_color_match_in_range(
        screen: &window_list::ScreenCaptureFrame,
        targets: &[RgbaColor],
        tolerance: u8,
        x_start: usize,
        x_end: usize,
        region: Option<&VisionRegion>,
    ) -> Option<ColorMatchHit> {
        let width = screen.width as i32;
        let height = screen.height as i32;
        if width <= 0 || height <= 0 || targets.is_empty() {
            return None;
        }
        let x_start = x_start.min(screen.width);
        let x_end = x_end.min(screen.width);
        if x_start >= x_end {
            return None;
        }
        let center_x = width / 2;
        let center_y = height / 2;
        let tolerance = tolerance as i16;
        let mut best_hit: Option<ColorMatchHit> = None;

        for y in 0..height {
            for x in x_start as i32..x_end as i32 {
                let candidate = color_match_candidate_for_pixel(
                    screen, targets, tolerance, x, y, center_x, center_y, region,
                );
                if let Some(candidate) = candidate {
                    let replace = match best_hit {
                        None => true,
                        Some(current) if candidate.score < current.score => true,
                        Some(current) if candidate.score == current.score => {
                            candidate.distance_sq < current.distance_sq
                        }
                        _ => false,
                    };
                    if replace {
                        best_hit = Some(candidate);
                    }
                }
            }
        }

        best_hit
    }

    fn color_match_candidate_for_pixel(
        screen: &window_list::ScreenCaptureFrame,
        targets: &[RgbaColor],
        tolerance: i16,
        x: i32,
        y: i32,
        reference_x: i32,
        reference_y: i32,
        region: Option<&VisionRegion>,
    ) -> Option<ColorMatchHit> {
        if x < 0 || y < 0 || x >= screen.width as i32 || y >= screen.height as i32 {
            return None;
        }
        if !image_search_region_contains_point(region, screen.screen_x + x, screen.screen_y + y) {
            return None;
        }
        let index = ((y as usize) * screen.width + (x as usize)) * 4;
        if index + 3 >= screen.rgba.len() {
            return None;
        }
        let r = screen.rgba[index] as i16;
        let g = screen.rgba[index + 1] as i16;
        let b = screen.rgba[index + 2] as i16;
        let mut best_hit: Option<ColorMatchHit> = None;
        for target in targets {
            let dr = (r - target.r as i16).abs();
            let dg = (g - target.g as i16).abs();
            let db = (b - target.b as i16).abs();
            if dr > tolerance || dg > tolerance || db > tolerance {
                continue;
            }

            let score = (dr as u32) + (dg as u32) + (db as u32);
            let distance_sq = (x - reference_x).pow(2) + (y - reference_y).pow(2);
            let candidate = ColorMatchHit {
                x,
                y,
                score,
                distance_sq,
                matched_color: *target,
            };
            let replace = match best_hit {
                None => true,
                Some(current) if candidate.score < current.score => true,
                Some(current) if candidate.score == current.score => {
                    candidate.distance_sq < current.distance_sq
                }
                _ => false,
            };
            if replace {
                best_hit = Some(candidate);
            }
        }
        best_hit
    }

    fn find_color_match_from_anchor(
        screen: &window_list::ScreenCaptureFrame,
        targets: &[RgbaColor],
        tolerance: u8,
        anchor_x: i32,
        anchor_y: i32,
        region: Option<&VisionRegion>,
    ) -> Option<ColorMatchHit> {
        let width = screen.width as i32;
        let height = screen.height as i32;
        if width <= 0 || height <= 0 || targets.is_empty() {
            return None;
        }
        if anchor_x < 0 || anchor_y < 0 || anchor_x >= width || anchor_y >= height {
            return None;
        }

        let tolerance = tolerance as i16;
        let max_radius = (anchor_x)
            .max(width - 1 - anchor_x)
            .max(anchor_y)
            .max(height - 1 - anchor_y);

        for radius in 0..=max_radius {
            let left = (anchor_x - radius).max(0);
            let right = (anchor_x + radius).min(width - 1);
            let top = (anchor_y - radius).max(0);
            let bottom = (anchor_y + radius).min(height - 1);
            let mut best_in_radius: Option<ColorMatchHit> = None;

            for x in left..=right {
                for y in [top, bottom] {
                    if let Some(candidate) = color_match_candidate_for_pixel(
                        screen, targets, tolerance, x, y, anchor_x, anchor_y, region,
                    ) {
                        let replace = match best_in_radius {
                            None => true,
                            Some(current) if candidate.score < current.score => true,
                            Some(current) if candidate.score == current.score => {
                                candidate.distance_sq < current.distance_sq
                            }
                            _ => false,
                        };
                        if replace {
                            best_in_radius = Some(candidate);
                        }
                    }
                }
            }

            if top + 1 <= bottom.saturating_sub(1) {
                for y in (top + 1)..bottom {
                    for x in [left, right] {
                        if let Some(candidate) = color_match_candidate_for_pixel(
                            screen, targets, tolerance, x, y, anchor_x, anchor_y, region,
                        ) {
                            let replace = match best_in_radius {
                                None => true,
                                Some(current) if candidate.score < current.score => true,
                                Some(current) if candidate.score == current.score => {
                                    candidate.distance_sq < current.distance_sq
                                }
                                _ => false,
                            };
                            if replace {
                                best_in_radius = Some(candidate);
                            }
                        }
                    }
                }
            }

            if best_in_radius.is_some() {
                return best_in_radius;
            }
        }

        None
    }

    fn find_color_match(
        screen: &window_list::ScreenCaptureFrame,
        targets: &[RgbaColor],
        tolerance: u8,
        region: Option<&VisionRegion>,
    ) -> Option<ColorMatchHit> {
        find_color_match_in_range(screen, targets, tolerance, 0, screen.width, region)
    }

    fn find_dual_color_midpoint_match(
        screen: &window_list::ScreenCaptureFrame,
        targets: &[RgbaColor],
        tolerance: u8,
        region: Option<&VisionRegion>,
    ) -> Option<ColorMatchHit> {
        let mid = (screen.width / 2).max(1);
        let (left_hit, right_hit) = thread::scope(|scope| {
            let left = scope
                .spawn(|| find_color_match_in_range(screen, targets, tolerance, 0, mid, region));
            let right = scope.spawn(|| {
                find_color_match_in_range(screen, targets, tolerance, mid, screen.width, region)
            });
            (left.join().ok().flatten(), right.join().ok().flatten())
        });
        match (left_hit, right_hit) {
            (Some(left), Some(right)) => Some(ColorMatchHit {
                x: ((left.x + right.x) / 2).max(0),
                y: ((left.y + right.y) / 2).max(0),
                score: left.score.min(right.score),
                distance_sq: left.distance_sq.min(right.distance_sq),
                matched_color: left.matched_color,
            }),
            (Some(hit), None) | (None, Some(hit)) => Some(hit),
            (None, None) => None,
        }
    }

    fn image_search_target_colors(preset: &VisionPreset) -> Vec<RgbaColor> {
        if !preset.target_colors.is_empty() {
            return preset.target_colors.clone();
        }
        preset.target_color.into_iter().collect()
    }

    fn capture_near_last_image_search_region(
        capture_x: i32,
        capture_y: i32,
        template_width: usize,
        template_height: usize,
    ) -> Option<window_list::ScreenCaptureFrame> {
        let (virtual_left, virtual_top, virtual_width, virtual_height) =
            window_list::virtual_screen_bounds();
        let padding_x = ((template_width as i32) * 10).max(280);
        let padding_y = ((template_height as i32) * 6).max(140);
        let desired_left = capture_x - padding_x;
        let desired_top = capture_y - padding_y;
        let desired_right = capture_x + template_width as i32 + padding_x;
        let desired_bottom = capture_y + template_height as i32 + padding_y;
        let left = desired_left.max(virtual_left);
        let top = desired_top.max(virtual_top);
        let right = desired_right.min(virtual_left + virtual_width);
        let bottom = desired_bottom.min(virtual_top + virtual_height);
        let width = (right - left).max(template_width as i32);
        let height = (bottom - top).max(template_height as i32);
        window_list::capture_virtual_screen_region(left, top, width, height)
    }

    fn find_template_match_exact_rgba(
        screen: &window_list::ScreenCaptureFrame,
        template_rgba: &[u8],
        template_width: usize,
        template_height: usize,
        max_average_diff: f32,
        anchor_hint_screen: Option<(i32, i32)>,
        search_region: Option<&VisionRegion>,
    ) -> Option<TemplateMatchHit> {
        if template_width == 0
            || template_height == 0
            || screen.width < template_width
            || screen.height < template_height
        {
            return None;
        }

        let anchor_hint =
            anchor_hint_screen.map(|(x, y)| (x - screen.screen_x, y - screen.screen_y));
        let mut best_hit: Option<TemplateMatchHit> = None;

        for y in 0..=(screen.height - template_height) {
            for x in 0..=(screen.width - template_width) {
                let center_x = screen.screen_x + x as i32 + (template_width as i32 / 2);
                let center_y = screen.screen_y + y as i32 + (template_height as i32 / 2);
                if !image_search_region_contains_point(search_region, center_x, center_y) {
                    continue;
                }
                let mut total_diff = 0u64;
                let mut over_budget = false;
                for row in 0..template_height {
                    let screen_row = ((y + row) * screen.width + x) * 4;
                    let template_row = row * template_width * 4;
                    for col in 0..template_width {
                        let screen_idx = screen_row + col * 4;
                        let template_idx = template_row + col * 4;
                        let dr =
                            screen.rgba[screen_idx].abs_diff(template_rgba[template_idx]) as u64;
                        let dg = screen.rgba[screen_idx + 1]
                            .abs_diff(template_rgba[template_idx + 1])
                            as u64;
                        let db = screen.rgba[screen_idx + 2]
                            .abs_diff(template_rgba[template_idx + 2])
                            as u64;
                        total_diff += dr + dg + db;
                        let processed = ((row * template_width) + (col + 1)) as f32;
                        let average = total_diff as f32 / processed / 3.0;
                        if average > max_average_diff {
                            over_budget = true;
                            break;
                        }
                    }
                    if over_budget {
                        break;
                    }
                }
                if over_budget {
                    continue;
                }

                let pixel_count = (template_width * template_height) as f32;
                let avg_diff = total_diff as f32 / pixel_count / 3.0;
                let candidate = TemplateMatchHit {
                    x: x as i32,
                    y: y as i32,
                    width: template_width as i32,
                    height: template_height as i32,
                    scale: 1.0,
                    confidence: (1.0 - (avg_diff / 255.0)).clamp(0.0, 1.0),
                };
                if select_better_template_match(candidate, best_hit, anchor_hint) {
                    best_hit = Some(candidate);
                }
            }
        }

        best_hit
    }

    fn image_search_template_file(preset_id: u32) -> PathBuf {
        let hook_state = HOOK_STATE.lock();
        hook_state
            .vision_dir
            .join(format!("preset-{preset_id}.png"))
    }

    fn run_vision_once_with_options(
        preset: &VisionPreset,
        move_cursor: bool,
        fire_click: bool,
    ) -> Result<VisionRunOutcome> {
        if preset.use_color_matching {
            let target_colors = image_search_target_colors(preset);
            if target_colors.is_empty() {
                bail!("No target colors have been picked yet.");
            }
            let configured_region = configured_image_search_region(preset);
            let screen = if let Some(region) = configured_region {
                window_list::capture_virtual_screen_region(
                    region.left,
                    region.top,
                    region.width,
                    region.height,
                )
                .context("Failed to capture the selected search area")?
            } else if preset.target_window_title.is_some()
                || !preset.extra_target_window_titles.is_empty()
            {
                window_list::capture_window_region_with_candidates(
                    preset.target_window_title.as_deref(),
                    &preset.extra_target_window_titles,
                    preset.match_duplicate_window_titles,
                )
                .context("Failed to capture the target window")?
            } else {
                let (left, top, width, height) = window_list::virtual_screen_bounds();
                window_list::capture_virtual_screen_region(left, top, width, height)
                    .context("Failed to capture the screen")?
            };

            let anchor = if preset.color_priority_from_anchor {
                Some(
                    preset
                        .color_priority_anchor_screen_x
                        .zip(preset.color_priority_anchor_screen_y)
                        .ok_or_else(|| anyhow::anyhow!("No priority point has been picked yet."))?,
                )
            } else {
                None
            };
            let hit = if let Some((anchor_x, anchor_y)) = anchor {
                find_color_match_from_anchor(
                    &screen,
                    &target_colors,
                    preset.color_tolerance,
                    anchor_x - screen.screen_x,
                    anchor_y - screen.screen_y,
                    configured_region.as_ref(),
                )
            } else if preset.dual_color_scan_midpoint {
                find_dual_color_midpoint_match(
                    &screen,
                    &target_colors,
                    preset.color_tolerance,
                    configured_region.as_ref(),
                )
            } else {
                find_color_match(
                    &screen,
                    &target_colors,
                    preset.color_tolerance,
                    configured_region.as_ref(),
                )
            };
            let Some(hit) = hit else {
                let color_list = target_colors
                    .iter()
                    .map(|color| format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b))
                    .collect::<Vec<_>>()
                    .join(", ");
                return Ok(VisionRunOutcome {
                    matched: false,
                    status: format!(
                        "No color match found for [{color_list}] with tolerance {}.",
                        preset.color_tolerance
                    ),
                });
            };

            let center_x = screen.screen_x + hit.x;
            let center_y = screen.screen_y + hit.y;
            let moved_x = center_x + preset.move_offset_x;
            let moved_y = center_y + preset.move_offset_y;
            if move_cursor {
                settle_image_search_mouse_move(moved_x, moved_y, preset.non_interception_move_passes, preset.non_interception_move_delay_ms,
                )?;
            }
            if fire_click {
                thread::sleep(Duration::from_millis(12));
                send_mouse_left_click_backend()?;
            }
            return Ok(VisionRunOutcome {
                matched: true,
                status: if anchor.is_some() {
                    format!(
                        "Matched colors from priority point at {moved_x}, {moved_y} with tolerance {} and offset {:+}, {:+}.",
                        preset.color_tolerance, preset.move_offset_x, preset.move_offset_y
                    )
                } else if preset.dual_color_scan_midpoint {
                    format!(
                        "Matched colors midpoint at {moved_x}, {moved_y} with tolerance {} and offset {:+}, {:+}.",
                        preset.color_tolerance, preset.move_offset_x, preset.move_offset_y
                    )
                } else {
                    format!(
                        "Matched color #{:02X}{:02X}{:02X} at {moved_x}, {moved_y} with tolerance {} and offset {:+}, {:+}.",
                        hit.matched_color.r,
                        hit.matched_color.g,
                        hit.matched_color.b,
                        preset.color_tolerance,
                        preset.move_offset_x,
                        preset.move_offset_y
                    )
                },
            });
        }

        let template_file = image_search_template_file(preset.id);
        if !template_file.exists() {
            bail!("No image template has been captured yet.");
        }
        let template = image::open(&template_file)
            .with_context(|| format!("Failed to open template {}", template_file.display()))?
            .to_rgba8();
        let template_width = template.width() as usize;
        let template_height = template.height() as usize;
        let template_rgba = template.into_raw();
        let anchor_hint = match (preset.last_capture_screen_x, preset.last_capture_screen_y) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        };
        let configured_region = configured_image_search_region(preset);

        let used_roi_capture = configured_region.is_some()
            || (preset.target_window_title.is_none()
                && preset.extra_target_window_titles.is_empty()
                && anchor_hint.is_some());
        let screen = if let Some(region) = configured_region {
            let region =
                expand_search_region_to_fit(region, template_width as i32, template_height as i32);
            window_list::capture_virtual_screen_region(
                region.left,
                region.top,
                region.width,
                region.height,
            )
            .context("Failed to capture the selected search area")?
        } else if preset.target_window_title.is_some()
            || !preset.extra_target_window_titles.is_empty()
        {
            window_list::capture_window_region_with_candidates(
                preset.target_window_title.as_deref(),
                &preset.extra_target_window_titles,
                preset.match_duplicate_window_titles,
            )
            .context("Failed to capture the target window")?
        } else if let Some((capture_x, capture_y)) = anchor_hint {
            capture_near_last_image_search_region(
                capture_x,
                capture_y,
                template_width,
                template_height,
            )
            .context("Failed to capture the area near the original template")?
        } else {
            let (left, top, width, height) = window_list::virtual_screen_bounds();
            window_list::capture_virtual_screen_region(left, top, width, height)
                .context("Failed to capture the screen")?
        };

        let fallback_average_diff =
            ((1.0 - preset.confidence_threshold.clamp(0.35, 0.99)) * 48.0).clamp(2.0, 18.0);
        let exact_hit = if used_roi_capture
            || configured_region.is_some()
            || (screen.width <= 960 && screen.height <= 960)
        {
            find_template_match_exact_rgba(
                &screen,
                &template_rgba,
                template_width,
                template_height,
                fallback_average_diff,
                anchor_hint,
                configured_region.as_ref(),
            )
        } else {
            None
        };
        let scales = [1.0_f32, 0.9, 1.1, 0.8, 1.2, 1.33];
        let opencv_hit = find_template_match_opencv(
            &screen,
            &template_rgba,
            template_width,
            template_height,
            &scales,
            anchor_hint,
            false,
            configured_region.as_ref(),
        )?;
        let hit = match (exact_hit, opencv_hit) {
            (Some(exact), Some(opencv)) => {
                if exact.confidence > opencv.confidence + 0.08 {
                    exact
                } else {
                    opencv
                }
            }
            (Some(exact), None) => exact,
            (None, Some(opencv)) => opencv,
            (None, None) => {
                if configured_region.is_some() {
                    return Ok(VisionRunOutcome {
                        matched: false,
                        status: "No match found inside the selected search area.".to_owned(),
                    });
                }
                if used_roi_capture {
                    return Ok(VisionRunOutcome {
                        matched: false,
                        status: "No match found near the captured area.".to_owned(),
                    });
                }
                return Ok(VisionRunOutcome {
                    matched: false,
                    status: "No match found on screen.".to_owned(),
                });
            }
        };

        let center_x = screen.screen_x + hit.x + (hit.width / 2);
        let center_y = screen.screen_y + hit.y + (hit.height / 2);
        let moved_x = center_x + preset.move_offset_x;
        let moved_y = center_y + preset.move_offset_y;
        let required_confidence = preset.confidence_threshold.clamp(0.35, 0.99);
        if hit.confidence < required_confidence {
            return Ok(VisionRunOutcome {
                matched: false,
                status: format!(
                    "Best match near {moved_x}, {moved_y} scored {:.3} at scale {:.2}x, below threshold {:.2}.",
                    hit.confidence, hit.scale, required_confidence
                ),
            });
        }
        if move_cursor {
            settle_image_search_mouse_move(moved_x, moved_y, preset.non_interception_move_passes, preset.non_interception_move_delay_ms,
            )?;
        }
        if fire_click {
            thread::sleep(Duration::from_millis(12));
            send_mouse_left_click_backend()?;
        }
        Ok(VisionRunOutcome {
            matched: true,
            status: format!(
                "OpenCV matched at {moved_x}, {moved_y} with confidence {:.3} on {:.2}x (offset {:+}, {:+}).",
                hit.confidence, hit.scale, preset.move_offset_x, preset.move_offset_y
            ),
        })
    }

    fn run_vision_once(preset: &VisionPreset) -> Result<String> {
        run_vision_once_with_options(preset, true, preset.click_after_move)
            .map(|outcome| outcome.status)
    }

    fn vision_preset_by_id(spec: &str) -> Result<VisionPreset> {
        let preset_id = spec
            .trim()
            .parse::<u32>()
            .context("Vision preset id is invalid")?;
        HOOK_STATE
            .lock()
            .vision_presets
            .iter()
            .find(|preset| preset.id == preset_id)
            .cloned()
            .context("Vision preset was not found")
    }

    fn start_vision_following(spec: &str) -> Result<()> {
        let preset = vision_preset_by_id(spec)?;
        if image_search_following_is_active(preset.id) {
            return Ok(());
        }
        let ui_tx = HOOK_STATE.lock().ui_tx.clone();
        set_image_search_following_active(preset.id, true);
        thread::spawn(move || run_image_search_follow_loop(preset, ui_tx));
        Ok(())
    }

    fn stop_vision_following(spec: &str) -> Result<()> {
        let preset = vision_preset_by_id(spec)?;
        set_image_search_following_active(preset.id, false);
        Ok(())
    }

    fn stop_vision_following_ids(preset_ids: &[u32]) {
        for preset_id in preset_ids {
            set_image_search_following_active(*preset_id, false);
        }
    }

    fn trigger_vision_move(spec: &str) -> Result<()> {
        let preset = vision_preset_by_id(spec)?;
        let status = run_vision_once(&preset)?;
        if let Some(tx) = HOOK_STATE.lock().ui_tx.clone() {
            let _ = tx.send(UiCommand::VisionFinished(format!(
                "{}: {status}",
                preset.name
            )));
        }
        Ok(())
    }

    fn trigger_vision_move_with_options(
        preset: &VisionPreset,
        move_cursor: bool,
        wait_until_found: bool,
        trigger_macro_enabled: bool,
        trigger_macro_preset_id: Option<u32>,
        macro_preset_id: u32,
        press_locked_keys: &mut Vec<String>,
        press_locked_mouse_count: &mut usize,
        stop_immediately_on_retrigger: bool,
        target_window_title: Option<&str>,
        extra_target_window_titles: &[String],
        match_duplicate_window_titles: bool,
    ) -> MacroRunFlow {
        let ui_tx = HOOK_STATE.lock().ui_tx.clone();
        let wait_generation = image_search_wait_generation(preset.id);
        let mut sent_wait_status = false;
        loop {
            if !macro_runtime_target_matches(
                target_window_title,
                extra_target_window_titles,
                match_duplicate_window_titles,
            ) {
                return MacroRunFlow::StopExecution;
            }
            if stop_immediately_on_retrigger
                && STOP_REQUESTED_MACRO_PRESETS
                    .lock()
                    .contains(&macro_preset_id)
            {
                return MacroRunFlow::StopExecution;
            }
            if image_search_wait_generation(preset.id) != wait_generation {
                if let Some(tx) = ui_tx.as_ref() {
                    let _ = tx.send(UiCommand::VisionFinished(format!(
                        "{}: waiting cancelled.",
                        preset.name
                    )));
                }
                return MacroRunFlow::Continue;
            }

            let outcome = match run_vision_once_with_options(preset, move_cursor, false) {
                Ok(outcome) => outcome,
                Err(error) => {
                    eprintln!("Vision macro step failed: {error}");
                    return MacroRunFlow::Continue;
                }
            };

            if outcome.matched {
                if let Some(tx) = ui_tx.as_ref() {
                    let _ = tx.send(UiCommand::VisionFinished(format!(
                        "{}: {}",
                        preset.name, outcome.status
                    )));
                }
                if trigger_macro_enabled {
                    if let Some(trigger_preset_id) = trigger_macro_preset_id {
                        let _ = trigger_nested_macro_preset(
                            &trigger_preset_id.to_string(),
                            press_locked_keys,
                            press_locked_mouse_count,
                            stop_immediately_on_retrigger,
                            target_window_title,
                            extra_target_window_titles,
                            match_duplicate_window_titles,
                        );
                    }
                }
                return MacroRunFlow::Continue;
            }

            if !wait_until_found {
                return MacroRunFlow::Continue;
            }

            if !sent_wait_status {
                if let Some(tx) = ui_tx.as_ref() {
                    let _ = tx.send(UiCommand::VisionFinished(format!(
                        "{}: waiting...",
                        preset.name
                    )));
                }
                sent_wait_status = true;
            }

            thread::sleep(Duration::from_millis(25));
        }
    }

    fn stop_vision_waiting(spec: &str) -> Result<()> {
        let preset = vision_preset_by_id(spec)?;
        bump_image_search_wait_generation(preset.id);
        Ok(())
    }

    fn is_extended_key(vk: u32) -> bool {
        matches!(vk, 0x21..=0x28 | 0x2D | 0x2E | 0x5B | 0x5C)
    }

    fn internal_app_window_class(hwnd: HWND) -> Option<String> {
        unsafe {
            let mut buffer = [0u16; 256];
            let copied = GetClassNameW(hwnd, &mut buffer);
            if copied <= 0 {
                return None;
            }
            Some(String::from_utf16_lossy(&buffer[..copied as usize]))
        }
    }

    fn is_internal_app_window(hwnd: HWND) -> bool {
        internal_app_window_class(hwnd).is_some_and(|class_name| {
            matches!(
                class_name.as_str(),
                "CrosshairController" | "CrosshairOverlay" | "CrosshairToolbox" | "Magnifier"
            )
        })
    }

    fn window_belongs_to_current_process(hwnd: HWND) -> bool {
        unsafe {
            let mut pid = 0u32;
            let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
            pid != 0 && pid == GetCurrentProcessId()
        }
    }

    fn looks_like_main_ui_window(hwnd: HWND) -> bool {
        unsafe {
            if hwnd.0.is_null()
                || !window_belongs_to_current_process(hwnd)
                || is_internal_app_window(hwnd)
            {
                return false;
            }
            if GetAncestor(hwnd, GA_ROOT) != hwnd {
                return false;
            }
            if GetWindow(hwnd, GW_OWNER).is_ok_and(|owner| !owner.0.is_null()) {
                return false;
            }
            let style = windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::GWL_STYLE,
            ) as u32;
            (style & WS_OVERLAPPEDWINDOW.0) != 0 || (style & WS_CAPTION.0) != 0
        }
    }

    #[derive(Default)]
    struct AppUiWindowSearch {
        visible: Option<HWND>,
        hidden: Option<HWND>,
    }

    unsafe fn find_app_ui_window() -> Option<HWND> {
        let mut found = AppUiWindowSearch::default();
        let _ = windows::Win32::UI::WindowsAndMessaging::EnumWindows(
            Some(find_app_ui_window_proc),
            LPARAM((&mut found) as *mut _ as isize),
        );
        found.visible.or(found.hidden)
    }

    unsafe extern "system" fn find_app_ui_window_proc(
        hwnd: HWND,
        lparam: LPARAM,
    ) -> windows::core::BOOL {
        let found = &mut *(lparam.0 as *mut AppUiWindowSearch);
        if !looks_like_main_ui_window(hwnd) {
            return true.into();
        }
        if windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd).as_bool() {
            found.visible = Some(hwnd);
            false.into()
        } else {
            if found.hidden.is_none() {
                found.hidden = Some(hwnd);
            }
            true.into()
        }
    }

    fn is_ui_in_foreground() -> bool {
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground.0.is_null() {
                return false;
            }
            let root = GetAncestor(foreground, GA_ROOT);
            if root.0.is_null() {
                return false;
            }
            window_belongs_to_current_process(root) && !is_internal_app_window(root)
        }
    }

    fn is_click_inside_ui(pt: POINT) -> bool {
        unsafe {
            let hwnd = WindowFromPoint(pt);
            if hwnd.0.is_null() {
                return false;
            }
            let root = GetAncestor(hwnd, GA_ROOT);
            if root.0.is_null() {
                return false;
            }
            window_belongs_to_current_process(root) && !is_internal_app_window(root)
        }
    }

    fn hide_ui_window_native() {
        unsafe {
            let Some(app) = find_app_ui_window() else {
                return;
            };
            if app.0.is_null() {
                return;
            }
            let _ = ShowWindow(app, SW_HIDE);
        }
    }

    fn show_ui_window_native() {
        unsafe {
            let Some(app) = find_app_ui_window() else {
                return;
            };
            if app.0.is_null() {
                return;
            }
            let _ = ShowWindow(app, SW_SHOWNA);
        }
    }

    fn restore_ui_window_native() {
        unsafe {
            let Some(app) = find_app_ui_window() else {
                return;
            };
            if app.0.is_null() {
                return;
            }
            let _ = ShowWindow(app, SW_SHOWNA);
        }
    }

    fn apply_window_preset_for_macro(preset: &WindowPreset) -> Result<()> {
        window_preset::apply_window_preset_for_macro(preset)
    }

    fn apply_window_preset(preset: &WindowPreset) -> Result<()> {
        window_preset::apply_window_preset(preset)
    }

    fn apply_window_preset_impl(preset: &WindowPreset, require_enabled: bool) -> Result<()> {
        if require_enabled && !preset.enabled {
            return Ok(());
        }
        unsafe {
            let target = resolve_window_target(
                preset.target_window_title.as_deref(),
                &preset.extra_target_window_titles,
                preset.match_duplicate_window_titles,
                false,
            );
            if target.0.is_null() {
                bail!("No foreground window is available");
            }
            let target_root = GetAncestor(target, GA_ROOT);
            if !target_root.0.is_null()
                && window_belongs_to_current_process(target_root)
                && !is_internal_app_window(target_root)
            {
                return Ok(());
            }

            let _ = ShowWindow(target, SW_RESTORE);
            if preset.remove_title_bar {
                let _ = remove_window_title_bar(target);
            } else {
                let _ = restore_window_title_bar(target);
            }
            let bounds = calculate_window_bounds(target, preset)?;
            let _ = SetWindowPos(
                target,
                None,
                bounds.left,
                bounds.top,
                bounds.right - bounds.left,
                bounds.bottom - bounds.top,
                windows::Win32::UI::WindowsAndMessaging::SWP_FRAMECHANGED
                    | SWP_NOACTIVATE
                    | SWP_NOZORDER,
            );
        }
        Ok(())
    }

    fn apply_window_preset_animated(preset: &WindowPreset) -> Result<()> {
        window_preset::apply_window_preset_animated(preset)
    }

    fn restore_window_title_bar_for_preset(preset: &WindowPreset) -> Result<()> {
        window_preset::restore_window_title_bar_for_preset(preset)
    }

    #[allow(dead_code)]
    fn expand_window_edge(direction: WindowExpandDirection, amount_px: i32) -> Result<()> {
        unsafe {
            let target = resolve_window_target(None, &[], false, false);
            if target.0.is_null() {
                bail!("No foreground window is available");
            }
            let target_root = GetAncestor(target, GA_ROOT);
            if !target_root.0.is_null()
                && window_belongs_to_current_process(target_root)
                && !is_internal_app_window(target_root)
            {
                return Ok(());
            }

            ensure_window_restored(target);
            let mut rect = RECT::default();
            GetWindowRect(target, &mut rect)?;
            match direction {
                WindowExpandDirection::Up => rect.top -= amount_px,
                WindowExpandDirection::Down => rect.bottom += amount_px,
                WindowExpandDirection::Left => rect.left -= amount_px,
                WindowExpandDirection::Right => rect.right += amount_px,
            }
            let _ = SetWindowPos(
                target,
                None,
                rect.left,
                rect.top,
                (rect.right - rect.left).max(1),
                (rect.bottom - rect.top).max(1),
                SWP_NOACTIVATE | SWP_NOZORDER,
            );
        }
        Ok(())
    }

    fn animate_window_rect(target: HWND, start: RECT, end: RECT, duration_ms: u64) -> Result<()> {
        let start_width = (start.right - start.left).max(1);
        let start_height = (start.bottom - start.top).max(1);
        let end_width = (end.right - end.left).max(1);
        let end_height = (end.bottom - end.top).max(1);
        let resizing = start_width != end_width || start_height != end_height;
        let duration = Duration::from_millis(duration_ms.max(if resizing { 160 } else { 120 }));
        let frame_sleep = if resizing {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(8)
        };
        let start_time = Instant::now();
        let mut last_rect = start;

        loop {
            let elapsed = start_time.elapsed();
            let t = (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0);
            let eased = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
            let left = lerp_i32(start.left, end.left, eased);
            let top = lerp_i32(start.top, end.top, eased);
            let right = lerp_i32(start.right, end.right, eased);
            let bottom = lerp_i32(start.bottom, end.bottom, eased);
            let next_rect = RECT {
                left,
                top,
                right,
                bottom,
            };
            if next_rect.left == last_rect.left
                && next_rect.top == last_rect.top
                && next_rect.right == last_rect.right
                && next_rect.bottom == last_rect.bottom
                && t < 1.0
            {
                thread::sleep(frame_sleep);
                continue;
            }
            unsafe {
                let _ = SetWindowPos(
                    target,
                    None,
                    left,
                    top,
                    (right - left).max(1),
                    (bottom - top).max(1),
                    SWP_ASYNCWINDOWPOS | SWP_NOACTIVATE | SWP_NOZORDER,
                );
            }
            last_rect = next_rect;

            if t >= 1.0 {
                break;
            }
            thread::sleep(frame_sleep);
        }
        Ok(())
    }

    fn lerp_i32(start: i32, end: i32, t: f32) -> i32 {
        start + ((end - start) as f32 * t).round() as i32
    }

    fn remove_window_title_bar(target: HWND) -> Result<()> {
        unsafe {
            let style = windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(
                target,
                windows::Win32::UI::WindowsAndMessaging::GWL_STYLE,
            ) as u32;
            let caption = windows::Win32::UI::WindowsAndMessaging::WS_CAPTION.0;
            let thickframe = windows::Win32::UI::WindowsAndMessaging::WS_THICKFRAME.0;
            let new_style = style & !caption & !thickframe;
            if new_style != style {
                let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(
                    target,
                    windows::Win32::UI::WindowsAndMessaging::GWL_STYLE,
                    new_style as i32,
                );
                let _ = SetWindowPos(
                    target,
                    None,
                    0,
                    0,
                    0,
                    0,
                    windows::Win32::UI::WindowsAndMessaging::SWP_FRAMECHANGED
                        | SWP_NOACTIVATE
                        | SWP_NOZORDER
                        | SWP_NOMOVE
                        | SWP_NOSIZE,
                );
            }
        }
        Ok(())
    }

    fn restore_window_title_bar(target: HWND) -> Result<()> {
        unsafe {
            let style = windows::Win32::UI::WindowsAndMessaging::GetWindowLongW(
                target,
                windows::Win32::UI::WindowsAndMessaging::GWL_STYLE,
            ) as u32;
            let new_style = style | WS_OVERLAPPEDWINDOW.0;
            if new_style != style {
                let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowLongW(
                    target,
                    windows::Win32::UI::WindowsAndMessaging::GWL_STYLE,
                    new_style as i32,
                );
            }

            let mut rect = RECT::default();
            GetWindowRect(target, &mut rect)?;
            let _ = SetWindowPos(
                target,
                None,
                rect.left,
                rect.top,
                (rect.right - rect.left).max(1),
                (rect.bottom - rect.top).max(1),
                windows::Win32::UI::WindowsAndMessaging::SWP_FRAMECHANGED
                    | SWP_NOACTIVATE
                    | SWP_NOZORDER,
            );
        }
        Ok(())
    }

    fn ensure_window_restored(target: HWND) {
        unsafe {
            if IsZoomed(target).as_bool() {
                let _ = ShowWindow(target, SW_RESTORE);
                for _ in 0..18 {
                    if !IsZoomed(target).as_bool() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            } else {
                let _ = ShowWindow(target, SW_RESTORE);
            }
        }
    }

    fn wait_for_window_frame_to_settle(target: HWND) {
        unsafe {
            let mut previous = RECT::default();
            let _ = GetWindowRect(target, &mut previous);
            for _ in 0..8 {
                thread::sleep(Duration::from_millis(12));
                let mut current = RECT::default();
                if GetWindowRect(target, &mut current).is_ok()
                    && current.left == previous.left
                    && current.top == previous.top
                    && current.right == previous.right
                    && current.bottom == previous.bottom
                {
                    break;
                }
                previous = current;
            }
        }
    }

    fn calculate_window_bounds(hwnd: HWND, preset: &WindowPreset) -> Result<RECT> {
        unsafe {
            let mut window_rect = RECT::default();
            GetWindowRect(hwnd, &mut window_rect)?;
            let mut client_rect = RECT::default();
            GetClientRect(hwnd, &mut client_rect)?;
            let frame_extra_width =
                (window_rect.right - window_rect.left) - (client_rect.right - client_rect.left);
            let frame_extra_height =
                (window_rect.bottom - window_rect.top) - (client_rect.bottom - client_rect.top);

            let mut frame_rect = RECT::default();
            let frame_result = DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut frame_rect as *mut _ as *mut c_void,
                size_of::<RECT>() as u32,
            );

            let (left_invisible, top_invisible) = if frame_result.is_ok() {
                (
                    frame_rect.left - window_rect.left,
                    frame_rect.top - window_rect.top,
                )
            } else {
                (0, 0)
            };
            let (right_invisible, bottom_invisible) = if frame_result.is_ok() {
                (
                    window_rect.right - frame_rect.right,
                    window_rect.bottom - frame_rect.bottom,
                )
            } else {
                (0, 0)
            };

            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut monitor_info = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let monitor_rect = if GetMonitorInfoW(monitor, &mut monitor_info).as_bool() {
                monitor_info.rcMonitor
            } else {
                RECT {
                    left: 0,
                    top: 0,
                    right: GetSystemMetrics(SM_CXSCREEN),
                    bottom: GetSystemMetrics(SM_CYSCREEN),
                }
            };
            let screen_width = monitor_rect.right - monitor_rect.left;
            let screen_height = monitor_rect.bottom - monitor_rect.top;
            let client_width = preset.width.max(1);
            let client_height = preset.height.max(1);
            let outer_width = client_width + frame_extra_width;
            let outer_height = client_height + frame_extra_height;
            let visible_width = (outer_width - left_invisible - right_invisible).max(1);
            let visible_height = (outer_height - top_invisible - bottom_invisible).max(1);
            let (target_x, target_y) = match preset.anchor {
                WindowAnchor::Manual => (preset.x, preset.y),
                WindowAnchor::Center => (
                    monitor_rect.left + ((screen_width - visible_width) / 2),
                    monitor_rect.top + ((screen_height - visible_height) / 2),
                ),
                WindowAnchor::TopLeft => (monitor_rect.left, monitor_rect.top),
                WindowAnchor::Top => (
                    monitor_rect.left + ((screen_width - visible_width) / 2),
                    monitor_rect.top,
                ),
                WindowAnchor::TopRight => (
                    monitor_rect.left + (screen_width - visible_width),
                    monitor_rect.top,
                ),
                WindowAnchor::Left => (
                    monitor_rect.left,
                    monitor_rect.top + ((screen_height - visible_height) / 2),
                ),
                WindowAnchor::Right => (
                    monitor_rect.left + (screen_width - visible_width),
                    monitor_rect.top + ((screen_height - visible_height) / 2),
                ),
                WindowAnchor::BottomLeft => (
                    monitor_rect.left,
                    monitor_rect.top + (screen_height - visible_height),
                ),
                WindowAnchor::Bottom => (
                    monitor_rect.left + ((screen_width - visible_width) / 2),
                    monitor_rect.top + (screen_height - visible_height),
                ),
                WindowAnchor::BottomRight => (
                    monitor_rect.left + (screen_width - visible_width),
                    monitor_rect.top + (screen_height - visible_height),
                ),
            };

            let left = target_x - left_invisible;
            let top = target_y - top_invisible;
            Ok(RECT {
                left,
                top,
                right: left + client_width + frame_extra_width,
                bottom: top + client_height + frame_extra_height,
            })
        }
    }

    fn macro_target_matches(group: &MacroGroup) -> bool {
        if group.target_window_title.is_none() && group.extra_target_window_titles.is_empty() {
            return true;
        }
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground.0.is_null() {
                return false;
            }
            window_matches_any_selector(
                foreground,
                group.target_window_title.as_deref(),
                &group.extra_target_window_titles,
                group.match_duplicate_window_titles,
            )
        }
    }

    fn macro_preset_trigger_matches(preset: &MacroPreset, binding: &HotkeyBinding) -> bool {
        if preset
            .hotkey
            .as_ref()
            .is_some_and(|hotkey| trigger_binding_matches(hotkey, binding))
        {
            return true;
        }

        let trigger_keys = preset.trigger_keys.trim();
        if trigger_keys.is_empty() {
            return false;
        }

        hotkey::split_binding_list(trigger_keys)
            .iter()
            .filter_map(|entry| hotkey::parse_binding(entry))
            .any(|expected| trigger_binding_matches(&expected, binding))
    }

    fn window_focus_matches(
        target_title: Option<&str>,
        extra_target_titles: &[String],
        match_duplicate_window_titles: bool,
    ) -> bool {
        if target_title.is_none() && extra_target_titles.is_empty() {
            return true;
        }
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground.0.is_null() {
                return false;
            }
            window_matches_any_selector(
                foreground,
                target_title,
                extra_target_titles,
                match_duplicate_window_titles,
            )
        }
    }

    fn resolve_window_target(
        target_title: Option<&str>,
        extra_target_titles: &[String],
        match_duplicate_window_titles: bool,
        prefer_other_if_foreground_matches: bool,
    ) -> HWND {
        unsafe {
            let foreground = GetForegroundWindow();
            if !foreground.0.is_null()
                && window_matches_any_selector(
                    foreground,
                    target_title,
                    extra_target_titles,
                    match_duplicate_window_titles,
                )
            {
                if prefer_other_if_foreground_matches {
                    if let Some(target) = target_title
                        && let Some(hwnd) = find_window_by_selector_excluding(
                            target,
                            match_duplicate_window_titles,
                            Some(foreground),
                        )
                    {
                        return hwnd;
                    }
                    for title in extra_target_titles {
                        if let Some(hwnd) = find_window_by_selector_excluding(
                            title,
                            match_duplicate_window_titles,
                            Some(foreground),
                        ) {
                            return hwnd;
                        }
                    }
                }
                return foreground;
            }
            if let Some(title) = target_title
                && let Some(hwnd) =
                    find_window_by_selector_excluding(title, match_duplicate_window_titles, None)
            {
                return hwnd;
            }
            for title in extra_target_titles {
                if let Some(hwnd) =
                    find_window_by_selector_excluding(title, match_duplicate_window_titles, None)
                {
                    return hwnd;
                }
            }
            foreground
        }
    }

    fn find_target_window_hwnd(
        target_title: Option<&str>,
        extra_target_titles: &[String],
        match_duplicate_window_titles: bool,
        prefer_other_if_foreground_matches: bool,
    ) -> Option<HWND> {
        let hwnd = resolve_window_target(
            target_title,
            extra_target_titles,
            match_duplicate_window_titles,
            prefer_other_if_foreground_matches,
        );
        if hwnd.0.is_null() { None } else { Some(hwnd) }
    }

    fn shutdown_application(hwnd: HWND, runtime: &Runtime) -> Result<()> {
        let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &notify_icon(hwnd)) };
        let _ = restore_mouse_sensitivity_on_exit();
        let _ = unsafe { ShowWindow(runtime.overlay_hwnd, SW_HIDE) };
        let _ = unsafe { ShowWindow(runtime.hud_hwnd, SW_HIDE) };
        let _ = unsafe { ShowWindow(runtime.pin_hwnd, SW_HIDE) };
        if let Some(active) = &runtime.active_pin_thumbnail {
            if let Some(thumbnail_id) = active.thumbnail_id {
                let _ = unsafe { DwmUnregisterThumbnail(thumbnail_id) };
            }
        }
        if !runtime.keyboard_hook.0.is_null() {
            let _ = unsafe { UnhookWindowsHookEx(runtime.keyboard_hook) };
        }
        if !runtime.mouse_hook.0.is_null() {
            let _ = unsafe { UnhookWindowsHookEx(runtime.mouse_hook) };
        }
        {
            let mut hook_state = HOOK_STATE.lock();
            hook_state.window_presets.clear();
            hook_state.window_expand_controls = WindowExpandControls::default();
            hook_state.pin_presets.clear();
            hook_state.active_pin_preset_id = None;
            hook_state.macro_groups.clear();
            hook_state.locked_inputs.clear();
            hook_state.locked_mouse_count = 0;
            hook_state.active_hold_macros.clear();
            hook_state.held_mouse_buttons.clear();
        }
        std::process::exit(0);
    }

    unsafe fn find_window_by_title(title: &str) -> Option<HWND> {
        let mut found = None;
        let _ = windows::Win32::UI::WindowsAndMessaging::EnumWindows(
            Some(find_window_by_selector_proc),
            LPARAM((&mut (title, &mut found)) as *mut _ as isize),
        );
        found
    }

    unsafe fn find_window_by_selector_excluding(
        title: &str,
        match_duplicate_window_titles: bool,
        exclude: Option<HWND>,
    ) -> Option<HWND> {
        let mut found = None;
        let mut payload = (title, match_duplicate_window_titles, exclude, &mut found);
        let _ = windows::Win32::UI::WindowsAndMessaging::EnumWindows(
            Some(find_window_by_selector_excluding_proc),
            LPARAM((&mut payload) as *mut _ as isize),
        );
        found
    }

    unsafe extern "system" fn find_window_by_selector_proc(
        hwnd: HWND,
        lparam: LPARAM,
    ) -> windows::core::BOOL {
        let (target, found) = &mut *(lparam.0 as *mut (&str, &mut Option<HWND>));
        if !windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd).as_bool() {
            return true.into();
        }
        if window_matches_selector(hwnd, target) {
            **found = Some(hwnd);
            return false.into();
        }
        true.into()
    }

    unsafe extern "system" fn find_window_by_selector_excluding_proc(
        hwnd: HWND,
        lparam: LPARAM,
    ) -> windows::core::BOOL {
        let (target, match_duplicate_window_titles, exclude, found) =
            &mut *(lparam.0 as *mut (&str, bool, Option<HWND>, &mut Option<HWND>));
        if !windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd).as_bool() {
            return true.into();
        }
        if exclude.is_some_and(|excluded| excluded == hwnd) {
            return true.into();
        }
        if window_matches_selector_with_duplicate_titles(
            hwnd,
            target,
            *match_duplicate_window_titles,
        ) {
            **found = Some(hwnd);
            return false.into();
        }
        true.into()
    }

    fn selector_base_title(target: &str) -> &str {
        if let Some(prefix) = target.strip_suffix(')')
            && let Some((base, _)) = prefix.rsplit_once(" (0x")
        {
            return base;
        }
        target
    }

    unsafe fn window_matches_selector(hwnd: HWND, target: &str) -> bool {
        let Some(title) = window_title(hwnd) else {
            return false;
        };
        title == target || format!("{title} (0x{:X})", hwnd.0 as usize) == target
    }

    unsafe fn window_matches_selector_with_duplicate_titles(
        hwnd: HWND,
        target: &str,
        match_duplicate_window_titles: bool,
    ) -> bool {
        if window_matches_selector(hwnd, target) {
            return true;
        }
        let base_title = selector_base_title(target);
        if base_title != target
            && window_title(hwnd).is_some_and(|title| title == base_title)
        {
            return true;
        }
        if !match_duplicate_window_titles {
            return false;
        }
        let Some(title) = window_title(hwnd) else {
            return false;
        };
        title == selector_base_title(target)
    }

    unsafe fn window_matches_any_selector(
        hwnd: HWND,
        target_title: Option<&str>,
        extra_target_titles: &[String],
        match_duplicate_window_titles: bool,
    ) -> bool {
        if let Some(target) = target_title
            && window_matches_selector_with_duplicate_titles(
                hwnd,
                target,
                match_duplicate_window_titles,
            )
        {
            return true;
        }
        extra_target_titles.iter().any(|target| {
            window_matches_selector_with_duplicate_titles(
                hwnd,
                target,
                match_duplicate_window_titles,
            )
        })
    }

    unsafe fn window_title(hwnd: HWND) -> Option<String> {
        let length = windows::Win32::UI::WindowsAndMessaging::GetWindowTextLengthW(hwnd);
        if length <= 0 {
            return None;
        }
        let mut buffer = vec![0u16; length as usize + 1];
        let copied = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut buffer);
        if copied <= 0 {
            None
        } else {
            Some(String::from_utf16_lossy(&buffer[..copied as usize]))
        }
    }

    unsafe fn paint_overlay(
        hwnd: HWND,
        style: &CrosshairStyle,
        rendered: RenderedCrosshair,
    ) -> Result<()> {
        let window_x = style.x_offset - rendered.center_x;
        let window_y = style.y_offset - rendered.center_y;

        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            window_x,
            window_y,
            rendered.width as i32,
            rendered.height as i32,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );

        let screen_dc = GetDC(None);
        if screen_dc.0.is_null() {
            bail!("Failed to acquire the screen DC");
        }
        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        if mem_dc.0.is_null() {
            let _ = ReleaseDC(None, screen_dc);
            bail!("Failed to create a memory DC");
        }

        let mut bitmap_info = BITMAPINFO::default();
        bitmap_info.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: rendered.width as i32,
            biHeight: -(rendered.height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };

        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(
            Some(mem_dc),
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        )?;
        if bitmap.0.is_null() {
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(None, screen_dc);
            bail!("Failed to create the DIB surface");
        }

        let old_bitmap = SelectObject(mem_dc, HGDIOBJ(bitmap.0));
        let bgra = rgba_to_bgra(&rendered.rgba);
        std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, bgra.len());

        let destination = POINT {
            x: window_x,
            y: window_y,
        };
        let source = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: rendered.width as i32,
            cy: rendered.height as i32,
        };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        let _ = UpdateLayeredWindow(
            hwnd,
            Some(screen_dc),
            Some(&destination),
            Some(&size),
            Some(mem_dc),
            Some(&source),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        let _ = SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(None, screen_dc);
        Ok(())
    }

    unsafe fn paint_mouse_trail(hwnd: HWND, points: &[POINT]) -> Result<()> {
        let screen_width = GetSystemMetrics(SM_CXSCREEN).max(1);
        let screen_height = GetSystemMetrics(SM_CYSCREEN).max(1);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            screen_width,
            screen_height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );

        let screen_dc = GetDC(None);
        if screen_dc.0.is_null() {
            bail!("Failed to acquire the screen DC");
        }
        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        if mem_dc.0.is_null() {
            let _ = ReleaseDC(None, screen_dc);
            bail!("Failed to create a memory DC");
        }

        let mut bitmap_info = BITMAPINFO::default();
        bitmap_info.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: screen_width,
            biHeight: -screen_height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };

        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(
            Some(mem_dc),
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        )?;
        if bitmap.0.is_null() {
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(None, screen_dc);
            bail!("Failed to create mouse trail DIB");
        }

        let old_bitmap = SelectObject(mem_dc, HGDIOBJ(bitmap.0));
        let pixel_len = (screen_width as usize) * (screen_height as usize) * 4;
        let pixels = std::slice::from_raw_parts_mut(bits as *mut u8, pixel_len);
        pixels.fill(0);
        for segment in points.windows(2) {
            if let [from, to] = segment {
                draw_line_rgba(
                    pixels,
                    screen_width as usize,
                    screen_height as usize,
                    from.x,
                    from.y,
                    to.x,
                    to.y,
                    [255, 40, 40, 180],
                );
            }
        }

        let destination = POINT { x: 0, y: 0 };
        let source = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: screen_width,
            cy: screen_height,
        };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        let _ = UpdateLayeredWindow(
            hwnd,
            Some(screen_dc),
            Some(&destination),
            Some(&size),
            Some(mem_dc),
            Some(&source),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        let _ = SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(None, screen_dc);
        Ok(())
    }

    unsafe fn paint_search_area_overlay(
        hwnd: HWND,
        regions: &[VisionRegion],
        preview_region: Option<VisionRegion>,
    ) -> Result<()> {
        let screen_width = GetSystemMetrics(SM_CXSCREEN).max(1);
        let screen_height = GetSystemMetrics(SM_CYSCREEN).max(1);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            screen_width,
            screen_height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );

        let screen_dc = GetDC(None);
        if screen_dc.0.is_null() {
            bail!("Failed to acquire the screen DC");
        }
        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        if mem_dc.0.is_null() {
            let _ = ReleaseDC(None, screen_dc);
            bail!("Failed to create a memory DC");
        }

        let mut bitmap_info = BITMAPINFO::default();
        bitmap_info.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: screen_width,
            biHeight: -screen_height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };

        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(
            Some(mem_dc),
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        )?;
        if bitmap.0.is_null() {
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(None, screen_dc);
            bail!("Failed to create search area DIB");
        }

        let old_bitmap = SelectObject(mem_dc, HGDIOBJ(bitmap.0));
        let pixel_len = (screen_width as usize) * (screen_height as usize) * 4;
        let pixels = std::slice::from_raw_parts_mut(bits as *mut u8, pixel_len);
        pixels.fill(0);

        for region in regions {
            let fill = [78, 214, 186, 36];
            let outline = [92, 220, 255, 210];
            if region.is_circle {
                fill_ellipse_rgba(
                    pixels,
                    screen_width as usize,
                    screen_height as usize,
                    region.left,
                    region.top,
                    region.width,
                    region.height,
                    fill,
                );
                draw_ellipse_outline_rgba(
                    pixels,
                    screen_width as usize,
                    screen_height as usize,
                    region.left,
                    region.top,
                    region.width,
                    region.height,
                    outline,
                );
                let center_x = region.left + region.width / 2;
                let center_y = region.top + region.height / 2;
                let rx = region.width as f32 / 2.0;
                let ry = region.height as f32 / 2.0;

                if let Some(angle_deg) = region.angle_offset_deg {
                    // 1. Draw START ANGLE (0% - Orange Line)
                    let rad0 = angle_deg.to_radians();
                    let x0 = center_x as f32 + rx * rad0.sin();
                    let y0 = center_y as f32 - ry * rad0.cos();
                    draw_line_rgba(pixels, screen_width as usize, screen_height as usize, center_x, center_y, x0 as i32, y0 as i32, [255, 120, 0, 255]);
                    
                    // 2. Draw END ANGLE (100% - Bright Green Line) based on SPAN!
                    if let Some(span) = region.angle_span_deg {
                        if span < 360.0 {
                            let end_deg = (angle_deg + span) % 360.0;
                            let rad1 = end_deg.to_radians();
                            let x1 = center_x as f32 + rx * rad1.sin();
                            let y1 = center_y as f32 - ry * rad1.cos();
                            draw_line_rgba(pixels, screen_width as usize, screen_height as usize, center_x, center_y, x1 as i32, y1 as i32, [50, 255, 50, 255]);
                        }
                    }
                }
            } else {
                fill_rect_rgba(
                    pixels,
                    screen_width as usize,
                    screen_height as usize,
                    region.left,
                    region.top,
                    region.width,
                    region.height,
                    fill,
                );
                draw_rect_outline_rgba(
                    pixels,
                    screen_width as usize,
                    screen_height as usize,
                    region.left,
                    region.top,
                    region.width,
                    region.height,
                    outline,
                );
            }
        }

        if let Some(region) = preview_region {
            let fill = [120, 220, 255, 36];
            let outline = [255, 216, 96, 230];
            fill_rect_rgba(
                pixels,
                screen_width as usize,
                screen_height as usize,
                region.left,
                region.top,
                region.width,
                region.height,
                fill,
            );
            draw_rect_outline_rgba(
                pixels,
                screen_width as usize,
                screen_height as usize,
                region.left,
                region.top,
                region.width,
                region.height,
                outline,
            );
        }

        let destination = POINT { x: 0, y: 0 };
        let source = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: screen_width,
            cy: screen_height,
        };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        let _ = UpdateLayeredWindow(
            hwnd,
            Some(screen_dc),
            Some(&destination),
            Some(&size),
            Some(mem_dc),
            Some(&source),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        let _ = SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(None, screen_dc);
        Ok(())
    }

    fn blend_rgba_pixel(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        x: i32,
        y: i32,
        color: [u8; 4],
    ) {
        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
            return;
        }
        let index = (y as usize * width + x as usize) * 4;
        let alpha = color[3] as f32 / 255.0;
        let inv = 1.0 - alpha;
        let dst = &mut pixels[index..index + 4];
        dst[0] = (dst[0] as f32 * inv + color[2] as f32 * alpha).round() as u8;
        dst[1] = (dst[1] as f32 * inv + color[1] as f32 * alpha).round() as u8;
        dst[2] = (dst[2] as f32 * inv + color[0] as f32 * alpha).round() as u8;
        dst[3] = dst[3].max(color[3]);
    }

    fn fill_rect_rgba(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        left: i32,
        top: i32,
        rect_width: i32,
        rect_height: i32,
        color: [u8; 4],
    ) {
        let right = left.saturating_add(rect_width).max(left + 1);
        let bottom = top.saturating_add(rect_height).max(top + 1);
        for y in top.max(0)..bottom {
            for x in left.max(0)..right {
                blend_rgba_pixel(pixels, width, height, x, y, color);
            }
        }
    }

    fn point_in_ellipse(x: i32, y: i32, left: i32, top: i32, width: i32, height: i32) -> bool {
        let center_x = left as f32 + width as f32 * 0.5;
        let center_y = top as f32 + height as f32 * 0.5;
        let radius_x = (width as f32 * 0.5).max(1.0);
        let radius_y = (height as f32 * 0.5).max(1.0);
        let dx = (x as f32 + 0.5 - center_x) / radius_x;
        let dy = (y as f32 + 0.5 - center_y) / radius_y;
        dx * dx + dy * dy <= 1.0
    }

    fn fill_ellipse_rgba(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        left: i32,
        top: i32,
        ellipse_width: i32,
        ellipse_height: i32,
        color: [u8; 4],
    ) {
        let right = left.saturating_add(ellipse_width).max(left + 1);
        let bottom = top.saturating_add(ellipse_height).max(top + 1);
        for y in top.max(0)..bottom {
            for x in left.max(0)..right {
                if point_in_ellipse(x, y, left, top, ellipse_width, ellipse_height) {
                    blend_rgba_pixel(pixels, width, height, x, y, color);
                }
            }
        }
    }

    fn draw_rect_outline_rgba(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        left: i32,
        top: i32,
        rect_width: i32,
        rect_height: i32,
        color: [u8; 4],
    ) {
        let right = left.saturating_add(rect_width).max(left + 1) - 1;
        let bottom = top.saturating_add(rect_height).max(top + 1) - 1;
        draw_line_rgba(pixels, width, height, left, top, right, top, color);
        draw_line_rgba(pixels, width, height, right, top, right, bottom, color);
        draw_line_rgba(pixels, width, height, right, bottom, left, bottom, color);
        draw_line_rgba(pixels, width, height, left, bottom, left, top, color);
    }

    fn draw_ellipse_outline_rgba(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        left: i32,
        top: i32,
        ellipse_width: i32,
        ellipse_height: i32,
        color: [u8; 4],
    ) {
        let steps = ((ellipse_width.max(ellipse_height) as f32) * std::f32::consts::TAU / 2.0)
            .round()
            .clamp(32.0, 360.0) as i32;
        let center_x = left as f32 + ellipse_width as f32 * 0.5;
        let center_y = top as f32 + ellipse_height as f32 * 0.5;
        let radius_x = ellipse_width as f32 * 0.5;
        let radius_y = ellipse_height as f32 * 0.5;
        let mut prev_x = center_x + radius_x;
        let mut prev_y = center_y;
        for step in 1..=steps {
            let angle = (step as f32 / steps as f32) * std::f32::consts::TAU;
            let next_x = center_x + radius_x * angle.cos();
            let next_y = center_y + radius_y * angle.sin();
            draw_line_rgba(
                pixels,
                width,
                height,
                prev_x.round() as i32,
                prev_y.round() as i32,
                next_x.round() as i32,
                next_y.round() as i32,
                color,
            );
            prev_x = next_x;
            prev_y = next_y;
        }
    }

    fn draw_line_rgba(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color: [u8; 4],
    ) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let steps = dx.abs().max(dy.abs()).max(1);
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let x = x0 as f32 + dx as f32 * t;
            let y = y0 as f32 + dy as f32 * t;
            for ox in -1..=1 {
                for oy in -1..=1 {
                    blend_rgba_pixel(
                        pixels,
                        width,
                        height,
                        x.round() as i32 + ox,
                        y.round() as i32 + oy,
                        color,
                    );
                }
            }
        }
    }

    fn rgba_to_bgra(rgba: &[u8]) -> Vec<u8> {
        let mut bgra = rgba.to_vec();
        for pixel in bgra.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        bgra
    }

    unsafe fn add_tray_icon(hwnd: HWND) -> Result<()> {
        let mut data = notify_icon(hwnd);
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.uCallbackMessage = WMAPP_TRAYICON;
        let icon_path = runtime_icon_path(hwnd, HOOK_STATE.lock().macros_master_enabled)?;
        data.hIcon = windows::Win32::UI::WindowsAndMessaging::HICON(
            LoadImageW(
                None,
                PCWSTR(icon_path.as_ptr()),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE,
            )?
            .0,
        );
        let tip = "MacroNest".encode_utf16().collect::<Vec<_>>();
        for (index, value) in tip.into_iter().enumerate() {
            if index >= data.szTip.len().saturating_sub(1) {
                break;
            }
            data.szTip[index] = value;
        }
        let _ = Shell_NotifyIconW(NIM_ADD, &data);
        Ok(())
    }

    unsafe fn update_tray_icon(hwnd: HWND, enabled: bool) -> Result<()> {
        let mut data = notify_icon(hwnd);
        data.uFlags = NIF_ICON;
        let icon_path = runtime_icon_path(hwnd, enabled)?;
        data.hIcon = windows::Win32::UI::WindowsAndMessaging::HICON(
            LoadImageW(
                None,
                PCWSTR(icon_path.as_ptr()),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE,
            )?
            .0,
        );
        let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
        if !data.hIcon.is_invalid() {
            let _ = DestroyIcon(data.hIcon);
        }
        Ok(())
    }

    fn notify_icon(hwnd: HWND) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_UID,
            ..Default::default()
        }
    }

    fn widestring(value: &str) -> Vec<u16> {
        let mut wide: Vec<u16> = value.encode_utf16().collect();
        wide.push(0);
        wide
    }

    unsafe fn runtime_icon_path(hwnd: HWND, enabled: bool) -> Result<Vec<u16>> {
        let runtime = runtime_mut(hwnd).context("Runtime was not available for tray icon")?;
        let path = if enabled {
            &runtime.paths.icon_file
        } else {
            &runtime.paths.icon_file_disabled
        };
        Ok(widestring(&path.to_string_lossy()))
    }
}

#[cfg(windows)]
pub use windows_overlay::*;

#[cfg(not(windows))]
mod fallback {
    use anyhow::{Result, bail};

    use crate::{
        model::{
            AudioSettings, CrosshairStyle, VisionPreset, MacroGroup, ProfileRecord, RgbaColor,
            WindowExpandControls, WindowFocusPreset, WindowPreset,
        },
        storage::AppPaths,
    };

    #[derive(Debug, Clone)]
    pub enum OverlayCommand {
        Update(CrosshairStyle),
        UpdateProfiles(Vec<ProfileRecord>),
        UpdateCrosshairProfile {
            index: usize,
            profile: ProfileRecord,
        },
        UpdateWindowPresets(Vec<WindowPreset>),
        UpdateWindowFocusPresets(Vec<WindowFocusPreset>),
        UpdateWindowExpandControls(WindowExpandControls),
        UpdateMacroPresets(Vec<MacroGroup>),
        UpdateAudioSettings(AudioSettings),

        UpdateKeyboardArrowMouseSettings {
            enabled: bool,
            step_px: u32,
        },
        UpdateVisionPresets(Vec<VisionPreset>),
        SetMacrosMasterEnabled(bool),
        SetUiVisible(bool),
        SetTrayIconVisible(bool),
        Exit,
        ToggleMacroRecording(u32, u32, String),
    }

    #[derive(Debug, Clone)]
    pub enum UiCommand {
        ShowWindow,
        Exit,
        VisionFinished(String),
        VisionPointCaptureCancelled(String),
        MacroRealtimeStepRemoved(u32, u32),
    }

    pub struct OverlayHandle;

    impl OverlayHandle {
        pub fn send(&self, _command: OverlayCommand) {}
    }

    pub fn wake_command_queue() {}

    pub fn start(
        _paths: AppPaths,
        _initial_style: CrosshairStyle,
        _ui_tx: crossbeam_channel::Sender<UiCommand>,
    ) -> Result<OverlayHandle> {
        bail!("This application currently supports Windows only")
    }
}

#[cfg(not(windows))]
pub use fallback::*;
