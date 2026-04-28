#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_icon;
mod audio;
mod hotkey;
mod lang;
mod model;
mod overlay;
mod platform;
mod profile_code;
mod render;
mod storage;
mod ui;
mod window_list;

use anyhow::Result;
use crossbeam_channel::unbounded;

use crate::{
    overlay::OverlayCommand,
    storage::AppPaths,
    ui::{CrosshairApp, PopupBlobApp, PopupBlobKind},
};

#[cfg(not(windows))]
compile_error!("This application currently supports Windows only.");

fn main() -> Result<()> {
    if std::env::args().any(|arg| arg == "--already-running-popup") {
        return run_popup_blob(PopupBlobKind::AlreadyRunning);
    }

    if platform::relaunch_as_admin_if_needed()? {
        return Ok(());
    }
    let _single_instance = match platform::acquire_single_instance()? {
        Some(guard) => guard,
        None => return Ok(()),
    };
    platform::set_high_priority();

    #[cfg(windows)]
    unsafe {
        let _ = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
    }

    let paths = AppPaths::discover()?;
    app_icon::ensure_ico_file(&paths.icon_file, 64)?;
    app_icon::ensure_disabled_ico_file(&paths.icon_file_disabled, 64)?;
    let mut state = paths.load_state()?;
    state.show_window = true;
    let (ui_tx, ui_rx) = unbounded();
    let overlay = overlay::start(paths.clone(), state.active_style.clone(), ui_tx)?;
    overlay.send(OverlayCommand::Update(state.active_style.clone()));
    overlay.send(OverlayCommand::UpdateProfiles(state.profiles.clone()));
    overlay.send(OverlayCommand::UpdateWindowPresets(
        state.window_presets.clone(),
    ));
    overlay.send(OverlayCommand::UpdateWindowFocusPresets(
        state.window_focus_presets.clone(),
    ));
    overlay.send(OverlayCommand::UpdateMouseSensitivityPresets(
        state.mouse_sensitivity_presets.clone(),
    ));
    overlay.send(OverlayCommand::UpdateMouseSensitivitySettings {
        restore_on_exit: state.mouse_sensitivity_restore_on_exit,
        restore_speed: state.mouse_sensitivity_restore_speed,
    });
    overlay.send(OverlayCommand::UpdateMouseDriverSettings(
        state.mouse_use_interception_driver,
    ));
    overlay.send(OverlayCommand::UpdateKeyboardArrowMouseSettings {
        enabled: state.keyboard_arrow_mouse_enabled,
        step_px: state.keyboard_arrow_mouse_step_px,
    });
    overlay.send(OverlayCommand::UpdateImageSearchPresets(
        state.image_search_presets.clone(),
    ));
    overlay.send(OverlayCommand::UpdateMacroPresets(
        state.macro_groups.clone(),
    ));
    overlay.send(OverlayCommand::UpdateAudioSettings(
        state.audio_settings.clone(),
    ));

    let (overlay_tx, overlay_rx) = unbounded::<OverlayCommand>();
    std::thread::spawn(move || {
        while let Ok(command) = overlay_rx.recv() {
            overlay.send(command.clone());
            if matches!(command, OverlayCommand::Exit) {
                break;
            }
        }
    });

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("MacroNest")
            .with_inner_size([980.0, 980.0])
            .with_min_inner_size([900.0, 900.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_icon(std::sync::Arc::new(app_icon::icon_data(128)?)),
        ..Default::default()
    };

    eframe::run_native(
        "MacroNest",
        native_options,
        Box::new(move |cc| {
            ui::configure_fonts(&cc.egui_ctx);
            Ok(Box::new(CrosshairApp::new(paths, state, overlay_tx, ui_rx)))
        }),
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    Ok(())
}

fn run_popup_blob(kind: PopupBlobKind) -> Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("MacroNest")
            .with_inner_size([560.0, 260.0])
            .with_min_inner_size([560.0, 260.0])
            .with_max_inner_size([560.0, 260.0])
            .with_resizable(false)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "MacroNest",
        native_options,
        Box::new(move |cc| {
            ui::configure_fonts(&cc.egui_ctx);
            Ok(Box::new(PopupBlobApp::new(
                kind,
                crate::model::UiThemeMode::Dark,
            )))
        }),
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    Ok(())
}
