#![allow(unsafe_op_in_unsafe_fn)]

#[cfg(windows)]
mod windows_overlay {
    use std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        ffi::c_void,
        mem::size_of,
        path::{Path, PathBuf},
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
                    CreateCompatibleDC, CreateDIBSection, CreateFontW, DEFAULT_CHARSET,
                    DIB_RGB_COLORS, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject,
                    DrawTextW, EndPaint, FF_DONTCARE, FW_MEDIUM, GetDC, GetMonitorInfoW, HGDIOBJ,
                    MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow, OUT_DEFAULT_PRECIS,
                    PAINTSTRUCT, ReleaseDC, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
                },
            },
            System::{
                LibraryLoader::GetModuleHandleW,
                Threading::{AttachThreadInput, GetCurrentProcessId, GetCurrentThreadId},
            },
            UI::{
                Input::KeyboardAndMouse::{
                    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
                    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, KEYEVENTF_UNICODE,
                    MAPVK_VK_TO_VSC, MOD_ALT, MOD_CONTROL, MOUSEEVENTF_ABSOLUTE,
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
        audio, hotkey,
        model::{
            AudioSettings, CrosshairStyle, HotkeyBinding, ImageSearchPreset, MacroAction,
            MacroGroup, MacroPreset, MacroStep, MacroTriggerMode, MousePathEvent,
            MousePathEventKind, MousePathPreset, MouseSensitivityPreset, PinPreset, ProfileRecord,
            RgbaColor, SoundLibraryItem, SoundPreset, ToolboxPreset, WindowAnchor,
            WindowExpandControls, WindowExpandDirection, WindowFocusPreset, WindowPreset,
        },
        platform, window_list,
        render::{RenderedCrosshair, render_crosshair},
        storage::AppPaths,
    };

    const HOTKEY_ID: i32 = 1001;
    const TIMER_ID: usize = 1;
    const TRAY_UID: u32 = 7001;
    const XBUTTON1_DATA: u16 = 0x0001;
    const XBUTTON2_DATA: u16 = 0x0002;
    const WMAPP_TRAYICON: u32 = WM_APP + 1;
    const WMAPP_PROCESS_QUEUE: u32 = WM_APP + 2;
    const MACRO_PRESET_BASE_ID: i32 = 10000;
    const INTERCEPTION_MOUSE_DEVICE_START: i32 = 11;
    const INTERCEPTION_MOUSE_DEVICE_END: i32 = 20;
    const INTERCEPTION_MOUSE_LEFT_BUTTON_DOWN: u16 = 0x001;
    const INTERCEPTION_MOUSE_LEFT_BUTTON_UP: u16 = 0x002;
    const INTERCEPTION_MOUSE_RIGHT_BUTTON_DOWN: u16 = 0x004;
    const INTERCEPTION_MOUSE_RIGHT_BUTTON_UP: u16 = 0x008;
    const INTERCEPTION_MOUSE_MIDDLE_BUTTON_DOWN: u16 = 0x010;
    const INTERCEPTION_MOUSE_MIDDLE_BUTTON_UP: u16 = 0x020;
    const INTERCEPTION_MOUSE_BUTTON_4_DOWN: u16 = 0x040;
    const INTERCEPTION_MOUSE_BUTTON_4_UP: u16 = 0x080;
    const INTERCEPTION_MOUSE_BUTTON_5_DOWN: u16 = 0x100;
    const INTERCEPTION_MOUSE_BUTTON_5_UP: u16 = 0x200;
    const INTERCEPTION_MOUSE_WHEEL: u16 = 0x400;
    const INTERCEPTION_MOUSE_MOVE_ABSOLUTE: u16 = 0x001;

    const MENU_TOGGLE: usize = 2001;
    const MENU_SHOW: usize = 2002;
    const MENU_EXIT: usize = 2003;

    static SUPPRESSED_MACRO_HOTKEYS: Lazy<Mutex<HashSet<i32>>> =
        Lazy::new(|| Mutex::new(HashSet::new()));
    static STOP_REQUESTED_MACRO_PRESETS: Lazy<Mutex<HashSet<u32>>> =
        Lazy::new(|| Mutex::new(HashSet::new()));
    static TOOLBOX_DISPLAY: Lazy<Mutex<Option<ToolboxDisplayState>>> =
        Lazy::new(|| Mutex::new(None));
    static TOOLBOX_PREVIEW_DISPLAY: Lazy<Mutex<Option<ToolboxDisplayState>>> =
        Lazy::new(|| Mutex::new(None));
    static MOUSE_RECORDING: Lazy<Mutex<Option<MouseRecordingSession>>> =
        Lazy::new(|| Mutex::new(None));
    static HOOK_STATE: Lazy<Mutex<HookState>> = Lazy::new(|| Mutex::new(HookState::default()));
    thread_local! {
        static INTERCEPTION_MOUSE_SENDER: RefCell<InterceptionMouseSender> =
            RefCell::new(InterceptionMouseSender::default());
    }
    static OVERLAY_COMMAND_TX: Lazy<Mutex<Option<Sender<OverlayCommand>>>> =
        Lazy::new(|| Mutex::new(None));
    static UI_CONTEXT: Lazy<Mutex<Option<egui::Context>>> = Lazy::new(|| Mutex::new(None));
    static CONTROLLER_HWND: AtomicIsize = AtomicIsize::new(0);
    #[derive(Debug, Clone)]
    pub enum OverlayCommand {
        Update(CrosshairStyle),
        UpdateProfiles(Vec<ProfileRecord>),
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
        UpdateMouseDriverSettings(bool),
        UpdateKeyboardArrowMouseSettings {
            enabled: bool,
            step_px: u32,
        },
        UpdateImageSearchPresets(Vec<ImageSearchPreset>),
        ApplyMouseSensitivityPreset(u32),
        RestoreMouseSensitivity,
        UpdateToolboxPresets(Vec<ToolboxPreset>),
        PreviewToolboxPreset(Option<ToolboxPreset>),
        UpdateMacroPresets(Vec<MacroGroup>),
        UpdateAudioSettings(AudioSettings),
        SetMacrosMasterEnabled(bool),
        RefreshPinOverlay,
        SetUiVisible(bool),
        Exit,
    }

    #[derive(Debug, Clone)]
    pub enum UiCommand {
        ShowWindow,
        Exit,
        SyncMacroGroups(Vec<MacroGroup>, String),
        SetMacrosMasterEnabled(bool, String),
        MousePathRecordingStarted(u32, String),
        MousePathRecordingFinished(u32, Vec<MousePathEvent>, String),
        ImageSearchFinished(String),
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
        mouse_use_interception_driver: bool,
        keyboard_arrow_mouse_enabled: bool,
        keyboard_arrow_mouse_step_px: u32,
        image_search_presets: Vec<ImageSearchPreset>,
        image_search_dir: PathBuf,
        interception_dll_path: PathBuf,
        mouse_sensitivity_restore_on_exit: bool,
        mouse_sensitivity_exit_restore_speed: u32,
        active_pin_preset_id: Option<u32>,
        toolbox_presets: Vec<ToolboxPreset>,
        macro_groups: Vec<MacroGroup>,
        macros_master_enabled: bool,
        locked_inputs: HashMap<String, usize>,
        locked_mouse_count: usize,
        current_style: CrosshairStyle,
        profiles: Vec<ProfileRecord>,
        sound_presets: Vec<SoundPreset>,
        sound_library: Vec<SoundLibraryItem>,
        active_hold_macros: HashMap<u32, ActiveHoldMacro>,
        pending_selector: Option<PendingMacroSelector>,
        next_hold_run_token: u64,
        pending_tray_toggle: Option<bool>,
        tray_double_click_suppress_next_up: bool,
        stop_ignore_keys: HashMap<u32, String>,
        press_trigger_suppression: HashMap<String, usize>,
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
                mouse_use_interception_driver: false,
                keyboard_arrow_mouse_enabled: false,
                keyboard_arrow_mouse_step_px: 12,
                image_search_presets: Vec::new(),
                image_search_dir: PathBuf::new(),
                interception_dll_path: PathBuf::new(),
                mouse_sensitivity_restore_on_exit: false,
                mouse_sensitivity_exit_restore_speed: 6,
                active_pin_preset_id: None,
                toolbox_presets: Vec::new(),
                macro_groups: Vec::new(),
                macros_master_enabled: true,
                locked_inputs: HashMap::new(),
                locked_mouse_count: 0,
                current_style: CrosshairStyle::default(),
                profiles: Vec::new(),
                sound_presets: Vec::new(),
                sound_library: Vec::new(),
                active_hold_macros: HashMap::new(),
                pending_selector: None,
                next_hold_run_token: 1,
                pending_tray_toggle: None,
                tray_double_click_suppress_next_up: false,
                stop_ignore_keys: HashMap::new(),
                press_trigger_suppression: HashMap::new(),
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

    type InterceptionContext = *mut c_void;
    type InterceptionDevice = i32;
    type InterceptionCreateContextFn = unsafe extern "C" fn() -> InterceptionContext;
    type InterceptionDestroyContextFn = unsafe extern "C" fn(InterceptionContext);
    type InterceptionSendFn = unsafe extern "C" fn(
        InterceptionContext,
        InterceptionDevice,
        *const InterceptionMouseStroke,
        u32,
    ) -> i32;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct InterceptionMouseStroke {
        state: u16,
        flags: u16,
        rolling: i16,
        x: i32,
        y: i32,
        information: u32,
    }

    struct InterceptionApi {
        _library: Library,
        create_context: InterceptionCreateContextFn,
        destroy_context: InterceptionDestroyContextFn,
        send: InterceptionSendFn,
    }

    impl InterceptionApi {
        unsafe fn load(dll_path: &Path) -> Option<Self> {
            let library = Library::new(dll_path).ok()?;
            let create_context = *library
                .get::<InterceptionCreateContextFn>(b"interception_create_context\0")
                .ok()?;
            let destroy_context = *library
                .get::<InterceptionDestroyContextFn>(b"interception_destroy_context\0")
                .ok()?;
            let send = *library
                .get::<InterceptionSendFn>(b"interception_send\0")
                .ok()?;
            Some(Self {
                _library: library,
                create_context,
                destroy_context,
                send,
            })
        }
    }

    #[derive(Default)]
    struct InterceptionMouseSender {
        api: Option<InterceptionApi>,
        context: Option<InterceptionContext>,
        mouse_device: Option<i32>,
        loaded_dll_path: Option<PathBuf>,
    }

    impl InterceptionMouseSender {
        fn reset(&mut self) {
            if let (Some(api), Some(context)) = (self.api.as_ref(), self.context.take()) {
                unsafe { (api.destroy_context)(context) };
            }
            self.api = None;
            self.mouse_device = None;
            self.loaded_dll_path = None;
        }

        fn ensure_api(&mut self, dll_path: &Path) -> bool {
            if self.loaded_dll_path.as_deref() == Some(dll_path) && self.api.is_some() {
                return true;
            }
            self.reset();
            let Some(api) = (unsafe { InterceptionApi::load(dll_path) }) else {
                return false;
            };
            let context = unsafe { (api.create_context)() };
            if context.is_null() {
                return false;
            }
            self.context = Some(context);
            self.loaded_dll_path = Some(dll_path.to_path_buf());
            self.api = Some(api);
            true
        }

        fn send(&mut self, dll_path: &Path, strokes: &[InterceptionMouseStroke]) -> bool {
            if !dll_path.exists() || !self.ensure_api(dll_path) {
                return false;
            }
            let preferred_device = self.mouse_device;
            let Some(api) = self.api.as_ref() else {
                return false;
            };
            let Some(context) = self.context else {
                return false;
            };

            if let Some(device) = preferred_device
                && unsafe { (api.send)(context, device, strokes.as_ptr(), strokes.len() as u32) } > 0
            {
                return true;
            }

            for device in INTERCEPTION_MOUSE_DEVICE_START..=INTERCEPTION_MOUSE_DEVICE_END {
                if unsafe { (api.send)(context, device, strokes.as_ptr(), strokes.len() as u32) } > 0
                {
                    self.mouse_device = Some(device);
                    return true;
                }
            }

            false
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
        toolbox_hwnd: HWND,
        pin_hwnd: HWND,
        last_pin_update: Instant,
        toolbox_display: Option<ToolboxDisplayState>,
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

    #[derive(Clone)]
    struct PendingMacroSelector {
        group_id: u32,
        selector_id: u32,
        prompt_text: String,
        options: Vec<PendingSelectorOption>,
    }

    #[derive(Clone)]
    struct PendingSelectorOption {
        option_id: u32,
        choice_key: String,
        enable_preset_ids: Vec<u32>,
        disable_preset_ids: Vec<u32>,
        toolbox_text: String,
    }

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
        locked_keys: Vec<String>,
        locked_mouse_count: usize,
        run_token: u64,
        completed: bool,
    }

    #[derive(Clone, PartialEq)]
    struct ToolboxDisplayState {
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
        thumbnail_id: isize,
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
            hook_state.interception_dll_path = paths.interception_dll_file.clone();
            hook_state.image_search_dir = paths.image_search_dir.clone();
        }
        unsafe {
            let instance = HINSTANCE(GetModuleHandleW(None)?.0);
            register_class(
                instance,
                w!("CrosshairController"),
                Some(controller_wnd_proc),
            )?;
            register_class(instance, w!("CrosshairOverlay"), Some(overlay_wnd_proc))?;
            register_class(instance, w!("CrosshairToolbox"), Some(toolbox_wnd_proc))?;
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

            let toolbox_hwnd = CreateWindowExW(
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
            let _ = AppendMenuW(tray_menu, MF_STRING, MENU_TOGGLE, w!("Toggle crosshair"));
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
                toolbox_hwnd,
                pin_hwnd,
                last_pin_update: Instant::now() - Duration::from_secs(1),
                toolbox_display: None,
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
                    let _ = add_tray_icon(hwnd);
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
                            let _ = ShowWindow(runtime.toolbox_hwnd, SW_HIDE);
                            let _ = ShowWindow(runtime.mouse_trail_hwnd, SW_HIDE);
                        } else {
                            let _ = refresh_pin_overlay(runtime);
                            let _ = refresh_toolbox(runtime);
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

                        let toolbox_active = TOOLBOX_DISPLAY.lock().is_some()
                            || TOOLBOX_PREVIEW_DISPLAY.lock().is_some()
                            || runtime.toolbox_display.is_some();
                        if toolbox_active {
                            let _ = refresh_toolbox(runtime);
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

                    refresh_overlay_timer(hwnd, runtime);
                }
                LRESULT(0)
            }
            WMAPP_PROCESS_QUEUE => {
                if let Some(runtime) = runtime_mut(hwnd) {
                    process_pending_commands(hwnd, runtime);
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
                        MENU_TOGGLE => {
                            runtime.style.enabled = !runtime.style.enabled;
                            let _ = refresh_overlay(runtime);
                        }
                        MENU_SHOW => {
                            mark_ui_visible(runtime, true);
                            refresh_overlay_timer(hwnd, runtime);
                            show_ui_window_native();
                            let _ = runtime.ui_tx.send(UiCommand::ShowWindow);
                        }
                        MENU_EXIT => {
                            platform::show_goodbye_popup();
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
                            let ui_visible = runtime.ui_visible;
                            if ui_visible {
                                let (enabled, previous) = {
                                    let mut hook_state = HOOK_STATE.lock();
                                    let previous = hook_state.macros_master_enabled;
                                    hook_state.macros_master_enabled =
                                        !hook_state.macros_master_enabled;
                                    hook_state.pending_tray_toggle = Some(previous);
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
                                let _ = runtime.ui_tx.send(UiCommand::ShowWindow);
                                wake_command_queue();
                            }
                        }
                    }
                    WM_LBUTTONDBLCLK => {
                        if let Some(runtime) = runtime_mut(hwnd) {
                            {
                                let mut hook_state = HOOK_STATE.lock();
                                if let Some(previous) = hook_state.pending_tray_toggle.take() {
                                    hook_state.macros_master_enabled = previous;
                                    let _ = update_tray_icon(hwnd, previous);
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
                            mark_ui_visible(runtime, true);
                            refresh_overlay_timer(hwnd, runtime);
                            show_ui_window_native();
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
                    let _ = ShowWindow(runtime.toolbox_hwnd, SW_HIDE);
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
                hook_state.sound_library.clear();
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

    unsafe extern "system" fn toolbox_wnd_proc(
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
        if code == HC_ACTION as i32 && is_ui_in_foreground() {
            return CallNextHookEx(None, code, wparam, lparam);
        }
        if code == HC_ACTION as i32 {
            let info = *(lparam.0 as *const KBDLLHOOKSTRUCT);
            let msg = wparam.0 as u32;
            let is_key_event = matches!(msg, WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP);
            let injected = info.flags.0 & 0x10 != 0;
            if is_key_event && !injected {
                let is_key_down = matches!(msg, WM_KEYDOWN | WM_SYSKEYDOWN);
                let is_key_up = matches!(msg, WM_KEYUP | WM_SYSKEYUP);
                let key_name = hotkey::vk_to_key_name(info.vkCode).map(str::to_owned);

                if let Some(key_name) = key_name.clone() {
                    let binding = binding_from_event(&key_name);
                    let mut swallow = false;
                    if is_key_down {
                        let repeat = is_repeat_key(&key_name);
                        if let Some(binding_swallow) = process_binding_press(&binding, repeat) {
                            swallow |= binding_swallow;
                        }
                    }
                    if is_key_up {
                        swallow |= process_binding_release(&binding);
                    }

                    update_held_key(&key_name, is_key_down, is_key_up);
                    swallow |= keyboard_arrow_mouse_should_swallow(&key_name);
                    swallow |= is_locked_input(&key_name);
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
        if code == HC_ACTION as i32 && is_ui_in_foreground() {
            return CallNextHookEx(None, code, wparam, lparam);
        }
        if code == HC_ACTION as i32 {
            let info = *(lparam.0 as *const MSLLHOOKSTRUCT);
            let injected = info.flags & 0x01 != 0;
            if injected {
                return CallNextHookEx(None, code, wparam, lparam);
            }
            let message = wparam.0 as u32;
            record_mouse_event(message, &info);
            let mouse_lock_active = is_mouse_locked();
            if mouse_lock_active {
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
            let event = match (wparam.0 as u32, ((info.mouseData >> 16) & 0xFFFF) as u16) {
                (WM_LBUTTONDOWN, _) => Some((binding_from_event("MouseLeft"), true)),
                (WM_LBUTTONUP, _) => Some((binding_from_event("MouseLeft"), false)),
                (WM_RBUTTONDOWN, _) => Some((binding_from_event("MouseRight"), true)),
                (WM_RBUTTONUP, _) => Some((binding_from_event("MouseRight"), false)),
                (WM_MBUTTONDOWN, _) => Some((binding_from_event("MouseMiddle"), true)),
                (windows::Win32::UI::WindowsAndMessaging::WM_MBUTTONUP, _) => {
                    Some((binding_from_event("MouseMiddle"), false))
                }
                (WM_XBUTTONDOWN, XBUTTON1_DATA) => Some((binding_from_event("MouseX1"), true)),
                (WM_XBUTTONUP, XBUTTON1_DATA) => Some((binding_from_event("MouseX1"), false)),
                (WM_XBUTTONDOWN, XBUTTON2_DATA) => Some((binding_from_event("MouseX2"), true)),
                (WM_XBUTTONUP, XBUTTON2_DATA) => Some((binding_from_event("MouseX2"), false)),
                _ => None,
            };
            if let Some((binding, is_down)) = event {
                update_held_mouse_button(message, ((info.mouseData >> 16) & 0xFFFF) as u16);
                let swallow = if is_down {
                    process_binding_press(&binding, false).unwrap_or(false)
                } else {
                    process_binding_release(&binding)
                };
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
        HotkeyBinding {
            ctrl: ctrl_down && !key_name.eq_ignore_ascii_case("Ctrl"),
            alt: alt_down && !key_name.eq_ignore_ascii_case("Alt"),
            shift: shift_down && !key_name.eq_ignore_ascii_case("Shift"),
            win: win_down && !key_name.eq_ignore_ascii_case("Win"),
            key: key_name.to_owned(),
        }
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
                        && preset.record_hotkey.as_ref().is_some_and(|hotkey| {
                            hotkey::binding_matches(
                                hotkey,
                                &binding.key,
                                binding.ctrl,
                                binding.alt,
                                binding.shift,
                                binding.win,
                            )
                        })
                })
                .cloned()
        };
        let Some(preset) = matched else {
            return None;
        };
        toggle_mouse_recording(preset.id, preset.name);
        Some(true)
    }

    fn process_image_search_hotkey(binding: &HotkeyBinding, is_repeat: bool) -> Option<bool> {
        if is_repeat {
            return None;
        }

        let (matched, ui_tx) = {
            let hook_state = HOOK_STATE.lock();
            let matched = hook_state
                .image_search_presets
                .iter()
                .find(|preset| {
                    preset.enabled
                        && window_focus_matches(
                            preset.target_window_title.as_deref(),
                            &preset.extra_target_window_titles,
                            preset.match_duplicate_window_titles,
                        )
                        && preset.hotkey.as_ref().is_some_and(|hotkey| {
                            hotkey::binding_matches(
                                hotkey,
                                &binding.key,
                                binding.ctrl,
                                binding.alt,
                                binding.shift,
                                binding.win,
                            )
                        })
                })
                .cloned();
            (matched, hook_state.ui_tx.clone())
        };

        let Some(preset) = matched else {
            return None;
        };

        let status = match run_image_search_once(&preset) {
            Ok(status) => status,
            Err(error) => format!("Image search failed: {error}"),
        };
        if let Some(tx) = ui_tx {
            let _ = tx.send(UiCommand::ImageSearchFinished(format!("{}: {status}", preset.name)));
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
                        && preset.hotkey.as_ref().is_some_and(|hotkey| {
                            hotkey::binding_matches(
                                hotkey,
                                &binding.key,
                                binding.ctrl,
                                binding.alt,
                                binding.shift,
                                binding.win,
                            )
                        })
                })
                .cloned()
        };
        let Some(preset) = matched else {
            return None;
        };
        let _ = toggle_mouse_sensitivity_preset(&preset);
        Some(true)
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

    fn process_binding_press(binding: &HotkeyBinding, is_repeat: bool) -> Option<bool> {
        if let Some(swallow) = process_mouse_sensitivity_hotkey(binding, is_repeat) {
            return Some(swallow);
        }
        if let Some(swallow) = process_image_search_hotkey(binding, is_repeat) {
            return Some(swallow);
        }
        if is_ui_in_foreground() {
            return Some(false);
        }

        if let Some(swallow) = process_mouse_path_record_hotkey(binding, is_repeat) {
            return Some(swallow);
        }

        if !binding.ctrl
            && !binding.alt
            && !binding.shift
            && !binding.win
            && let Ok(consumed) = apply_selector_choice(&binding.key)
            && consumed
        {
            return Some(true);
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
                && hotkey::binding_matches(
                    hotkey,
                    &binding.key,
                    binding.ctrl,
                    binding.alt,
                    binding.shift,
                    binding.win,
                )
                && !is_repeat
            {
                matched_any_window = true;
                window_actions.push(WindowHotkeyAction::Apply(preset.clone()));
            }
        }

        for preset in &hook_state.window_focus_presets {
            if !preset.enabled {
                continue;
            }
            if let Some(hotkey) = preset.hotkey.as_ref()
                && hotkey::binding_matches(
                    hotkey,
                    &binding.key,
                    binding.ctrl,
                    binding.alt,
                    binding.shift,
                    binding.win,
                )
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
                && hotkey::binding_matches(
                    hotkey,
                    &binding.key,
                    binding.ctrl,
                    binding.alt,
                    binding.shift,
                    binding.win,
                )
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
            if preset.animate_enabled
                && let Some(hotkey) = preset.animate_hotkey.as_ref()
                && hotkey::binding_matches(
                    hotkey,
                    &binding.key,
                    binding.ctrl,
                    binding.alt,
                    binding.shift,
                    binding.win,
                )
                && !is_repeat
            {
                matched_any_window = true;
                window_actions.push(WindowHotkeyAction::Animate(preset.clone()));
            }
            if preset.restore_titlebar_enabled
                && let Some(hotkey) = preset.titlebar_hotkey.as_ref()
                && hotkey::binding_matches(
                    hotkey,
                    &binding.key,
                    binding.ctrl,
                    binding.alt,
                    binding.shift,
                    binding.win,
                )
                && !is_repeat
            {
                matched_any_window = true;
                window_actions.push(WindowHotkeyAction::RestoreTitleBar(preset.clone()));
            }
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
        let mut selector_matches: Vec<(u32, u32)> = Vec::new();

        for group in &hook_state.macro_groups {
            if !group.enabled {
                continue;
            }
            if !macro_target_matches(group) {
                continue;
            }
            for selector in &group.selector_presets {
                if !selector.enabled {
                    continue;
                }
                if let Some(hotkey) = selector.hotkey.as_ref()
                    && hotkey::binding_matches(
                        hotkey,
                        &binding.key,
                        binding.ctrl,
                        binding.alt,
                        binding.shift,
                        binding.win,
                    )
                    && !is_repeat
                {
                    matched_any_macro = true;
                    selector_matches.push((group.id, selector.id));
                }
            }
            for preset in &group.presets {
                if !preset.enabled {
                    continue;
                }
                if let Some(hotkey) = preset.hotkey.as_ref()
                    && hotkey::binding_matches(
                        hotkey,
                        &binding.key,
                        binding.ctrl,
                        binding.alt,
                        binding.shift,
                        binding.win,
                    )
                {
                    matched_any_macro = true;
                    if preset.trigger_mode == MacroTriggerMode::Hold {
                        if !hook_state.active_hold_macros.contains_key(&preset.id) {
                            hold_matches.push((
                                preset.clone(),
                                hotkey.clone(),
                                group.target_window_title.clone(),
                                group.extra_target_window_titles.clone(),
                                group.match_duplicate_window_titles,
                                binding.key.clone(),
                            ));
                        }
                        continue;
                    }

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
        }

        drop(hook_state);

        for (group_id, selector_id) in selector_matches {
            let _ = activate_selector_prompt(group_id, selector_id);
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
        if is_press_trigger_suppressed(&binding.key) {
            decrement_press_trigger_suppression(&binding.key);
            return true;
        }

        let preset_ids = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .active_hold_macros
                .iter()
                .filter(|(_, active)| {
                    hotkey::binding_matches(
                        &active.trigger,
                        &binding.key,
                        binding.ctrl,
                        binding.alt,
                        binding.shift,
                        binding.win,
                    )
                })
                .map(|(preset_id, _)| *preset_id)
                .collect::<Vec<_>>()
        };

        if preset_ids.is_empty() {
            return false;
        }

        for preset_id in preset_ids {
            deactivate_hold_macro(preset_id);
        }
        true
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
                OverlayCommand::UpdateMouseDriverSettings(enabled) => {
                    HOOK_STATE.lock().mouse_use_interception_driver = enabled;
                }
                OverlayCommand::UpdateKeyboardArrowMouseSettings { enabled, step_px } => {
                    let mut hook_state = HOOK_STATE.lock();
                    hook_state.keyboard_arrow_mouse_enabled = enabled;
                    hook_state.keyboard_arrow_mouse_step_px = step_px.clamp(1, 100) as u32;
                }
                OverlayCommand::UpdateImageSearchPresets(presets) => {
                    HOOK_STATE.lock().image_search_presets = presets;
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
                OverlayCommand::UpdateToolboxPresets(presets) => {
                    HOOK_STATE.lock().toolbox_presets = presets;
                }
                OverlayCommand::PreviewToolboxPreset(preset) => {
                    *TOOLBOX_PREVIEW_DISPLAY.lock() =
                        preset.map(toolbox_preview_display_from_preset);
                    let _ = refresh_toolbox(runtime);
                }
                OverlayCommand::UpdateMacroPresets(presets) => {
                    runtime.macro_groups = presets;
                    let _ = sync_macro_hotkeys(hwnd, runtime);
                }
                OverlayCommand::UpdateAudioSettings(settings) => {
                    let mut hook_state = HOOK_STATE.lock();
                    hook_state.sound_presets = settings.presets.clone();
                    hook_state.sound_library = settings.library.clone();
                    runtime.audio_settings = settings;
                }
                OverlayCommand::SetMacrosMasterEnabled(enabled) => {
                    HOOK_STATE.lock().macros_master_enabled = enabled;
                    let _ = update_tray_icon(hwnd, enabled);
                }
                OverlayCommand::RefreshPinOverlay => {
                    let _ = refresh_pin_overlay(runtime);
                }
                OverlayCommand::SetUiVisible(visible) => {
                    runtime.ui_visible = visible;
                    if visible {
                        let _ = set_input_hooks_enabled(runtime, desired_hooks_enabled(runtime));
                        show_ui_window_native();
                        let _ = runtime.ui_tx.send(UiCommand::ShowWindow);
                        let _ = ShowWindow(runtime.pin_hwnd, SW_HIDE);
                        let _ = ShowWindow(runtime.toolbox_hwnd, SW_HIDE);
                        let _ = ShowWindow(runtime.mouse_trail_hwnd, SW_HIDE);
                    } else {
                        *TOOLBOX_PREVIEW_DISPLAY.lock() = None;
                        let _ = set_input_hooks_enabled(runtime, desired_hooks_enabled(runtime));
                        hide_ui_window_native();
                        let _ = refresh_overlay(runtime);
                        let _ = refresh_pin_overlay(runtime);
                        let _ = refresh_toolbox(runtime);
                    }
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
            let _ = ShowWindow(runtime.toolbox_hwnd, SW_HIDE);
            let _ = ShowWindow(runtime.mouse_trail_hwnd, SW_HIDE);
        }
    }

    unsafe fn refresh_overlay(runtime: &mut Runtime) -> Result<()> {
        if !runtime.style.enabled {
            let _ = ShowWindow(runtime.overlay_hwnd, SW_HIDE);
            return Ok(());
        }

        let custom_path = runtime
            .style
            .custom_asset
            .as_ref()
            .map(|name| runtime.paths.asset_path(name));

        let rendered = render_crosshair(&runtime.style, custom_path.as_deref())?;
        paint_overlay(runtime.overlay_hwnd, &runtime.style, rendered)?;
        let _ = ShowWindow(runtime.overlay_hwnd, SW_SHOWNA);
        Ok(())
    }

    fn refresh_toolbox(runtime: &mut Runtime) -> Result<()> {
        let display = {
            let mut preview_guard = TOOLBOX_PREVIEW_DISPLAY.lock();
            if let Some(active) = preview_guard.as_ref()
                && let Some(expires_at) = active.expires_at
                && Instant::now() >= expires_at
            {
                *preview_guard = None;
            }
            if let Some(preview) = preview_guard.clone() {
                Some(preview)
            } else {
                let mut guard = TOOLBOX_DISPLAY.lock();
                if let Some(active) = guard.as_ref()
                    && let Some(expires_at) = active.expires_at
                    && Instant::now() >= expires_at
                {
                    *guard = None;
                }
                guard.clone()
            }
        };
        if runtime.toolbox_display == display {
            return Ok(());
        }
        runtime.toolbox_display = display.clone();

        let Some(display) = display else {
            let _ = unsafe { ShowWindow(runtime.toolbox_hwnd, SW_HIDE) };
            return Ok(());
        };

        unsafe { paint_toolbox(runtime.toolbox_hwnd, &display) }
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

    fn desired_timer_interval_ms(runtime: &Runtime) -> u32 {
        if is_ui_in_foreground() {
            return 100;
        }

        let mouse_recording_active = MOUSE_RECORDING.lock().is_some();
        if mouse_recording_active {
            return 33;
        }

        let toolbox_active = TOOLBOX_DISPLAY.lock().is_some()
            || TOOLBOX_PREVIEW_DISPLAY.lock().is_some()
            || runtime.toolbox_display.is_some();
        if toolbox_active {
            return 100;
        }

        let pin_active = runtime.active_pin_thumbnail.is_some()
            || HOOK_STATE.lock().active_pin_preset_id.is_some();
        if pin_active {
            return 100;
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
        !is_ui_in_foreground()
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
                if let Some(active) = runtime.active_pin_thumbnail.take() {
                    let _ = DwmUnregisterThumbnail(active.thumbnail_id);
                }
                let _ = ShowWindow(runtime.pin_hwnd, SW_HIDE);
            }
            runtime.last_pin_update = Instant::now();
            return Ok(());
        };

        if runtime.active_pin_thumbnail.is_some()
            && runtime.last_pin_update.elapsed() < Duration::from_millis(100)
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

            let needs_register = runtime
                .active_pin_thumbnail
                .as_ref()
                .is_none_or(|active| active.preset_id != preset.id || active.source_hwnd != source);
            if needs_register {
                if let Some(active) = runtime.active_pin_thumbnail.take() {
                    let _ = DwmUnregisterThumbnail(active.thumbnail_id);
                }
                let thumbnail_id = DwmRegisterThumbnail(runtime.pin_hwnd, source)?;
                runtime.active_pin_thumbnail = Some(ActivePinThumbnail {
                    preset_id: preset.id,
                    source_hwnd: source,
                    thumbnail_id,
                    last_target_bounds: (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
                    last_source_crop: None,
                });
            }

            let mut source_rect = RECT::default();
            GetWindowRect(source, &mut source_rect)?;
            let (target_x, target_y, target_w, target_h) = if preset.use_custom_bounds {
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

            if let Some(active) = runtime.active_pin_thumbnail.as_ref() {
                let mut source_flags = DWM_TNP_SOURCECLIENTAREAONLY;
                let mut source_rect_crop = RECT::default();
                let mut source_crop_key = None;
                if preset.use_source_crop {
                    let source_width = (source_rect.right - source_rect.left).max(1);
                    let source_height = (source_rect.bottom - source_rect.top).max(1);
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
                    source_rect_crop = RECT {
                        left: crop_x,
                        top: crop_y,
                        right: crop_x + crop_w,
                        bottom: crop_y + crop_h,
                    };
                    source_crop_key = Some((crop_x, crop_y, crop_w, crop_h));
                    source_flags |= DWM_TNP_RECTSOURCE;
                }
                let target_bounds = (target_x, target_y, target_w, target_h);
                let needs_apply = active.last_target_bounds != target_bounds
                    || active.last_source_crop != source_crop_key;
                if needs_apply {
                    let _ = SetWindowPos(
                        runtime.pin_hwnd,
                        Some(HWND_TOPMOST),
                        target_x,
                        target_y,
                        target_w,
                        target_h,
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
                            right: target_w,
                            bottom: target_h,
                        },
                        rcSource: source_rect_crop,
                        opacity: 255,
                        fVisible: true.into(),
                        fSourceClientAreaOnly: false.into(),
                        ..Default::default()
                    };
                    let _ = DwmUpdateThumbnailProperties(active.thumbnail_id, &properties);
                    if let Some(active_mut) = runtime.active_pin_thumbnail.as_mut() {
                        active_mut.last_target_bounds = target_bounds;
                        active_mut.last_source_crop = source_crop_key;
                    }
                }
            }
            let _ = ShowWindow(runtime.pin_hwnd, SW_SHOWNA);
        }
        runtime.last_pin_update = Instant::now();
        Ok(())
    }

    unsafe fn paint_toolbox(hwnd: HWND, display: &ToolboxDisplayState) -> Result<()> {
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

        hide_toolbox_for_owner(preset_id);
        HOOK_STATE.lock().stop_ignore_keys.remove(&preset_id);
    }

    fn current_hold_run_matches(preset_id: u32, run_token: u64) -> bool {
        HOOK_STATE
            .lock()
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

    fn apply_window_preset_by_id(spec: &str) -> Result<()> {
        let preset_id = spec
            .trim()
            .parse::<u32>()
            .context("Window preset id is invalid")?;
        let mut preset = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .window_presets
                .iter()
                .find(|preset| preset.id == preset_id)
                .cloned()
        }
        .context("Window preset was not found")?;
        preset.enabled = true;
        apply_window_preset(&preset)
    }

    fn focus_window_by_preset_id(spec: &str) -> Result<()> {
        let preset_id = spec
            .trim()
            .parse::<u32>()
            .context("Window preset id is invalid")?;
        let preset = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .window_focus_presets
                .iter()
                .find(|preset| preset.id == preset_id)
                .cloned()
        }
        .context("Window preset was not found")?;
        focus_window_for_title(
            preset.target_window_title.as_deref(),
            &preset.extra_target_window_titles,
            preset.match_duplicate_window_titles,
            true,
        )
    }

    fn focus_window_for_preset(preset: &WindowFocusPreset) -> Result<()> {
        focus_window_for_title(
            preset.target_window_title.as_deref(),
            &preset.extra_target_window_titles,
            preset.match_duplicate_window_titles,
            true,
        )
    }

    fn focus_window_for_title(
        target_title: Option<&str>,
        extra_target_titles: &[String],
        match_duplicate_window_titles: bool,
        prefer_other_if_foreground_matches: bool,
    ) -> Result<()> {
        let hwnd = find_target_window_hwnd(
            target_title,
            extra_target_titles,
            match_duplicate_window_titles,
            prefer_other_if_foreground_matches,
        )
        .context("Target window was not found")?;
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground == hwnd && !IsIconic(hwnd).as_bool() {
                return Ok(());
            }
            let current_thread = GetCurrentThreadId();
            let target_thread = GetWindowThreadProcessId(hwnd, None);
            let foreground_thread = if foreground.0.is_null() {
                0
            } else {
                GetWindowThreadProcessId(foreground, None)
            };

            let attach_foreground = foreground_thread != 0 && foreground_thread != current_thread;
            let attach_target = target_thread != 0 && target_thread != current_thread;

            if attach_foreground {
                let _ = AttachThreadInput(foreground_thread, current_thread, true);
            }
            if attach_target {
                let _ = AttachThreadInput(target_thread, current_thread, true);
            }

            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = BringWindowToTop(hwnd);
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_NOTOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            );
            let _ = SetForegroundWindow(hwnd);
            let _ = SetActiveWindow(hwnd);
            let _ = SetFocus(Some(hwnd));
            thread::sleep(Duration::from_millis(18));
            replay_held_inputs_after_focus();

            if attach_target {
                let _ = AttachThreadInput(target_thread, current_thread, false);
            }
            if attach_foreground {
                let _ = AttachThreadInput(foreground_thread, current_thread, false);
            }
        }
        Ok(())
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
        let profile = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .profiles
                .iter()
                .find(|profile| profile.name == profile_name)
                .cloned()
        }
        .context("Crosshair profile was not found")?;
        let mut style = profile.style;
        style.enabled = true;
        HOOK_STATE.lock().current_style = style.clone();
        send_overlay_command(OverlayCommand::Update(style));
        Ok(())
    }

    fn disable_crosshair_overlay() {
        let mut style = HOOK_STATE.lock().current_style.clone();
        style.enabled = false;
        HOOK_STATE.lock().current_style = style.clone();
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
        let clips = {
            let hook_state = HOOK_STATE.lock();
            let preset = hook_state
                .sound_presets
                .iter()
                .find(|preset| preset.id == preset_id)
                .cloned()
                .context("Sound preset was not found")?;
            let mut base_clip = preset.clip.clone();
            base_clip.enabled = true;
            let mut clips = vec![base_clip];
            for library_id in &preset.sequence_library_ids {
                if let Some(item) = hook_state
                    .sound_library
                    .iter()
                    .find(|item| item.id == *library_id)
                {
                    let mut clip = item.clip.clone();
                    clip.enabled = true;
                    clips.push(clip);
                }
            }
            clips
        };
        audio::play_clip_sequence_async(clips);
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
        let (events, use_interception_driver) = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .mouse_path_presets
                .iter()
                .find(|preset| preset.id == mouse_path_preset_id)
                .map(|preset| (preset.events.clone(), preset.use_interception_driver))
                .context("Mouse path preset was not found")?
        };
        if events.is_empty() {
            return Ok(());
        }

        if step.smooth_mouse_path {
            let speed = step.mouse_speed_percent.max(10) as f32 / 100.0;
            let mut last_pos: Option<(i32, i32)> = None;
            for event in &events {
                if preset_id
                    .is_some_and(|id| macro_stop_requested(id, stop_immediately_on_retrigger))
                {
                    return Ok(());
                }
                match event.kind {
                    MousePathEventKind::Move => {
                        if let Some((from_x, from_y)) = last_pos {
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
                                send_mouse_move_absolute_backend(
                                    x.round() as i32,
                                    y.round() as i32,
                                    use_interception_driver,
                                )?;
                                if sleep_for_mouse_path_delay(
                                    preset_id,
                                    8,
                                    stop_immediately_on_retrigger,
                                ) {
                                    return Ok(());
                                }
                            }
                        } else {
                            send_mouse_move_absolute_backend(
                                event.x,
                                event.y,
                                use_interception_driver,
                            )?;
                        }
                        last_pos = Some((event.x, event.y));
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
                        send_mouse_event_with_backend(
                            &pseudo_step,
                            use_interception_driver,
                        )?;
                    }
                }
            }
        } else {
            for event in &events {
                if sleep_for_mouse_path_delay(
                    preset_id,
                    event.delay_ms,
                    stop_immediately_on_retrigger,
                ) {
                    return Ok(());
                }
                let pseudo_step = MacroStep {
                    action: match event.kind {
                        MousePathEventKind::Move => MacroAction::MouseMoveAbsolute,
                        MousePathEventKind::LeftDown => MacroAction::MouseLeftDown,
                        MousePathEventKind::LeftUp => MacroAction::MouseLeftUp,
                        MousePathEventKind::RightDown => MacroAction::MouseRightDown,
                        MousePathEventKind::RightUp => MacroAction::MouseRightUp,
                        MousePathEventKind::MiddleDown => MacroAction::MouseMiddleDown,
                        MousePathEventKind::MiddleUp => MacroAction::MouseMiddleUp,
                        MousePathEventKind::WheelUp => MacroAction::MouseWheelUp,
                        MousePathEventKind::WheelDown => MacroAction::MouseWheelDown,
                    },
                    x: event.x,
                    y: event.y,
                    ..MacroStep::default()
                };
                send_mouse_event_with_backend(&pseudo_step, use_interception_driver)?;
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

    fn activate_selector_prompt(group_id: u32, selector_id: u32) -> Result<()> {
        let pending = {
            let hook_state = HOOK_STATE.lock();
            let group = hook_state
                .macro_groups
                .iter()
                .find(|group| group.id == group_id)
                .context("Macro group was not found")?;
            let selector = group
                .selector_presets
                .iter()
                .find(|selector| selector.id == selector_id)
                .cloned()
                .context("Selector preset was not found")?;
            PendingMacroSelector {
                group_id,
                selector_id,
                prompt_text: selector.prompt_text.clone(),
                options: selector
                    .options
                    .iter()
                    .map(|option| PendingSelectorOption {
                        option_id: option.id,
                        choice_key: option.choice_key.clone(),
                        enable_preset_ids: option.enable_preset_ids.clone(),
                        disable_preset_ids: option.disable_preset_ids.clone(),
                        toolbox_text: option.toolbox_text.clone(),
                    })
                    .collect(),
            }
        };
        HOOK_STATE.lock().pending_selector = Some(pending.clone());
        let prompt = if pending.prompt_text.trim().is_empty() {
            "Choose an option".to_owned()
        } else {
            pending.prompt_text
        };
        show_selector_toolbox_message(prompt);
        Ok(())
    }

    fn apply_selector_choice(binding_key: &str) -> Result<bool> {
        let pending = HOOK_STATE.lock().pending_selector.clone();
        let Some(pending) = pending else {
            return Ok(false);
        };
        let Some(option) = pending
            .options
            .iter()
            .find(|option| option.choice_key.eq_ignore_ascii_case(binding_key))
            .cloned()
        else {
            return Ok(false);
        };

        let mut status = "Selector choice applied.".to_owned();
        let updated_groups = {
            let mut hook_state = HOOK_STATE.lock();
            let group = hook_state
                .macro_groups
                .iter_mut()
                .find(|group| group.id == pending.group_id)
                .context("Macro group was not found")?;
            for selector in &mut group.selector_presets {
                if selector.id == pending.selector_id {
                    selector.active_option_id = Some(option.option_id);
                }
            }
            for preset in &mut group.presets {
                if option.enable_preset_ids.contains(&preset.id) {
                    preset.enabled = true;
                }
                if option.disable_preset_ids.contains(&preset.id) {
                    preset.enabled = false;
                }
            }
            let enabled_labels = option
                .enable_preset_ids
                .iter()
                .filter_map(|id| {
                    group
                        .presets
                        .iter()
                        .find(|preset| preset.id == *id)
                        .map(|preset| hotkey::format_binding(preset.hotkey.as_ref()))
                })
                .collect::<Vec<_>>();
            if !enabled_labels.is_empty() {
                status = format!("Selected {} in {}.", enabled_labels.join(", "), group.name);
            }
            hook_state.pending_selector = None;
            hook_state.macro_groups.clone()
        };

        let message = if option.toolbox_text.trim().is_empty() {
            status.clone()
        } else {
            option.toolbox_text.clone()
        };
        show_selector_toolbox_message(message);
        if let Some(tx) = HOOK_STATE.lock().ui_tx.clone() {
            let _ = tx.send(UiCommand::SyncMacroGroups(updated_groups, status));
        }
        Ok(true)
    }

    fn show_selector_toolbox_message(text: String) {
        let trimmed = text.trim().to_owned();
        if trimmed.is_empty() {
            return;
        }
        *TOOLBOX_DISPLAY.lock() = Some(ToolboxDisplayState {
            owner_preset_id: None,
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
            background_opacity: 0.78,
            rounded_background: true,
            font_size: 28.0,
            x: 660,
            y: 36,
            width: 600,
            height: 80,
            auto_hide_on_owner_completion: false,
            expires_at: Some(Instant::now() + Duration::from_millis(1800)),
        });
    }

    fn execute_hold_abort_step(preset_id: u32, step: &MacroStep) {
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
            MacroAction::ShowToolbox => {
                trigger_toolbox_display(preset_id, step);
            }
            MacroAction::HideToolbox => {
                hide_toolbox_now();
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
            let hold_duration_ms = if step.action == MacroAction::KeyDown {
                step.delay_ms
            } else {
                0
            };
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
                MacroAction::ShowToolbox => {
                    trigger_toolbox_display(preset_id, step);
                }
                MacroAction::HideToolbox => {
                    hide_toolbox_now();
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
            let hold_duration_ms = if step.action == MacroAction::KeyDown {
                step.delay_ms
            } else {
                0
            };
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
                MacroAction::ShowToolbox => {
                    trigger_toolbox_display(preset_id, step);
                }
                MacroAction::HideToolbox => {
                    hide_toolbox_now();
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
            if !current_hold_run_matches(preset_id, run_token) {
                return true;
            }
            if !macro_runtime_target_matches(
                target_window_title,
                extra_target_window_titles,
                match_duplicate_window_titles,
            ) {
                return true;
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
            if !macro_runtime_target_matches(
                target_window_title,
                extra_target_window_titles,
                match_duplicate_window_titles,
            ) {
                return true;
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

    fn show_toolbox_preset(owner_preset_id: u32, step: &MacroStep) -> Result<()> {
        let preset_id = step
            .key
            .trim()
            .parse::<u32>()
            .context("Toolbox preset id is invalid")?;
        let preset = {
            let hook_state = HOOK_STATE.lock();
            hook_state
                .toolbox_presets
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
            hide_toolbox_now();
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

        *TOOLBOX_DISPLAY.lock() = Some(ToolboxDisplayState {
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

    fn toolbox_preview_display_from_preset(preset: ToolboxPreset) -> ToolboxDisplayState {
        let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) }.max(1);
        let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) }.max(1);
        let scale_x = screen_width as f32 / 1920.0;
        let scale_y = screen_height as f32 / 1080.0;
        ToolboxDisplayState {
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

    fn show_legacy_toolbox_text(owner_preset_id: u32, step: &MacroStep) {
        let text = if step.text_override.trim().is_empty() {
            step.key.trim().to_owned()
        } else {
            step.text_override.trim().to_owned()
        };
        let trimmed = text.trim().to_owned();
        if trimmed.is_empty() {
            hide_toolbox_now();
            return;
        }
        *TOOLBOX_DISPLAY.lock() = Some(ToolboxDisplayState {
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

    fn trigger_toolbox_display(owner_preset_id: u32, step: &MacroStep) {
        if show_toolbox_preset(owner_preset_id, step).is_err() {
            show_legacy_toolbox_text(owner_preset_id, step);
        }
        wake_command_queue();
    }

    fn hide_toolbox_now() {
        *TOOLBOX_DISPLAY.lock() = None;
        wake_command_queue();
    }

    fn hide_toolbox_for_owner(owner_preset_id: u32) {
        let mut guard = TOOLBOX_DISPLAY.lock();
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
            match step.action {
                MacroAction::KeyDown => {
                    held_keys.insert(step.key.clone());
                }
                MacroAction::KeyUp | MacroAction::KeyPress => {
                    held_keys.remove(&step.key);
                }
                MacroAction::TypeText
                | MacroAction::ApplyWindowPreset
                | MacroAction::FocusWindowPreset
                | MacroAction::TriggerMacroPreset
                | MacroAction::EnableCrosshairProfile
                | MacroAction::DisableCrosshair
                | MacroAction::EnablePinPreset
                | MacroAction::DisablePin
                | MacroAction::PlayMousePathPreset
                | MacroAction::ApplyMouseSensitivityPreset
                | MacroAction::EnableZoomPreset
                | MacroAction::DisableZoom
                | MacroAction::PlaySoundPreset => {}
                MacroAction::LoopStart
                | MacroAction::LoopEnd
                | MacroAction::StopIfTriggerPressedAgain
                | MacroAction::StopIfKeyPressed
                | MacroAction::ShowToolbox
                | MacroAction::HideToolbox
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
            MacroAction::ApplyWindowPreset
            | MacroAction::FocusWindowPreset
            | MacroAction::TriggerMacroPreset
            | MacroAction::EnableCrosshairProfile
            | MacroAction::DisableCrosshair
            | MacroAction::EnablePinPreset
            | MacroAction::DisablePin
            | MacroAction::PlayMousePathPreset
            | MacroAction::ApplyMouseSensitivityPreset
            | MacroAction::EnableZoomPreset
            | MacroAction::DisableZoom
            | MacroAction::PlaySoundPreset => return Ok(()),
            MacroAction::LoopStart
            | MacroAction::LoopEnd
            | MacroAction::StopIfTriggerPressedAgain
            | MacroAction::StopIfKeyPressed
            | MacroAction::ShowToolbox
            | MacroAction::HideToolbox
            | MacroAction::LockKeys
            | MacroAction::UnlockKeys
            | MacroAction::LockMouse
            | MacroAction::UnlockMouse
            | MacroAction::EnableMacroPreset
            | MacroAction::DisableMacroPreset => return Ok(()),
            MacroAction::KeyPress | MacroAction::KeyDown | MacroAction::KeyUp => {}
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

    fn current_interception_dll_path() -> PathBuf {
        HOOK_STATE.lock().interception_dll_path.clone()
    }

    fn interception_mouse_stroke(
        state: u16,
        flags: u16,
        rolling: i16,
        x: i32,
        y: i32,
    ) -> InterceptionMouseStroke {
        InterceptionMouseStroke {
            state,
            flags,
            rolling,
            x,
            y,
            information: 0,
        }
    }

    fn send_mouse_strokes_interception(
        prefer_interception: bool,
        strokes: &[InterceptionMouseStroke],
    ) -> bool {
        if !prefer_interception {
            return false;
        }
        let dll_path = current_interception_dll_path();
        INTERCEPTION_MOUSE_SENDER.with(|sender| sender.borrow_mut().send(&dll_path, strokes))
    }

    fn send_mouse_event(step: &MacroStep) -> Result<()> {
        send_mouse_event_with_backend(step, false)
    }

    fn send_mouse_event_with_backend(step: &MacroStep, prefer_interception: bool) -> Result<()> {
        match step.action {
            MacroAction::MouseMoveAbsolute => {
                send_mouse_move_absolute_backend(step.x, step.y, prefer_interception)?;
                return Ok(());
            }
            MacroAction::MouseMoveRelative => {
                send_mouse_move_relative_with_backend(
                    step.x,
                    step.y,
                    prefer_interception,
                )?;
                return Ok(());
            }
            _ => {}
        }

        let (flags, mouse_data, repeat_up, interception_strokes) = match step.action {
            MacroAction::MouseLeftClick => (
                MOUSEEVENTF_LEFTDOWN,
                0,
                Some(MOUSEEVENTF_LEFTUP),
                Some(vec![
                    interception_mouse_stroke(INTERCEPTION_MOUSE_LEFT_BUTTON_DOWN, 0, 0, 0, 0),
                    interception_mouse_stroke(INTERCEPTION_MOUSE_LEFT_BUTTON_UP, 0, 0, 0, 0),
                ]),
            ),
            MacroAction::MouseLeftDown => (
                MOUSEEVENTF_LEFTDOWN,
                0,
                None,
                Some(vec![interception_mouse_stroke(
                    INTERCEPTION_MOUSE_LEFT_BUTTON_DOWN,
                    0,
                    0,
                    0,
                    0,
                )]),
            ),
            MacroAction::MouseLeftUp => (
                MOUSEEVENTF_LEFTUP,
                0,
                None,
                Some(vec![interception_mouse_stroke(
                    INTERCEPTION_MOUSE_LEFT_BUTTON_UP,
                    0,
                    0,
                    0,
                    0,
                )]),
            ),
            MacroAction::MouseRightClick => (
                MOUSEEVENTF_RIGHTDOWN,
                0,
                Some(MOUSEEVENTF_RIGHTUP),
                Some(vec![
                    interception_mouse_stroke(INTERCEPTION_MOUSE_RIGHT_BUTTON_DOWN, 0, 0, 0, 0),
                    interception_mouse_stroke(INTERCEPTION_MOUSE_RIGHT_BUTTON_UP, 0, 0, 0, 0),
                ]),
            ),
            MacroAction::MouseRightDown => (
                MOUSEEVENTF_RIGHTDOWN,
                0,
                None,
                Some(vec![interception_mouse_stroke(
                    INTERCEPTION_MOUSE_RIGHT_BUTTON_DOWN,
                    0,
                    0,
                    0,
                    0,
                )]),
            ),
            MacroAction::MouseRightUp => (
                MOUSEEVENTF_RIGHTUP,
                0,
                None,
                Some(vec![interception_mouse_stroke(
                    INTERCEPTION_MOUSE_RIGHT_BUTTON_UP,
                    0,
                    0,
                    0,
                    0,
                )]),
            ),
            MacroAction::MouseMiddleClick => (
                MOUSEEVENTF_MIDDLEDOWN,
                0,
                Some(MOUSEEVENTF_MIDDLEUP),
                Some(vec![
                    interception_mouse_stroke(INTERCEPTION_MOUSE_MIDDLE_BUTTON_DOWN, 0, 0, 0, 0),
                    interception_mouse_stroke(INTERCEPTION_MOUSE_MIDDLE_BUTTON_UP, 0, 0, 0, 0),
                ]),
            ),
            MacroAction::MouseMiddleDown => (
                MOUSEEVENTF_MIDDLEDOWN,
                0,
                None,
                Some(vec![interception_mouse_stroke(
                    INTERCEPTION_MOUSE_MIDDLE_BUTTON_DOWN,
                    0,
                    0,
                    0,
                    0,
                )]),
            ),
            MacroAction::MouseMiddleUp => (
                MOUSEEVENTF_MIDDLEUP,
                0,
                None,
                Some(vec![interception_mouse_stroke(
                    INTERCEPTION_MOUSE_MIDDLE_BUTTON_UP,
                    0,
                    0,
                    0,
                    0,
                )]),
            ),
            MacroAction::MouseX1Click => (
                MOUSEEVENTF_XDOWN,
                XBUTTON1_DATA as u32,
                Some(MOUSEEVENTF_XUP),
                Some(vec![
                    interception_mouse_stroke(INTERCEPTION_MOUSE_BUTTON_4_DOWN, 0, 0, 0, 0),
                    interception_mouse_stroke(INTERCEPTION_MOUSE_BUTTON_4_UP, 0, 0, 0, 0),
                ]),
            ),
            MacroAction::MouseX1Down => (
                MOUSEEVENTF_XDOWN,
                XBUTTON1_DATA as u32,
                None,
                Some(vec![interception_mouse_stroke(
                    INTERCEPTION_MOUSE_BUTTON_4_DOWN,
                    0,
                    0,
                    0,
                    0,
                )]),
            ),
            MacroAction::MouseX1Up => (
                MOUSEEVENTF_XUP,
                XBUTTON1_DATA as u32,
                None,
                Some(vec![interception_mouse_stroke(
                    INTERCEPTION_MOUSE_BUTTON_4_UP,
                    0,
                    0,
                    0,
                    0,
                )]),
            ),
            MacroAction::MouseX2Click => (
                MOUSEEVENTF_XDOWN,
                XBUTTON2_DATA as u32,
                Some(MOUSEEVENTF_XUP),
                Some(vec![
                    interception_mouse_stroke(INTERCEPTION_MOUSE_BUTTON_5_DOWN, 0, 0, 0, 0),
                    interception_mouse_stroke(INTERCEPTION_MOUSE_BUTTON_5_UP, 0, 0, 0, 0),
                ]),
            ),
            MacroAction::MouseX2Down => (
                MOUSEEVENTF_XDOWN,
                XBUTTON2_DATA as u32,
                None,
                Some(vec![interception_mouse_stroke(
                    INTERCEPTION_MOUSE_BUTTON_5_DOWN,
                    0,
                    0,
                    0,
                    0,
                )]),
            ),
            MacroAction::MouseX2Up => (
                MOUSEEVENTF_XUP,
                XBUTTON2_DATA as u32,
                None,
                Some(vec![interception_mouse_stroke(
                    INTERCEPTION_MOUSE_BUTTON_5_UP,
                    0,
                    0,
                    0,
                    0,
                )]),
            ),
            MacroAction::MouseWheelUp => (
                MOUSEEVENTF_WHEEL,
                120u32,
                None,
                Some(vec![interception_mouse_stroke(
                    INTERCEPTION_MOUSE_WHEEL,
                    0,
                    120,
                    0,
                    0,
                )]),
            ),
            MacroAction::MouseWheelDown => (
                MOUSEEVENTF_WHEEL,
                (-120i32) as u32,
                None,
                Some(vec![interception_mouse_stroke(
                    INTERCEPTION_MOUSE_WHEEL,
                    0,
                    -120,
                    0,
                    0,
                )]),
            ),
            _ => bail!("Unsupported mouse action"),
        };

        if let Some(strokes) = interception_strokes.as_deref()
            && send_mouse_strokes_interception(prefer_interception, strokes)
        {
            return Ok(());
        }

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: mouse_data,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        unsafe {
            let mut inputs = vec![input];
            if let Some(up_flags) = repeat_up {
                inputs.push(INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: 0,
                            dy: 0,
                            mouseData: mouse_data,
                            dwFlags: up_flags,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                });
            }
            let sent = SendInput(&inputs, size_of::<INPUT>() as i32);
            if sent == 0 {
                bail!("SendInput failed");
            }
        }

        Ok(())
    }

    fn send_mouse_move_absolute(x: i32, y: i32) -> Result<()> {
        send_mouse_move_absolute_backend(x, y, false)
    }

    fn send_mouse_move_absolute_backend(
        x: i32,
        y: i32,
        prefer_interception: bool,
    ) -> Result<()> {
        let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) }.max(1);
        let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) }.max(1);
        let normalized_x =
            ((x.clamp(0, screen_w - 1) as i64) * 65535 / (screen_w - 1).max(1) as i64) as i32;
        let normalized_y =
            ((y.clamp(0, screen_h - 1) as i64) * 65535 / (screen_h - 1).max(1) as i64) as i32;
        if send_mouse_strokes_interception(
            prefer_interception,
            &[interception_mouse_stroke(
                0,
                INTERCEPTION_MOUSE_MOVE_ABSOLUTE,
                0,
                normalized_x,
                normalized_y,
            )],
        ) {
            return Ok(());
        }
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

    fn send_mouse_move_relative(dx: i32, dy: i32) -> Result<()> {
        send_mouse_move_relative_with_backend(dx, dy, false)
    }

    fn send_mouse_move_relative_with_backend(
        dx: i32,
        dy: i32,
        prefer_interception: bool,
    ) -> Result<()> {
        if send_mouse_strokes_interception(
            prefer_interception,
            &[interception_mouse_stroke(0, 0, 0, dx, dy)],
        ) {
            return Ok(());
        }
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
        if send_mouse_strokes_interception(false, &[
            interception_mouse_stroke(INTERCEPTION_MOUSE_LEFT_BUTTON_DOWN, 0, 0, 0, 0),
            interception_mouse_stroke(INTERCEPTION_MOUSE_LEFT_BUTTON_UP, 0, 0, 0, 0),
        ]) {
            return Ok(());
        }
        let inputs = [
            INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: 0,
                        dwFlags: MOUSEEVENTF_LEFTDOWN,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: 0,
                        dwFlags: MOUSEEVENTF_LEFTUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
        ];
        unsafe {
            let sent = SendInput(&inputs, size_of::<INPUT>() as i32);
            if sent == 0 {
                bail!("SendInput failed");
            }
        }
        Ok(())
    }

    fn send_mouse_left_click_backend(prefer_interception: bool) -> Result<()> {
        if send_mouse_strokes_interception(prefer_interception, &[
                interception_mouse_stroke(INTERCEPTION_MOUSE_LEFT_BUTTON_DOWN, 0, 0, 0, 0),
                interception_mouse_stroke(INTERCEPTION_MOUSE_LEFT_BUTTON_UP, 0, 0, 0, 0),
            ]) {
            return Ok(());
        }
        let inputs = [
            INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: 0,
                        dwFlags: MOUSEEVENTF_LEFTDOWN,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: 0,
                        dwFlags: MOUSEEVENTF_LEFTUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
        ];
        unsafe {
            let sent = SendInput(&inputs, size_of::<INPUT>() as i32);
            if sent == 0 {
                bail!("SendInput failed");
            }
        }
        Ok(())
    }

    fn pixels_match_with_tolerance(template_px: &[u8], screen_px: &[u8], tolerance: u8) -> bool {
        if template_px.len() < 4 || screen_px.len() < 4 {
            return false;
        }
        let tolerance = tolerance as i16;
        (template_px[0] as i16 - screen_px[0] as i16).abs() <= tolerance
            && (template_px[1] as i16 - screen_px[1] as i16).abs() <= tolerance
            && (template_px[2] as i16 - screen_px[2] as i16).abs() <= tolerance
    }

    fn find_template_match(
        screen_rgba: &[u8],
        screen_width: usize,
        screen_height: usize,
        template_rgba: &[u8],
        template_width: usize,
        template_height: usize,
        tolerance: u8,
    ) -> Option<(usize, usize)> {
        if template_width == 0
            || template_height == 0
            || screen_width < template_width
            || screen_height < template_height
        {
            return None;
        }
        let row_bytes = template_width.checked_mul(4)?;
        let screen_row_bytes = screen_width.checked_mul(4)?;
        let anchor = template_rgba.get(0..4)?;

        for y in 0..=screen_height - template_height {
            let screen_row_offset = y * screen_row_bytes;
            for x in 0..=screen_width - template_width {
                let screen_offset = screen_row_offset + x * 4;
                let Some(screen_anchor) = screen_rgba.get(screen_offset..screen_offset + 4) else {
                    continue;
                };
                if !pixels_match_with_tolerance(anchor, screen_anchor, tolerance) {
                    continue;
                }

                let mut matched = true;
                for ty in 0..template_height {
                    let screen_start = (y + ty) * screen_row_bytes + x * 4;
                    let template_start = ty * row_bytes;
                    let Some(screen_row) = screen_rgba.get(screen_start..screen_start + row_bytes)
                    else {
                        matched = false;
                        break;
                    };
                    let Some(template_row) =
                        template_rgba.get(template_start..template_start + row_bytes)
                    else {
                        matched = false;
                        break;
                    };
                    for (screen_px, template_px) in
                        screen_row.chunks_exact(4).zip(template_row.chunks_exact(4))
                    {
                        if !pixels_match_with_tolerance(template_px, screen_px, tolerance) {
                            matched = false;
                            break;
                        }
                    }
                    if !matched {
                        matched = false;
                        break;
                    }
                }

                if matched {
                    return Some((x, y));
                }
            }
        }
        None
    }

    fn image_search_template_file(preset_id: u32) -> PathBuf {
        let hook_state = HOOK_STATE.lock();
        hook_state
            .image_search_dir
            .join(format!("preset-{preset_id}.png"))
    }

    fn run_image_search_once(preset: &ImageSearchPreset) -> Result<String> {
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

        let screen = if preset.target_window_title.is_some()
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
        let template_image = image::RgbaImage::from_raw(
            template_width as u32,
            template_height as u32,
            template_rgba.clone(),
        )
        .context("Template image data is invalid.")?;
        let scales = [1.0_f32, 0.75, 0.85, 0.9, 1.1, 1.25, 1.5];
        let mut best_match = None;
        for scale in scales {
            let (candidate_rgba, candidate_width, candidate_height) =
                if (scale - 1.0).abs() < f32::EPSILON {
                    (template_rgba.clone(), template_width, template_height)
                } else {
                    let scaled_width = ((template_width as f32) * scale).round().max(1.0) as u32;
                    let scaled_height =
                        ((template_height as f32) * scale).round().max(1.0) as u32;
                    if scaled_width as usize > screen.width || scaled_height as usize > screen.height
                    {
                        continue;
                    }
                    let scaled = image::imageops::resize(
                        &template_image,
                        scaled_width,
                        scaled_height,
                        image::imageops::FilterType::Triangle,
                    );
                    (
                        scaled.into_raw(),
                        scaled_width as usize,
                        scaled_height as usize,
                    )
                };
            if let Some((match_x, match_y)) = find_template_match(
                &screen.rgba,
                screen.width,
                screen.height,
                &candidate_rgba,
                candidate_width,
                candidate_height,
                14,
            ) {
                best_match = Some((match_x, match_y, candidate_width, candidate_height, scale));
                break;
            }
        }
        let Some((match_x, match_y, matched_width, matched_height, matched_scale)) = best_match else {
            return Ok("No match found on screen.".to_owned());
        };

        let center_x = screen.screen_x + match_x as i32 + (matched_width as i32 / 2);
        let center_y = screen.screen_y + match_y as i32 + (matched_height as i32 / 2);
        send_mouse_move_absolute_backend(center_x, center_y, preset.use_interception_driver)?;
        if preset.click_after_move {
            send_mouse_left_click_backend(preset.use_interception_driver)?;
        }
        Ok(format!(
            "Matched at {center_x}, {center_y} (scale {:.2}x).",
            matched_scale
        ))
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
            let mut rect = RECT::default();
            if GetWindowRect(app, &mut rect).is_ok() {
                let width = (rect.right - rect.left).max(1);
                let height = (rect.bottom - rect.top).max(1);
                let center_x = rect.left + width / 2;
                let center_y = rect.top + height / 2;
                let start_w = width.min(160).max(96);
                let start_h = height.min(160).max(96);
                let start_x = center_x - start_w / 2;
                let start_y = center_y - start_h / 2;
                let _ = SetWindowPos(
                    app,
                    None,
                    start_x,
                    start_y,
                    start_w,
                    start_h,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            let _ = ShowWindow(app, SW_SHOWNA);
        }
    }

    fn apply_window_preset(preset: &WindowPreset) -> Result<()> {
        if !preset.enabled {
            return Ok(());
        }
        unsafe {
            let target = resolve_window_target(
                preset.target_window_title.as_deref(),
                &preset.extra_target_window_titles,
                false,
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
        if !preset.enabled || !preset.animate_enabled {
            return Ok(());
        }
        unsafe {
            let target = resolve_window_target(
                preset.target_window_title.as_deref(),
                &preset.extra_target_window_titles,
                false,
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

            ensure_window_restored(target);
            if preset.remove_title_bar {
                let _ = remove_window_title_bar(target);
            } else {
                let _ = restore_window_title_bar(target);
            }
            wait_for_window_frame_to_settle(target);

            let mut start = RECT::default();
            GetWindowRect(target, &mut start)?;
            let end = calculate_window_bounds(target, preset)?;
            animate_window_rect(target, start, end, preset.animate_duration_ms.max(60))?;
        }
        Ok(())
    }

    fn restore_window_title_bar_for_preset(preset: &WindowPreset) -> Result<()> {
        if !preset.restore_titlebar_enabled {
            return Ok(());
        }
        unsafe {
            let target = resolve_window_target(
                preset.target_window_title.as_deref(),
                &preset.extra_target_window_titles,
                false,
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
            restore_window_title_bar(target)?;
        }
        Ok(())
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
        let _ = unsafe { ShowWindow(runtime.toolbox_hwnd, SW_HIDE) };
        let _ = unsafe { ShowWindow(runtime.pin_hwnd, SW_HIDE) };
        if let Some(active) = &runtime.active_pin_thumbnail {
            let _ = unsafe { DwmUnregisterThumbnail(active.thumbnail_id) };
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
        let _ = audio::play_clip_blocking(&runtime.audio_settings.exit);
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
        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let screen_height = GetSystemMetrics(SM_CYSCREEN);
        let window_x = (screen_width / 2) + style.x_offset - rendered.center_x;
        let window_y = (screen_height / 2) + style.y_offset - rendered.center_y;

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
            AudioSettings, CrosshairStyle, ImageSearchPreset, MacroGroup, ProfileRecord,
            WindowExpandControls, WindowFocusPreset, WindowPreset,
        },
        storage::AppPaths,
    };

    #[derive(Debug, Clone)]
    pub enum OverlayCommand {
        Update(CrosshairStyle),
        UpdateProfiles(Vec<ProfileRecord>),
        UpdateWindowPresets(Vec<WindowPreset>),
        UpdateWindowFocusPresets(Vec<WindowFocusPreset>),
        UpdateWindowExpandControls(WindowExpandControls),
        UpdateMacroPresets(Vec<MacroGroup>),
        UpdateAudioSettings(AudioSettings),
        UpdateMouseDriverSettings(bool),
        UpdateKeyboardArrowMouseSettings { enabled: bool, step_px: u32 },
        UpdateImageSearchPresets(Vec<ImageSearchPreset>),
        SetMacrosMasterEnabled(bool),
        SetUiVisible(bool),
        Exit,
    }

    #[derive(Debug, Clone)]
    pub enum UiCommand {
        ShowWindow,
        Exit,
        ImageSearchFinished(String),
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
