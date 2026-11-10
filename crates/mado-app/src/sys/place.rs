//! Moving the window in front: the screens read from AppKit on the main
//! thread, and the window found, read and set through Accessibility off
//! it. The core decides where it goes (`layout::Memory::plan`); this
//! carries it out and says what the window took.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use mado_core::action::Action;
use mado_core::layout::{self, Memory, Plan, Rect, Screen};
use objc2::MainThreadMarker;
use objc2_app_kit::NSScreen;

use super::ax::{Element, API_DISABLED};
use super::Refused;

/// Every screen, its frame and the part a window may use, in the
/// Accessibility API's points: from the main screen's top left, y going
/// down, where AppKit's go up from its bottom left. On the main thread
/// alone, where AppKit keeps its screens; it is a read of what AppKit
/// holds, and asks no other app.
pub fn screens() -> Vec<Screen> {
    let Some(mtm) = MainThreadMarker::new() else { return Vec::new() };
    let all = NSScreen::screens(mtm);
    let Some(main) = all.firstObject() else { return Vec::new() };
    let top = main.frame().size.height;
    let flip = |r: objc2_foundation::NSRect| Rect::new(r.origin.x, top - r.origin.y - r.size.height, r.size.width, r.size.height);
    all.iter().map(|s| Screen { frame: flip(s.frame()), area: flip(s.visibleFrame()) }).collect()
}

/// Where the pointer is, in the Accessibility API's points. On the main
/// thread, as `screens`.
pub fn pointer() -> (f64, f64) {
    let Some(mtm) = MainThreadMarker::new() else { return (0., 0.) };
    let p = objc2_app_kit::NSEvent::mouseLocation();
    let top = NSScreen::screens(mtm).firstObject().map_or(0., |s| s.frame().size.height);
    (p.x, top - p.y)
}

/// Where a window is and how big, if it says.
fn frame_of(window: &Element) -> Option<Rect> {
    let (x, y) = window.position()?;
    let (w, h) = window.size()?;
    Some(Rect::new(x, y, w, h))
}

/// Size, then place, then size again: a window moved to a screen where
/// it does not fit at its old size is held to the screen's edge by the
/// system, and the second size is the one it keeps.
fn set_frame(window: &Element, r: &Rect) {
    window.set_size(r.w, r.h);
    window.set_position(r.x, r.y);
    window.set_size(r.w, r.h);
}

/// macOS's own Move & Resize command for an action, in an app's Window
/// menu (macOS 15 and later): named by its key equivalent, fn ⌃ with an
/// arrow or a letter, the same in every language. Chosen there, the app
/// moves its own window over a moment, drawing every step, which no
/// window moved from outside does (`docs/platform.md`).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Command {
    Arrow(u16),
    Letter(&'static str),
}

/// A key equivalent's modifiers as Accessibility gives them: fn and ⌃,
/// no ⌘.
const FN_CONTROL: f64 = 28.;

fn command(action: Action) -> Option<Command> {
    use Action::*;
    Some(match action {
        LeftHalf => Command::Arrow(123),
        RightHalf => Command::Arrow(124),
        BottomHalf => Command::Arrow(125),
        TopHalf => Command::Arrow(126),
        Maximize => Command::Letter("F"),
        Center => Command::Letter("C"),
        Restore => Command::Letter("R"),
        _ => return None,
    })
}

/// The menu items found for each app's commands, kept: a search walks
/// the app's menus, a question an item.
static ITEMS: std::sync::OnceLock<Mutex<std::collections::HashMap<(i32, Command), Element>>> = std::sync::OnceLock::new();

/// The app's menu item for a command, searched from its last menu back
/// (the Window menu is near the end).
fn find_item(app: &Element, c: Command) -> Option<Element> {
    fn search(el: &Element, c: Command, depth: u32) -> Option<Element> {
        if depth > 5 {
            return None;
        }
        for k in el.children() {
            let named = k.number("AXMenuItemCmdModifiers") == Some(FN_CONTROL)
                && match c {
                    Command::Arrow(key) => k.number("AXMenuItemCmdVirtualKey") == Some(key as f64),
                    Command::Letter(l) => k.string("AXMenuItemCmdChar").as_deref() == Some(l),
                };
            if named {
                return Some(k);
            }
            if let Some(f) = search(&k, c, depth + 1) {
                return Some(f);
            }
        }
        None
    }
    let bar = app.element("AXMenuBar").ok()?;
    bar.children().into_iter().rev().find_map(|menu| search(&menu, c, 0))
}

/// The command chosen in the app's menu: the item kept from before, else
/// found and kept; one that no longer takes a press is looked for again.
fn choose(app: &Element, pid: i32, c: Command) -> bool {
    let items = ITEMS.get_or_init(Default::default);
    let kept = items.lock().unwrap_or_else(|e| e.into_inner()).get(&(pid, c)).cloned();
    if kept.is_some_and(|item| item.press()) {
        return true;
    }
    let Some(item) = find_item(app, c) else { return false };
    let pressed = item.press();
    if pressed {
        items.lock().unwrap_or_else(|e| e.into_inner()).insert((pid, c), item);
    }
    pressed
}

/// A window moved by its app's own command for `action`, and the frame it
/// came to rest at; none where the app has no such command or it moved
/// nothing, for the caller to put it at once. Waits while the app moves
/// it, a second at most: off the main thread.
fn by_command(app: &Element, window: &Element, action: Action, before: Rect) -> Option<Rect> {
    if !GLIDE.load(Ordering::Relaxed) {
        return None;
    }
    let c = command(action)?;
    if !choose(app, app.pid()?, c) {
        return None;
    }
    let started = std::time::Instant::now();
    let mut last = before;
    let mut same = 0;
    let mut moved = false;
    while started.elapsed() < Duration::from_secs(1) {
        std::thread::sleep(Duration::from_millis(16));
        let now = frame_of(window)?;
        if now.near(&last) {
            same += 1;
        } else {
            same = 0;
            moved = true;
        }
        last = now;
        // At rest: still for three frames once it moved, or never moved
        // within a fifth of a second.
        if (moved && same >= 3) || (!moved && started.elapsed() > Duration::from_millis(200)) {
            break;
        }
    }
    moved.then_some(last)
}

/// Mado's own window, the window `id`, carried to `to` over a moment
/// by AppKit, as macOS's own tiling carries a window: its curve and time,
/// fitted to the frames of a window the system tiled (350 ms). The
/// window draws every step, being Mado's. Any action moves it so, Restore
/// included, which the system's command does not give.
/// On the main thread, which it does not hold: the move goes on after it
/// returns.
fn own_animate(id: u64, to: &Rect) -> bool {
    use objc2_app_kit::NSApplication;
    if !GLIDE.load(Ordering::Relaxed) {
        return false;
    }
    let Some(mtm) = MainThreadMarker::new() else { return false };
    let app = NSApplication::sharedApplication(mtm);
    let Some(window) = app.keyWindow().or_else(|| app.mainWindow()) else { return false };
    if window.windowNumber() as u64 != id {
        return false;
    }
    let top = NSScreen::screens(mtm).firstObject().map_or(0., |s| s.frame().size.height);
    let r = objc2_foundation::NSRect::new(objc2_foundation::NSPoint::new(to.x, top - to.y - to.h), objc2_foundation::NSSize::new(to.w, to.h));
    super::footprint::animate_with(TILE_MS, (0.3, 1., 0.3, 1.), move || {
        use objc2_app_kit::NSAnimatablePropertyContainer as _;
        window.animator().setFrame_display(r, true);
    });
    true
}

/// How long macOS's tiling takes to carry a window, and Mado's own with it.
pub const TILE_MS: u64 = 350;

/// Whether a window is carried to its place (the system's command, Mado's
/// own by AppKit) or put there at once: the person's Animate windows,
/// off unless turned on. Set at launch and at each change.
static GLIDE: AtomicBool = AtomicBool::new(false);

pub fn set_glide(on: bool) {
    GLIDE.store(on, Ordering::Relaxed);
}

/// The app's menu titled `title` made the one AppKit keeps as its Window
/// menu, which it fills with Move & Resize. gpui marks it only when it is
/// titled "Window" in English, and Mado's is in the person's language.
/// On the main thread, after the menus are set.
pub fn mark_windows_menu(title: &str) {
    use objc2_app_kit::NSApplication;
    let Some(mtm) = MainThreadMarker::new() else { return };
    let app = NSApplication::sharedApplication(mtm);
    let Some(main) = app.mainMenu() else { return };
    for item in main.itemArray().iter() {
        if let Some(sub) = item.submenu() {
            if sub.title().to_string() == title {
                app.setWindowsMenu(Some(&sub));
                return;
            }
        }
    }
}

/// Mado's own window, carried (`own_animate`), at rest: the frame it came
/// to noted in place of the one asked for, and put at once where it did
/// not come there (a size the window refuses). On the main thread,
/// outside any update, once the move is over.
pub fn own_settled(memory: &Mutex<Memory>, plan: &Plan, screens: &[Screen]) {
    let Ok(mut memory) = memory.try_lock() else { return };
    let Some(app) = Element::app(std::process::id() as i32) else { return };
    let Ok(window) = app.element("AXFocusedWindow").or_else(|_| app.element("AXMainWindow")) else { return };
    let Some(mut took) = frame_of(&window) else { return };
    if !took.near(&plan.frame) {
        took = apply(Some(&app), &window, plan, screens);
    }
    memory.settled(window.window_id(), plan.action, took);
    super::log(format_args!("{} (own, settled): {took:?} (asked {:?})", plan.action.key(), plan.frame));
}

/// A window put at a plan's frame, at once, and the frame it took. It
/// does not glide, as Rectangle's do not: an app shows each step only
/// once it has drawn the window at that size, and most cannot in a frame
/// (`docs/platform.md`). An app that an assistive tool has put in its
/// enhanced mode animates every frame it is given, slowly and not always
/// to the end: the mode is off while it moves, and back after. A window
/// that refused its size is kept at the screen's edges the plan put it at
/// (`layout::settle`).
pub fn apply(app: Option<&Element>, window: &Element, plan: &Plan, screens: &[Screen]) -> Rect {
    let enhanced = app.is_some_and(|a| a.bool("AXEnhancedUserInterface") == Some(true));
    if enhanced {
        if let Some(a) = app {
            a.set_bool("AXEnhancedUserInterface", false);
        }
    }
    set_frame(window, &plan.frame);
    let mut took = frame_of(window).unwrap_or(plan.frame);
    if let Some(at) = screens.get(plan.screen).and_then(|s| layout::settle(&plan.frame, &took, &s.area)) {
        window.set_position(at.x, at.y);
        took = frame_of(window).unwrap_or(at);
    }
    if enhanced {
        if let Some(a) = app {
            a.set_bool("AXEnhancedUserInterface", true);
        }
    }
    took
}

/// The app in front, as the workspace has it. The system-wide element's
/// `AXFocusedApplication` is asked only where the workspace names none:
/// it asks the app in front for its focus, and an app whose focused
/// element cannot answer (Emaki, in some of its sessions) fails it with
/// `kAXErrorCannotComplete` while its windows still answer, so a
/// shortcut there did nothing until its focus moved.
fn front_app() -> Result<Element, Refused> {
    let pid = objc2_app_kit::NSWorkspace::sharedWorkspace().frontmostApplication().map(|a| a.processIdentifier());
    if let Some(app) = pid.filter(|p| *p > 0).and_then(Element::app) {
        return Ok(app);
    }
    let system = Element::system().ok_or(Refused::NoWindow)?;
    system.element("AXFocusedApplication").map_err(|e| if e == API_DISABLED { Refused::NotTrusted } else { Refused::NoWindow })
}

/// The app's window in front: the one it says has focus, else its main
/// one, else the first of its standard windows that is not minimized,
/// as an app whose focus cannot answer still lists its windows. The
/// error that ends the search comes back when there is none.
fn front_window(app: &Element) -> Result<Element, i32> {
    let focused = match app.element("AXFocusedWindow").or_else(|_| app.element("AXMainWindow")) {
        Ok(w) => return Ok(w),
        Err(e) => e,
    };
    app.windows().into_iter().find(|w| w.string("AXSubrole").as_deref() == Some("AXStandardWindow") && w.bool("AXMinimized") != Some(true)).ok_or(focused)
}

/// The window in front moved as `action` asks. `memory` is held for the
/// whole move, so two presses in a row are carried out one after the
/// other. Off the main thread (`own` false) a window of Mado's own is
/// refused, to be moved on the main thread (`own` true), where the lock
/// is only tried: the main thread never waits on it. What it did comes
/// back: the plan, and whether the window is still being carried.
pub fn front(action: Action, screens: &[Screen], memory: &Mutex<Memory>, own: bool) -> Result<(Plan, bool), Refused> {
    let mut memory = if own {
        match memory.try_lock() {
            Ok(m) => m,
            Err(std::sync::TryLockError::Poisoned(e)) => e.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return Err(Refused::Busy),
        }
    } else {
        memory.lock().unwrap_or_else(|e| e.into_inner())
    };
    let app = front_app()?;
    let ours = app.pid() == Some(std::process::id() as i32);
    if ours != own {
        return Err(if ours { Refused::Ours } else { Refused::NoWindow });
    }
    let window = front_window(&app).map_err(|e| {
        super::log(format_args!("{}: no window in {:?}: {e}", action.key(), app.pid()));
        if e == API_DISABLED { Refused::NotTrusted } else { Refused::NoWindow }
    })?;
    if window.bool("AXFullScreen") == Some(true) {
        return Err(Refused::FullScreen);
    }
    // A sheet moves with its window, and is not one to place.
    if window.string("AXRole").as_deref() == Some("AXSheet") {
        return Err(Refused::NoWindow);
    }
    let before = frame_of(&window).ok_or(Refused::Unreadable)?;
    let id = window.window_id();
    let plan = memory.plan(id, action, before, screens).ok_or(Refused::NothingToDo)?;
    // A half, the whole screen, the middle or Restore: by the app's own
    // command, which moves the
    // window over a moment; Restore set again where the system's previous
    // size is not Mado's. Anything else, and an app without the command,
    // at once. Mado's own window is carried by AppKit over the same
    // moment, whatever the action, on the main thread, which never waits
    // for it to end: noted at the frame asked for, and at the one it came
    // to once it is still (`own_settled`).
    let by_system = !plan.crossed;
    if own && own_animate(id, &plan.frame) {
        memory.done(id, &plan, before, plan.frame, true);
        super::log(format_args!("{} (own, carried): {before:?} -> asked {:?}", action.key(), plan.frame));
        return Ok((plan, true));
    }
    let commanded = (!own && by_system).then(|| by_command(&app, &window, plan.action, before)).flatten();
    let took = match commanded {
        Some(t) if plan.action != Action::Restore || t.near(&plan.frame) => t,
        _ => apply(Some(&app), &window, &plan, screens),
    };
    memory.done(id, &plan, before, took, true);
    super::log(format_args!("{}: {} {before:?} -> {took:?} (asked {:?} as {})", action.key(), window.string("AXTitle").unwrap_or_default(), plan.frame, plan.action.key()));
    Ok((plan, false))
}

/// Every window on one screen tiled by `action` (Tile in Rows, in
/// Columns, in a Grid): the screen the window in front is on, else the
/// one under the pointer. The windows are those the window server shows
/// in the Space in front, each an app's standard window, not minimized
/// and not full screen; Mado's own are left where they are, since they
/// move on the main thread alone. Each is noted as a move, so Restore
/// puts it back. How many were placed comes back.
pub fn tile(action: Action, screens: &[Screen], memory: &Mutex<Memory>, pointer: (f64, f64)) -> Result<usize, Refused> {
    let mut memory = memory.lock().unwrap_or_else(|e| e.into_inner());
    let own = std::process::id() as i32;
    let front = match front_app() {
        Err(Refused::NotTrusted) => return Err(Refused::NotTrusted),
        Ok(app) if app.pid() != Some(own) => match app.element("AXFocusedWindow") {
            Err(API_DISABLED) => return Err(Refused::NotTrusted),
            w => w.ok().and_then(|w| frame_of(&w)),
        },
        _ => None,
    };
    let here = front.and_then(|f| layout::screen_of(&f, screens)).or_else(|| screens.iter().position(|s| s.frame.holds(pointer.0, pointer.1))).unwrap_or(0);
    let area = screens.get(here).ok_or(Refused::NoWindow)?.area;
    let shown = super::ax::windows_on_screen();
    let mut pids: Vec<i32> = Vec::new();
    for (pid, _) in &shown {
        if *pid != own && !pids.contains(pid) {
            pids.push(*pid);
        }
    }
    let mut found: Vec<(Element, Element, u64, Rect)> = Vec::new();
    for pid in pids {
        let Some(app) = Element::app(pid) else { continue };
        for w in app.windows() {
            if w.string("AXSubrole").as_deref() != Some("AXStandardWindow") || w.bool("AXMinimized") == Some(true) || w.bool("AXFullScreen") == Some(true) {
                continue;
            }
            let id = w.window_id();
            if !shown.contains(&(pid, id)) {
                continue;
            }
            let Some(frame) = frame_of(&w) else { continue };
            if layout::screen_of(&frame, screens) == Some(here) {
                found.push((app.clone(), w, id, frame));
            }
        }
    }
    if found.is_empty() {
        return Err(Refused::NothingToDo);
    }
    let before: Vec<Rect> = found.iter().map(|f| f.3).collect();
    let frames = layout::tile(action, &before, &area);
    // Each app out of its enhanced mode while its windows move (`apply`).
    let mut enhanced: Vec<&Element> = Vec::new();
    for (app, ..) in &found {
        if !enhanced.iter().any(|a| a.pid() == app.pid()) && app.bool("AXEnhancedUserInterface") == Some(true) {
            app.set_bool("AXEnhancedUserInterface", false);
            enhanced.push(app);
        }
    }
    for ((app, w, id, was), frame) in found.iter().zip(frames) {
        let plan = Plan { frame, screen: here, action, crossed: false };
        let took = apply(Some(app), w, &plan, screens);
        memory.done(*id, &plan, *was, took, true);
    }
    for app in enhanced {
        app.set_bool("AXEnhancedUserInterface", true);
    }
    super::log(format_args!("{}: {} windows on screen {here}", action.key(), found.len()));
    Ok(found.len())
}

/// Mado's own window in front: its number and its frame. On the main
/// thread, where Accessibility answers the app's own process.
pub fn own_window() -> Option<(u64, Rect)> {
    let app = Element::app(std::process::id() as i32)?;
    let window = app.element("AXFocusedWindow").or_else(|_| app.element("AXMainWindow")).ok()?;
    Some((window.window_id(), frame_of(&window)?))
}

/// Mado's own window, dragged by the person `AWAY` from where the drag
/// began at `start`: takes back its size as another app's window does
/// (`Memory::dragged`). On the main thread, outside any update of the
/// app, as `front` with `own`; the memory's lock is only tried.
pub fn own_dragged_away(memory: &Mutex<Memory>, id: u64, start: Rect, cursor_x: f64) {
    let Ok(mut memory) = memory.try_lock() else { return };
    let Some(app) = Element::app(std::process::id() as i32) else { return };
    let Ok(window) = app.element("AXFocusedWindow").or_else(|_| app.element("AXMainWindow")) else { return };
    if window.window_id() != id {
        return;
    }
    let Some(now) = frame_of(&window) else { return };
    if let Some(r) = memory.dragged(id, start, now, cursor_x) {
        window.set_size(r.w, r.h);
        window.set_position(r.x, now.y);
        super::log(format_args!("dragged away (own): {now:?} -> {r:?}"));
    }
}

/// Mado's own window let go over a snap area: carried to the plan's
/// frame as a shortcut carries it, else put there at once. On the main
/// thread, outside any update of the app, as `own_dragged_away`. Restore
/// goes back to `before`, where the drag began. True when it is carried,
/// for `own_settled` once it is still.
pub fn own_snapped(memory: &Mutex<Memory>, id: u64, before: Rect, plan: &Plan, screens: &[Screen]) -> bool {
    let Ok(mut memory) = memory.try_lock() else { return false };
    if own_animate(id, &plan.frame) {
        memory.done(id, plan, before, plan.frame, false);
        super::log(format_args!("snap {} (own, carried): {before:?} -> asked {:?}", plan.action.key(), plan.frame));
        return true;
    }
    let Some(app) = Element::app(std::process::id() as i32) else { return false };
    let Ok(window) = app.element("AXFocusedWindow").or_else(|_| app.element("AXMainWindow")) else { return false };
    if window.window_id() != id {
        return false;
    }
    let took = apply(Some(&app), &window, plan, screens);
    memory.done(id, plan, before, took, false);
    super::log(format_args!("snap {} (own): {before:?} -> {took:?}", plan.action.key()));
    false
}

/// Whether the left button is held, as the window server has it. On the
/// main thread.
pub fn left_button_down() -> bool {
    objc2_app_kit::NSEvent::pressedMouseButtons() & 1 != 0
}

/// A dragged window let go over a snap area: put at the plan's frame. The
/// place Restore goes back to is the one the drag noted as it began.
pub fn snapped(window: &Element, plan: &Plan, before: Rect, screens: &[Screen], memory: &Mutex<Memory>) {
    let app = window.pid().and_then(|pid| super::ax::Element::app(pid));
    // The release reaches Mado as the drag ends, and an app that drags its
    // own window (a title bar of its own) may move it once more after.
    // Put there before that, the window was taken off its place, and the
    // place noted was not where it stood. So: once it is still, put, and
    // put again if a late move took it off.
    still(window);
    // By the app's own command where it has one, as a shortcut's move.
    let from = frame_of(window).unwrap_or(plan.frame);
    let commanded = app.as_ref().and_then(|a| by_command(a, window, plan.action, from));
    let mut took = match commanded {
        Some(t) => t,
        None => apply(app.as_ref(), window, plan, screens),
    };
    for _ in 0..3 {
        std::thread::sleep(Duration::from_millis(40));
        match frame_of(window) {
            Some(now) if !now.near(&took) => took = apply(app.as_ref(), window, plan, screens),
            _ => break,
        }
    }
    memory.lock().unwrap_or_else(|e| e.into_inner()).done(window.window_id(), plan, before, took, false);
    super::log(format_args!("snap {}: {before:?} -> {took:?}", plan.action.key()));
}

/// Waits, a fifth of a second at most, until a window's frame reads the
/// same twice a frame apart.
fn still(window: &Element) {
    let mut last = frame_of(window);
    for _ in 0..10 {
        std::thread::sleep(Duration::from_millis(20));
        let now = frame_of(window);
        if now == last {
            return;
        }
        last = now;
    }
}

pub fn frame(window: &Element) -> Option<Rect> {
    frame_of(window)
}
