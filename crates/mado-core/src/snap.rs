//! Snap areas: where a window dragged to an edge or a corner of a screen
//! goes when it is let go. The app hands the cursor's point and the
//! screens; this says which screen and which action, or none. The margins are
//! Rectangle's: an edge is 5 points deep, a corner 25 points square, and
//! the 145 points of a side next to a corner are that end's half.
//!
//! The areas are fixed: the top edge maximizes, a side gives its half
//! and near a corner the top or bottom half, a corner its quarter. The
//! bottom edge is no area. Only whether a drag snaps is a setting.
//!
//! Points are the Accessibility API's: y goes down from the top of the
//! main screen, so the top edge is at a screen frame's `y`.

use crate::action::Action::{self, *};
use crate::layout::Screen;

/// How deep an edge's area is.
const EDGE: f64 = 5.;
/// A corner's area past the edge's.
const CORNER: f64 = 20.;
/// The part of a side next to a corner that is that end's half.
const SHORT: f64 = 145.;

/// The screen and the action the cursor at (`x`, `y`) asks for.
pub fn at(x: f64, y: f64, screens: &[Screen]) -> Option<(usize, Action)> {
    for (i, s) in screens.iter().enumerate() {
        let f = s.frame;
        if !f.holds(x, y) {
            continue;
        }
        let (left, right) = (x < f.x + EDGE + CORNER, x > f.right() - EDGE - CORNER);
        let (top, bottom) = (y < f.y + EDGE + CORNER, y > f.bottom() - EDGE - CORNER);
        let action = match () {
            _ if left && top => Some(TopLeft),
            _ if right && top => Some(TopRight),
            _ if left && bottom => Some(BottomLeft),
            _ if right && bottom => Some(BottomRight),
            _ if x < f.x + EDGE => Some(side(true, y, s)),
            _ if x > f.right() - EDGE => Some(side(false, y, s)),
            _ if y < f.y + EDGE => Some(Maximize),
            _ => None,
        };
        return action.map(|a| (i, a));
    }
    None
}

/// The left or the right side: its half, and near a corner the top or
/// the bottom half. A portrait screen's side is its top or bottom half,
/// by where along it the cursor is.
fn side(left: bool, y: f64, s: &Screen) -> Action {
    let f = s.frame;
    if y < f.y + EDGE + SHORT {
        return TopHalf;
    }
    if y > f.bottom() - EDGE - SHORT {
        return BottomHalf;
    }
    if f.h > f.w {
        return if y < f.y + f.h / 2. { TopHalf } else { BottomHalf };
    }
    if left {
        LeftHalf
    } else {
        RightHalf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Rect;

    fn screens() -> Vec<Screen> {
        vec![
            Screen { frame: Rect::new(0., 0., 1440., 900.), area: Rect::new(0., 25., 1440., 875.) },
            Screen { frame: Rect::new(1440., 0., 1080., 1920.), area: Rect::new(1440., 0., 1080., 1920.) },
        ]
    }

    #[test]
    fn the_edges_and_corners_of_a_landscape_screen() {
        let s = screens();
        assert_eq!(at(2., 450., &s), Some((0, LeftHalf)));
        assert_eq!(at(1439., 450., &s), Some((0, RightHalf)), "the right edge of the left screen");
        assert_eq!(at(700., 1., &s), Some((0, Maximize)));
        assert_eq!(at(10., 10., &s), Some((0, TopLeft)));
        assert_eq!(at(1430., 890., &s), Some((0, BottomRight)));
        assert_eq!(at(700., 450., &s), None, "the middle is no area");
        assert_eq!(at(2., 100., &s), Some((0, TopHalf)), "a side near a corner is that end's half");
        assert_eq!(at(2., 800., &s), Some((0, BottomHalf)));
        assert_eq!(at(700., 898., &s), None, "the bottom edge is no area");
    }

    #[test]
    fn a_portrait_screen_takes_halves_down_its_sides() {
        let s = screens();
        assert_eq!(at(1442., 600., &s), Some((1, TopHalf)));
        assert_eq!(at(1442., 1300., &s), Some((1, BottomHalf)));
    }
}
