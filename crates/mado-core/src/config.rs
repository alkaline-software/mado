//! The settings: defaults, then `~/.mado/config.json`, then the
//! environment. Loading never fails; a file that does not parse is the
//! defaults. Every field has a default, so a file written by an older
//! version, or edited by hand with a field missing, still loads; a field
//! no version reads now (`windows`, the gaps and the sizes a press again
//! stepped through) is passed over, and left out once the file is
//! written again.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::paths;
use crate::shortcut::Shortcut;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    /// `system` follows the OS; `light` and `dark` pin one look.
    pub appearance: String,
    /// `system`, or a language's code. On a Mac the app's own
    /// `AppleLanguages` is the setting and this is not read.
    pub language: String,
    /// Ask for the newest release once a day and say so in Settings.
    /// Nothing is installed without the person's press.
    pub check_updates: bool,
    /// A window is carried to its place over a moment, as macOS's own
    /// tiling carries one; off, it is put there at once.
    pub animate: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { appearance: "system".into(), language: "system".into(), check_updates: true, animate: false }
    }
}

/// The item in the menu bar: there or not, and which click shows its
/// menu (the other opens the settings).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MenuBar {
    pub shown: bool,
    /// A left click shows the menu, and a right click the settings.
    pub swap: bool,
}

impl Default for MenuBar {
    fn default() -> Self {
        Self { shown: true, swap: false }
    }
}

/// Dragging a window to an edge or a corner of the screen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Snapping {
    /// The areas are what they are (`snap::at`); only whether a drag snaps
    /// is chosen. What each edge did, an older version kept, and it is
    /// left alone.
    pub on: bool,
}

impl Default for Snapping {
    fn default() -> Self {
        Self { on: true }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub app: AppConfig,
    pub menu_bar: MenuBar,
    pub snapping: Snapping,
    /// The apps Mado leaves alone while they are in front, by bundle id:
    /// no shortcut is held and no drag snaps.
    pub ignored: Vec<String>,
    /// An action's shortcut by the action's key, as text ("ctrl-alt-left"),
    /// or "" for none. An action not named has its default.
    pub shortcuts: BTreeMap<String, String>,
}

impl Config {
    /// The settings as the app runs with them: the file, then the
    /// environment's overrides (`MADO_APPEARANCE` for a probe).
    pub fn load() -> Config {
        let mut cfg = Self::on_disk();
        if let Ok(v) = std::env::var("MADO_APPEARANCE") {
            if !v.is_empty() {
                cfg.app.appearance = v;
            }
        }
        cfg
    }

    fn on_disk() -> Config {
        paths::read_json(&paths::config_file()).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
    }

    /// Change the file on disk. It starts from the file, not from `load`,
    /// so an override in the environment is never written back.
    pub fn edit(change: impl FnOnce(&mut Config)) -> std::io::Result<()> {
        let mut cfg = Self::on_disk();
        change(&mut cfg);
        paths::ensure_dirs()?;
        paths::write_json(&paths::config_file(), &serde_json::to_value(&cfg)?)
    }

    /// The shortcut an action has: the person's, or its default. A text
    /// that no longer parses is none, never the default over it.
    pub fn shortcut(&self, action: Action) -> Option<Shortcut> {
        match self.shortcuts.get(action.key()) {
            Some(text) => Shortcut::from_config(text),
            None => action.default_shortcut(),
        }
    }

    /// Every action with the shortcut it has.
    pub fn shortcuts(&self) -> Vec<(Action, Shortcut)> {
        Action::ALL.into_iter().filter_map(|a| self.shortcut(a).map(|s| (a, s))).collect()
    }

    /// Give an action a shortcut, or none. One shortcut does one thing: an
    /// action that had it loses it, and is returned.
    pub fn set_shortcut(&mut self, action: Action, shortcut: Option<Shortcut>) -> Option<Action> {
        let taken = shortcut.and_then(|s| Action::ALL.into_iter().find(|&a| a != action && self.shortcut(a) == Some(s)));
        if let Some(other) = taken {
            self.shortcuts.insert(other.key().into(), String::new());
        }
        let text = shortcut.map(|s| s.to_config()).unwrap_or_default();
        if text == action.default_text() {
            self.shortcuts.remove(action.key());
        } else {
            self.shortcuts.insert(action.key().into(), text);
        }
        taken
    }

    /// Every action back to its default shortcut.
    pub fn reset_shortcuts(&mut self) {
        self.shortcuts.clear();
    }

    /// Every action left with no shortcut.
    pub fn clear_shortcuts(&mut self) {
        for a in Action::ALL {
            if a.default_text().is_empty() {
                self.shortcuts.remove(a.key());
            } else {
                self.shortcuts.insert(a.key().into(), String::new());
            }
        }
    }

    /// Whether no action has a shortcut.
    pub fn shortcuts_are_none(&self) -> bool {
        Action::ALL.into_iter().all(|a| self.shortcut(a).is_none())
    }

    /// Whether every action has its default shortcut.
    pub fn shortcuts_are_defaults(&self) -> bool {
        Action::ALL.into_iter().all(|a| self.shortcut(a) == a.default_shortcut())
    }
}
