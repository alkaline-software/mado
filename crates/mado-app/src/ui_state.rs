//! What Mado remembers between launches that is not a setting: the
//! settings' section last shown, and whether it has been opened before.
//! `<home>/state/ui.json`; an unreadable file means defaults, never a
//! refusal to start.

use mado_core::paths;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct UiState {
    /// The settings' section last shown, where they open again.
    pub settings_section: Option<String>,
    /// Mado has been opened before: the first launch shows the settings,
    /// where Accessibility is allowed, and later ones start in the menu bar.
    pub opened: bool,
}

fn file() -> std::path::PathBuf {
    paths::state_dir().join("ui.json")
}

impl UiState {
    pub fn load() -> Self {
        paths::read_json(&file()).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(v) = serde_json::to_value(self) {
            let _ = paths::write_json(&file(), &v);
        }
    }
}
