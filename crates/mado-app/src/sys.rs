//! What is asked of the system, one function a job with a body for each
//! system: nothing else in the app shells out to `open`, `osascript` or
//! `id`, and nothing else names a system's own API.
//!
//! What Mado does is the Mac's: the Accessibility API that moves another
//! app's window (`ax`, `place`), the system's hot keys (`hotkeys`), the
//! keys read while a shortcut is recorded (`keys`) and the menu bar item
//! (`status_item`) are in modules of their own, built on a Mac alone.
//! Elsewhere each function here says what happens instead, and Mado
//! moves nothing (`docs/platform.md`, Gates).

use std::sync::Mutex;

use mado_core::action::Action;
use mado_core::layout::{Memory, Plan, Rect, Screen};
use mado_core::shortcut::Shortcut;

#[cfg(target_os = "macos")]
mod ax;
#[cfg(target_os = "macos")]
mod hotkeys;
#[cfg(target_os = "macos")]
mod keys;
#[cfg(target_os = "macos")]
mod place;
#[cfg(target_os = "macos")]
mod status_item;
#[cfg(target_os = "macos")]
mod footprint;
#[cfg(target_os = "macos")]
mod snap;

/// A line in `~/Library/Logs/mado-dev.log`, in debug builds only: what a
/// shortcut did, for a dev build being tried.
pub fn log(line: std::fmt::Arguments) {
    if cfg!(debug_assertions) {
        use std::io::Write as _;
        let Some(dir) = dirs::home_dir() else { return };
        let path = dir.join("Library/Logs/mado-dev.log");
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            let _ = writeln!(f, "{}.{:03} {line}", t.as_secs() % 100_000, t.subsec_millis());
        }
    }
}

// -- moving windows ------------------------------------------------------------

/// The screens, with the part of each a window may use, in the points a
/// window's frame is read in. On the main thread. Elsewhere none.
#[cfg(target_os = "macos")]
pub fn screens() -> Vec<Screen> {
    place::screens()
}
#[cfg(not(target_os = "macos"))]
pub fn screens() -> Vec<Screen> {
    Vec::new()
}

/// Why the window in front was not moved.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub enum Refused {
    /// Accessibility is not given.
    NotTrusted,
    /// No app in front, or one with no window.
    NoWindow,
    /// A window in full screen is the system's to place.
    FullScreen,
    /// It is where the action would put it, or there is nowhere to go.
    NothingToDo,
    /// The window would not say where it is.
    Unreadable,
    /// The window in front is Mado's own: moved on the main thread
    /// instead (`place_own`).
    Ours,
    /// A move is under way; a press on Mado's own window does not wait.
    Busy,
    /// Windows are moved on macOS alone.
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    Elsewhere,
}

/// The window in front moved as `action` asks, off the main thread: each
/// step is a question to another app. Elsewhere it refuses.
#[cfg(target_os = "macos")]
pub fn place_front(action: Action, screens: &[Screen], memory: &Mutex<Memory>) -> Result<Plan, Refused> {
    place::front(action, screens, memory, false).map(|(plan, _)| plan)
}
#[cfg(not(target_os = "macos"))]
pub fn place_front(_action: Action, _screens: &[Screen], _memory: &Mutex<Memory>) -> Result<Plan, Refused> {
    Err(Refused::Elsewhere)
}

/// Mado's own window in front, moved on the main thread. Accessibility
/// answers a question to the app's own process in that process, and sets
/// a frame through AppKit there, which only the main thread may do: off
/// it the app stops. No other app is asked, so nothing waits on one.
/// True with the plan when it is carried there over a moment:
/// `own_settled` once it is still.
#[cfg(target_os = "macos")]
pub fn place_own(action: Action, screens: &[Screen], memory: &Mutex<Memory>) -> Result<(Plan, bool), Refused> {
    place::front(action, screens, memory, true)
}
#[cfg(not(target_os = "macos"))]
pub fn place_own(_action: Action, _screens: &[Screen], _memory: &Mutex<Memory>) -> Result<(Plan, bool), Refused> {
    Err(Refused::Elsewhere)
}

/// Whether windows are carried to their places over a moment or put
/// there at once (`AppConfig::animate`). Elsewhere nothing moves.
#[cfg(target_os = "macos")]
pub fn set_animate(on: bool) {
    place::set_glide(on);
}
#[cfg(not(target_os = "macos"))]
pub fn set_animate(_on: bool) {}

/// How long a move of Mado's own window takes, as macOS's tiling's.
#[cfg(target_os = "macos")]
pub const TILE_MS: u64 = place::TILE_MS;
#[cfg(not(target_os = "macos"))]
pub const TILE_MS: u64 = 0;

/// Mado's own window, carried over a moment, at rest: noted where it
/// came to. On the main thread, outside any update.
#[cfg(target_os = "macos")]
pub fn own_settled(memory: &Mutex<Memory>, plan: &Plan, screens: &[Screen]) {
    place::own_settled(memory, plan, screens)
}
#[cfg(not(target_os = "macos"))]
pub fn own_settled(_memory: &Mutex<Memory>, _plan: &Plan, _screens: &[Screen]) {}

/// The app's menu titled `title` made the system's Window menu, which
/// macOS fills with Move & Resize. On the main thread, after the menus
/// are set. Elsewhere nothing.
#[cfg(target_os = "macos")]
pub fn mark_windows_menu(title: &str) {
    place::mark_windows_menu(title)
}
#[cfg(not(target_os = "macos"))]
pub fn mark_windows_menu(_title: &str) {}

/// Every window on a screen tiled by `action` (rows, columns, a grid), off
/// the main thread: the screen the window in front is on, else the one
/// under the pointer. How many windows were placed comes back. Elsewhere
/// it refuses.
#[cfg(target_os = "macos")]
pub fn tile_windows(action: Action, screens: &[Screen], memory: &Mutex<Memory>, pointer: (f64, f64)) -> Result<usize, Refused> {
    place::tile(action, screens, memory, pointer)
}
#[cfg(not(target_os = "macos"))]
pub fn tile_windows(_action: Action, _screens: &[Screen], _memory: &Mutex<Memory>, _pointer: (f64, f64)) -> Result<usize, Refused> {
    Err(Refused::Elsewhere)
}

/// Where the pointer is, in the Accessibility API's points, read on the
/// main thread. Elsewhere, the origin.
#[cfg(target_os = "macos")]
pub fn pointer() -> (f64, f64) {
    place::pointer()
}
#[cfg(not(target_os = "macos"))]
pub fn pointer() -> (f64, f64) {
    (0., 0.)
}

/// Mado's own window in front, its number and frame; whether the left
/// button is held; and the window taking back its size when dragged
/// away. Each on the main thread, outside any update: a drag of the
/// app's own window reaches no global monitor (`snap.rs`), so the app
/// watches it itself. Elsewhere none, no, and nothing.
#[cfg(target_os = "macos")]
pub fn own_window() -> Option<(u64, Rect)> {
    place::own_window()
}
#[cfg(not(target_os = "macos"))]
pub fn own_window() -> Option<(u64, Rect)> {
    None
}
#[cfg(target_os = "macos")]
pub fn left_button_down() -> bool {
    place::left_button_down()
}
#[cfg(not(target_os = "macos"))]
pub fn left_button_down() -> bool {
    false
}
#[cfg(target_os = "macos")]
pub fn own_dragged_away(memory: &Mutex<Memory>, id: u64, start: Rect, cursor_x: f64) {
    place::own_dragged_away(memory, id, start, cursor_x)
}
#[cfg(not(target_os = "macos"))]
pub fn own_dragged_away(_memory: &Mutex<Memory>, _id: u64, _start: Rect, _cursor_x: f64) {}

/// Mado's own window let go over a snap area, put there; true when it
/// is carried over a moment (`own_settled` once it is still). On the
/// main thread, outside any update. Elsewhere nothing.
#[cfg(target_os = "macos")]
pub fn own_snapped(memory: &Mutex<Memory>, id: u64, before: Rect, plan: &Plan, screens: &[Screen]) -> bool {
    place::own_snapped(memory, id, before, plan, screens)
}
#[cfg(not(target_os = "macos"))]
pub fn own_snapped(_memory: &Mutex<Memory>, _id: u64, _before: Rect, _plan: &Plan, _screens: &[Screen]) -> bool {
    false
}

/// How long any question to another app may take, set once at launch.
#[cfg(target_os = "macos")]
pub fn limit_waits() {
    ax::limit_waits();
}
#[cfg(not(target_os = "macos"))]
pub fn limit_waits() {}

// -- macOS's own tiling ---------------------------------------------------------

/// Desktop & Dock in System Settings, where macOS's own tiling is turned
/// on and off.
pub const DESKTOP_SETTINGS: &str = "x-apple.systempreferences:com.apple.Desktop-Settings.extension";

/// Whether macOS tiles a window dragged to a side of the screen or to the
/// menu bar (Desktop & Dock's first two switches under Windows), which
/// takes the same drags as Mado's snap areas. A switch never touched is
/// on. Read from the window manager's settings, off the main thread. None
/// before macOS 15, which has no such tiling, and on other systems.
#[cfg(target_os = "macos")]
pub fn system_tiling() -> Option<bool> {
    use objc2::AllocAnyThread as _;
    use objc2_foundation::{NSProcessInfo, NSString, NSUserDefaults};
    if NSProcessInfo::processInfo().operatingSystemVersion().majorVersion < 15 {
        return None;
    }
    let settings = NSUserDefaults::initWithSuiteName(NSUserDefaults::alloc(), Some(&NSString::from_str("com.apple.WindowManager")))?;
    let on = |key: &str| {
        let key = NSString::from_str(key);
        settings.objectForKey(&key).is_none() || settings.boolForKey(&key)
    };
    Some(on("EnableTilingByEdgeDrag") || on("EnableTopTilingByEdgeDrag"))
}
#[cfg(not(target_os = "macos"))]
pub fn system_tiling() -> Option<bool> {
    None
}

// -- Accessibility ---------------------------------------------------------------

/// Whether Mado may move other apps' windows, found by trying: off the
/// main thread. Elsewhere there is nothing to allow, and it says yes.
#[cfg(target_os = "macos")]
pub fn trusted() -> bool {
    ax::trusted()
}
#[cfg(not(target_os = "macos"))]
pub fn trusted() -> bool {
    true
}

/// The system's question for Accessibility, after the entry it keeps for
/// Mado is forgotten (`tccutil reset`): the system binds a grant to the
/// app's signature, and an entry made for a copy signed otherwise (an
/// earlier build) shows as on and counts for nothing. Nothing to forget
/// is not an error.
#[cfg(target_os = "macos")]
pub fn ask_trust() {
    if let Some(id) = objc2_foundation::NSBundle::mainBundle().bundleIdentifier() {
        let out = mado_core::process::command("/usr/bin/tccutil").args(["reset", "Accessibility", &id.to_string()]).output();
        log(format_args!("forget_trust {id}: {:?}", out.map(|o| String::from_utf8_lossy(&o.stderr).trim().to_string())));
    }
    ax::ask_trust();
}
#[cfg(not(target_os = "macos"))]
pub fn ask_trust() {}

/// Where Accessibility is given in the system's settings.
pub const ACCESSIBILITY_SETTINGS: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

/// Whether the system's settings call Accessibility's list Device Control
/// and Data Access, as they do from macOS 27, so the row says the name a
/// person will look for there.
#[cfg(target_os = "macos")]
pub fn device_control() -> bool {
    objc2_foundation::NSProcessInfo::processInfo().operatingSystemVersion().majorVersion >= 27
}
#[cfg(not(target_os = "macos"))]
pub fn device_control() -> bool {
    false
}

/// A message each time Accessibility's list changes, for any app: the
/// system's own notice, so nothing asks over and over. Elsewhere none
/// comes.
#[cfg(target_os = "macos")]
pub fn trust_changes() -> futures::channel::mpsc::UnboundedReceiver<()> {
    use objc2_foundation::{NSDistributedNotificationCenter, NSNotification, NSOperationQueue, NSString};
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let block = block2::RcBlock::new(move |_: std::ptr::NonNull<NSNotification>| {
        let _ = tx.unbounded_send(());
    });
    let center = NSDistributedNotificationCenter::defaultCenter();
    let observer = unsafe { center.addObserverForName_object_queue_usingBlock(Some(&NSString::from_str("com.apple.accessibility.api")), None, Some(&NSOperationQueue::mainQueue()), &block) };
    // Kept for the app's life.
    std::mem::forget(observer);
    rx
}
#[cfg(not(target_os = "macos"))]
pub fn trust_changes() -> futures::channel::mpsc::UnboundedReceiver<()> {
    futures::channel::mpsc::unbounded().1
}

// -- the shortcuts ---------------------------------------------------------------

/// The presses of the shortcuts `set_hotkeys` holds, each as its action.
/// Elsewhere none come.
#[cfg(target_os = "macos")]
pub fn install_hotkeys() -> Option<futures::channel::mpsc::UnboundedReceiver<u32>> {
    hotkeys::install()
}
#[cfg(not(target_os = "macos"))]
pub fn install_hotkeys() -> Option<futures::channel::mpsc::UnboundedReceiver<u32>> {
    None
}

/// The shortcuts held from now on, each by its action's place in
/// `Action::ALL`; those the system would not take are returned.
#[cfg(target_os = "macos")]
pub fn set_hotkeys(keys: &[(Action, Shortcut)]) -> Vec<Action> {
    let numbered: Vec<(u32, Shortcut)> = keys.iter().filter_map(|(a, s)| Some((Action::ALL.iter().position(|x| x == a)? as u32, *s))).collect();
    hotkeys::set(&numbered).into_iter().filter_map(|n| Action::ALL.get(n as usize).copied()).collect()
}
#[cfg(not(target_os = "macos"))]
pub fn set_hotkeys(_keys: &[(Action, Shortcut)]) -> Vec<Action> {
    Vec::new()
}

/// The keys pressed in Mado's window while `record_keys(true)`, as flags
/// and a key code, kept from the window. Elsewhere none come.
#[cfg(target_os = "macos")]
pub fn install_key_watch() -> futures::channel::mpsc::UnboundedReceiver<(u64, u16)> {
    keys::install()
}
#[cfg(not(target_os = "macos"))]
pub fn install_key_watch() -> futures::channel::mpsc::UnboundedReceiver<(u64, u16)> {
    futures::channel::mpsc::unbounded().1
}

#[cfg(target_os = "macos")]
pub fn record_keys(on: bool) {
    keys::recording(on);
}
#[cfg(not(target_os = "macos"))]
pub fn record_keys(_on: bool) {}

// -- the menu bar, the Dock, login -------------------------------------------------

/// A line of the menu bar item's menu: an item (what it is answered by,
/// its words, the picture before them, its shortcut, a tick, whether it
/// can be chosen), or a line between groups.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub enum MenuLine {
    Item { key: String, title: String, icon: &'static [u8], keys: Option<Shortcut>, checked: bool, enabled: bool },
    Gap,
}

/// What the menu bar item sent: the click that opens the settings, or an
/// item of its menu by its key.
#[derive(Debug, Clone)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub enum MenuPick {
    Settings,
    Line(String),
}

/// The item in the menu bar, shown or not, with its clicks swapped or
/// not, and what is chosen from it. Elsewhere there is no item, and the
/// app opens its settings at launch instead.
#[cfg(target_os = "macos")]
pub fn install_menu_bar(swap: bool, shown: bool) -> Option<futures::channel::mpsc::UnboundedReceiver<MenuPick>> {
    status_item::install(swap, shown)
}
#[cfg(not(target_os = "macos"))]
pub fn install_menu_bar(_swap: bool, _shown: bool) -> Option<futures::channel::mpsc::UnboundedReceiver<MenuPick>> {
    None
}

/// Which click shows the menu, and whether the item is there at all.
#[cfg(target_os = "macos")]
pub fn set_menu_bar(swap: bool, shown: bool) {
    status_item::set(swap, shown);
}
#[cfg(not(target_os = "macos"))]
pub fn set_menu_bar(_swap: bool, _shown: bool) {}

/// The menu's lines from now on, built when it is next shown.
#[cfg(target_os = "macos")]
pub fn set_menu_lines(lines: Vec<MenuLine>) {
    status_item::set_lines(lines);
}
#[cfg(not(target_os = "macos"))]
pub fn set_menu_lines(_lines: Vec<MenuLine>) {}

// -- snapping, the app in front, the cursor ------------------------------------------

/// What the app keeps for the drags: whether they are watched, what they
/// do, and the memory every move shares. Elsewhere no drag reads it.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub struct SnapState {
    /// Drags are watched: Accessibility is given and the app in front is
    /// not one Mado leaves alone. A window Mado placed always takes back
    /// its size when dragged away, so they are watched with snapping off.
    pub on: std::sync::atomic::AtomicBool,
    /// The Snap Areas settings (`on` among them).
    pub how: Mutex<mado_core::config::Snapping>,
    pub memory: std::sync::Arc<Mutex<Memory>>,
}

/// What the footprint should do.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub enum Footprint {
    /// Shown at a frame: grown there from the dragged window's frame when
    /// it is given and nothing showed, else from a little inside it.
    Show(Rect, Option<Rect>),
    Hide,
}

/// The drags watched, once: what the footprint should show comes down the
/// channel. Elsewhere nothing is watched.
#[cfg(target_os = "macos")]
pub fn install_snapping(state: std::sync::Arc<SnapState>) -> futures::channel::mpsc::UnboundedReceiver<Footprint> {
    snap::install(state)
}
#[cfg(not(target_os = "macos"))]
pub fn install_snapping(_state: std::sync::Arc<SnapState>) -> futures::channel::mpsc::UnboundedReceiver<Footprint> {
    futures::channel::mpsc::unbounded().1
}

/// The footprint shown at a frame, or put away. On the main thread.
/// Elsewhere there is none.
#[cfg(target_os = "macos")]
pub fn footprint(news: Footprint) {
    match news {
        Footprint::Show(r, from) => footprint::show(&r, from.as_ref()),
        Footprint::Hide => footprint::hide(),
    }
}
#[cfg(not(target_os = "macos"))]
pub fn footprint(_news: Footprint) {}

/// The app in front: its bundle id and its name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrontApp {
    pub id: String,
    pub name: String,
}

/// The app in front now, and a message each time another comes to the
/// front. Elsewhere none.
#[cfg(target_os = "macos")]
pub fn front_apps() -> (Option<FrontApp>, futures::channel::mpsc::UnboundedReceiver<FrontApp>) {
    use objc2_app_kit::{NSRunningApplication, NSWorkspace, NSWorkspaceApplicationKey, NSWorkspaceDidActivateApplicationNotification};
    use objc2_foundation::NSNotification;
    let of = |app: &NSRunningApplication| FrontApp { id: app.bundleIdentifier().map(|s| s.to_string()).unwrap_or_default(), name: app.localizedName().map(|s| s.to_string()).unwrap_or_default() };
    let now = NSWorkspace::sharedWorkspace().frontmostApplication().map(|a| of(&a));
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let block = block2::RcBlock::new(move |n: std::ptr::NonNull<NSNotification>| {
        let n = unsafe { n.as_ref() };
        let app = n.userInfo().and_then(|info| info.objectForKey(unsafe { NSWorkspaceApplicationKey })).and_then(|a| a.downcast::<NSRunningApplication>().ok());
        if let Some(app) = app {
            let _ = tx.unbounded_send(of(&app));
        }
    });
    let center = NSWorkspace::sharedWorkspace().notificationCenter();
    let observer = unsafe { center.addObserverForName_object_queue_usingBlock(Some(NSWorkspaceDidActivateApplicationNotification), None, None, &block) };
    std::mem::forget(observer);
    (now, rx)
}
#[cfg(not(target_os = "macos"))]
pub fn front_apps() -> (Option<FrontApp>, futures::channel::mpsc::UnboundedReceiver<FrontApp>) {
    (None, futures::channel::mpsc::unbounded().1)
}

/// No Dock icon and no menu at the screen's top: an app of the menu bar.
/// gpui makes every app a regular one as it starts, over the bundle's
/// `LSUIElement`, so this is said again after it. Elsewhere nothing.
pub fn stay_in_menu_bar() {
    in_dock(false);
}

/// In the Dock and the app switcher while the settings are open, as any
/// app with a window is, so the window is found again once another app
/// covers it; back to the menu bar alone when it closes.
#[cfg(target_os = "macos")]
pub fn in_dock(on: bool) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
    let Some(mtm) = MainThreadMarker::new() else { return };
    let policy = if on { NSApplicationActivationPolicy::Regular } else { NSApplicationActivationPolicy::Accessory };
    NSApplication::sharedApplication(mtm).setActivationPolicy(policy);
}
#[cfg(not(target_os = "macos"))]
pub fn in_dock(_on: bool) {}

/// Every other Mado running, the release's or the build's, asked to quit,
/// and made to if it is still there a few seconds on: two would each take
/// the shortcuts. The one opened last stays. Elsewhere nothing.
#[cfg(target_os = "macos")]
pub fn end_other_copies() {
    use objc2_app_kit::NSRunningApplication;
    use objc2_foundation::NSString;
    let me = std::process::id() as i32;
    let mut others = Vec::new();
    for id in ["com.alkalinesoftware.mado", "com.alkalinesoftware.mado.dev"] {
        for app in NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(id)).iter() {
            let pid = app.processIdentifier();
            if pid != me && pid > 0 {
                app.terminate();
                others.push(pid);
            }
        }
    }
    if others.is_empty() {
        return;
    }
    log(format_args!("ending other copies: {others:?}"));
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(3));
        for pid in others {
            if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
                if !app.isTerminated() {
                    app.forceTerminate();
                }
            }
        }
    });
}
#[cfg(not(target_os = "macos"))]
pub fn end_other_copies() {}

/// Whether Mado opens when the person logs in, as System Settings' Login
/// Items has it: the system keeps it, and Mado never writes it down.
/// Elsewhere no.
#[cfg(target_os = "macos")]
pub fn launches_at_login() -> bool {
    use objc2_service_management::{SMAppService, SMAppServiceStatus};
    unsafe { SMAppService::mainAppService().status() == SMAppServiceStatus::Enabled }
}
#[cfg(not(target_os = "macos"))]
pub fn launches_at_login() -> bool {
    false
}

#[cfg(target_os = "macos")]
pub fn set_launch_at_login(on: bool) -> Result<(), String> {
    use objc2_service_management::SMAppService;
    let service = unsafe { SMAppService::mainAppService() };
    let done = unsafe { if on { service.registerAndReturnError() } else { service.unregisterAndReturnError() } };
    done.map_err(|e| e.localizedDescription().to_string())
}
#[cfg(not(target_os = "macos"))]
pub fn set_launch_at_login(_on: bool) -> Result<(), String> {
    Err("not on this system".into())
}

/// Open a file or a link with the app the system gives it.
pub fn open(target: &str) {
    let _ = opener::open(target);
}

/// The Dock icon, for a bare binary run from `target/`. A bundle carries
/// the same picture as its `.icns`.
#[cfg(target_os = "macos")]
pub fn install_dock_icon() {
    use objc2::{AllocAnyThread as _, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;
    let Some(mtm) = MainThreadMarker::new() else { return };
    let data = NSData::with_bytes(include_bytes!("../assets/icon/icon-1024.png"));
    if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
        unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&image)) };
    }
}
#[cfg(not(target_os = "macos"))]
pub fn install_dock_icon() {}

/// The native window's own ground, under everything the app draws. gpui
/// leaves it the system's, near white in the light appearance, and it
/// showed under a dark frame while the window was drawn or resized. Set
/// to the frame's color, whatever shows there is the frame.
#[cfg(target_os = "macos")]
pub fn set_window_ground(window: &gpui::Window, ground: gpui::Rgba) {
    use objc2_app_kit::{NSColor, NSView};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = HasWindowHandle::window_handle(window) else { return };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else { return };
    let Some(view) = (unsafe { (handle.ns_view.as_ptr() as *const NSView).as_ref() }) else { return };
    if let Some(native) = view.window() {
        let color = NSColor::colorWithSRGBRed_green_blue_alpha(ground.r as f64, ground.g as f64, ground.b as f64, 1.);
        native.setBackgroundColor(Some(&color));
    }
}
#[cfg(not(target_os = "macos"))]
pub fn set_window_ground(_window: &gpui::Window, _ground: gpui::Rgba) {}

#[cfg(target_os = "macos")]
mod languages {
    use objc2::runtime::AnyObject;
    use objc2_foundation::{NSArray, NSBundle, NSString, NSUserDefaults};

    const KEY: &str = "AppleLanguages";

    /// `AppleLanguages` as one domain of the defaults has it, alone: not
    /// the search list's, which falls back from the app's to the system's.
    fn list(domain: &str) -> Vec<String> {
        let defaults = NSUserDefaults::standardUserDefaults();
        let Some(dict) = defaults.persistentDomainForName(&NSString::from_str(domain)) else { return Vec::new() };
        let Some(value) = dict.objectForKey(&NSString::from_str(KEY)) else { return Vec::new() };
        let Ok(all) = value.downcast::<NSArray>() else { return Vec::new() };
        all.iter().filter_map(|o| o.downcast::<NSString>().ok().map(|s| s.to_string())).collect()
    }

    pub fn system() -> Vec<String> {
        list("NSGlobalDomain")
    }

    /// A bare executable has no bundle and so no domain of its own: it
    /// follows the system's.
    pub fn own() -> Option<String> {
        let id = NSBundle::mainBundle().bundleIdentifier()?;
        list(&id.to_string()).into_iter().next()
    }

    pub fn set_own(code: Option<&str>) {
        if NSBundle::mainBundle().bundleIdentifier().is_none() {
            return;
        }
        let defaults = NSUserDefaults::standardUserDefaults();
        let key = NSString::from_str(KEY);
        match code {
            Some(code) => {
                let one = NSArray::from_retained_slice(&[NSString::from_str(code)]);
                let one: &AnyObject = &one;
                unsafe { defaults.setObject_forKey(Some(one), &key) };
            }
            None => defaults.removeObjectForKey(&key),
        }
    }
}

/// The system's preferred languages, in its order, apart from any the app
/// was given of its own.
#[cfg(target_os = "macos")]
pub fn system_languages() -> Vec<String> {
    let all = languages::system();
    if all.is_empty() { sys_locale::get_locales().collect() } else { all }
}
#[cfg(not(target_os = "macos"))]
pub fn system_languages() -> Vec<String> {
    sys_locale::get_locales().collect()
}

/// The language the app was given of its own, if any: by its setting or
/// by System Settings, Language & Region, Applications, which write the
/// same key. Elsewhere the settings file keeps it.
#[cfg(target_os = "macos")]
pub fn own_language() -> Option<String> {
    languages::own()
}
#[cfg(not(target_os = "macos"))]
pub fn own_language() -> Option<String> {
    None
}

/// Give the app a language of its own (a bundle's code, `zh-Hans`), or
/// none, to follow the system's.
#[cfg(target_os = "macos")]
pub fn set_own_language(code: Option<&str>) {
    languages::set_own(code);
}
#[cfg(not(target_os = "macos"))]
pub fn set_own_language(_code: Option<&str>) {}

#[cfg(target_os = "macos")]
fn ns_view(window: &gpui::Window) -> Option<&objc2_app_kit::NSView> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else { return None };
    unsafe { (handle.ns_view.as_ptr() as *const objc2_app_kit::NSView).as_ref() }
}

/// A picture of the app's own window, written to `path` as a PNG, for
/// looking at a state from a script (`MADO_SHOT`). A process may capture
/// its own windows without Screen Recording access, which the terminal
/// that runs a probe usually lacks. `CGWindowListCreateImage` is looked
/// up by name: newer SDKs mark it unavailable in favor of
/// ScreenCaptureKit, which wants that access, while the function itself
/// is still there and still captures a window of the caller's own.
#[cfg(target_os = "macos")]
pub fn shoot_window(window: &gpui::Window, path: &std::path::Path) -> Result<(), String> {
    use std::ffi::{c_char, c_void, CString};
    use std::os::unix::ffi::OsStrExt;

    use objc2_app_kit::NSView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    #[repr(C)]
    struct Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    }
    type Capture = unsafe extern "C" fn(Rect, u32, u32, u32) -> *mut c_void;
    #[link(name = "ImageIO", kind = "framework")]
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn CFURLCreateFromFileSystemRepresentation(alloc: *const c_void, path: *const u8, len: isize, is_dir: u8) -> *mut c_void;
        fn CFStringCreateWithCString(alloc: *const c_void, text: *const c_char, encoding: u32) -> *mut c_void;
        fn CGImageDestinationCreateWithURL(url: *mut c_void, kind: *mut c_void, count: usize, options: *const c_void) -> *mut c_void;
        fn CGImageDestinationAddImage(dest: *mut c_void, image: *mut c_void, properties: *const c_void);
        fn CGImageDestinationFinalize(dest: *mut c_void) -> u8;
        fn CFRelease(obj: *mut c_void);
    }

    let handle = HasWindowHandle::window_handle(window).map_err(|e| e.to_string())?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else { return Err("not an AppKit window".into()) };
    let view = unsafe { (handle.ns_view.as_ptr() as *const NSView).as_ref() }.ok_or("no view")?;
    let number = view.window().ok_or("the view has no window")?.windowNumber();
    let bytes = path.as_os_str().as_bytes();
    unsafe {
        // RTLD_DEFAULT, then: no rectangle (the window's own bounds),
        // only this window, without its shadow.
        let capture = dlsym(-2isize as *mut c_void, c"CGWindowListCreateImage".as_ptr());
        if capture.is_null() {
            return Err("this system has no CGWindowListCreateImage".into());
        }
        let capture: Capture = std::mem::transmute(capture);
        let null = Rect { x: f64::INFINITY, y: f64::INFINITY, w: 0.0, h: 0.0 };
        // `MADO_SHOT_FRAME=1` keeps what the system draws around the
        // window, its shadow and its rim.
        let framing = if std::env::var_os("MADO_SHOT_FRAME").is_some() { 0 } else { 1 };
        let image = capture(null, 1 << 3, number as u32, framing);
        if image.is_null() {
            return Err("the window could not be captured".into());
        }
        let url = CFURLCreateFromFileSystemRepresentation(std::ptr::null(), bytes.as_ptr(), bytes.len() as isize, 0);
        let png = CString::new("public.png").unwrap();
        let kind = CFStringCreateWithCString(std::ptr::null(), png.as_ptr(), 0x0800_0100);
        let dest = CGImageDestinationCreateWithURL(url, kind, 1, std::ptr::null());
        let written = if dest.is_null() {
            false
        } else {
            CGImageDestinationAddImage(dest, image, std::ptr::null());
            let ok = CGImageDestinationFinalize(dest) != 0;
            CFRelease(dest);
            ok
        };
        CFRelease(kind);
        CFRelease(url);
        CFRelease(image);
        if written { Ok(()) } else { Err(format!("could not write {}", path.display())) }
    }
}
#[cfg(not(target_os = "macos"))]
pub fn shoot_window(_window: &gpui::Window, _path: &std::path::Path) -> Result<(), String> {
    Err("a picture of the window is only taken on macOS".into())
}

/// The window kept over every other app's, for a probe.
#[cfg(target_os = "macos")]
pub fn keep_on_top(window: &gpui::Window) {
    if let Some(native) = ns_view(window).and_then(|v| v.window()) {
        native.setLevel(objc2_app_kit::NSFloatingWindowLevel);
    }
}
#[cfg(not(target_os = "macos"))]
pub fn keep_on_top(_window: &gpui::Window) {}

