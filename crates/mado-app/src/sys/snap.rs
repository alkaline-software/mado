//! Dragging a window: to a snap area, and away from where Mado put it.
//! AppKit's global monitor reports the
//! left button's presses, drags and releases and the keys, on the main
//! thread, where they are only read and handed on; a thread of its own
//! asks Accessibility which window was pressed on and where it is at each
//! drag, since each is a question to another app. What the footprint
//! should show comes back to the app down a channel (`Footprint`).
//!
//! A drag moves its window when the window's place changes and its size
//! does not, or changes with fewer than two edges still where they were:
//! a window resized by an edge keeps two. Once the cursor is `AWAY` from
//! where it pressed, a window Mado placed takes back its size
//! (`Memory::dragged`), so a nudge leaves it placed. Let go over an area,
//! the window is placed as the area asks; Esc during a drag gives it up.

use std::ptr::NonNull;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use mado_core::action::Action;
use mado_core::layout::{Memory, Rect, Screen, AWAY};

use super::{Footprint, SnapState};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSEvent, NSEventMask, NSEventType, NSScreen};

use super::ax::Element;
use super::place;

enum Ev {
    Down { x: f64, y: f64, screens: Vec<Screen> },
    Drag { x: f64, y: f64 },
    Up { x: f64, y: f64 },
    Escape,
}

struct Drag {
    /// Where the press was.
    from: (f64, f64),
    window: Option<Element>,
    id: u64,
    start: Rect,
    /// The window's frame at the last drag event: where the footprint
    /// grows from.
    at: Rect,
    tries: u32,
    tried: Instant,
    moving: bool,
    /// The cursor has gone `AWAY` from the press, and the window was given
    /// back its size if it was Mado's to give.
    away: bool,
    /// No snap for the rest of this drag: it resizes, or Esc gave it up.
    still: bool,
    area: Option<(usize, Action)>,
    screens: Vec<Screen>,
}

/// The watch installed, once, and the footprint's news.
pub fn install(state: Arc<SnapState>) -> UnboundedReceiver<Footprint> {
    let (news, rx) = unbounded();
    let (tx, events) = mpsc::channel::<Ev>();
    let worker = state.clone();
    std::thread::Builder::new().name("mado-snap".into()).spawn(move || run(events, worker, news)).ok();
    let mask = NSEventMask::LeftMouseDown | NSEventMask::LeftMouseDragged | NSEventMask::LeftMouseUp | NSEventMask::KeyDown;
    let block = block2::RcBlock::new(move |event: NonNull<NSEvent>| {
        let e = unsafe { event.as_ref() };
        let kind = e.r#type();
        if kind == NSEventType::KeyDown {
            if e.keyCode() == mado_core::shortcut::ESCAPE {
                let _ = tx.send(Ev::Escape);
            }
            return;
        }
        // In the Accessibility API's points: down from the main screen's top.
        // Where the event happened, not where the cursor is as it is read:
        // handed on a beat late, a press read where the cursor had gone
        // was taken for a press on the window under it then, and the drag
        // of the window pressed went unseen. A global event has no window,
        // so its place in one is its place on the screens.
        let Some(mtm) = MainThreadMarker::new() else { return };
        let p = e.locationInWindow();
        let top = NSScreen::screens(mtm).firstObject().map_or(0., |s| s.frame().size.height);
        let (x, y) = (p.x, top - p.y);
        let ev = match kind {
            NSEventType::LeftMouseDown => {
                if !state.on.load(Ordering::Relaxed) {
                    return;
                }
                Ev::Down { x, y, screens: place::screens() }
            }
            NSEventType::LeftMouseDragged => Ev::Drag { x, y },
            _ => Ev::Up { x, y },
        };
        let _ = tx.send(ev);
    });
    let monitor = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &block);
    // Kept for the app's life.
    std::mem::forget(monitor);
    rx
}

/// The window a press at a point holds: the window server's topmost
/// there, found among its app's windows by its number. Accessibility's
/// hit test, asked first, named the window under a test window that did
/// not answer for its title bar, and the drag of the window pressed went
/// unwatched; it stands in only where the number cannot be had. Mado's
/// own windows are left alone.
fn find(x: f64, y: f64) -> Option<(Element, u64, Rect)> {
    let own = std::process::id() as i32;
    let w = match super::ax::window_under(x, y) {
        Some((pid, _)) if pid == own => return None,
        Some((pid, id)) => Element::app(pid)?.windows().into_iter().find(|w| w.window_id() == id),
        None => None,
    };
    let w = match w {
        Some(w) => w,
        None => Element::window_at(x, y).filter(|w| w.pid() != Some(own))?,
    };
    let frame = place::frame(&w)?;
    Some((w.clone(), w.window_id(), frame))
}

fn run(events: mpsc::Receiver<Ev>, state: Arc<SnapState>, news: UnboundedSender<Footprint>) {
    let mut drag: Option<Drag> = None;
    let show = |d: &mut Drag, hit: Option<(usize, Action)>| {
        if hit == d.area {
            return;
        }
        d.area = hit;
        let frame = hit.and_then(|(s, a)| Memory::snap_plan(a, &d.screens, s)).map(|p| p.frame);
        let _ = news.unbounded_send(match frame {
            Some(f) => Footprint::Show(f, Some(d.at)),
            None => Footprint::Hide,
        });
    };
    while let Ok(ev) = events.recv() {
        match ev {
            Ev::Down { x, y, screens } => {
                let found = find(x, y);
                let (window, id, start) = match found {
                    Some((w, id, f)) => (Some(w), id, f),
                    None => (None, 0, Rect::default()),
                };
                drag = Some(Drag { from: (x, y), window, id, start, at: start, tries: 1, tried: Instant::now(), moving: false, away: false, still: false, area: None, screens });
            }
            Ev::Drag { x, y } => {
                let Some(d) = drag.as_mut() else { continue };
                // The window under a press is not always known at once (an
                // app slow to answer): asked again, now and then.
                if d.window.is_none() {
                    if d.tries >= 20 || d.tried.elapsed() < Duration::from_millis(100) {
                        continue;
                    }
                    d.tries += 1;
                    d.tried = Instant::now();
                    if let Some((w, id, f)) = find(x, y) {
                        (d.window, d.id, d.start, d.at) = (Some(w), id, f, f);
                    }
                    continue;
                }
                if d.still {
                    continue;
                }
                let Some(now) = d.window.as_ref().and_then(place::frame) else { continue };
                d.at = now;
                let snapping = state.how.lock().map(|h| h.clone()).unwrap_or_default();
                if !d.moving {
                    let s = d.start;
                    let sized = (now.w - s.w).abs() > 0.5 || (now.h - s.h).abs() > 0.5;
                    let kept = [(now.x - s.x).abs() < 0.5, (now.right() - s.right()).abs() < 0.5, (now.y - s.y).abs() < 0.5, (now.bottom() - s.bottom()).abs() < 0.5].iter().filter(|k| **k).count();
                    if sized && kept >= 2 {
                        d.still = true;
                        continue;
                    }
                    if (now.x - s.x).abs() < 0.5 && (now.y - s.y).abs() < 0.5 {
                        continue;
                    }
                    d.moving = true;
                }
                if !d.away && (x - d.from.0).hypot(y - d.from.1) >= AWAY {
                    d.away = true;
                    let back = state.memory.lock().unwrap_or_else(|e| e.into_inner()).dragged(d.id, d.start, now, x);
                    match (back, d.window.as_ref()) {
                        (Some(r), Some(w)) => {
                            w.set_size(r.w, r.h);
                            w.set_position(r.x, now.y);
                            d.at = Rect { y: now.y, ..r };
                            super::log(format_args!("dragged away: {now:?} -> {r:?}"));
                        }
                        _ => super::log(format_args!("dragged away: {now:?}, not where Mado put it")),
                    }
                }
                if !snapping.on {
                    show(d, None);
                    continue;
                }
                let hit = mado_core::snap::at(x, y, &d.screens);
                show(d, hit);
            }
            Ev::Up { x, y } => {
                let Some(mut d) = drag.take() else { continue };
                if d.moving {
                    super::log(format_args!("drag let go at {x:.0},{y:.0}: {:?}", mado_core::snap::at(x, y, &d.screens).map(|(_, a)| a.key())));
                } else if d.window.is_none() && (x - d.from.0).hypot(y - d.from.1) >= AWAY {
                    super::log(format_args!("drag let go at {x:.0},{y:.0}: no window found under the press"));
                }
                if !d.moving || d.area.is_none() {
                    continue;
                }
                // The last drag event may be behind the release: asked again
                // where it was let go.
                let hit = mado_core::snap::at(x, y, &d.screens);
                let _ = news.unbounded_send(Footprint::Hide);
                d.area = None;
                let (Some((s, action)), Some(w)) = (hit, d.window.as_ref()) else { continue };
                if let Some(plan) = Memory::snap_plan(action, &d.screens, s) {
                    // On a thread of its own: it waits for the window to be
                    // still, and the next drag's events are not held.
                    let (w, start, screens, state) = (w.clone(), d.start, d.screens.clone(), state.clone());
                    std::thread::spawn(move || place::snapped(&w, &plan, start, &screens, &state.memory));
                }
            }
            Ev::Escape => {
                if let Some(d) = drag.as_mut() {
                    if d.area.is_some() {
                        let _ = news.unbounded_send(Footprint::Hide);
                    }
                    d.area = None;
                    d.still = true;
                }
            }
        }
    }
}
