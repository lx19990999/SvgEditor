use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Theme preference for the application.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    #[default]
    System,
    Dark,
    Light,
}

impl ThemePreference {
    pub fn to_egui(self) -> egui::ThemePreference {
        match self {
            ThemePreference::System => egui::ThemePreference::System,
            ThemePreference::Dark => egui::ThemePreference::Dark,
            ThemePreference::Light => egui::ThemePreference::Light,
        }
    }
}

/// Persistent application configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    /// DPI scaling factor (0 means auto-detect).
    pub dpi: f32,
    /// UI language code: "en" or "zh-CN".
    pub language: String,
    /// Theme preference.
    pub theme: ThemePreference,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            dpi: 0.0, // 0 = auto-detect
            language: detect_system_language(),
            theme: ThemePreference::System,
        }
    }
}

impl AppConfig {
    /// Path to the config file: ~/.config/svgeditor.json
    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config").join("svgeditor.json")
    }

    /// Load config from disk. Returns defaults if file missing or invalid.
    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<AppConfig>(&text) {
                Ok(config) => {
                    log::info!("Loaded config from {}", path.display());
                    config
                }
                Err(e) => {
                    log::warn!("Invalid config file: {}, using defaults", e);
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    /// Save config to disk.
    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(self) {
            Ok(text) => {
                if let Err(e) = std::fs::write(&path, text) {
                    log::error!("Failed to save config: {}", e);
                }
            }
            Err(e) => {
                log::error!("Failed to serialize config: {}", e);
            }
        }
    }

    /// Auto-detect DPI based on monitor resolution.
    /// Called on first frame when dpi == 0.
    pub fn auto_detect_dpi(ctx: &egui::Context) -> f32 {
        // Try to get monitor size in native pixels
        let monitor_size = ctx.input(|i| {
            i.viewport()
                .monitor_size
                .unwrap_or(egui::vec2(1920.0, 1080.0))
        });

        // monitor_size is in logical points; multiply by native_pixels_per_point
        // to get approximate native pixel resolution
        let native_ppp = ctx.native_pixels_per_point().unwrap_or(1.0);
        let native_width = monitor_size.x * native_ppp;
        let native_height = monitor_size.y * native_ppp;

        log::info!(
            "Monitor: {}x{} logical, {}x{} native, native_ppp={}",
            monitor_size.x,
            monitor_size.y,
            native_width,
            native_height,
            native_ppp
        );

        if native_width > 3840.0 || native_height > 2160.0 {
            2.0
        } else if native_width > 1920.0 || native_height > 1080.0 {
            1.5
        } else {
            1.0
        }
    }
}

/// Detect system language, return "zh-CN" if Chinese, else "en".
fn detect_system_language() -> String {
    if let Some(locale) = sys_locale::get_locale() {
        log::info!("System locale: {}", locale);
        if locale.starts_with("zh") {
            "zh-CN".to_string()
        } else {
            "en".to_string()
        }
    } else {
        log::warn!("Could not detect system locale, defaulting to English");
        "en".to_string()
    }
}
