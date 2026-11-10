//! Where a window goes: the frame an action asks for, from the window's
//! frame and the screens' usable areas, and what Mado remembers of each
//! window it moved, for Restore and for a drag away.
//!
//! Every frame here is in the Accessibility API's points: the origin at
//! the top left of the main screen, y going down, so a frame read from a
//! window is set back as it is. The app reads the screens and the window,
//! asks `Memory::plan`, sets the frame, reads back what the window took,
//! and tells `Memory::done`. Nothing here asks the system anything, so
//! every rule is tested without a Mac.

use std::collections::HashMap;

use crate::action::Action::{self, *};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }

    fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2., self.y + self.h / 2.)
    }

    fn overlap(&self, o: &Rect) -> f64 {
        let w = self.right().min(o.right()) - self.x.max(o.x);
        let h = self.bottom().min(o.bottom()) - self.y.max(o.y);
        if w > 0. && h > 0. { w * h } else { 0. }
    }

    /// Whether a point is inside, its far edges included.
    pub fn holds(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
    }

    /// Within a couple of points of `o` at every edge: a window an app
    /// sized to its own steps (a terminal's cells) is still where it was
    /// put.
    pub fn near(&self, o: &Rect) -> bool {
        const SLACK: f64 = 2.;
        (self.x - o.x).abs() <= SLACK && (self.y - o.y).abs() <= SLACK && (self.w - o.w).abs() <= SLACK && (self.h - o.h).abs() <= SLACK
    }

    /// The size of `o`, within a couple of points, wherever it stands.
    fn same_size(&self, o: &Rect) -> bool {
        Rect { x: o.x, y: o.y, ..*self }.near(o)
    }

    /// On whole points, edge by edge: a frame set at a fraction of a point
    /// lands on a whole one, and the next press would not know it. Each
    /// edge is rounded, not the size, so two halves still meet and a
    /// window never stands a point past the screen.
    fn round(self) -> Rect {
        let (x, y) = (self.x.round(), self.y.round());
        Rect { x, y, w: self.right().round() - x, h: self.bottom().round() - y }
    }

    /// Kept inside `area` where it fits, moved and never resized.
    fn inside(self, area: &Rect) -> Rect {
        let x = if self.w <= area.w { self.x.clamp(area.x, area.right() - self.w) } else { area.x };
        let y = if self.h <= area.h { self.y.clamp(area.y, area.bottom() - self.h) } else { area.y };
        Rect { x, y, ..self }
    }
}

/// A screen: all of it, and the part a window may use, without the menu
/// bar and the Dock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Screen {
    pub frame: Rect,
    pub area: Rect,
}

/// The screen a window is on: the one that holds the most of it, else the
/// one whose center is nearest its center.
pub fn screen_of(window: &Rect, screens: &[Screen]) -> Option<usize> {
    let most = screens.iter().enumerate().map(|(i, s)| (i, window.overlap(&s.frame))).filter(|(_, a)| *a > 0.).max_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((i, _)) = most {
        return Some(i);
    }
    let (cx, cy) = window.center();
    screens
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let (sx, sy) = s.frame.center();
            (i, (sx - cx).powi(2) + (sy - cy).powi(2))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// The screens in the order Next Display goes through them: in rows from
/// the top, a screen wholly above another first, and left to right within
/// a row.
pub fn by_place(screens: &[Screen]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..screens.len()).collect();
    order.sort_by(|&a, &b| {
        let (fa, fb) = (screens[a].frame, screens[b].frame);
        if fa.bottom() <= fb.y {
            std::cmp::Ordering::Less
        } else if fb.bottom() <= fa.y {
            std::cmp::Ordering::Greater
        } else {
            fa.x.total_cmp(&fb.x)
        }
    });
    order
}

/// The screen after (or before) `here` in `by_place`'s order, round again
/// at the end; none with one screen.
fn adjacent(screens: &[Screen], here: usize, forward: bool) -> Option<usize> {
    if screens.len() < 2 {
        return None;
    }
    let order = by_place(screens);
    let at = order.iter().position(|&i| i == here)?;
    let n = order.len();
    Some(order[if forward { (at + 1) % n } else { (at + n - 1) % n }])
}

/// A part of the area, in fractions of it: left, top, width, height.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Part(f64, f64, f64, f64);

/// Where an action that is a part of the area puts the window: the
/// halves, the corners and Maximize.
fn part(action: Action) -> Option<Part> {
    match action {
        LeftHalf => Some(Part(0., 0., 0.5, 1.)),
        RightHalf => Some(Part(0.5, 0., 0.5, 1.)),
        TopHalf => Some(Part(0., 0., 1., 0.5)),
        BottomHalf => Some(Part(0., 0.5, 1., 0.5)),
        TopLeft => Some(Part(0., 0., 0.5, 0.5)),
        TopRight => Some(Part(0.5, 0., 0.5, 0.5)),
        BottomLeft => Some(Part(0., 0.5, 0.5, 0.5)),
        BottomRight => Some(Part(0.5, 0.5, 0.5, 0.5)),
        Maximize => Some(Part(0., 0., 1., 1.)),
        _ => None,
    }
}

/// The cells `n` windows tiled by `action` take, in fractions of the
/// area, in the order the windows are given them. Rows run down the area
/// and columns across it. A grid is as near a square as the count
/// allows, with more columns than rows on a landscape screen (more rows
/// on a portrait one), and its last row takes what is left across the
/// whole width, so the windows fill the screen.
fn cells(action: Action, n: usize, portrait: bool) -> Vec<Part> {
    let k = n as f64;
    match action {
        TileRows => (0..n).map(|i| Part(0., i as f64 / k, 1., 1. / k)).collect(),
        TileColumns => (0..n).map(|i| Part(i as f64 / k, 0., 1. / k, 1.)).collect(),
        TileGrid if n > 0 => {
            let (cols, rows) = grid(n);
            let last = n - cols * (rows - 1);
            let grid = (0..n).map(|i| {
                let (r, c) = (i / cols, i % cols);
                let across = if r == rows - 1 { last } else { cols } as f64;
                Part(c as f64 / across, r as f64 / rows as f64, 1. / across, 1. / rows as f64)
            });
            // On a portrait screen, the same grid turned: columns of
            // windows, the last one the full height.
            if portrait { grid.map(|Part(x, y, w, h)| Part(y, x, h, w)).collect() } else { grid.collect() }
        }
        _ => Vec::new(),
    }
}

/// A grid for `n` windows: the columns of its full rows, and its rows.
fn grid(n: usize) -> (usize, usize) {
    let cols = ((n as f64).sqrt().ceil() as usize).max(1);
    (cols, n.div_ceil(cols))
}

/// The windows on a screen tiled by `action` (Tile in Rows, in Columns,
/// in a Grid) over its usable `area`: each window's frame, in the order
/// the windows were given. Each goes to the cell nearest the order of
/// its place: rows by how high it stands, columns by how far left, a
/// grid in reading order. Tiled again, every window keeps its cell.
pub fn tile(action: Action, windows: &[Rect], area: &Rect) -> Vec<Rect> {
    let n = windows.len();
    let portrait = area.h > area.w;
    let cells = cells(action, n, portrait);
    if cells.len() != n {
        return Vec::new();
    }
    // A grid's windows are read by the band of its rows (its columns on a
    // portrait screen) they stand in, so two in one row that an app made
    // a little taller or shorter are still read left to right.
    let lines = grid(n).1.max(1);
    let band = |len: f64, at: f64| ((at / (len / lines as f64)).floor() as i64).clamp(0, lines as i64 - 1);
    let key = |r: &Rect| -> (i64, f64, f64) {
        let (cx, cy) = r.center();
        let (cx, cy) = (cx - area.x, cy - area.y);
        match action {
            TileRows => (0, cy, cx),
            TileColumns => (0, cx, cy),
            _ if portrait => (band(area.w, cx), cy, cx),
            _ => (band(area.h, cy), cx, cy),
        }
    };
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        let (ka, kb) = (key(&windows[a]), key(&windows[b]));
        ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1)).then(ka.2.total_cmp(&kb.2))
    });
    let mut out = vec![Rect::default(); n];
    for (cell, &w) in cells.iter().zip(&order) {
        out[w] = place_part(*cell, area);
    }
    out
}

/// Where an action puts a window, as a picture of it shows it, on a
/// landscape screen, in fractions of the area (left, top,
/// width, height). Those that are not a part (Center, Larger, a display)
/// are drawn as a window of their own size where they leave it; a tiling
/// covers the area, its cells drawn by `preview_cells`.
pub fn preview(action: Action) -> (f64, f64, f64, f64) {
    if let Some(Part(x, y, w, h)) = part(action) {
        return (x, y, w, h);
    }
    match action {
        AlmostMaximize => (0.05, 0.05, 0.9, 0.9),
        MaximizeHeight => (0.3, 0., 0.4, 1.),
        Center => (0.25, 0.2, 0.5, 0.6),
        Larger => (0.12, 0.1, 0.76, 0.8),
        Smaller => (0.32, 0.28, 0.36, 0.44),
        Restore => (0.22, 0.18, 0.46, 0.52),
        NextDisplay => (0.56, 0.2, 0.4, 0.6),
        PreviousDisplay => (0.04, 0.2, 0.4, 0.6),
        _ => (0., 0., 1., 1.),
    }
}

/// A tiling's cells as its picture shows them: three windows in rows or
/// columns, four in a grid, in fractions of the area. None for any other
/// action.
pub fn preview_cells(action: Action) -> Vec<(f64, f64, f64, f64)> {
    let n = if action == TileGrid { 4 } else { 3 };
    cells(action, n, false).into_iter().map(|Part(x, y, w, h)| (x, y, w, h)).collect()
}

/// A part of `area`, on whole points.
fn place_part(p: Part, area: &Rect) -> Rect {
    Rect { x: area.x + area.w * p.0, y: area.y + area.h * p.1, w: (area.w * p.2).max(1.), h: (area.h * p.3).max(1.) }.round()
}

/// A window's frame carried from one area to another at the same place
/// in proportion, so a window on the left third of one screen is on the
/// left third of the next.
fn carry(window: &Rect, from: &Rect, to: &Rect) -> Rect {
    let kx = to.w / from.w;
    let ky = to.h / from.h;
    let w = (window.w * kx).min(to.w);
    let h = (window.h * ky).min(to.h);
    let x = (to.x + (window.x - from.x) * kx).clamp(to.x, to.right() - w);
    let y = (to.y + (window.y - from.y) * ky).clamp(to.y, to.bottom() - h);
    Rect { x, y, w, h }.round()
}

/// A size kept inside an area and centered in it.
fn centered(w: f64, h: f64, area: &Rect) -> Rect {
    let (w, h) = (w.min(area.w), h.min(area.h));
    Rect { x: area.x + (area.w - w) / 2., y: area.y + (area.h - h) / 2., w, h }.round()
}

/// How far Larger and Smaller move the window's size, half at each side.
const SIZE_STEP: f64 = 30.;
/// Smaller stops at a quarter of the area each way.
const LEAST: f64 = 0.25;
/// Almost Maximize leaves this much of the area around the window.
const ALMOST: f64 = 0.9;

/// How near the area's edge, in points, an edge of a window counts as
/// against it for Larger and Smaller.
const AGAINST: f64 = 5.;

/// Larger or Smaller, as Rectangle has them: the size changes by 30
/// points, half at each side, and an edge against the area's edge stays
/// there, so a window against the left grows to the right. Smaller never
/// goes under a quarter of the area each way.
fn resize(window: &Rect, area: &Rect, grow: bool) -> Option<Rect> {
    let o = if grow { SIZE_STEP } else { -SIZE_STEP };
    let t = AGAINST;
    let (l, r) = ((window.x - area.x).abs() <= t, (window.right() - area.right()).abs() <= t);
    let (top, bot) = ((window.y - area.y).abs() <= t, (window.bottom() - area.bottom()).abs() <= t);
    let mut w = window.w + o;
    let mut h = window.h + o;
    let mut x = window.x - (o / 2.).floor();
    let mut y = window.y - (o / 2.).floor();
    // Against every edge and shrinking: from both sides, as a centered
    // window does.
    let all = l && r && top && bot;
    if !all || grow {
        if l && r {
            x = area.x;
            w = area.w;
        } else if r {
            x = area.right() - w;
        } else if l {
            x = area.x;
        }
        if top && bot {
            y = area.y;
            h = area.h;
        } else if bot {
            y = area.bottom() - h;
        } else if top {
            y = area.y;
        }
    }
    w = w.min(area.w);
    h = h.min(area.h);
    if !grow && (w < (area.w * LEAST).floor().max(1.) || h < (area.h * LEAST).floor().max(1.)) {
        return None;
    }
    let out = Rect { x, y, w, h }.inside(area).round();
    (!out.near(window)).then_some(out)
}

/// Where the window goes, on which screen, and as what, for
/// `Memory::done` to note once it is there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan {
    pub frame: Rect,
    pub screen: usize,
    /// What it was placed as: a half pressed again across displays lands
    /// as the facing half there, and is noted so.
    pub action: Action,
    /// It went to another screen: the app raises the window.
    pub crossed: bool,
}

/// Which of a frame's two parts is set first when a window is put at
/// once: Accessibility sets a position and a size by two calls, and the
/// screen may show the window between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum First {
    /// Moved, then sized, then moved again where the size moved it.
    Position,
    /// Sized, then moved, then sized again where the app held the size back.
    Size,
}

/// The order that keeps covered what the window covers before and after:
/// a frame between the two that uncovered it showed what is behind the
/// window for a moment, a flash (full screen to the right half, sized
/// first, stood on the left half). A move is taken as it is asked, off
/// the screen too; a size that would carry the window past `bounds` is
/// held back there, as apps hold it, and the move after it stands at
/// that size. The order whose frames between cover the most of both is
/// taken, the move first where they cover alike, since a move does not
/// redraw. Right half to the whole screen has no such order: the left
/// edge cannot leave before the window is wide, nor the window widen
/// before the left edge leaves.
pub fn first(from: &Rect, to: &Rect, bounds: &Rect) -> First {
    let both = {
        let x = from.x.max(to.x);
        let y = from.y.max(to.y);
        Rect::new(x, y, (from.right().min(to.right()) - x).max(0.), (from.bottom().min(to.bottom()) - y).max(0.))
    };
    let kept = |between: &[Rect]| between.iter().map(|r| r.overlap(&both)).fold(f64::INFINITY, f64::min);
    let moved = Rect::new(to.x, to.y, from.w, from.h);
    let held = |size: f64, old: f64, at: f64, edge: f64| if size <= old { size } else { size.min((edge - at).max(old)) };
    let sized = Rect::new(from.x, from.y, held(to.w, from.w, from.x, bounds.right()), held(to.h, from.h, from.y, bounds.bottom()));
    let then_moved = Rect::new(to.x, to.y, sized.w, sized.h);
    if kept(&[moved]) >= kept(&[sized, then_moved]) { First::Position } else { First::Size }
}

/// The frame a window takes once the app has had its say. An app may
/// refuse a size (a least width, a terminal's cells): the window keeps
/// the edges the plan put nearer the area's edges, so a right half that
/// came out wider still meets the screen's right.
pub fn settle(plan: &Rect, took: &Rect, area: &Rect) -> Option<Rect> {
    if (took.w - plan.w).abs() <= 1. && (took.h - plan.h).abs() <= 1. {
        return None;
    }
    let to_right = area.right() - plan.right() < plan.x - area.x;
    let to_bottom = area.bottom() - plan.bottom() < plan.y - area.y;
    let mut x = if to_right { plan.right() - took.w } else { plan.x };
    let mut y = if to_bottom { plan.bottom() - took.h } else { plan.y };
    if took.w <= area.w {
        x = x.clamp(area.x, area.right() - took.w);
    }
    if took.h <= area.h {
        y = y.clamp(area.y, area.bottom() - took.h);
    }
    let out = Rect { x, y, w: took.w, h: took.h }.round();
    let moved = (out.x - took.x).abs() > 1. || (out.y - took.y).abs() > 1.;
    moved.then_some(out)
}

/// The last move Mado made of a window.
#[derive(Debug, Clone, Copy)]
struct Last {
    action: Action,
    frame: Rect,
}

/// What Mado remembers of a window it moved.
#[derive(Debug, Clone, Copy)]
struct Kept {
    /// Where Restore puts it: where it was before Mado last found it moved.
    before: Rect,
    last: Option<Last>,
    /// When it was last moved, in moves: the oldest is let go first.
    tick: u64,
}

/// What Mado remembers of the windows it moved, by the window server's
/// number for each. It is held for the app's life, and only so many
/// windows are remembered: a window long closed is let go.
#[derive(Debug, Default)]
pub struct Memory {
    windows: HashMap<u64, Kept>,
    tick: u64,
}

/// How far, in points, the cursor goes from where a drag pressed before
/// a window Mado placed takes back its size: a nudge leaves it placed.
pub const AWAY: f64 = 30.;

/// The most windows remembered at once.
const REMEMBERED: usize = 256;

impl Memory {
    /// The last move of the window, while it is still where that put it.
    fn last(&self, id: u64, window: &Rect) -> Option<Last> {
        self.windows.get(&id).and_then(|k| k.last).filter(|l| l.frame.near(window))
    }

    /// The frame `action` asks for, for the window `id` at `window`, or
    /// none when there is nothing to do (Restore with nothing to go back
    /// to, a display that is the only one, a window already as small as
    /// Smaller goes). Pressed again, an action asks for the same frame, so
    /// the window stays where it is.
    pub fn plan(&self, id: u64, action: Action, window: Rect, screens: &[Screen]) -> Option<Plan> {
        let here = screen_of(&window, screens)?;
        let last = self.last(id, &window);
        let at = |frame: Rect| Some(Plan { frame, screen: here, action, crossed: false });
        match action {
            Restore => {
                let k = self.windows.get(&id)?;
                (!k.before.near(&window)).then(|| Plan { frame: k.before, screen: screen_of(&k.before, screens).unwrap_or(here), action: Restore, crossed: false })
            }
            NextDisplay | PreviousDisplay => {
                let to = adjacent(screens, here, action == NextDisplay)?;
                let dest = screens[to].area;
                // A window Mado placed is placed the same way there; any
                // other keeps its place in proportion.
                if let Some(l) = last.filter(|l| part(l.action).is_some()) {
                    return Self::place(l.action, screens, to, true);
                }
                Some(Plan { frame: carry(&window, &screens[here].area, &dest), screen: to, action, crossed: true })
            }
            Center => at(centered(window.w, window.h, &screens[here].area)),
            AlmostMaximize => {
                let area = screens[here].area;
                at(centered(area.w * ALMOST, area.h * ALMOST, &area))
            }
            MaximizeHeight => {
                let area = screens[here].area;
                let w = window.w.min(area.w);
                let x = window.x.clamp(area.x, area.right() - w);
                at(Rect { x, y: area.y, w, h: area.h }.round())
            }
            Larger | Smaller => resize(&window, &screens[here].area, action == Larger).and_then(at),
            // Every window on the screen at once: `tile`.
            TileRows | TileColumns | TileGrid => None,
            _ => Self::place(action, screens, here, false),
        }
    }

    /// An action that is a part of the area, on the screen `to`.
    fn place(action: Action, screens: &[Screen], to: usize, crossed: bool) -> Option<Plan> {
        let area = screens.get(to)?.area;
        Some(Plan { frame: place_part(part(action)?, &area), screen: to, action, crossed })
    }

    /// Where a drag that ends in a snap area puts a window: the action's
    /// part of the screen the cursor is on.
    pub fn snap_plan(action: Action, screens: &[Screen], to: usize) -> Option<Plan> {
        Self::place(action, screens, to, false)
    }

    /// The window `id` was moved by a plan from `before` and now stands at
    /// `took`, the frame read back from it. A move from a snap area keeps
    /// the place Restore goes back to (`restore` false): the drag noted it
    /// as it began (`dragged`).
    pub fn done(&mut self, id: u64, plan: &Plan, before: Rect, took: Rect, restore: bool) {
        self.tick += 1;
        let tick = self.tick;
        let kept = self.windows.entry(id).or_insert(Kept { before, last: None, tick });
        // A window moved since Mado last put it (by hand, by the app) has
        // that place to go back to, not the one before.
        let ours = kept.last.is_some_and(|l| l.frame.near(&before));
        if restore && !ours {
            kept.before = before;
        }
        kept.tick = tick;
        // Back where it was, the next move notes its place afresh.
        kept.last = (plan.action != Restore).then_some(Last { action: plan.action, frame: took });
        if self.windows.len() > REMEMBERED {
            if let Some(oldest) = self.windows.iter().min_by_key(|(_, k)| k.tick).map(|(id, _)| *id) {
                self.windows.remove(&oldest);
            }
        }
    }

    /// The window `id`, put by `action` and noted at the frame asked for
    /// while it was still moving (an app's own command moves it over a
    /// moment), came to rest at `took`: it counts as placed from there.
    /// A move since by another action is left as it is.
    pub fn settled(&mut self, id: u64, action: Action, took: Rect) {
        if let Some(last) = self.windows.get_mut(&id).and_then(|k| k.last.as_mut()).filter(|l| l.action == action) {
            last.frame = took;
        }
    }

    /// A drag of the window `id` has taken it `AWAY` from where it was
    /// pressed, from `start`; it is
    /// at `now`, with the cursor at `cursor_x`. A window Mado placed and
    /// that had not moved since takes back the size Restore would give it,
    /// staying under the cursor; any other notes `start` as where Restore
    /// goes back to.
    pub fn dragged(&mut self, id: u64, start: Rect, now: Rect, cursor_x: f64) -> Option<Rect> {
        self.tick += 1;
        let tick = self.tick;
        // Placed by Mado: still where it put it, or moved since with the
        // size it gave (an app that moved it once more as a snap's drag
        // ended). Taken for one never placed, a snapped window noted its
        // snapped size as the one to go back to, and the next drag away
        // gave it back that size.
        let placed = self.windows.get(&id).and_then(|k| k.last).is_some_and(|l| l.frame.near(&start) || l.frame.same_size(&start));
        let kept = self.windows.entry(id).or_insert(Kept { before: start, last: None, tick });
        kept.tick = tick;
        if !placed {
            kept.before = start;
            return None;
        }
        kept.last = None;
        let (w, h) = (kept.before.w, kept.before.h);
        // Under the cursor: the window shifts right as far as it must to
        // keep the point it was held by inside it.
        let inset = (w / 2.).min(32.);
        let shift = (cursor_x - now.x - (w - inset)).clamp(0., (now.w - w).max(0.));
        Some(Rect { x: now.x + shift, y: now.y, w, h }.round())
    }

    pub fn len(&self) -> usize {
        self.windows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The screen of the tests on the order of a frame's two calls: 2048
    /// by 1045 under a menu bar of 30.
    const WIDE: Rect = Rect::new(0., 30., 2048., 1045.);

    #[test]
    fn full_screen_to_the_right_half_is_moved_first_and_keeps_the_right_covered() {
        let full = WIDE;
        let right = Rect::new(1024., 30., 1024., 1045.);
        assert_eq!(first(&full, &right, &WIDE), First::Position);
    }

    #[test]
    fn a_window_grown_in_place_or_shrunk_in_place_is_moved_first() {
        let left = Rect::new(0., 30., 1024., 1045.);
        assert_eq!(first(&left, &WIDE, &WIDE), First::Position);
        assert_eq!(first(&WIDE, &left, &WIDE), First::Position);
    }

    #[test]
    fn the_right_half_to_the_whole_screen_has_no_order_without_a_gap_and_is_moved_first() {
        let right = Rect::new(1024., 30., 1024., 1045.);
        assert_eq!(first(&right, &WIDE, &WIDE), First::Position);
    }

    #[test]
    fn a_small_window_left_of_the_middle_grows_where_it_is_before_it_moves() {
        // Moved first, it would leave its place for the corner; sized
        // first, it widens to the screen's edge, still over its place, and
        // moved to the corner at that width it still covers it.
        let small = Rect::new(300., 200., 600., 400.);
        assert_eq!(first(&small, &WIDE, &WIDE), First::Size);
    }

    /// A 1440 by 900 screen with a 25-point menu bar, and one to its right.
    fn one() -> Vec<Screen> {
        vec![Screen { frame: Rect::new(0., 0., 1440., 900.), area: Rect::new(0., 25., 1440., 875.) }]
    }

    fn two() -> Vec<Screen> {
        let mut s = one();
        s.push(Screen { frame: Rect::new(1440., 0., 1920., 1080.), area: Rect::new(1440., 0., 1920., 1080.) });
        s
    }

    const W: Rect = Rect::new(200., 200., 600., 400.);

    fn place(m: &mut Memory, action: Action, window: Rect, screens: &[Screen]) -> Option<Rect> {
        let p = m.plan(1, action, window, screens)?;
        m.done(1, &p, window, p.frame, true);
        Some(p.frame)
    }

    #[test]
    fn halves_and_corners_fill_the_area() {
        let s = one();
        let m = Memory::default();
        let at = |a| m.plan(9, a, W, &s).unwrap().frame;
        assert_eq!(at(LeftHalf), Rect::new(0., 25., 720., 875.));
        assert_eq!(at(RightHalf), Rect::new(720., 25., 720., 875.));
        assert_eq!(at(TopHalf), Rect::new(0., 25., 1440., 438.));
        assert_eq!(at(BottomRight), Rect::new(720., 463., 720., 437.));
        assert_eq!(at(Maximize), Rect::new(0., 25., 1440., 875.));
    }

    #[test]
    fn pressed_again_an_action_leaves_the_window_where_it_is() {
        let s = one();
        let mut m = Memory::default();
        let a = place(&mut m, LeftHalf, W, &s).unwrap();
        assert_eq!(place(&mut m, LeftHalf, a, &s), Some(a));
        let b = place(&mut m, BottomRight, a, &s).unwrap();
        assert_eq!(place(&mut m, BottomRight, b, &s), Some(b));
        let c = place(&mut m, Maximize, b, &s).unwrap();
        assert_eq!(place(&mut m, Maximize, c, &s), Some(c));
    }

    #[test]
    fn a_window_that_settled_off_the_frame_asked_is_still_mados() {
        let s = one();
        let mut m = Memory::default();
        // Noted at the frame asked for while the app's command moved it,
        // it came to rest lower: once told, a move from there keeps the
        // place Restore goes back to.
        let a = place(&mut m, LeftHalf, W, &s).unwrap();
        let rest = Rect { y: a.y + 10., h: a.h - 10., ..a };
        m.settled(1, LeftHalf, rest);
        let b = place(&mut m, Maximize, rest, &s).unwrap();
        assert_eq!(place(&mut m, Restore, b, &s), Some(W));
        // Another action's rest changes nothing.
        let mut m = Memory::default();
        let a = place(&mut m, LeftHalf, W, &s).unwrap();
        let rest = Rect { y: a.y + 10., h: a.h - 10., ..a };
        m.settled(1, RightHalf, rest);
        let b = place(&mut m, Maximize, rest, &s).unwrap();
        assert_eq!(place(&mut m, Restore, b, &s), Some(rest));
    }

    #[test]
    fn restore_goes_back_to_before_the_first_move() {
        let s = one();
        let mut m = Memory::default();
        let a = place(&mut m, LeftHalf, W, &s).unwrap();
        let b = place(&mut m, Maximize, a, &s).unwrap();
        assert_eq!(place(&mut m, Restore, b, &s), Some(W));
        assert!(m.plan(1, Restore, W, &s).is_none(), "it is there already");
    }

    #[test]
    fn a_window_moved_by_hand_restores_to_where_the_hand_left_it() {
        let s = one();
        let mut m = Memory::default();
        place(&mut m, LeftHalf, W, &s).unwrap();
        let by_hand = Rect::new(300., 300., 500., 300.);
        let c = place(&mut m, Maximize, by_hand, &s).unwrap();
        assert_eq!(place(&mut m, Restore, c, &s), Some(by_hand));
    }

    #[test]
    fn the_next_display_places_a_half_the_same_way_and_others_in_proportion() {
        let s = two();
        let mut m = Memory::default();
        let a = place(&mut m, LeftHalf, W, &s).unwrap();
        let b = place(&mut m, NextDisplay, a, &s).unwrap();
        assert_eq!(b, Rect::new(1440., 0., 960., 1080.));
        assert_eq!(place(&mut m, NextDisplay, b, &s).unwrap(), Rect::new(0., 25., 720., 875.));
        let free = Memory::default().plan(2, PreviousDisplay, W, &s).unwrap().frame;
        assert_eq!(free.x, 1440. + (200f64 * 1920. / 1440.).round());
        assert!(Memory::default().plan(2, NextDisplay, W, &one()).is_none());
    }

    #[test]
    fn screens_go_in_rows_then_left_to_right() {
        let s = vec![
            Screen { frame: Rect::new(0., 0., 100., 100.), area: Rect::new(0., 0., 100., 100.) },
            Screen { frame: Rect::new(-100., -100., 100., 100.), area: Rect::new(-100., -100., 100., 100.) },
            Screen { frame: Rect::new(-100., 0., 100., 100.), area: Rect::new(-100., 0., 100., 100.) },
        ];
        assert_eq!(by_place(&s), [1, 2, 0]);
    }

    #[test]
    fn the_screen_is_the_one_with_most_of_the_window() {
        let s = two();
        assert_eq!(screen_of(&Rect::new(1300., 100., 400., 300.), &s), Some(1));
        assert_eq!(screen_of(&Rect::new(1200., 100., 400., 300.), &s), Some(0));
        assert_eq!(screen_of(&Rect::new(-5000., 100., 400., 300.), &s), Some(0), "off every screen: the nearest");
    }

    #[test]
    fn larger_and_smaller_keep_an_edge_at_the_screen() {
        let s = one();
        let m = Memory::default();
        assert_eq!(m.plan(1, Larger, W, &s).unwrap().frame, Rect::new(185., 185., 630., 430.));
        let left = Rect::new(0., 25., 720., 875.);
        let shrunk = m.plan(1, Smaller, left, &s).unwrap().frame;
        assert_eq!((shrunk.x, shrunk.w), (0., 690.), "the left edge stays at the screen's");
        let full = Rect::new(0., 25., 1440., 875.);
        assert!(m.plan(1, Larger, full, &s).is_none(), "nowhere to grow");
        let least = Rect::new(500., 300., 380., 230.);
        assert!(m.plan(1, Smaller, least, &s).is_none(), "at the least already");
    }

    #[test]
    fn center_almost_maximize_and_maximize_height() {
        let s = one();
        let m = Memory::default();
        assert_eq!(m.plan(1, Center, W, &s).unwrap().frame, Rect::new(420., 263., 600., 400.));
        assert_eq!(m.plan(1, AlmostMaximize, W, &s).unwrap().frame, Rect::new(72., 69., 1296., 787.));
        assert_eq!(m.plan(1, MaximizeHeight, W, &s).unwrap().frame, Rect::new(200., 25., 600., 875.));
    }

    #[test]
    fn a_window_that_refused_its_size_keeps_the_screen_edge() {
        let area = Rect::new(0., 25., 1440., 875.);
        let plan = Rect::new(720., 25., 720., 875.);
        assert_eq!(settle(&plan, &Rect::new(720., 25., 800., 875.), &area), Some(Rect::new(640., 25., 800., 875.)));
        let left = Rect::new(0., 25., 720., 875.);
        assert_eq!(settle(&left, &Rect::new(0., 25., 800., 875.), &area), None);
        assert_eq!(settle(&plan, &plan, &area), None);
    }

    #[test]
    fn a_placed_window_dragged_away_takes_back_its_size() {
        let s = one();
        let mut m = Memory::default();
        let a = place(&mut m, LeftHalf, W, &s).unwrap();
        // Held near its right edge: it stays under the cursor.
        let back = m.dragged(1, a, a, 700.).unwrap();
        assert_eq!((back.w, back.h), (600., 400.));
        assert_eq!(back.x, 120., "shifted as far as the width it gave up, so the cursor is still on it");
        // Restore then has nothing to go back to: it is that size again.
        assert!(m.dragged(1, back, back, 700.).is_none(), "dragged again, it is not Mado's to resize");
        // A window never placed notes where the drag began.
        let mut m = Memory::default();
        assert!(m.dragged(2, W, W, 300.).is_none());
        let p = Memory::snap_plan(LeftHalf, &s, 0).unwrap();
        m.done(2, &p, W, p.frame, false);
        assert_eq!(m.plan(2, Restore, p.frame, &s).unwrap().frame, W);
    }

    #[test]
    fn snapped_and_dragged_away_again_and_again_it_takes_back_its_own_size() {
        let s = one();
        let mut m = Memory::default();
        let mut at = W;
        for (round, action) in [LeftHalf, RightHalf, LeftHalf, RightHalf].into_iter().enumerate() {
            // A drag from where it stands: the first notes that place, a
            // later one takes back the size from the snap before it.
            let back = m.dragged(3, at, at, at.x + 20.);
            if round == 0 {
                assert!(back.is_none());
            } else {
                let b = back.unwrap();
                assert_eq!((b.w, b.h), (W.w, W.h), "round {round}: its own size, not the snapped one");
                at = b;
            }
            // Let go over the area; the app moved it a little after, so
            // it stands off where Mado put it, at the size Mado gave.
            let p = Memory::snap_plan(action, &s, 0).unwrap();
            let took = Rect { x: p.frame.x + 30., y: p.frame.y + 40., ..p.frame };
            m.done(3, &p, at, took, false);
            at = took;
        }
        assert_eq!(m.plan(3, Restore, at, &s).unwrap().frame, W, "Restore goes back to where the first drag began");
    }

    #[test]
    fn rows_and_columns_share_the_area_in_the_order_the_windows_stand() {
        let area = Rect::new(0., 25., 1440., 875.);
        let low = Rect::new(100., 600., 400., 200.);
        let high = Rect::new(500., 100., 400., 200.);
        let rows = tile(TileRows, &[low, high], &area);
        assert_eq!(rows, [Rect::new(0., 463., 1440., 437.), Rect::new(0., 25., 1440., 438.)], "the higher window takes the top row");
        let cols = tile(TileColumns, &[high, low, W], &area);
        assert_eq!(cols.iter().map(|r| (r.x, r.w)).collect::<Vec<_>>(), [(960., 480.), (0., 480.), (480., 480.)]);
        // Tiled again, each keeps its cell.
        assert_eq!(tile(TileColumns, &cols, &area), cols);
    }

    #[test]
    fn a_grid_is_near_square_and_fills_the_screen() {
        let area = Rect::new(0., 0., 1200., 800.);
        let w = |x: f64, y: f64| Rect::new(x, y, 300., 200.);
        // Three: two above, one across the whole width below.
        let three = tile(TileGrid, &[w(0., 0.), w(800., 0.), w(0., 500.)], &area);
        assert_eq!(three, [Rect::new(0., 0., 600., 400.), Rect::new(600., 0., 600., 400.), Rect::new(0., 400., 1200., 400.)]);
        // Five: three above, two below.
        let five: Vec<Rect> = (0..5).map(|i| w(i as f64 * 100., i as f64 * 100.)).collect();
        let g = tile(TileGrid, &five, &area);
        assert_eq!(g.iter().filter(|r| r.y == 0.).count(), 3);
        assert_eq!(g.iter().filter(|r| r.y == 400.).map(|r| r.w).collect::<Vec<_>>(), [600., 600.]);
        assert_eq!(tile(TileGrid, &g, &area), g, "tiled again, each keeps its cell");
        // One fills the screen; a tall screen turns the grid.
        assert_eq!(tile(TileGrid, &[W], &area), [area]);
        let tall = Rect::new(0., 0., 800., 1200.);
        let t = tile(TileGrid, &[w(0., 0.), w(0., 800.), w(500., 0.)], &tall);
        assert_eq!(t[2], Rect::new(400., 0., 400., 1200.), "the last column the full height");
        assert!(tile(TileGrid, &[], &area).is_empty());
    }

    #[test]
    fn every_action_has_a_picture_inside_the_screen() {
        for a in Action::ALL {
            let (x, y, w, h) = preview(a);
            assert!(x >= 0. && y >= 0. && w > 0. && h > 0. && x + w <= 1. + 1e-9 && y + h <= 1. + 1e-9, "{}", a.key());
            assert_eq!(preview_cells(a).is_empty(), !a.tiles(), "{}", a.key());
        }
        assert_eq!(preview(RightHalf), (0.5, 0., 0.5, 1.));
    }

    #[test]
    fn only_so_many_windows_are_remembered() {
        let s = one();
        let mut m = Memory::default();
        for id in 0..(REMEMBERED as u64 + 40) {
            let p = m.plan(id, LeftHalf, W, &s).unwrap();
            m.done(id, &p, W, p.frame, true);
        }
        assert_eq!(m.len(), REMEMBERED);
    }
}
