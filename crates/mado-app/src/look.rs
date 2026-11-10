//! The window's look: which appearance to draw (the system's, or light or
//! dark pinned), the accent, and code's inks.
//!
//! The palette stays in `themes/mado.json`, one config per appearance.
//! The accent is painted over the keys that carry it there (the primary
//! button, the focus ring, links, the selection) in both configs before
//! they are handed to the toolkit, so `Theme::change` keeps it through
//! every appearance switch with nothing to patch afterwards.

use gpui::{App, Window};
use gpui_component::theme::{Theme, ThemeConfig, ThemeMode, ThemeSet};

/// An accent: its base, hover and active shades in each appearance, and
/// the link's shade in each.
pub struct Accent {
    pub light: [&'static str; 3],
    pub dark: [&'static str; 3],
    pub link: [&'static str; 2],
}

/// Mado wears orchid, the studio's accent at hue 318 (`docs/design.md`,
/// The accent), and has no setting for it: a menu bar app with one window
/// wears one. Its dark shade is the icon's pane.
pub const ACCENT: Accent = Accent { light: ["#845293", "#7E4C8D", "#724181"], dark: ["#C498D2", "#D1A4DF", "#B78BC4"], link: ["#7E4C8D", "#C498D2"] };

/// What is drawn on the accent: white on the light appearance's shade,
/// the ink on the dark one's.
const ON_LIGHT: &str = "#FFFFFF";
const ON_DARK: &str = "#1F1E1D";

/// The inks code is written in, one set an appearance: everything that
/// colors code or a terminal takes them from here, by the names the theme
/// file gives them ("@l-blue", "@d-rose"). The hues are Rosé Pine's (pine,
/// iris, gold, rose, foam, love) with a sage green beside them, each
/// taken darker or lighter until it reads at 4.5 to 1 or more on the
/// window's page, the gray for comments a step quieter.
pub struct Inks {
    pub blue: u32,
    pub violet: u32,
    pub gold: u32,
    pub rose: u32,
    pub teal: u32,
    pub green: u32,
    pub red: u32,
    pub gray: u32,
}

pub const INKS_LIGHT: Inks = Inks { blue: 0x286983, violet: 0x7A6398, gold: 0x8A6A1E, rose: 0xA3566F, teal: 0x357783, green: 0x4C7B58, red: 0xB0505F, gray: 0x85818F };
pub const INKS_DARK: Inks = Inks { blue: 0x7FB4D0, violet: 0xC4A7E7, gold: 0xE9C083, rose: 0xE8B0AE, teal: 0x9CCFD8, green: 0xA3CCA4, red: 0xEB8FA6, gray: 0x8F8BA0 };

/// The theme file with each ink's name replaced by its color.
fn inked(json: &str) -> String {
    let mut out = json.to_string();
    for (p, inks) in [("l", &INKS_LIGHT), ("d", &INKS_DARK)] {
        for (name, c) in [("blue", inks.blue), ("violet", inks.violet), ("gold", inks.gold), ("rose", inks.rose), ("teal", inks.teal), ("green", inks.green), ("red", inks.red), ("gray", inks.gray)] {
            out = out.replace(&format!("\"@{p}-{name}\""), &format!("\"#{c:06X}\""));
        }
    }
    out
}

fn paint(cfg: &mut ThemeConfig, a: &Accent) {
    let dark = cfg.mode.is_dark();
    let [base, hover, active] = if dark { a.dark } else { a.light };
    let link = if dark { a.link[1] } else { a.link[0] };
    let on = if dark { ON_DARK } else { ON_LIGHT };
    let c = &mut cfg.colors;
    c.primary = Some(base.into());
    c.primary_foreground = Some(on.into());
    c.sidebar_primary_foreground = Some(on.into());
    c.primary_hover = Some(hover.into());
    c.primary_active = Some(active.into());
    c.ring = Some(base.into());
    c.link = Some(link.into());
    c.link_hover = Some(active.into());
    c.link_active = Some(active.into());
    c.sidebar_primary = Some(base.into());
    c.list_active_border = Some(base.into());
    c.drag_border = Some(base.into());
    c.progress_bar = Some(base.into());
    c.slider_bar = Some(base.into());
    c.drop_target = Some(format!("{base}33").into());
    c.selection = Some(format!("{base}{}", if dark { "66" } else { "59" }).into());
}

/// Hand the toolkit both palettes with the accent painted in. Called once
/// at start; `apply` then draws one.
pub fn install(cx: &mut App) {
    let set: ThemeSet = match serde_json::from_str(&inked(include_str!("../themes/mado.json"))) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mado: the theme file is invalid, using the default look: {e}");
            return;
        }
    };
    let a = &ACCENT;
    let theme = Theme::global_mut(cx);
    for mut cfg in set.themes {
        paint(&mut cfg, a);
        let cfg = std::rc::Rc::new(cfg);
        if cfg.mode.is_dark() {
            theme.dark_theme = cfg;
        } else {
            theme.light_theme = cfg;
        }
    }
}

/// Draw the appearance asked for: the system's for `system`, else the
/// one named. Called at start, at every change of the system's
/// appearance, and from the settings. Never call the toolkit's
/// `sync_system_appearance` elsewhere: a pinned appearance would flip
/// with the system.
pub fn apply(appearance: &str, window: Option<&mut Window>, cx: &mut App) {
    match appearance {
        "light" => Theme::change(ThemeMode::Light, window, cx),
        "dark" => Theme::change(ThemeMode::Dark, window, cx),
        _ => Theme::sync_system_appearance(window, cx),
    }
    // The toolkit draws again only the window it is handed; every window
    // wears the palette.
    cx.refresh_windows();
}
