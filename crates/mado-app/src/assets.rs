//! The pictures the window draws, over the set gpui-component ships.
//!
//! `Icon::new(IconName::…)` and `Icon::default().path(…)` resolve through
//! the app's `AssetSource`, and this one answers first for every file it
//! carries, then hands the rest to the toolkit's own set:
//!
//! - `icons/`: Tabler's outline icons (MIT, `TABLER-LICENSE` beside them),
//!   each under the name the toolkit asks for (`close.svg` is Tabler's
//!   `x`), so the icons the toolkit draws by itself are Tabler's too; the
//!   flags, drawn here; and `menubar.png`, the menu bar item's picture.
//! - `icon/`: the app's own icon at each size `scripts/make-icon.sh` writes.
//!
//! The folders are built in whole, so a file put there is served: no list
//! to keep beside them.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "icons/**"]
#[include = "icon/*.png"]
pub struct Own;

pub struct Assets;

/// A carried file's bytes, for what draws them outside the window (the
/// menu bar item's menu); none for one not carried.
pub fn bytes(path: &str) -> &'static [u8] {
    match Own::get(path).map(|f| f.data) {
        Some(Cow::Borrowed(b)) => b,
        _ => &[],
    }
}

/// An action's picture (`scripts/actions/icons.py`).
pub fn action_icon(a: mado_core::action::Action) -> &'static [u8] {
    bytes(&format!("icons/actions/{}.svg", a.key()))
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(f) = Own::get(path) {
            return Ok(Some(f.data));
        }
        gpui_component_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut v = gpui_component_assets::Assets.list(path)?;
        v.extend(Own::iter().filter(|p| p.starts_with(path)).map(|p| SharedString::from(p.into_owned())));
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every icon the window names by path is carried: one that is not
    /// draws nothing, and nothing says so.
    #[test]
    fn every_named_icon_is_carried() {
        let sources = [include_str!("main.rs"), include_str!("widgets.rs"), include_str!("mado.rs"), include_str!("settings.rs"), include_str!("menu.rs"), include_str!("i18n.rs")];
        let mut missing = Vec::new();
        for src in sources {
            for (at, _) in src.match_indices("\"icons/") {
                let rest = &src[at + 1..];
                let path = &rest[..rest.find('"').unwrap_or(0)];
                // A path made from a pattern is each action's picture, held
                // by `every_action_has_its_picture`.
                if path.ends_with(".svg") && !path.contains('{') && Own::get(path).is_none() {
                    missing.push(path.to_string());
                }
            }
        }
        assert!(missing.is_empty(), "not in assets/: {missing:?}");
    }

    #[test]
    fn every_action_has_its_picture() {
        for a in mado_core::action::Action::ALL {
            assert!(!action_icon(a).is_empty(), "{}: run scripts/actions/icons.py", a.key());
        }
    }

    #[test]
    fn the_app_icon_is_carried() {
        for size in ["icon/icon-128.png", "icon/icon-512.png"] {
            assert!(Own::get(size).is_some(), "{size}: run scripts/make-icon.sh");
        }
    }
}
