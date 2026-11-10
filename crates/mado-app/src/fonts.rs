//! The faces the window carries, and which face each place uses.
//!
//! The chrome is the system's face and is never set here. The app's name
//! (About's title card) is Optima where the
//! system has it, else Belleza, a free face with the same flared strokes,
//! carried for Windows and Linux. A product whose content is words adds
//! its content faces and its monospace faces here, each a static file a
//! weight under a license that lets an app carry it, with that license
//! beside the files in `assets/fonts/`: handed a variable font, gpui holds
//! one face of it and draws every bold as regular.

use std::borrow::Cow;

use gpui::{App, Global, SharedString};

const FACES: &[&[u8]] = &[include_bytes!("../assets/fonts/Belleza-Regular.ttf")];

/// What the system has of the faces wanted, found once at start: listing
/// every installed face takes about a tenth of a second on a Mac with many
/// fonts, and asked as the window draws it held every frame that long.
struct Found {
    wordmark: SharedString,
    kanji: Option<SharedString>,
}

impl Global for Found {}

/// Load the carried faces and find the system's. Called once at start.
pub fn install(cx: &mut App) {
    let faces = FACES.iter().map(|b| Cow::Borrowed(*b)).collect();
    if let Err(e) = cx.text_system().add_fonts(faces) {
        eprintln!("mado: the carried faces did not load: {e}");
    }
    let names = cx.text_system().all_font_names();
    let has = |face: &str| names.iter().any(|n| n == face);
    let wordmark = if has("Optima") { "Optima".into() } else { "Belleza".into() };
    let kanji = KANJI_FACES.iter().find(|f| has(f)).map(|f| SharedString::from(*f));
    cx.set_global(Found { wordmark, kanji });
}

/// The face a Japanese name's kanji is set in on About: a Mincho, the
/// first the system has. Each system's own and never carried: a face with
/// kanji in it is a large file for two characters.
const KANJI_FACES: &[&str] = &["Hiragino Mincho ProN", "Hiragino Mincho Pro", "Yu Mincho", "YuMincho", "Noto Serif JP", "Noto Serif CJK JP"];

pub fn kanji_family(cx: &App) -> Option<SharedString> {
    cx.global::<Found>().kanji.clone()
}

/// The app's name's face. Optima has only regular and bold, so the
/// wordmark draws its regular twice half a pixel apart for a weight
/// between (`widgets::wordmark`).
pub fn wordmark_family(cx: &App) -> SharedString {
    cx.global::<Found>().wordmark.clone()
}
