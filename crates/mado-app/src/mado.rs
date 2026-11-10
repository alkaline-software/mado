//! The app's one `Mado`: the settings, the shortcuts the system holds for
//! it, Accessibility, launch at login and the updates. It outlives the
//! settings window, which draws it (`settings.rs`), so a shortcut never
//! waits on a window and a change shows in the window at once.
//!
//! A shortcut's press, or a line of the menu, comes in on the main thread
//! (`run`); the screens are read there, and the move goes to the
//! background executor, since each step is a question to another app
//! (`sys::place_front`). A drag to a snap area is watched by a thread of
//! its own (`sys::snap`), which shares the memory of what was moved.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::*;

use mado_core::action::Action;
use mado_core::config::Config;
use mado_core::layout::Memory;
use mado_core::shortcut::{Shortcut, BACKSPACE, ESCAPE, FORWARD_DELETE, MODS};
use mado_core::update;

use crate::i18n::{t, tf};
use crate::menu::{Entry, Menus};
use crate::widgets::Motion;

actions!(mado, [OpenSettings, Escape]);

/// The app's `Mado`, for what reaches no view: a hot key, the menu bar.
pub struct Global_(pub Entity<Mado>);

impl Global for Global_ {}

pub fn get(cx: &App) -> Entity<Mado> {
    cx.global::<Global_>().0.clone()
}

pub struct Mado {
    pub(crate) cfg: Config,
    pub(crate) motion: Motion,
    pub(crate) menus: Menus,
    /// What Mado remembers of the windows it moved. Held by a move for as
    /// long as it runs, off the main thread; nothing on the main thread
    /// waits for it.
    memory: Arc<Mutex<Memory>>,
    /// What the drags' thread reads: whether they snap and how.
    pub(crate) snap: Arc<crate::sys::SnapState>,
    /// The app in front, for Ignore and for the menu's line naming it.
    pub(crate) front: Option<crate::sys::FrontApp>,
    pub(crate) settings: Option<AnyWindowHandle>,
    /// The settings window's native ground as last set: AppKit is told of
    /// a change only, never at every frame.
    pub(crate) ground: std::cell::Cell<Option<Hsla>>,
    pub(crate) settings_section: &'static str,
    /// The action chosen on Shortcuts, and the one before it with the
    /// change's number, for the hero's plate to slide from.
    pub(crate) picked: Action,
    pub(crate) picked_from: (Action, u32),
    pub(crate) settings_scroll: ScrollHandle,
    pub(crate) lang_serial: u32,
    pub(crate) update: UpdateRow,
    /// Whether Accessibility is given, as last found by trying.
    pub(crate) trusted: bool,
    /// Allow was pressed once: the next press opens System Settings.
    pub(crate) asked_trust: bool,
    pub(crate) at_login: bool,
    pub(crate) login_said: Option<String>,
    /// The action whose shortcut is being recorded.
    pub(crate) recording: Option<Action>,
    /// What a row says of its shortcut after a press: why it cannot be
    /// one, or which action it was taken from.
    pub(crate) record_said: Option<(Action, String)>,
    /// The shortcuts the system would not hold for Mado.
    pub(crate) refused: Vec<Action>,
    /// A copy run by a script with a home of its own: it holds no
    /// shortcut and ends no other copy.
    pub(crate) probe: bool,
    /// A probe drew Accessibility as it asked (`MADO_GO=untrusted`), and
    /// no try overrides it.
    trust_drawn: bool,
    /// How many `run:` steps a probe has asked for, to space them out.
    probe_runs: u64,
    /// The settings window is being dragged by the person.
    own_drag: bool,
    /// Whether macOS's own tiling takes a window dragged to an edge too
    /// (`sys::system_tiling`); none until read, and where there is none.
    pub system_tiling: Option<bool>,
    /// A probe drew macOS's tiling as on (`MADO_GO=tiling`), and no read
    /// overrides it.
    tiling_drawn: bool,
}

/// What the Updates row says and does.
#[derive(Default, Clone)]
pub(crate) struct UpdateRow {
    pub checking: bool,
    pub found: Option<String>,
    /// While a download runs: how far, from 0 to 100.
    pub downloading: Option<u32>,
    pub said: Option<String>,
}

impl Mado {
    pub fn new(probe: bool, cx: &mut Context<Self>) -> Self {
        let ui = crate::ui_state::UiState::load();
        let cfg = Config::load();
        crate::sys::set_animate(cfg.app.animate);
        let memory: Arc<Mutex<Memory>> = Arc::default();
        let snap = Arc::new(crate::sys::SnapState { on: std::sync::atomic::AtomicBool::new(false), how: Mutex::new(cfg.snapping.clone()), memory: memory.clone() });
        let mado = Self {
            cfg,
            motion: Motion::default(),
            menus: Menus::default(),
            memory,
            snap,
            front: None,
            settings: None,
            ground: std::cell::Cell::new(None),
            settings_section: crate::settings::section_named(ui.settings_section.as_deref().unwrap_or("general")),
            picked: Action::LeftHalf,
            picked_from: (Action::LeftHalf, 0),
            settings_scroll: ScrollHandle::new(),
            lang_serial: 0,
            update: UpdateRow::default(),
            // Until the first try says otherwise, so a granted Mado does
            // not show Allow for a frame.
            trusted: true,
            asked_trust: false,
            at_login: crate::sys::launches_at_login(),
            login_said: None,
            recording: None,
            record_said: None,
            refused: Vec::new(),
            probe,
            trust_drawn: false,
            probe_runs: 0,
            own_drag: false,
            system_tiling: None,
            tiling_drawn: false,
        };
        mado.check_system_tiling(cx);
        if mado.cfg.app.check_updates && !probe {
            mado.check_updates_later(cx);
        }
        mado
    }

    // -- the shortcuts --------------------------------------------------------------

    /// Whether the app in front is one Mado leaves alone.
    pub fn ignoring(&self) -> bool {
        self.front.as_ref().is_some_and(|f| self.cfg.ignored.contains(&f.id))
    }

    /// The shortcuts handed to the system as the config has them, or none
    /// while one is being recorded, so the keys pressed reach the window,
    /// and none while an app Mado leaves alone is in front, so its keys
    /// reach it.
    pub fn apply_shortcuts(&mut self) {
        if self.probe {
            return;
        }
        let keys = if self.recording.is_some() || self.ignoring() { Vec::new() } else { self.cfg.shortcuts() };
        self.refused = crate::sys::set_hotkeys(&keys);
        if !self.refused.is_empty() {
            crate::sys::log(format_args!("refused: {:?}", self.refused));
        }
    }

    /// A shortcut was pressed: the window in front goes where it says, or
    /// for a tiling, every window on its screen.
    pub fn run(&mut self, action: Action, cx: &mut Context<Self>) {
        let screens = crate::sys::screens();
        let memory = self.memory.clone();
        if action.tiles() {
            let pointer = crate::sys::pointer();
            let task = cx.background_executor().spawn(async move { crate::sys::tile_windows(action, &screens, &memory, pointer) });
            cx.spawn(async move |this, cx| {
                if let Err(why) = task.await {
                    let _ = this.update(cx, |this, cx| this.refused_move(action, why, cx));
                }
            })
            .detach();
            return;
        }
        let own = self.memory.clone();
        let task = cx.background_executor().spawn(async move { crate::sys::place_front(action, &screens, &memory) });
        cx.spawn(async move |this, cx| {
            let mut done = task.await;
            let mut settling = None;
            if done == Err(crate::sys::Refused::Ours) {
                // Mado's own window is in front: moved here, on the main
                // thread, where AppKit sets its frame, and outside any
                // update of the app. Inside one, gpui cannot take the
                // window's resize as it happens, and the window kept its
                // old drawing stretched over the new frame.
                done = crate::sys::place_own(action, &crate::sys::screens(), &own).map(|(plan, moving)| {
                    settling = moving.then_some(plan);
                    plan
                });
            }
            if let Some(plan) = settling {
                // Carried over about a third of a second: noted where it
                // came to once it is over.
                cx.background_executor().timer(Duration::from_millis(crate::sys::TILE_MS + 80)).await;
                crate::sys::own_settled(&own, &plan, &crate::sys::screens());
            }
            if let Err(why) = done {
                let _ = this.update(cx, |this, cx| this.refused_move(action, why, cx));
            }
        })
        .detach();
    }

    /// A move that did not go: noted in the log, and when Accessibility is
    /// what it wants, the settings show where it is given, since nothing
    /// moves until it is.
    fn refused_move(&mut self, action: Action, why: crate::sys::Refused, cx: &mut Context<Self>) {
        crate::sys::log(format_args!("{}: {why:?}", action.key()));
        if why == crate::sys::Refused::NotTrusted {
            self.trusted = false;
            self.open_settings(Some("general"), cx);
        }
    }

    /// The settings window's top was pressed and dragged: the window server
    /// moves it, and no global monitor sees the drag of the app's own
    /// window, so the app watches it here, a frame at a time while the
    /// button is held. Once the cursor is `AWAY` from where it began, the
    /// window takes back its size as another app's does; over a snap area
    /// the footprint shows, and let go there, the window is put there.
    pub fn own_drag_began(&mut self, cx: &mut Context<Self>) {
        let snapping = self.cfg.snapping.on;
        if self.own_drag || self.probe {
            return;
        }
        let Some((id, start)) = crate::sys::own_window() else { return };
        self.own_drag = true;
        let from = crate::sys::pointer();
        let memory = self.memory.clone();
        let screens = crate::sys::screens();
        cx.spawn(async move |this, cx| {
            // Watched until the button is let go, not only until the
            // window is away: the drag's later moves would start another
            // watch, which took the window's place mid-drag for where it
            // began and gave Restore that place to go back to.
            let mut away = false;
            let mut area: Option<(usize, mado_core::action::Action)> = None;
            let plan = |hit: Option<(usize, mado_core::action::Action)>| hit.and_then(|(s, a)| mado_core::layout::Memory::snap_plan(a, &screens, s));
            while crate::sys::left_button_down() {
                cx.background_executor().timer(Duration::from_millis(16)).await;
                let (x, y) = crate::sys::pointer();
                if !away && (x - from.0).hypot(y - from.1) >= mado_core::layout::AWAY {
                    away = true;
                    // Here, in the task's own body: inside an update gpui
                    // would not take the resize as it happens.
                    crate::sys::own_dragged_away(&memory, id, start, x);
                }
                // A press that has not moved yet asks for no area: the window
                // may be pressed where one is.
                let moved = (x - from.0).hypot(y - from.1) >= 3.;
                let hit = if snapping && moved { mado_core::snap::at(x, y, &screens) } else { None };
                if hit != area {
                    area = hit;
                    crate::sys::footprint(match plan(hit) {
                        Some(p) => crate::sys::Footprint::Show(p.frame, crate::sys::own_window().map(|(_, f)| f)),
                        None => crate::sys::Footprint::Hide,
                    });
                }
            }
            if let Some(p) = plan(area) {
                crate::sys::footprint(crate::sys::Footprint::Hide);
                // The window server may move the window once more as the
                // drag ends: put after that.
                cx.background_executor().timer(Duration::from_millis(50)).await;
                if crate::sys::own_snapped(&memory, id, start, &p, &screens) {
                    cx.background_executor().timer(Duration::from_millis(crate::sys::TILE_MS + 80)).await;
                    crate::sys::own_settled(&memory, &p, &screens);
                }
            }
            let _ = this.update(cx, |this, _| this.own_drag = false);
        })
        .detach();
    }

    /// An action chosen on Shortcuts: the hero's plate slides from the
    /// last one's place to its.
    pub fn pick(&mut self, action: Action, cx: &mut Context<Self>) {
        if self.picked != action {
            self.stop_recording(cx);
            self.record_said = None;
            self.picked_from = (self.picked, self.picked_from.1 + 1);
            self.picked = action;
        }
        cx.notify();
    }

    pub fn start_recording(&mut self, action: Action, cx: &mut Context<Self>) {
        self.recording = Some(action);
        self.record_said = None;
        crate::sys::record_keys(true);
        self.apply_shortcuts();
        cx.notify();
    }

    pub fn stop_recording(&mut self, cx: &mut Context<Self>) {
        if self.recording.take().is_some() {
            crate::sys::record_keys(false);
            self.apply_shortcuts();
        }
        cx.notify();
    }

    /// A key pressed while a shortcut is recorded: Esc alone gives up,
    /// Delete alone leaves the action with none, and anything else is the
    /// shortcut, or the row says why it cannot be.
    pub fn recorded(&mut self, flags: u64, key: u16, cx: &mut Context<Self>) {
        let Some(action) = self.recording else { return };
        let alone = flags & MODS == 0;
        if key == ESCAPE && alone {
            self.stop_recording(cx);
            return;
        }
        if (key == BACKSPACE || key == FORWARD_DELETE) && alone {
            self.set_shortcut(action, None);
            self.stop_recording(cx);
            return;
        }
        match Shortcut::new(flags, key) {
            Ok(s) => {
                self.set_shortcut(action, Some(s));
                self.stop_recording(cx);
            }
            Err(e) => {
                self.record_said = Some((action, t(e.said()).to_string()));
                cx.notify();
            }
        }
    }

    /// An action's shortcut set, or none. One that was another action's is
    /// taken from it, and the row says so.
    pub fn set_shortcut(&mut self, action: Action, shortcut: Option<Shortcut>) {
        let taken = self.cfg.set_shortcut(action, shortcut);
        let shortcuts = self.cfg.shortcuts.clone();
        let _ = Config::edit(|c| c.shortcuts = shortcuts);
        self.record_said = taken.map(|other| (action, tf("Taken from {action}", &[("action", &t(other.name()))])));
        self.apply_shortcuts();
        self.sync();
    }

    pub fn reset_shortcuts(&mut self, cx: &mut Context<Self>) {
        self.cfg.reset_shortcuts();
        let _ = Config::edit(|c| c.reset_shortcuts());
        self.record_said = None;
        self.apply_shortcuts();
        self.sync();
        cx.notify();
    }

    /// Every action left with no shortcut.
    pub fn clear_shortcuts(&mut self, cx: &mut Context<Self>) {
        self.cfg.clear_shortcuts();
        let _ = Config::edit(|c| c.clear_shortcuts());
        self.record_said = None;
        self.apply_shortcuts();
        self.sync();
        cx.notify();
    }

    /// Whether macOS's own tiling is on, read off the main thread: at
    /// launch and each time the settings come back in front, the person
    /// having maybe turned it off in Desktop & Dock meanwhile.
    pub fn check_system_tiling(&self, cx: &mut Context<Self>) {
        if self.tiling_drawn {
            return;
        }
        let task = cx.background_executor().spawn(async { crate::sys::system_tiling() });
        cx.spawn(async move |this, cx| {
            let now = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.system_tiling != now && !this.tiling_drawn {
                    this.system_tiling = now;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    // -- Accessibility --------------------------------------------------------------

    /// Whether Accessibility is given, found by trying, off the main
    /// thread. Asked when the settings open or come back in front, after
    /// Allow, and at each of the system's notices; never on a timer.
    pub fn check_trust(&mut self, cx: &mut Context<Self>) {
        if self.trust_drawn {
            return;
        }
        let task = cx.background_executor().spawn(async { crate::sys::trusted() });
        cx.spawn(async move |this, cx| {
            let now = task.await;
            let _ = this.update(cx, |this, cx| this.trust_found(now, cx));
        })
        .detach();
    }

    fn trust_found(&mut self, now: bool, cx: &mut Context<Self>) {
        let gained = now && !self.trusted;
        self.trusted = now;
        self.sync();
        if gained && self.asked_trust {
            // The system's question or System Settings are done: the
            // settings come back in front.
            self.asked_trust = false;
            self.open_settings(None, cx);
        }
        cx.notify();
    }

    /// The system's notice that Accessibility's list changed comes a
    /// moment before the change is written: asked again over the next few
    /// seconds, until it is given.
    pub fn check_trust_soon(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            for _ in 0..12 {
                cx.background_executor().timer(Duration::from_millis(250)).await;
                let now = cx.background_executor().spawn(async { crate::sys::trusted() }).await;
                let done = this.update(cx, |this, cx| {
                    this.trust_found(now, cx);
                    now
                });
                if done.unwrap_or(true) {
                    return;
                }
            }
        })
        .detach();
    }

    /// Allow: the system's question; pressed again, System Settings, where
    /// the switch is.
    pub fn ask_trust(&mut self, cx: &mut Context<Self>) {
        if self.asked_trust {
            crate::sys::open(crate::sys::ACCESSIBILITY_SETTINGS);
        } else {
            self.asked_trust = true;
            cx.background_executor().spawn(async { crate::sys::ask_trust() }).detach();
        }
        self.check_trust_soon(cx);
        cx.notify();
    }

    // -- the settings that are not shortcuts -------------------------------------------

    pub fn set_launch_at_login(&mut self, on: bool, cx: &mut Context<Self>) {
        match crate::sys::set_launch_at_login(on) {
            Ok(()) => self.login_said = None,
            Err(e) => self.login_said = Some(e),
        }
        self.at_login = crate::sys::launches_at_login();
        cx.notify();
    }

    /// Animate windows: carried to their places, or put there at once.
    pub fn set_animate(&mut self, on: bool, cx: &mut Context<Self>) {
        self.cfg.app.animate = on;
        let _ = Config::edit(|c| c.app.animate = on);
        crate::sys::set_animate(on);
        cx.notify();
    }

    pub fn set_menu_bar(&mut self, shown: bool, swap: bool, cx: &mut Context<Self>) {
        self.cfg.menu_bar.shown = shown;
        self.cfg.menu_bar.swap = swap;
        let _ = Config::edit(|c| {
            c.menu_bar.shown = shown;
            c.menu_bar.swap = swap;
        });
        crate::sys::set_menu_bar(swap, shown);
        self.sync();
        cx.notify();
    }

    /// Changes to how windows are placed, each written to the config and
    /// handed to the drags' thread.
    pub fn edit(&mut self, change: impl Fn(&mut Config), cx: &mut Context<Self>) {
        change(&mut self.cfg);
        let _ = Config::edit(|c| change(c));
        self.sync();
        cx.notify();
    }

    /// The drags' thread, the shortcuts and the menu brought in step with
    /// the config, Accessibility and the app in front.
    pub fn sync(&mut self) {
        let on = self.trusted && !self.ignoring() && !self.probe;
        self.snap.on.store(on, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut how) = self.snap.how.lock() {
            *how = self.cfg.snapping.clone();
        }
        crate::sys::set_menu_lines(self.menu_lines());
    }

    /// Another app came to the front.
    pub fn front_changed(&mut self, front: crate::sys::FrontApp) {
        // Mado itself in front (its settings): the app before it stays the
        // one the menu names.
        if front.id.starts_with("com.alkalinesoftware.mado") {
            return;
        }
        let was = self.ignoring();
        self.front = Some(front);
        if was != self.ignoring() {
            self.apply_shortcuts();
        }
        self.sync();
    }

    /// The app in front left alone, or no longer.
    pub fn toggle_ignore(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.front.as_ref().map(|f| f.id.clone()).filter(|id| !id.is_empty()) else { return };
        self.edit(
            move |c| {
                if let Some(at) = c.ignored.iter().position(|x| *x == id) {
                    c.ignored.remove(at);
                } else {
                    c.ignored.push(id.clone());
                }
            },
            cx,
        );
        self.apply_shortcuts();
    }

    /// The menu bar item's menu, in the settings' order: each group's
    /// actions with a line between the groups, each with its picture and
    /// its shortcut. Then the app in front to leave alone, the settings,
    /// About and Quit.
    pub fn menu_lines(&self) -> Vec<crate::sys::MenuLine> {
        use crate::sys::MenuLine;
        use mado_core::action::Group;
        let item = |a: Action| MenuLine::Item { key: a.key().into(), title: t(a.name()).into(), icon: crate::assets::action_icon(a), keys: self.cfg.shortcut(a).filter(|_| !self.ignoring()), checked: false, enabled: true };
        let mut lines = Vec::new();
        for g in Group::ALL {
            if !lines.is_empty() {
                lines.push(MenuLine::Gap);
            }
            lines.extend(Action::ALL.into_iter().filter(|a| a.group() == g).map(item));
        }
        lines.push(MenuLine::Gap);
        let icon = |path: &str| crate::assets::bytes(path);
        if let Some(front) = self.front.as_ref().filter(|f| !f.id.is_empty()) {
            lines.push(MenuLine::Item { key: "ignore".into(), title: tf("Ignore {app}", &[("app", &front.name)]), icon: icon("icons/eye-off.svg"), keys: None, checked: self.ignoring(), enabled: true });
            lines.push(MenuLine::Gap);
        }
        lines.push(MenuLine::Item { key: "settings".into(), title: t("Settings…").into(), icon: icon("icons/settings.svg"), keys: None, checked: false, enabled: true });
        lines.push(MenuLine::Item { key: "about".into(), title: t("About Mado").into(), icon: icon("icons/info.svg"), keys: None, checked: false, enabled: true });
        lines.push(MenuLine::Item { key: "quit".into(), title: t("Quit Mado").into(), icon: icon("icons/logout.svg"), keys: Shortcut::from_config("cmd-q"), checked: false, enabled: true });
        lines
    }

    /// A line chosen from the menu bar item's menu.
    pub fn menu_pick(&mut self, key: &str, cx: &mut Context<Self>) {
        match key {
            "quit" => cx.quit(),
            "settings" => self.open_settings(None, cx),
            "about" => self.open_settings(Some("about"), cx),
            "ignore" => self.toggle_ignore(cx),
            other => {
                if let Some(a) = Action::from_key(other) {
                    self.run(a, cx);
                }
            }
        }
    }

    // -- the settings window and the menu bar ---------------------------------------------

    /// The settings shown, on `section` or where they were left.
    pub fn open_settings(&mut self, section: Option<&'static str>, cx: &mut Context<Self>) {
        if let Some(s) = section {
            if self.settings_section != s {
                self.settings_section = s;
                self.settings_scroll.set_offset(point(px(0.), px(0.)));
            }
        }
        // After this update: opening reads the model this call is inside.
        let weak = cx.entity().downgrade();
        cx.defer(move |cx| crate::settings::open(weak, cx));
        cx.notify();
    }

    /// The settings window has closed: out of the Dock, and a recording
    /// given up.
    pub fn settings_closed(&mut self, cx: &mut Context<Self>) {
        self.settings = None;
        self.ground.set(None);
        self.menus.close(cx);
        self.stop_recording(cx);
        crate::sys::in_dock(false);
    }

    fn check_updates_later(&self, cx: &mut Context<Self>) {
        if update::from_store() {
            return;
        }
        let state = update::UpdateState::load();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.);
        if now - state.last_check < 86_400. {
            if update::is_newer(&state.latest, update::current_version()) {
                let latest = state.latest.clone();
                cx.spawn(async move |this, cx| {
                    let _ = this.update(cx, |this, cx| {
                        this.update.found = Some(latest);
                        cx.notify();
                    });
                })
                .detach();
            }
            return;
        }
        cx.spawn(async move |this, cx| {
            let _ = this.update(cx, |this, cx| this.check_updates(cx));
        })
        .detach();
    }

    /// Ask for the newest version, off the main thread.
    pub(crate) fn check_updates(&mut self, cx: &mut Context<Self>) {
        self.update.checking = true;
        self.update.said = None;
        cx.notify();
        let task = cx.background_executor().spawn(async { update::latest() });
        cx.spawn(async move |this, cx| {
            let found = task.await;
            let _ = this.update(cx, |this, cx| {
                this.update.checking = false;
                match found {
                    Ok(latest) => {
                        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.);
                        let _ = update::UpdateState { last_check: now, latest: latest.clone() }.save();
                        if update::is_newer(&latest, update::current_version()) {
                            this.update.found = Some(latest);
                        } else {
                            this.update.found = None;
                            this.update.said = Some(t("Up to date.").into());
                        }
                    }
                    Err(e) => this.update.said = Some(tf("The check did not go through: {why}", &[("why", &e)])),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Fetch the newest version and put it in place, then quit: the new
    /// app opens once this one has gone.
    pub(crate) fn install_update(&mut self, cx: &mut Context<Self>) {
        let Some(version) = self.update.found.clone() else { return };
        self.update.downloading = Some(0);
        cx.notify();
        let (tx, rx) = std::sync::mpsc::channel::<u32>();
        let task = cx.background_executor().spawn(async move {
            update::download(&version, &|done, total| {
                if let Some(total) = total.filter(|t| *t > 0) {
                    let _ = tx.send((done * 100 / total) as u32);
                }
            })
            .and_then(|file| update::install(&file))
        });
        cx.spawn(async move |this, cx| {
            let ticker = async {
                loop {
                    cx.background_executor().timer(Duration::from_millis(200)).await;
                    let p = rx.try_iter().last();
                    if this
                        .update(cx, |this, cx| {
                            if let Some(p) = p {
                                this.update.downloading = Some(p);
                                cx.notify();
                            }
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            };
            let done = first_of(task, ticker).await;
            let _ = this.update(cx, |this, cx| {
                this.update.downloading = None;
                match done {
                    Ok(()) => cx.quit(),
                    Err(e) => this.update.said = Some(tf("The update did not go through: {e}", &[("e", &e)])),
                }
                cx.notify();
            });
        })
        .detach();
    }

    // -- probes ------------------------------------------------------------------

    /// For a script: states drawn without the person's hands, by name
    /// (`MADO_GO=untrusted;record`).
    pub fn go(&mut self, steps: &str, cx: &mut Context<Self>) {
        for step in steps.split(';') {
            match step {
                // Accessibility drawn as not given, and as after one Allow.
                "untrusted" | "asked" => {
                    self.trust_drawn = true;
                    self.trusted = false;
                    self.asked_trust = step == "asked";
                }
                // The first shortcut on the page waiting for keys.
                "record" => {
                    self.recording = Some(self.picked);
                }
                // A press that cannot be a shortcut, on the first row.
                "wrong" => {
                    self.recording = Some(self.picked);
                    self.record_said = Some((self.picked, t(mado_core::shortcut::Invalid::NoModifier.said()).to_string()));
                }
                // An action run on the window in front, a second after the
                // settings open: a probe copy is in front, so it moves its
                // own window, and the dev log says where it went.
                _ if step.starts_with("run:") => {
                    if let Some(action) = Action::from_key(&step[4..]) {
                        let n = self.probe_runs;
                        self.probe_runs += 1;
                        cx.spawn(async move |this, cx| {
                            cx.background_executor().timer(Duration::from_millis(1000 + 700 * n)).await;
                            let _ = this.update(cx, |this, cx| this.run(action, cx));
                        })
                        .detach();
                    }
                }
                // An action chosen on Shortcuts.
                _ if step.starts_with("pick:") => {
                    if let Some(a) = Action::from_key(&step[5..]) {
                        self.pick(a, cx);
                    }
                }
                // macOS's own tiling drawn as on, whatever the system's is.
                "tiling" => {
                    self.tiling_drawn = true;
                    self.system_tiling = Some(true);
                }
                // The footprint over the left half of the probe's own
                // window, for a picture of it over something known.
                "footprint" => {
                    cx.spawn(async move |_, cx| {
                        cx.background_executor().timer(Duration::from_millis(900)).await;
                        if let Some((_, f)) = crate::sys::own_window() {
                            crate::sys::footprint(crate::sys::Footprint::Show(mado_core::layout::Rect::new(f.x + 16., f.y + 60., f.w / 2. - 24., f.h - 76.), None));
                        }
                    })
                    .detach();
                }
                // A newer version waits: About's dot and its button.
                "updatefound" => self.update.found = Some("9.9.9".into()),
                "menu" => {
                    let entries = vec![Entry::choice("system", "icons/globe.svg", t("System")), Entry::Rule];
                    self.menus.show("language", point(px(600.), px(46.)), entries);
                }
                _ => {}
            }
        }
        cx.notify();
    }
}

/// The first of a task and a loop that never ends: the task's answer.
async fn first_of<T: 'static>(task: Task<T>, ticker: impl std::future::Future<Output = ()>) -> T {
    use std::future::Future as _;
    use std::pin::pin;
    let mut task = pin!(task);
    let mut ticker = pin!(ticker);
    std::future::poll_fn(move |cx| {
        if let std::task::Poll::Ready(v) = task.as_mut().poll(cx) {
            return std::task::Poll::Ready(v);
        }
        let _ = ticker.as_mut().poll(cx);
        std::task::Poll::Pending
    })
    .await
}
