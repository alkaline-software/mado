//! The footprint: where a dragged window will go if it is let go now,
//! drawn over every app while it is dragged over a snap area. A
//! borderless panel that ignores the pointer, so the drag goes on under
//! it, holding a rounded pane of thin glass: the system's blur of what is
//! behind it at part strength, so what is behind still shows, with a
//! breath of the accent and a hairline of a pale lilac.
//!
//! First shown, it grows from the dragged window's frame to the area's
//! while it fades in, over 280 ms, as macOS's own tiling shows where a
//! window will go; it slides to the next area over 220 ms and fades out
//! over 160 ms, all on the studio's curve (ease out quint, as
//! cubic-bezier 0.23, 1, 0.32, 1). Faded out it stays ordered in at no
//! opacity, so showing it again is one change.

use std::cell::RefCell;
use std::ptr::NonNull;

use mado_core::layout::Rect;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{msg_send, MainThreadMarker, MainThreadOnly as _};
use objc2_app_kit::{NSAnimatablePropertyContainer as _, NSAnimationContext, NSAutoresizingMaskOptions, NSBackingStoreType, NSBox, NSBoxType, NSColor, NSScreen, NSTitlePosition, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize};

thread_local! {
    static PANEL: RefCell<Option<(Retained<NSWindow>, bool)>> = const { RefCell::new(None) };
}

/// The accent's pale shade, the lilac the dark appearance wears, for the
/// wash and the edge in both, and the plate's corner.
const LILAC: (f64, f64, f64) = (0xC4 as f64 / 255., 0x98 as f64 / 255., 0xD2 as f64 / 255.);
const RADIUS: f64 = 12.;

/// AppKit's frame for a rect in the Accessibility API's points, which go
/// down from the top of the main screen where AppKit's go up from its
/// bottom.
fn cocoa(r: &Rect, mtm: MainThreadMarker) -> NSRect {
    let top = NSScreen::screens(mtm).firstObject().map_or(0., |s| s.frame().size.height);
    NSRect::new(NSPoint::new(r.x, top - r.y - r.h), NSSize::new(r.w, r.h))
}

fn panel(mtm: MainThreadMarker, dark: bool) -> Retained<NSWindow> {
    if let Some(p) = PANEL.with(|p| p.borrow().as_ref().filter(|(_, d)| *d == dark).map(|(w, _)| w.clone())) {
        return p;
    }
    let window = plate(mtm, dark);
    if let Some((old, _)) = PANEL.with(|p| p.borrow_mut().take()) {
        old.orderOut(None);
    }
    PANEL.with(|p| *p.borrow_mut() = Some((window.clone(), dark)));
    window
}

/// A borderless window over every app that the pointer goes through,
/// holding the plate, ordered in at no opacity.
fn plate(mtm: MainThreadMarker, dark: bool) -> Retained<NSWindow> {
    let window = unsafe { NSWindow::initWithContentRect_styleMask_backing_defer(NSWindow::alloc(mtm), NSRect::new(NSPoint::new(0., 0.), NSSize::new(10., 10.)), NSWindowStyleMask::Borderless, NSBackingStoreType::Buffered, false) };
    unsafe { window.setReleasedWhenClosed(false) };
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setIgnoresMouseEvents(true);
    window.setHasShadow(false);
    // Over every app's window, and on every Space, never in a cycle.
    window.setLevel(8);
    window.setCollectionBehavior(NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::Transient | NSWindowCollectionBehavior::IgnoresCycle | NSWindowCollectionBehavior::FullScreenAuxiliary);
    // The panel drawn in the appearance the accent's shade is for, so
    // the glass is the dark one under the dark shade.
    unsafe {
        let name = objc2_foundation::NSString::from_str(if dark { "NSAppearanceNameDarkAqua" } else { "NSAppearanceNameAqua" });
        if let Some(cls) = AnyClass::get(c"NSAppearance") {
            let look: Option<Retained<AnyObject>> = msg_send![cls, appearanceNamed: &*name];
            let _: () = msg_send![&*window, setAppearance: look.as_deref()];
        }
    }
    // The glass: what is behind the panel, blurred by the system, cut to
    // the plate's corners.
    let glass = NSVisualEffectView::new(mtm);
    glass.setMaterial(if dark { NSVisualEffectMaterial::HUDWindow } else { NSVisualEffectMaterial::FullScreenUI });
    glass.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    glass.setState(NSVisualEffectState::Active);
    // At part strength: the glass is a veil, and what is behind it shows.
    glass.setAlphaValue(if dark { 0.35 } else { 0.5 });
    glass.setWantsLayer(true);
    unsafe {
        let layer: Option<Retained<AnyObject>> = msg_send![&*glass, layer];
        if let Some(layer) = layer {
            let _: () = msg_send![&*layer, setCornerRadius: RADIUS];
            let _: () = msg_send![&*layer, setMasksToBounds: true];
        }
    }
    // Over it, a breath of the accent and a hairline of lilac.
    let (r, g, b) = LILAC;
    let (wash, edge) = if dark { (0.06, 0.45) } else { (0.08, 0.75) };
    let tint = NSBox::new(mtm);
    tint.setBoxType(NSBoxType::Custom);
    tint.setTitlePosition(NSTitlePosition::NoTitle);
    tint.setCornerRadius(RADIUS);
    tint.setBorderWidth(1.);
    tint.setBorderColor(&NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, edge));
    tint.setFillColor(&NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, wash));
    let pane = NSView::new(mtm);
    for v in [&*glass as &NSView, &*tint as &NSView] {
        v.setFrame(pane.bounds());
        v.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);
        pane.addSubview(v);
    }
    window.setContentView(Some(&pane));
    window.setAlphaValue(0.);
    window.orderFrontRegardless();
    window
}

/// Whether the system draws dark now, for the accent's shade.
fn dark(mtm: MainThreadMarker) -> bool {
    let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
    let name: Option<Retained<objc2_foundation::NSString>> = unsafe {
        let appearance: Option<Retained<AnyObject>> = msg_send![&*app, effectiveAppearance];
        appearance.and_then(|a| msg_send![&*a, name])
    };
    name.is_some_and(|n| n.to_string().contains("Dark"))
}

/// A cubic curve by its control points, made by name: QuartzCore is
/// loaded with AppKit.
fn curve((a, b, c, d): (f32, f32, f32, f32)) -> Option<Retained<AnyObject>> {
    use objc2::runtime::MessageReceiver as _;
    let cls = AnyClass::get(c"CAMediaTimingFunction")?;
    // `functionWithControlPoints::::`, a selector `msg_send!` cannot name.
    let f: *mut AnyObject = unsafe { cls.send_message(objc2::sel!(functionWithControlPoints::::), (a, b, c, d)) };
    unsafe { Retained::retain(f) }
}

/// The studio's curve (ease out quint).
const EASE: (f32, f32, f32, f32) = (0.23, 1., 0.32, 1.);

fn animate(ms: u64, change: impl Fn() + 'static) {
    animate_with(ms, EASE, change);
}

/// What `change` does to an animator, carried over `ms` on a curve.
pub(super) fn animate_with(ms: u64, points: (f32, f32, f32, f32), change: impl Fn() + 'static) {
    let block = block2::RcBlock::new(move |ctx: NonNull<NSAnimationContext>| {
        let ctx = unsafe { ctx.as_ref() };
        ctx.setDuration(ms as f64 / 1000.);
        if let Some(f) = curve(points) {
            let _: () = unsafe { msg_send![ctx, setTimingFunction: &*f] };
        }
        change();
    });
    NSAnimationContext::runAnimationGroup(&block);
}


/// The footprint at `frame`: grown there from `from` (the dragged
/// window's frame) as it fades in when it was not showing, else from a
/// little inside `frame`; slid there when it was.
pub fn show(frame: &Rect, from: Option<&Rect>) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let window = panel(mtm, dark(mtm));
    let to = cocoa(frame, mtm);
    if window.alphaValue() < 0.5 {
        let start = from.copied().unwrap_or_else(|| {
            let (dw, dh) = (frame.w * 0.06, frame.h * 0.06);
            Rect::new(frame.x + dw / 2., frame.y + dh / 2., frame.w - dw, frame.h - dh)
        });
        window.setAlphaValue(0.);
        window.setFrame_display(cocoa(&start, mtm), true);
        window.orderFrontRegardless();
        let w = window.clone();
        animate(280, move || {
            w.animator().setFrame_display(to, true);
            w.animator().setAlphaValue(1.);
        });
    } else {
        let w = window.clone();
        animate(220, move || {
            w.animator().setFrame_display(to, true);
            w.animator().setAlphaValue(1.);
        });
    }
}

pub fn hide() {
    let Some(window) = PANEL.with(|p| p.borrow().as_ref().map(|(w, _)| w.clone())) else { return };
    animate(160, move || window.animator().setAlphaValue(0.));
}

