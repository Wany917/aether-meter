//! User settings: one JSON file in the app's config folder.
//!
//! Every value coming from the frontend (or a hand-edited file) goes through
//! `sanitize`, which clamps numbers and drops what is not valid, so the rest
//! of the app never sees an out-of-range setting.

use std::path::PathBuf;
use std::str::FromStr;

use aether_engine::LANGUAGES;
use aether_engine::model::TargetMode;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_plugin_global_shortcut::Shortcut;

use crate::error::AppError;

const FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// One of `aether_engine::LANGUAGES`.
    pub language: String,
    pub target_mode: TargetMode,
    /// Overlays ignore the mouse (and pass clicks to the game).
    pub click_through: bool,
    pub meter: MeterSettings,
    pub tracker: TrackerSettings,
    /// The routine as a small overlay (what is left today, reserves near their cap).
    pub routine_overlay: bool,
    pub hotkeys: HotkeySettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum NumberFormat {
    /// 1.2M
    Compact,
    /// 1,234,567
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum NameDisplay {
    Full,
    /// "J*****" for streaming and screenshots.
    Masked,
    /// The class name instead of the player's.
    Class,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MeterSettings {
    /// Background opacity, 0.2..=1.
    #[specta(type = specta_typescript::Number)]
    pub opacity: f32,
    /// UI scale, 0.8..=1.5.
    #[specta(type = specta_typescript::Number)]
    pub scale: f32,
    /// 3..=12 rows before the list scrolls.
    pub max_rows: u8,
    pub number_format: NumberFormat,
    pub name_display: NameDisplay,
    pub show_combat_power: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrackerSettings {
    pub enabled: bool,
    pub show_buffs: bool,
    pub show_cooldowns: bool,
    /// Icon edge in px, 24..=64.
    pub icon_size: u8,
    /// Timers the player hid, as `buff:<id>` / `cooldown:<id>`.
    pub hidden: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HotkeySettings {
    pub click_through: String,
    pub reset: String,
    pub toggle_overlays: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "fr".into(),
            target_mode: TargetMode::Auto,
            click_through: false,
            meter: MeterSettings::default(),
            tracker: TrackerSettings::default(),
            routine_overlay: false,
            hotkeys: HotkeySettings::default(),
        }
    }
}

impl Default for MeterSettings {
    fn default() -> Self {
        Self {
            opacity: 0.86,
            scale: 1.0,
            max_rows: 8,
            number_format: NumberFormat::Compact,
            name_display: NameDisplay::Full,
            show_combat_power: false,
        }
    }
}

impl Default for TrackerSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            show_buffs: true,
            show_cooldowns: true,
            icon_size: 38,
            hidden: Vec::new(),
        }
    }
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            click_through: "CommandOrControl+Shift+L".into(),
            reset: "CommandOrControl+Shift+R".into(),
            toggle_overlays: "CommandOrControl+Shift+H".into(),
        }
    }
}

impl Settings {
    /// Clamp what can be clamped; reject what cannot be fixed silently.
    pub fn sanitize(mut self) -> Result<Self, AppError> {
        if !LANGUAGES.contains(&self.language.as_str()) {
            return Err(AppError::Settings(format!(
                "unknown language `{}`",
                self.language
            )));
        }
        self.meter.opacity = clamp_finite(
            self.meter.opacity,
            0.2,
            1.0,
            MeterSettings::default().opacity,
        );
        self.meter.scale = clamp_finite(self.meter.scale, 0.8, 1.5, 1.0);
        self.meter.max_rows = self.meter.max_rows.clamp(3, 12);
        self.tracker.icon_size = self.tracker.icon_size.clamp(24, 64);
        self.tracker.hidden.retain(|key| is_timer_key(key));
        self.tracker.hidden.sort();
        self.tracker.hidden.dedup();
        for (name, accelerator) in [
            ("click-through", &self.hotkeys.click_through),
            ("reset", &self.hotkeys.reset),
            ("toggle overlays", &self.hotkeys.toggle_overlays),
        ] {
            if !accelerator.is_empty() && Shortcut::from_str(accelerator).is_err() {
                return Err(AppError::Hotkey(format!(
                    "`{accelerator}` is not a valid shortcut for {name}"
                )));
            }
        }
        Ok(self)
    }
}

/// A settings file from an older (or newer) version may miss fields or carry
/// unknown ones: lay it over the defaults instead of rejecting it.
fn from_partial_json(text: &str) -> Result<Settings, serde_json::Error> {
    let mut merged = serde_json::to_value(Settings::default())?;
    merge(&mut merged, serde_json::from_str(text)?);
    serde_json::from_value(merged)
}

fn merge(base: &mut serde_json::Value, overlay: serde_json::Value) {
    match (base, overlay) {
        (serde_json::Value::Object(base), serde_json::Value::Object(overlay)) => {
            for (key, value) in overlay {
                if let Some(slot) = base.get_mut(&key) {
                    merge(slot, value);
                }
            }
        }
        (slot, value) => *slot = value,
    }
}

fn clamp_finite(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

fn is_timer_key(key: &str) -> bool {
    key.split_once(':')
        .is_some_and(|(kind, id)| matches!(kind, "buff" | "cooldown") && id.parse::<u32>().is_ok())
}

pub struct SettingsStore {
    path: PathBuf,
    current: RwLock<Settings>,
}

impl SettingsStore {
    /// Load from `dir`, falling back to defaults on a missing or broken file.
    pub fn load(dir: PathBuf) -> Self {
        let path = dir.join(FILE_NAME);
        let settings = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| match from_partial_json(&text) {
                Ok(s) => Some(s),
                Err(e) => {
                    tracing::warn!("{} unreadable, using defaults: {e}", path.display());
                    None
                }
            })
            .and_then(|s| {
                s.sanitize()
                    .inspect_err(|e| tracing::warn!("settings rejected: {e}"))
                    .ok()
            })
            .unwrap_or_default();
        Self {
            path,
            current: RwLock::new(settings),
        }
    }

    pub fn get(&self) -> Settings {
        self.current.read().clone()
    }

    /// Validate, persist and return what was stored.
    pub fn replace(&self, settings: Settings) -> Result<Settings, AppError> {
        let settings = settings.sanitize()?;
        self.persist(&settings)?;
        *self.current.write() = settings.clone();
        Ok(settings)
    }

    fn persist(&self, settings: &Settings) -> Result<(), AppError> {
        let io = |e: std::io::Error| {
            AppError::Settings(format!("cannot write {}: {e}", self.path.display()))
        };
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        let json =
            serde_json::to_vec_pretty(settings).map_err(|e| AppError::Settings(e.to_string()))?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(io)?;
        std::fs::rename(&tmp, &self.path).map_err(io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_of_range_values_are_clamped() {
        let mut s = Settings::default();
        s.meter.opacity = 4.0;
        s.meter.scale = f32::NAN;
        s.meter.max_rows = 40;
        s.tracker.hidden = vec![
            "buff:12".into(),
            "buff:12".into(),
            "nope".into(),
            "cooldown:x".into(),
        ];
        let s = s.sanitize().unwrap();
        assert_eq!(
            (s.meter.opacity, s.meter.scale, s.meter.max_rows),
            (1.0, 1.0, 12)
        );
        assert_eq!(s.tracker.hidden, vec!["buff:12".to_string()]);
    }

    #[test]
    fn partial_files_keep_the_defaults_they_lack() {
        let s =
            from_partial_json(r#"{"language":"en","meter":{"opacity":0.5},"unknown":1}"#).unwrap();
        assert_eq!(s.language, "en");
        assert_eq!(s.meter.opacity, 0.5);
        assert_eq!(s.meter.max_rows, MeterSettings::default().max_rows);
        assert_eq!(s.hotkeys, HotkeySettings::default());
    }

    #[test]
    fn bad_language_and_shortcuts_are_rejected() {
        let s = Settings {
            language: "xx".into(),
            ..Settings::default()
        };
        assert!(matches!(s.sanitize(), Err(AppError::Settings(_))));

        let mut s = Settings::default();
        s.hotkeys.reset = "Ctrl+Banana".into();
        assert!(matches!(s.sanitize(), Err(AppError::Hotkey(_))));
    }
}
