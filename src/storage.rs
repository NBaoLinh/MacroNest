use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use directories::ProjectDirs;

use crate::model::{AppState, ImageSearchPreset, ProfileRecord};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
    pub state_file: PathBuf,
    pub profiles_dir: PathBuf,
    pub custom_dir: PathBuf,
    pub icon_file: PathBuf,
    pub icon_file_disabled: PathBuf,
    pub interception_dir: PathBuf,
    pub interception_zip_file: PathBuf,
    pub interception_extract_dir: PathBuf,
    pub interception_package_root: PathBuf,
    pub interception_installer_dir: PathBuf,
    pub interception_installer_exe: PathBuf,
    pub interception_dll_file: PathBuf,
    pub image_search_dir: PathBuf,
    pub image_search_template_file: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self> {
        let dirs = ProjectDirs::from("com", "Crosshair", "Crosshair")
            .context("Failed to locate the application data folder")?;
        let root = dirs.data_local_dir().to_path_buf();
        let state_file = root.join("state.json");
        let profiles_dir = root.join("profiles");
        let custom_dir = root.join("custom-crosshairs");
        let icon_file = root.join("app-icon.ico");
        let icon_file_disabled = root.join("app-icon-disabled.ico");
        let interception_dir = root.join("interception");
        let interception_zip_file = interception_dir.join("Interception.zip");
        let interception_extract_dir = interception_dir.join("package");
        let interception_package_root = interception_extract_dir.join("Interception");
        let interception_installer_dir =
            interception_package_root.join("command line installer");
        let interception_installer_exe =
            interception_installer_dir.join("install-interception.exe");
        let interception_dll_file = interception_package_root
            .join("library")
            .join("x64")
            .join("interception.dll");
        let image_search_dir = root.join("image-search");
        let image_search_template_file = image_search_dir.join("template.png");

        fs::create_dir_all(&root)?;
        fs::create_dir_all(&profiles_dir)?;
        fs::create_dir_all(&custom_dir)?;
        fs::create_dir_all(&interception_dir)?;
        fs::create_dir_all(&image_search_dir)?;

        Ok(Self {
            root,
            state_file,
            profiles_dir,
            custom_dir,
            icon_file,
            icon_file_disabled,
            interception_dir,
            interception_zip_file,
            interception_extract_dir,
            interception_package_root,
            interception_installer_dir,
            interception_installer_exe,
            interception_dll_file,
            image_search_dir,
            image_search_template_file,
        })
    }

    pub fn image_search_template_file_for(&self, preset_id: u32) -> PathBuf {
        self.image_search_dir.join(format!("preset-{preset_id}.png"))
    }

    pub fn load_state(&self) -> Result<AppState> {
        if !self.state_file.exists() {
            let state = AppState::default();
            self.save_state(&state)?;
            self.save_profiles(&state.profiles)?;
            return Ok(state);
        }

        let content = fs::read_to_string(&self.state_file)?;
        let mut state: AppState =
            serde_json::from_str(&content).context("Failed to read state.json")?;
        state.profiles = self.load_profiles()?;
        if state.profiles.is_empty() {
            state.profiles = AppState::default().profiles;
        }
        if state.selected_profile.is_none() {
            state.selected_profile = state.profiles.first().map(|p| p.name.clone());
        }
        let next_preset_id = state
            .window_presets
            .iter()
            .map(|preset| preset.id)
            .max()
            .unwrap_or(0)
            + 1;
        if state.next_preset_id < next_preset_id {
            state.next_preset_id = next_preset_id;
        }
        for preset in &mut state.window_presets {
            preset.collapsed = true;
        }
        state.window_expand_controls.enabled = false;
        let next_focus_preset_id = state
            .window_focus_presets
            .iter()
            .map(|preset| preset.id)
            .max()
            .unwrap_or(0)
            + 1;
        if state.next_window_focus_preset_id < next_focus_preset_id {
            state.next_window_focus_preset_id = next_focus_preset_id;
        }
        for preset in &mut state.window_focus_presets {
            preset.collapsed = true;
        }
        if state.image_search_presets.is_empty() {
            let mut preset = ImageSearchPreset::default();
            preset.enabled = state.image_search_settings.enabled
                || self.image_search_template_file.exists();
            preset.hotkey = state.image_search_settings.trigger_hotkey.clone();
            preset.click_after_move = state.image_search_settings.click_after_move;
            state.image_search_presets.push(preset);
        }
        let next_image_search_preset_id = state
            .image_search_presets
            .iter()
            .map(|preset| preset.id)
            .max()
            .unwrap_or(0)
            + 1;
        if state.next_image_search_preset_id < next_image_search_preset_id {
            state.next_image_search_preset_id = next_image_search_preset_id;
        }
        for preset in &mut state.image_search_presets {
            preset.collapsed = true;
            if preset.target_color.is_none() {
                preset.use_color_matching = false;
            }
        }
        let legacy_image_search_template = self.image_search_template_file.exists();
        if legacy_image_search_template {
            let first_template = state
                .image_search_presets
                .first()
                .map(|preset| self.image_search_template_file_for(preset.id));
            if let Some(first_template) = first_template
                && !first_template.exists()
            {
                let _ = fs::copy(&self.image_search_template_file, &first_template);
            }
        }
        if !state.macro_presets.is_empty() {
            let mut used_preset_ids = state
                .macro_groups
                .iter()
                .flat_map(|group| group.presets.iter().map(|preset| preset.id))
                .collect::<std::collections::HashSet<_>>();
            let mut next_generated_preset_id =
                used_preset_ids.iter().copied().max().unwrap_or(0) + 1;
            let migrated_presets = state
                .macro_presets
                .clone()
                .into_iter()
                .map(|legacy| {
                    let mut preset_id = legacy.id;
                    if !used_preset_ids.insert(preset_id) {
                        while used_preset_ids.contains(&next_generated_preset_id) {
                            next_generated_preset_id += 1;
                        }
                        preset_id = next_generated_preset_id;
                        used_preset_ids.insert(preset_id);
                        next_generated_preset_id += 1;
                    }
                    crate::model::MacroPreset {
                        id: preset_id,
                        enabled: legacy.enabled,
                        favorite: false,
                        collapsed: legacy.collapsed,
                        trigger_mode: crate::model::MacroTriggerMode::Press,
                        stop_on_retrigger_immediate: false,
                        hotkey: legacy.hotkey,
                        hold_stop_step_enabled: false,
                        hold_stop_step: crate::model::MacroStep::default(),
                        steps: legacy.steps,
                    }
                })
                .collect();
            let migrated_group_id = state
                .macro_groups
                .iter()
                .map(|group| group.id)
                .max()
                .unwrap_or(0)
                + 1;
            state.macro_groups.push(crate::model::MacroGroup {
                id: migrated_group_id,
                name: "Migrated Macros".to_owned(),
                enabled: true,
                collapsed: false,
                folder_id: None,
                target_window_title: None,
                extra_target_window_titles: Vec::new(),
                match_duplicate_window_titles: false,
                selector_presets: Vec::new(),
                presets: migrated_presets,
            });
            state.macro_presets.clear();
        }
        if state.macro_folders.len() == 1 {
            let folder = &state.macro_folders[0];
            let is_auto_default_folder = folder.name == format!("Folder {}", folder.id)
                && state
                    .macro_groups
                    .iter()
                    .all(|group| group.folder_id == Some(folder.id));
            if is_auto_default_folder {
                for group in &mut state.macro_groups {
                    group.folder_id = None;
                }
                state.macro_folders.clear();
            }
        }
        let valid_folder_ids = state
            .macro_folders
            .iter()
            .map(|folder| folder.id)
            .collect::<std::collections::HashSet<_>>();
        for group in &mut state.macro_groups {
            if group
                .folder_id
                .is_some_and(|folder_id| !valid_folder_ids.contains(&folder_id))
            {
                group.folder_id = None;
            }
        }
        let next_macro_folder_id = state
            .macro_folders
            .iter()
            .map(|folder| folder.id)
            .max()
            .unwrap_or(0)
            + 1;
        if state.next_macro_folder_id < next_macro_folder_id {
            state.next_macro_folder_id = next_macro_folder_id;
        }
        let next_macro_group_id = state
            .macro_groups
            .iter()
            .map(|group| group.id)
            .max()
            .unwrap_or(0)
            + 1;
        if state.next_macro_group_id < next_macro_group_id {
            state.next_macro_group_id = next_macro_group_id;
        }
        let next_macro_preset_id = state
            .macro_groups
            .iter()
            .flat_map(|group| group.presets.iter().map(|preset| preset.id))
            .max()
            .unwrap_or(0)
            + 1;
        if state.next_macro_preset_id < next_macro_preset_id {
            state.next_macro_preset_id = next_macro_preset_id;
        }
        for group in &mut state.macro_groups {
            for selector in &mut group.selector_presets {
                selector.collapsed = true;
                for option in &mut selector.options {
                    if option.enable_preset_ids.is_empty()
                        && let Some(legacy_target) = option.legacy_target_preset_id.take()
                    {
                        option.enable_preset_ids.push(legacy_target);
                    }
                }
            }
            for preset in &mut group.presets {
                preset.collapsed = true;
            }
        }
        let next_selector_preset_id = state
            .macro_groups
            .iter()
            .flat_map(|group| group.selector_presets.iter().map(|selector| selector.id))
            .max()
            .unwrap_or(0)
            + 1;
        if state.next_macro_selector_preset_id < next_selector_preset_id {
            state.next_macro_selector_preset_id = next_selector_preset_id;
        }
        let next_selector_option_id = state
            .macro_groups
            .iter()
            .flat_map(|group| {
                group
                    .selector_presets
                    .iter()
                    .flat_map(|selector| selector.options.iter().map(|option| option.id))
            })
            .max()
            .unwrap_or(0)
            + 1;
        if state.next_macro_selector_option_id < next_selector_option_id {
            state.next_macro_selector_option_id = next_selector_option_id;
        }
        let next_sound_preset_id = state
            .audio_settings
            .presets
            .iter()
            .map(|preset| preset.id)
            .max()
            .unwrap_or(0)
            + 1;
        if state.audio_settings.next_preset_id < next_sound_preset_id {
            state.audio_settings.next_preset_id = next_sound_preset_id;
        }
        let next_sound_library_id = state
            .audio_settings
            .library
            .iter()
            .map(|item| item.id)
            .max()
            .unwrap_or(0)
            + 1;
        if state.audio_settings.next_library_item_id < next_sound_library_id {
            state.audio_settings.next_library_item_id = next_sound_library_id;
        }
        for item in &mut state.audio_settings.library {
            item.collapsed = true;
        }
        let next_zoom_preset_id = state
            .zoom_presets
            .iter()
            .map(|preset| preset.id)
            .max()
            .unwrap_or(0)
            + 1;
        if state.next_zoom_preset_id < next_zoom_preset_id {
            state.next_zoom_preset_id = next_zoom_preset_id;
        }
        let next_master_preset_id = state
            .master_presets
            .iter()
            .map(|preset| preset.id)
            .max()
            .unwrap_or(0)
            + 1;
        if state.next_master_preset_id < next_master_preset_id {
            state.next_master_preset_id = next_master_preset_id;
        }
        for preset in &mut state.master_presets {
            preset.collapsed = true;
        }
        if state.selected_master_preset_id.is_none() {
            state.selected_master_preset_id = state.master_presets.first().map(|preset| preset.id);
        }
        Ok(state)
    }

    pub fn save_state(&self, state: &AppState) -> Result<()> {
        let mut state = state.clone();
        state.macro_presets.clear();
        let content = serde_json::to_string_pretty(&state)?;
        fs::write(&self.state_file, content)?;
        Ok(())
    }

    pub fn load_profiles(&self) -> Result<Vec<ProfileRecord>> {
        let mut profiles = Vec::new();
        for entry in fs::read_dir(&self.profiles_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read profile {}", path.display()))?;
            let profile: ProfileRecord = serde_json::from_str(&content)
                .with_context(|| format!("Profile is invalid: {}", path.display()))?;
            profiles.push(profile);
        }
        profiles.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(profiles)
    }

    pub fn save_profiles(&self, profiles: &[ProfileRecord]) -> Result<()> {
        fs::create_dir_all(&self.profiles_dir)?;
        for entry in fs::read_dir(&self.profiles_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                let _ = fs::remove_file(path);
            }
        }
        for profile in profiles {
            let file = self
                .profiles_dir
                .join(format!("{}.json", sanitize_name(&profile.name)));
            let content = serde_json::to_string_pretty(profile)?;
            fs::write(file, content)?;
        }
        Ok(())
    }

    pub fn list_custom_assets(&self) -> Result<Vec<String>> {
        let mut assets = Vec::new();
        for entry in fs::read_dir(&self.custom_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if is_supported_asset(&path) {
                if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
                    assets.push(name.to_owned());
                }
            }
        }
        assets.sort_by_key(|name| name.to_lowercase());
        Ok(assets)
    }

    pub fn asset_path(&self, asset_name: &str) -> PathBuf {
        self.custom_dir.join(asset_name)
    }
}

fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect();

    if cleaned.trim_matches('_').is_empty() {
        "profile".to_owned()
    } else {
        cleaned
    }
}

fn is_supported_asset(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref(),
        Some("svg" | "png" | "jpg" | "jpeg" | "bmp" | "webp" | "ico")
    )
}
