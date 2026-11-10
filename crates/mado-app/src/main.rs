//! Mado (窓, a window): the window in front, moved and sized from the
//! keyboard.
//!
//! A menu bar app: an item in the menu bar and no Dock icon, except while
//! the settings are open. A shortcut puts the window in front where it
//! says (a half, a corner, the whole screen, the next display), or every
//! window on its screen in rows, columns or a grid; the core decides where
//! (`mado_core::layout`), and `sys` carries it out.

// A release build on Windows has no console window beside it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod csd;
mod fonts;
mod i18n;
mod look;
mod mado;
mod menu;
mod settings;
mod sys;
mod ui_state;
mod widgets;

use gpui::*;

use i18n::t;
use mado::{Escape, Mado, OpenSettings};
use mado_core::action::Action;

actions!(mado_app, [Quit, CloseWindow, Hide, HideOthers, ShowAll, Minimize]);

/// A shortcut is named as the Mac has it and bound as `secondary-<key>`,
/// which gpui makes ⌘ on a Mac and Ctrl elsewhere.
fn key_bindings() -> Vec<KeyBinding> {
    let mut keys = vec![
        KeyBinding::new("escape", Escape, Some(settings::SETTINGS_CONTEXT)),
        KeyBinding::new("secondary-,", OpenSettings, None),
        KeyBinding::new("secondary-w", CloseWindow, None),
        KeyBinding::new("secondary-q", Quit, None),
    ];
    if cfg!(target_os = "macos") {
        keys.extend([KeyBinding::new("secondary-h", Hide, None), KeyBinding::new("secondary-alt-h", HideOthers, None), KeyBinding::new("secondary-m", Minimize, None)]);
    }
    keys
}

/// The app's menus at the screen's top, there while the settings are
/// open and Mado is in the Dock: the Mac's own choices for an app, and
/// Edit for what is typed, made again when the language changes.
pub(crate) fn app_menus() -> Vec<Menu> {
    let mac = cfg!(target_os = "macos");
    let mut app_items = vec![MenuItem::action(t("Settings…"), OpenSettings), MenuItem::separator()];
    if mac {
        app_items.extend([MenuItem::action(t("Hide Mado"), Hide), MenuItem::action(t("Hide Others"), HideOthers), MenuItem::action(t("Show All"), ShowAll), MenuItem::separator()]);
    }
    app_items.push(MenuItem::action(t("Quit Mado"), Quit));
    let mut menus = vec![Menu { name: "Mado".into(), items: app_items, disabled: false }, Menu { name: t("File").into(), disabled: false, items: vec![MenuItem::action(t("Close Window"), CloseWindow)] }];
    menus.push(Menu { name: t("Window").into(), disabled: false, items: vec![MenuItem::action(t("Minimize"), Minimize)] });
    menus
}

/// The app's menus set, and the Window menu marked as the system's, so
/// macOS puts Move & Resize in it and Mado's own window moves by them as
/// any app's does (`sys::mark_windows_menu`).
pub(crate) fn set_app_menus(cx: &mut App) {
    cx.set_menus(app_menus());
    crate::sys::mark_windows_menu(&t("Window"));
}

fn with_active_window(cx: &mut App, f: impl FnOnce(&mut Window) + 'static) {
    let Some(w) = cx.active_window() else { return };
    cx.defer(move |cx| {
        w.update(cx, |_, window, _| f(window)).ok();
    });
}

/// For a script: a picture of the settings once they are drawn, and after
/// any moves the steps asked for (`run:`, 0.7 s apart from a second in),
/// then quit.
fn shoot_later(path: std::path::PathBuf, runs: u64, cx: &mut App) {
    let wait = if runs == 0 { 900 } else { 1000 + 700 * runs + 600 };
    cx.spawn(async move |cx| {
        cx.background_executor().timer(std::time::Duration::from_millis(wait)).await;
        let _ = cx.update(|cx| {
            let handle = mado::get(cx).read(cx).settings;
            let shot = match handle {
                Some(h) => h.update(cx, |_, w, _| sys::shoot_window(w, &path)).unwrap_or_else(|e| Err(e.to_string())),
                None => Err("the settings are not open".into()),
            };
            if let Err(e) = shot {
                eprintln!("mado: MADO_SHOT: {e}");
            }
            cx.quit();
        });
    })
    .detach();
}

fn main() {
    // Velopack's installer runs the app with a hook to answer before
    // anything else.
    #[cfg(windows)]
    velopack::VelopackApp::build().run();
    sys::limit_waits();
    let app = gpui_platform::application().with_assets(assets::Assets);
    // Opened again from Finder or Spotlight while running: the settings,
    // which is the way back to them with the menu bar item hidden.
    app.on_reopen(|cx| {
        if cx.try_global::<mado::Global_>().is_some() {
            let m = mado::get(cx);
            m.update(cx, |m, cx| m.open_settings(None, cx));
        }
    });
    app.run(move |cx: &mut App| {
        gpui_component::init(cx);
        gpui_component::tooltip::set_label(|text, window, cx| widgets::tip(text, window, cx));
        sys::install_dock_icon();
        sys::stay_in_menu_bar();
        let cfg = mado_core::config::Config::load();
        i18n::set(&i18n::chosen(&cfg.app.language));
        fonts::install(cx);
        look::install(cx);
        look::apply(&cfg.app.appearance, None, cx);

        // A copy with a home of its own is a script's: it holds no
        // shortcut, has no menu bar item and ends no other copy. Two that
        // held the shortcuts would each take them, so a copy opened while
        // another runs ends it, and the one opened last stays.
        let probe = std::env::var_os("MADO_HOME").is_some_and(|h| !h.is_empty());
        if !probe {
            sys::end_other_copies();
        }
        let model = cx.new(|cx| Mado::new(probe, cx));
        cx.set_global(mado::Global_(model.clone()));

        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_action(|_: &CloseWindow, cx| with_active_window(cx, |w| w.remove_window()));
        cx.on_action(|_: &Minimize, cx| with_active_window(cx, |w| w.minimize_window()));
        cx.on_action(|_: &Hide, cx| cx.hide());
        cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
        cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
        cx.on_action(|_: &OpenSettings, cx| {
            let m = mado::get(cx);
            m.update(cx, |m, cx| m.open_settings(None, cx));
        });
        cx.bind_keys(key_bindings());
        set_app_menus(cx);

        // The settings closed by any way (its button, ⌘W, Escape): out of
        // the Dock. `on_window_should_close` is not asked by ⌘W.
        cx.on_window_closed(|cx, id| {
            let m = mado::get(cx);
            let ours = m.read(cx).settings.is_some_and(|h| h.window_id() == id);
            if ours {
                m.update(cx, |m, cx| m.settings_closed(cx));
            }
        })
        .detach();

        if !probe {
            // The shortcuts: each press, by its action's place in the list.
            if let Some(mut presses) = sys::install_hotkeys() {
                let m = model.clone();
                cx.spawn(async move |cx| {
                    use futures::StreamExt as _;
                    while let Some(n) = presses.next().await {
                        let Some(action) = Action::ALL.get(n as usize).copied() else { continue };
                        let _ = m.update(cx, |m, cx| m.run(action, cx));
                    }
                })
                .detach();
            }
            model.update(cx, |m, _| m.apply_shortcuts());

            // The menu bar item: one click opens the settings, the other
            // shows its menu of the actions.
            let (swap, shown) = (cfg.menu_bar.swap, cfg.menu_bar.shown);
            if let Some(mut picks) = sys::install_menu_bar(swap, shown) {
                let m = model.clone();
                cx.spawn(async move |cx| {
                    use futures::StreamExt as _;
                    while let Some(pick) = picks.next().await {
                        let _ = m.update(cx, |m, cx| match pick {
                            sys::MenuPick::Line(key) => m.menu_pick(&key, cx),
                            sys::MenuPick::Settings => m.open_settings(None, cx),
                        });
                    }
                })
                .detach();
            }

            // The app in front, for Ignore and the menu's line naming it.
            let (now, mut fronts) = sys::front_apps();
            model.update(cx, |m, _| {
                if let Some(now) = now {
                    m.front_changed(now);
                }
            });
            let m = model.clone();
            cx.spawn(async move |cx| {
                use futures::StreamExt as _;
                while let Some(front) = fronts.next().await {
                    let _ = m.update(cx, |m, _| m.front_changed(front));
                }
            })
            .detach();

            // Drags to the snap areas, watched off the main thread; the
            // footprint is drawn here.
            let state = model.read(cx).snap.clone();
            let mut footprints = sys::install_snapping(state);
            cx.spawn(async move |_| {
                use futures::StreamExt as _;
                while let Some(news) = footprints.next().await {
                    sys::footprint(news);
                }
            })
            .detach();
        }
        model.update(cx, |m, _| m.sync());

        // The keys pressed while a shortcut is recorded.
        let mut pressed = sys::install_key_watch();
        let m = model.clone();
        cx.spawn(async move |cx| {
            use futures::StreamExt as _;
            while let Some((flags, key)) = pressed.next().await {
                let _ = m.update(cx, |m, cx| m.recorded(flags, key, cx));
            }
        })
        .detach();

        // Accessibility given or taken in System Settings, while Mado runs.
        let mut trust = sys::trust_changes();
        let m = model.clone();
        cx.spawn(async move |cx| {
            use futures::StreamExt as _;
            while trust.next().await.is_some() {
                let _ = m.update(cx, |m, cx| m.check_trust_soon(cx));
            }
        })
        .detach();

        // For a script: a section of the settings, steps, a picture.
        let section = std::env::var("MADO_SETTINGS").ok();
        let go = std::env::var("MADO_GO").ok();
        let shot = std::env::var_os("MADO_SHOT").filter(|p| !p.is_empty());
        let first_launch = !ui_state::UiState::load().opened;
        if section.is_some() || go.is_some() || shot.is_some() || first_launch || !cfg!(target_os = "macos") {
            // The first launch shows the settings, where Accessibility is
            // allowed; later ones start in the menu bar.
            if first_launch && !probe {
                let mut ui = ui_state::UiState::load();
                ui.opened = true;
                ui.save();
            }
            model.update(cx, |m, cx| {
                m.open_settings(section.as_deref().map(settings::section_named), cx);
                if let Some(steps) = go.as_deref() {
                    m.go(steps, cx);
                }
            });
        }
        if let Some(path) = shot {
            let runs = go.as_deref().map_or(0, |g| g.split(';').filter(|s| s.starts_with("run:")).count() as u64);
            shoot_later(path.into(), runs, cx);
        }
    });
}
