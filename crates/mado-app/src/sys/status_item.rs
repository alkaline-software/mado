//! Mado's item in the menu bar: it has no Dock icon and no window most
//! of the time, so this is where the actions, the settings and Quit are.
//!
//! The item is the icon's window as a template image
//! (`assets/icons/menubar.png`, drawn by `scripts/make-icon.sh`), which
//! the system tints. One click opens the settings where they were left,
//! the other shows the menu; General can swap the two and hide the item
//! (`set`).
//!
//! The item has no menu of its own: the button sends both clicks to an
//! Objective-C target, which reads the click from the app's current event
//! and either sends Settings down the channel or builds the menu from the
//! lines the app last handed over (`set_lines`), sets it, clicks the
//! button and takes it off again once it closes. An item with a menu set
//! would show it for either click. Each line is known by its tag.
//!
//! The menu's pictures are the window's SVGs drawn by resvg at one and
//! two pixels a point into template images, so the system tints them for
//! the light, the dark and the highlighted line; each is drawn once and
//! kept. An app built on the macOS 27 SDK has its menus' images hidden
//! unless each item asks for its own (`preferredImageVisibility`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use mado_core::shortcut::{key_name, Shortcut};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol as _};
use objc2::{define_class, msg_send, sel, AllocAnyThread as _, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSBitmapImageRep, NSEventMask, NSEventModifierFlags, NSEventType, NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength};
use objc2_foundation::{NSData, NSObject, NSSize, NSString};

use super::{MenuLine, MenuPick};

static PICKS: OnceLock<UnboundedSender<MenuPick>> = OnceLock::new();
/// A left click shows the menu, and a right click opens the settings.
static MENU_ON_LEFT: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// The item, kept for the app's life.
    static ITEM: RefCell<Option<Retained<NSStatusItem>>> = const { RefCell::new(None) };
    /// What the item and the menu call, which they hold weakly.
    static TARGET: RefCell<Option<Retained<MenuTarget>>> = const { RefCell::new(None) };
    /// The menu's lines as the app last handed them, and each item's key
    /// by its tag once built.
    static LINES: RefCell<Vec<MenuLine>> = const { RefCell::new(Vec::new()) };
    static KEYS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// Each picture drawn once, by where its bytes are.
    static IMAGES: RefCell<HashMap<usize, Retained<NSImage>>> = RefCell::new(HashMap::new());
}

fn send(pick: MenuPick) {
    if let Some(tx) = PICKS.get() {
        let _ = tx.unbounded_send(pick);
    }
}

define_class!(
    // The item and the menu's choices call here, on the main thread, and
    // what was asked goes on to the app as a message.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "MadoMenuTarget"]
    struct MenuTarget;

    impl MenuTarget {
        /// A click on the item, either button. A left click with Control
        /// held is a right one, as everywhere on a Mac.
        #[unsafe(method(clicked:))]
        fn clicked(&self, _sender: Option<&AnyObject>) {
            let mtm = self.mtm();
            let right = NSApplication::sharedApplication(mtm).currentEvent().is_some_and(|e| e.r#type() == NSEventType::RightMouseUp || e.modifierFlags().contains(NSEventModifierFlags::Control));
            if right == MENU_ON_LEFT.load(Ordering::Relaxed) {
                send(MenuPick::Settings);
                return;
            }
            show_menu(mtm, self);
        }

        #[unsafe(method(pick:))]
        fn pick(&self, sender: Option<&NSMenuItem>) {
            let key = sender.and_then(|s| KEYS.with(|k| k.borrow().get(usize::try_from(s.tag()).ok()?).cloned()));
            if let Some(key) = key {
                send(MenuPick::Line(key));
            }
        }
    }
);

/// The menu, built from the lines last handed over and set on the item
/// while it is shown, so the click that opens the settings reaches the
/// target; the click below returns once the menu has closed.
fn show_menu(mtm: MainThreadMarker, target: &MenuTarget) {
    let Some(status) = ITEM.with(|i| i.borrow().clone()) else { return };
    let menu = KEYS.with(|keys| {
        let mut keys = keys.borrow_mut();
        keys.clear();
        LINES.with(|lines| build(mtm, target, &lines.borrow(), &mut keys))
    });
    status.setMenu(Some(&menu));
    if let Some(button) = status.button(mtm) {
        unsafe { button.performClick(None) };
    }
    status.setMenu(None);
}

fn build(mtm: MainThreadMarker, target: &MenuTarget, lines: &[MenuLine], keys: &mut Vec<String>) -> Retained<NSMenu> {
    let menu = NSMenu::new(mtm);
    // The items' enabled state is the app's, not AppKit's guess.
    menu.setAutoenablesItems(false);
    for line in lines {
        match line {
            MenuLine::Gap => menu.addItem(&NSMenuItem::separatorItem(mtm)),
            MenuLine::Item { key, title, icon, keys: shortcut, checked, enabled } => {
                let (equivalent, mask) = shortcut.and_then(equivalent).unwrap_or((String::new(), 0));
                let it = unsafe { NSMenuItem::initWithTitle_action_keyEquivalent(NSMenuItem::alloc(mtm), &NSString::from_str(title), Some(sel!(pick:)), &NSString::from_str(&equivalent)) };
                it.setKeyEquivalentModifierMask(NSEventModifierFlags::from_bits_retain(mask as _));
                unsafe { it.setTarget(Some(target)) };
                it.setTag(keys.len() as isize);
                keys.push(key.clone());
                it.setEnabled(*enabled);
                if *checked {
                    it.setState(1);
                }
                picture(&it, icon);
                menu.addItem(&it);
            }
        }
    }
    menu
}

fn picture(item: &NSMenuItem, svg: &'static [u8]) {
    if svg.is_empty() {
        return;
    }
    let image = IMAGES.with(|cache| cache.borrow_mut().entry(svg.as_ptr() as usize).or_insert_with(|| line_image(svg).unwrap_or_else(|| NSImage::initWithSize(NSImage::alloc(), NSSize::new(16., 16.)))).clone());
    item.setImage(Some(&image));
    // An app built on the macOS 27 SDK has its menus' images hidden unless
    // each item asks for its own (`.visible` = 1).
    if item.respondsToSelector(sel!(setPreferredImageVisibility:)) {
        let _: () = unsafe { msg_send![item, setPreferredImageVisibility: 1isize] };
    }
}

/// A picture for a menu's line from its SVG, 16 points tall, or 18 by 12
/// for an action's wide one, drawn for one and two pixels a point, as a
/// template the system tints.
fn line_image(svg: &[u8]) -> Option<Retained<NSImage>> {
    let tree = resvg::usvg::Tree::from_data(svg, &resvg::usvg::Options::default()).ok()?;
    let wide = tree.size().width() > tree.size().height();
    let (pw, ph) = if wide { (18., 12.) } else { (16., 16.) };
    let image = NSImage::initWithSize(NSImage::alloc(), NSSize::new(pw, ph));
    for scale in [1u32, 2] {
        let (w, h) = ((pw as u32) * scale, (ph as u32) * scale);
        let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
        let k = w as f32 / tree.size().width();
        resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(k, k), &mut pixmap.as_mut());
        let png = pixmap.encode_png().ok()?;
        let rep = NSBitmapImageRep::imageRepWithData(&NSData::with_bytes(&png))?;
        rep.setSize(NSSize::new(pw, ph));
        image.addRepresentation(&rep);
    }
    image.setTemplate(true);
    Some(image)
}

/// A shortcut as a menu writes it beside its line: the character AppKit
/// takes for the key, and the modifier flags, which are the event's own.
fn equivalent(s: Shortcut) -> Option<(String, u64)> {
    let name = key_name(s.key)?;
    let c = |u: u32| char::from_u32(u).map(String::from);
    let key = match name {
        "left" => c(0xF702)?,
        "right" => c(0xF703)?,
        "up" => c(0xF700)?,
        "down" => c(0xF701)?,
        "enter" => "\r".into(),
        "keypad-enter" => c(0x3)?,
        "backspace" => c(0x8)?,
        "delete" => c(0xF728)?,
        "tab" => "\t".into(),
        "space" => " ".into(),
        "escape" => c(0x1B)?,
        "home" => c(0xF729)?,
        "end" => c(0xF72B)?,
        "pageup" => c(0xF72C)?,
        "pagedown" => c(0xF72D)?,
        f if f.len() > 1 && f.starts_with('f') => c(0xF704 + f[1..].parse::<u32>().ok()? - 1)?,
        other => other.to_string(),
    };
    Some((key, s.mods))
}

/// The item in the menu bar, shown or not, with its clicks swapped or
/// not, and what is chosen from it.
pub fn install(swap: bool, shown: bool) -> Option<UnboundedReceiver<MenuPick>> {
    let mtm = MainThreadMarker::new()?;
    let (tx, rx) = unbounded();
    let _ = PICKS.set(tx);
    MENU_ON_LEFT.store(swap, Ordering::Relaxed);
    let target: Retained<MenuTarget> = unsafe { msg_send![MenuTarget::alloc(mtm), init] };
    let status = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    if let Some(button) = status.button(mtm) {
        let data = NSData::with_bytes(include_bytes!("../../assets/icons/menubar.png"));
        if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
            image.setSize(NSSize::new(18., 18.));
            image.setTemplate(true);
            button.setImage(Some(&image));
        }
        button.setToolTip(Some(&NSString::from_str("Mado")));
        unsafe {
            button.setTarget(Some(&target));
            button.setAction(Some(sel!(clicked:)));
        }
        button.sendActionOn(NSEventMask::LeftMouseUp | NSEventMask::RightMouseUp);
    }
    status.setVisible(shown);
    ITEM.with(|i| *i.borrow_mut() = Some(status));
    TARGET.with(|t| *t.borrow_mut() = Some(target));
    Some(rx)
}

/// Which click shows the menu, and whether the item is in the bar at
/// all: both change at once. Hidden, the settings are reached by opening
/// Mado again (`on_reopen` in main.rs).
pub fn set(swap: bool, shown: bool) {
    MENU_ON_LEFT.store(swap, Ordering::Relaxed);
    ITEM.with(|i| {
        if let Some(status) = i.borrow().as_ref() {
            status.setVisible(shown);
        }
    });
}

/// The menu's lines from now on: built when the menu is next shown.
pub fn set_lines(lines: Vec<MenuLine>) {
    LINES.with(|l| *l.borrow_mut() = lines);
}
