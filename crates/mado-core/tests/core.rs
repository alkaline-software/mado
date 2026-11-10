//! The core's tests. Each points `MADO_HOME` at a temp folder of its
//! own (`isolated`), so none reads or writes the person's files.

use std::sync::Mutex;

use mado_core::action::Action;
use mado_core::shortcut::Shortcut;
use mado_core::{config::Config, paths, update};

/// Tests run on threads of one process and share its environment.
static ENV: Mutex<()> = Mutex::new(());

fn isolated() -> (tempfile::TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().expect("a temp folder");
    std::env::set_var("MADO_HOME", dir.path());
    std::env::remove_var("MADO_APPEARANCE");
    (dir, guard)
}

#[test]
fn a_missing_or_broken_file_is_the_defaults() {
    let (_dir, _g) = isolated();
    assert_eq!(Config::load(), Config::default());
    std::fs::write(paths::config_file(), "{ not json").unwrap();
    assert_eq!(Config::load(), Config::default());
}

#[test]
fn a_file_with_a_field_missing_still_loads() {
    let (_dir, _g) = isolated();
    // An older version's `windows` (gaps, a press again's sizes) is passed
    // over, and what is read keeps its value.
    std::fs::write(paths::config_file(), r#"{"windows": {"gap": 12, "cycle": true}, "menu_bar": {"swap": true}}"#).unwrap();
    let cfg = Config::load();
    assert!(cfg.menu_bar.swap);
    assert!(cfg.menu_bar.shown);
    assert!(cfg.snapping.on);
    assert_eq!(cfg.app.appearance, "system");
    assert_eq!(cfg.shortcut(Action::LeftHalf), Action::LeftHalf.default_shortcut());
}

#[test]
fn an_override_is_never_written_back() {
    let (_dir, _g) = isolated();
    std::env::set_var("MADO_APPEARANCE", "dark");
    assert_eq!(Config::load().app.appearance, "dark");
    Config::edit(|c| c.snapping.on = false).unwrap();
    std::env::remove_var("MADO_APPEARANCE");
    let cfg = Config::load();
    assert_eq!(cfg.app.appearance, "system");
    assert!(!cfg.snapping.on);
}

#[test]
fn a_shortcut_given_to_one_action_is_taken_from_the_other() {
    let (_dir, _g) = isolated();
    let left = Action::LeftHalf.default_shortcut().unwrap();
    let mut cfg = Config::default();
    assert_eq!(cfg.set_shortcut(Action::Maximize, Some(left)), Some(Action::LeftHalf));
    assert_eq!(cfg.shortcut(Action::Maximize), Some(left));
    assert_eq!(cfg.shortcut(Action::LeftHalf), None);
    Config::edit(|c| *c = cfg.clone()).unwrap();
    let back = Config::load();
    assert_eq!(back.shortcut(Action::LeftHalf), None, "none is kept as none, not the default");
    assert!(!back.shortcuts_are_defaults());
    let mut back = back;
    back.reset_shortcuts();
    assert!(back.shortcuts_are_defaults());
    back.clear_shortcuts();
    assert!(back.shortcuts_are_none());
    assert!(back.shortcuts().is_empty());
    Config::edit(|c| *c = back.clone()).unwrap();
    assert!(Config::load().shortcuts_are_none(), "none is kept for every action");
}

#[test]
fn a_default_given_back_is_not_written() {
    let mut cfg = Config::default();
    let s = Shortcut::from_config("ctrl-alt-cmd-l").unwrap();
    cfg.set_shortcut(Action::LeftHalf, Some(s));
    cfg.set_shortcut(Action::LeftHalf, Action::LeftHalf.default_shortcut());
    assert!(cfg.shortcuts.is_empty());
}

#[test]
fn a_write_leaves_no_temp_file() {
    let (dir, _g) = isolated();
    paths::write_json(&paths::state_dir().join("x.json"), &serde_json::json!({"a": 1})).unwrap();
    let names: Vec<String> = std::fs::read_dir(dir.path().join("state")).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["x.json"]);
}

#[test]
fn the_version_is_read_off_the_redirect() {
    assert_eq!(update::version_from_location("https://github.com/o/r/releases/tag/v1.2.3").as_deref(), Some("1.2.3"));
    assert_eq!(update::version_from_location("https://github.com/o/r/releases/tag/1.2.3/").as_deref(), Some("1.2.3"));
    assert_eq!(update::version_from_location("https://github.com/o/r/releases"), None);
    assert!(update::is_newer("0.10.0", "0.9.9"));
    assert!(!update::is_newer("1.0", "1.0.0"));
}

#[test]
fn every_system_has_an_installer_name() {
    let name = update::asset_name().expect("this system has an installer");
    assert!(name.starts_with(update::APP), "{name}");
}
