//! What a shortcut, a menu line or a drag asks of a window: an edge, a
//! corner, the whole screen, a size, the next display, or every window on
//! the screen in rows, columns or a grid. Each has a key the config names
//! it by, its name in English (the key of its other languages), the group
//! it stands in, and the shortcut it has until the person gives it
//! another.

use crate::shortcut::Shortcut;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    TileRows,
    TileColumns,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    TileGrid,
    Maximize,
    AlmostMaximize,
    MaximizeHeight,
    Larger,
    Smaller,
    Center,
    Restore,
    NextDisplay,
    PreviousDisplay,
}

/// The groups the actions stand in: a row of the settings' palette, a run
/// of lines in the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Edges,
    Corners,
    Size,
    Displays,
}

impl Group {
    /// The groups in the order the settings and the menu show them.
    pub const ALL: [Group; 4] = [Group::Edges, Group::Corners, Group::Size, Group::Displays];

    /// The group's name in English.
    pub fn name(self) -> &'static str {
        match self {
            Group::Edges => "Edges",
            Group::Corners => "Corners",
            Group::Size => "Size",
            Group::Displays => "Displays",
        }
    }
}

use Action::*;

impl Action {
    /// Every action, in the order the settings and the menu show them.
    #[rustfmt::skip]
    pub const ALL: [Action; 20] = [
        LeftHalf, RightHalf, TopHalf, BottomHalf, TileRows, TileColumns,
        TopLeft, TopRight, BottomLeft, BottomRight, TileGrid,
        Maximize, AlmostMaximize, MaximizeHeight, Larger, Smaller, Center, Restore,
        NextDisplay, PreviousDisplay,
    ];

    pub fn key(self) -> &'static str {
        match self {
            LeftHalf => "left-half",
            RightHalf => "right-half",
            TopHalf => "top-half",
            BottomHalf => "bottom-half",
            TileRows => "tile-rows",
            TileColumns => "tile-columns",
            TopLeft => "top-left",
            TopRight => "top-right",
            BottomLeft => "bottom-left",
            BottomRight => "bottom-right",
            TileGrid => "tile-grid",
            Maximize => "maximize",
            AlmostMaximize => "almost-maximize",
            MaximizeHeight => "maximize-height",
            Larger => "larger",
            Smaller => "smaller",
            Center => "center",
            Restore => "restore",
            NextDisplay => "next-display",
            PreviousDisplay => "previous-display",
        }
    }

    pub fn from_key(key: &str) -> Option<Action> {
        Self::ALL.into_iter().find(|a| a.key() == key)
    }

    /// The name the window shows, in English.
    pub fn name(self) -> &'static str {
        match self {
            LeftHalf => "Left Half",
            RightHalf => "Right Half",
            TopHalf => "Top Half",
            BottomHalf => "Bottom Half",
            TileRows => "Tile in Rows",
            TileColumns => "Tile in Columns",
            TopLeft => "Top Left",
            TopRight => "Top Right",
            BottomLeft => "Bottom Left",
            BottomRight => "Bottom Right",
            TileGrid => "Tile in a Grid",
            Maximize => "Maximize",
            AlmostMaximize => "Almost Maximize",
            MaximizeHeight => "Maximize Height",
            Larger => "Larger",
            Smaller => "Smaller",
            Center => "Center",
            Restore => "Restore",
            NextDisplay => "Next Display",
            PreviousDisplay => "Previous Display",
        }
    }

    pub fn group(self) -> Group {
        match self {
            LeftHalf | RightHalf | TopHalf | BottomHalf | TileRows | TileColumns => Group::Edges,
            TopLeft | TopRight | BottomLeft | BottomRight | TileGrid => Group::Corners,
            Maximize | AlmostMaximize | MaximizeHeight | Larger | Smaller | Center | Restore => Group::Size,
            NextDisplay | PreviousDisplay => Group::Displays,
        }
    }

    /// Whether it places every window on the screen, not the one in front.
    pub fn tiles(self) -> bool {
        matches!(self, TileRows | TileColumns | TileGrid)
    }

    /// The shortcut a new install gives it, as the config writes it:
    /// Rectangle's recommended set, all under one hand on ⌃⌥. The arrows
    /// for the edges, U I J K for the corners as they lie on the keyboard.
    /// The tilings have none until the person gives them one.
    pub fn default_text(self) -> &'static str {
        match self {
            LeftHalf => "ctrl-alt-left",
            RightHalf => "ctrl-alt-right",
            TopHalf => "ctrl-alt-up",
            BottomHalf => "ctrl-alt-down",
            TopLeft => "ctrl-alt-u",
            TopRight => "ctrl-alt-i",
            BottomLeft => "ctrl-alt-j",
            BottomRight => "ctrl-alt-k",
            Maximize => "ctrl-alt-enter",
            MaximizeHeight => "ctrl-alt-shift-up",
            Larger => "ctrl-alt-=",
            Smaller => "ctrl-alt--",
            Center => "ctrl-alt-c",
            Restore => "ctrl-alt-backspace",
            NextDisplay => "ctrl-alt-cmd-right",
            PreviousDisplay => "ctrl-alt-cmd-left",
            _ => "",
        }
    }

    pub fn default_shortcut(self) -> Option<Shortcut> {
        Shortcut::from_config(self.default_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_names_one_action() {
        for a in Action::ALL {
            assert_eq!(Action::from_key(a.key()), Some(a));
        }
    }

    #[test]
    fn every_default_parses_and_none_is_taken_twice() {
        let mut seen = std::collections::HashSet::new();
        for a in Action::ALL {
            if a.default_text().is_empty() {
                continue;
            }
            let s = a.default_shortcut().unwrap_or_else(|| panic!("{}", a.key()));
            assert!(seen.insert(s), "{} shares its shortcut", a.key());
        }
    }

    #[test]
    fn the_groups_run_in_order() {
        let order: Vec<Group> = Action::ALL.iter().map(|a| a.group()).collect();
        let mut at = 0;
        for g in Group::ALL {
            while at < order.len() && order[at] == g {
                at += 1;
            }
        }
        assert_eq!(at, order.len(), "an action stands outside its group's run");
    }
}
