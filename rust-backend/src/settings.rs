use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Enable automatic fix on system startup
    pub auto_fix_on_startup: bool,

    /// Delay in seconds before running the fix after login
    pub startup_delay_seconds: u32,

    /// Fix lighting priority when app launches
    pub fix_on_app_launch: bool,

    #[serde(default = "default_brightness_level")]
    pub brightness_level: f32,

    #[serde(default = "default_ambient_sample_left")]
    pub ambient_sample_left_fraction: f32,

    #[serde(default = "default_ambient_sample_width")]
    pub ambient_sample_width_fraction: f32,

    #[serde(default = "default_preset_cycle_shortcut")]
    pub preset_cycle_shortcut: Option<String>,

    #[serde(default = "default_preset_cycle_effects")]
    pub preset_cycle_effects: Vec<String>,

    #[serde(default = "default_preset_tweaks")]
    pub preset_tweaks: std::collections::HashMap<
        String,
        std::collections::HashMap<String, crate::presets::ParameterValue>,
    >,

    #[serde(default = "default_current_preset")]
    pub current_preset: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_fix_on_startup: false,
            startup_delay_seconds: 60,
            fix_on_app_launch: false, // not applicable on Linux
            brightness_level: 1.0,
            ambient_sample_left_fraction: 0.0,
            ambient_sample_width_fraction: 1.0,
            preset_cycle_shortcut: None,
            preset_cycle_effects: Vec::new(),
            preset_tweaks: std::collections::HashMap::new(),
            current_preset: String::new(),
        }
    }
}

fn default_brightness_level() -> f32 { 1.0 }
fn default_ambient_sample_left() -> f32 { 0.0 }
fn default_ambient_sample_width() -> f32 { 1.0 }
fn default_preset_cycle_shortcut() -> Option<String> { None }
fn default_preset_cycle_effects() -> Vec<String> { Vec::new() }
fn default_current_preset() -> String { "rainbowwave".to_string() }
fn default_preset_tweaks() -> std::collections::HashMap<
    String,
    std::collections::HashMap<String, crate::presets::ParameterValue>,
> {
    std::collections::HashMap::new()
}

/// Get the path to the settings file.
/// Uses XDG_CONFIG_HOME on Linux, %APPDATA% on Windows.
fn get_settings_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    let config_base = {
        let appdata = std::env::var("APPDATA")?;
        PathBuf::from(appdata)
    };

    #[cfg(not(target_os = "windows"))]
    let config_base = {
        std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(home).join(".config")
            })
    };

    let app_dir = config_base.join("loq-rgb");
    fs::create_dir_all(&app_dir)?;
    Ok(app_dir.join("settings.json"))
}

/// Load settings from file, or return defaults if the file does not exist.
pub fn load_settings() -> Result<AppSettings, Box<dyn std::error::Error>> {
    let settings_path = get_settings_path()?;
    if !settings_path.exists() {
        return Ok(AppSettings::default());
    }
    let contents = fs::read_to_string(settings_path)?;
    let settings: AppSettings = serde_json::from_str(&contents)?;
    Ok(settings)
}

/// Save settings to disk.
pub fn save_settings(settings: &AppSettings) -> Result<(), Box<dyn std::error::Error>> {
    let settings_path = get_settings_path()?;
    let json = serde_json::to_string_pretty(settings)?;
    fs::write(settings_path, json)?;
    Ok(())
}