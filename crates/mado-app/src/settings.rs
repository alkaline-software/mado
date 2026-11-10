//! The settings window: Mado's one window, as the Mac's own settings are.
//! The rail of sections at its left under the traffic lights; at its
//! right the chosen section's name in the window's top, level with the
//! traffic lights, over its groups, each a card of rows with a name at
//! the left and a control at the right.
//!
//! The window draws `Mado` (`SettingsView` renders `Mado::render_settings`),
//! so every listener on it is the model's and the window can come and go
//! while the shortcuts go on working. Every section fits the window as it
//! opens; the page scrolls only for a window made smaller.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::tooltip::ManagedTooltipExt as _;
use gpui_component::{h_flex, v_flex, ActiveTheme as _, Icon, Sizable as _};

use mado_core::action::{Action, Group};
use mado_core::config::Config;
use mado_core::update;

use crate::i18n::{phrase, t, tf, weave};
use crate::mado::{Escape, Mado, OpenSettings};
use crate::menu::Entry;
use crate::widgets::{choice_pill, kanji, key_caps, page_in, small_pill, swallow_click, tip, wordmark, Press};

/// The window's size as it opens, and the least it is dragged to. It is
/// never narrower than it opens: the rows are laid out for that width,
/// and narrower, names broke mid-word. Shorter, the page scrolls.
pub const SETTINGS_W: Pixels = px(820.);
pub const SETTINGS_H: Pixels = px(600.);
/// About's name in kanji: 窓, a window.
const KANJI: Option<&str> = Some("窓");
/// The product's site, for About's first pill, once it has one.
const SITE: Option<&str> = None;
const SETTINGS_MIN: Size<Pixels> = Size { width: SETTINGS_W, height: px(460.) };
const RAIL_W: Pixels = px(190.);
const PAGE_X: Pixels = px(36.);
/// The window's top, clear for the traffic lights and dragged by.
const TOP_H: Pixels = px(52.);
/// What the widest controls at a row's right take, so those on a page
/// start and end together.
const CONTROL_W: Pixels = px(336.);
const CONTROL_H: Pixels = px(28.);
/// A shortcut's field in the hero: wide enough for four caps and its ×.
const FIELD_W: Pixels = px(220.);
const CARD_R: Pixels = px(12.);
pub const SETTINGS_CONTEXT: &str = "Settings";

/// The sections in the rail's order: key, name, icon. Actions, the two
/// ways Mado moves a window (its shortcuts and its snap areas), is next
/// to General; About is last.
pub const SECTIONS: &[(&str, &str, &str)] = &[
    ("general", phrase("General"), "icons/settings.svg"),
    ("shortcuts", phrase("Actions"), "icons/keyboard.svg"),
    ("about", phrase("About"), "icons/info.svg"),
];

/// A section by its key, the first when there is none of that name.
/// Snap Areas, a section of its own once, is on Actions now; Behavior,
/// gone, is General.
pub fn section_named(key: &str) -> &'static str {
    let key = if key == "snap" { "shortcuts" } else { key };
    SECTIONS.iter().find(|(k, _, _)| *k == key).map_or(SECTIONS[0].0, |(k, _, _)| k)
}

/// The palette's rows: each group of actions, Rectangle's, with its name
/// at the row's start.
fn families() -> Vec<(Group, Vec<Action>)> {
    Group::ALL.into_iter().map(|g| (g, Action::ALL.into_iter().filter(|a| a.group() == g).collect())).collect()
}

/// The hero's screen, its menu bar's strip, and the room around the part
/// a window may take, in points.
const SCREEN_W: f32 = 120.;
const SCREEN_H: f32 = 75.;
const SCREEN_BAR: f32 = 6.;
const SCREEN_PAD: f32 = 5.;
/// A tile of the palette, and the room its rows' names take.
const TILE_W: f32 = 50.;
const TILE_H: f32 = 34.;
const FAMILY_W: f32 = 112.;

/// An action's picture, as the palette draws it.
pub fn action_icon(a: Action) -> SharedString {
    format!("icons/actions/{}.svg", a.key()).into()
}

/// What a snap area's picture shows: the pointer at an edge or a corner,
/// and where the window goes.
#[derive(Clone, Copy)]
enum Snapped {
    Top,
    Side,
    Corner,
}

/// A snap area's picture: a small screen with its menu bar, the edge or
/// the corner the pointer is at lit in the accent, the pointer there,
/// and where the window goes as the footprint shows it, a pale plate
/// of the accent inset from the screen's edges.
fn snap_picture(what: Snapped, theme: &gpui_component::Theme) -> AnyElement {
    const W: f32 = 68.;
    const H: f32 = 42.;
    const BAR: f32 = 5.;
    const INSET: f32 = 4.;
    const LIT: f32 = 2.;
    let (aw, ah) = (W - 2., H - BAR - 2.);
    let half = (aw - INSET * 3.) / 2.;
    let plate = match what {
        Snapped::Top => (INSET, INSET, aw - INSET * 2., ah - INSET * 2.),
        Snapped::Side => (INSET, INSET, half, ah - INSET * 2.),
        Snapped::Corner => (INSET, INSET, half, (ah - INSET * 3.) / 2.),
    };
    // The lit edges, and the pointer's tip, inside the screen under its bar.
    let lit: Vec<(f32, f32, f32, f32)> = match what {
        Snapped::Top => vec![(0., 0., aw, LIT)],
        Snapped::Side => vec![(0., 0., LIT, ah)],
        Snapped::Corner => vec![(0., 0., 16., LIT), (0., 0., LIT, 14.)],
    };
    // Just inside the lit edge, so the pointer leaves it showing.
    let tip = match what {
        Snapped::Top => (aw * 0.62, LIT + 2.),
        Snapped::Side => (LIT + 2., ah * 0.55),
        Snapped::Corner => (LIT + 2., LIT + 2.),
    };
    let at = |d: Div, (x, y, w, h): (f32, f32, f32, f32)| d.absolute().left(px(1. + x)).top(px(BAR + 1. + y)).w(px(w)).h(px(h));
    // The pointer as the Mac draws it, black on a white halo, in both
    // appearances.
    let pointer = |path: &'static str, ink: Hsla| Icon::default().path(path).w(px(11.)).h(px(15.)).text_color(ink);
    let (px_, py_) = (1. + tip.0 - 1.5, BAR + 1. + tip.1 - 1.5);
    div()
        .relative()
        .flex_shrink_0()
        .w(px(W))
        .h(px(H))
        .rounded(px(8.))
        .border_1()
        .border_color(theme.foreground.opacity(0.16))
        .bg(theme.popover)
        .overflow_hidden()
        .child(div().absolute().top_0().left_0().right_0().h(px(BAR)).bg(theme.foreground.opacity(0.06)))
        .child(at(div(), plate).rounded(px(4.)).border_1().border_color(theme.primary.opacity(0.55)).bg(theme.primary.opacity(0.16)))
        .children(lit.into_iter().map(|r| at(div(), r).bg(theme.primary)))
        .child(div().absolute().left(px(px_)).top(px(py_)).child(pointer("icons/pointer-halo.svg", white())))
        .child(div().absolute().left(px(px_)).top(px(py_)).child(pointer("icons/pointer.svg", black())))
        .into_any_element()
}

/// The settings window's root: `Mado`'s settings, drawn in a window.
pub struct SettingsView {
    mado: WeakEntity<Mado>,
    focus: FocusHandle,
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus = self.focus.clone();
        match self.mado.upgrade() {
            Some(m) => m.update(cx, |m, cx| m.render_settings(focus, window, cx)),
            None => div().into_any_element(),
        }
    }
}

/// Show the settings: the window comes forward, or opens, centered as a
/// Mac's settings window opens. Mado is in the Dock while it is open.
pub fn open(mado: WeakEntity<Mado>, cx: &mut App) {
    let Some(m) = mado.upgrade() else { return };
    // In the Dock before the window opens, so it opens in front.
    crate::sys::in_dock(true);
    crate::set_app_menus(cx);
    if let Some(handle) = m.read(cx).settings {
        if handle.update(cx, |_, window, _| window.activate_window()).is_ok() {
            cx.activate(true);
            return;
        }
    }
    let bounds = Bounds::centered(None, size(SETTINGS_W, SETTINGS_H), cx);
    let titlebar = if cfg!(target_os = "macos") {
        TitlebarOptions { title: Some(t("Settings").into()), appears_transparent: true, traffic_light_position: Some(point(px(17.), px(19.))) }
    } else {
        TitlebarOptions { title: Some(t("Settings").into()), appears_transparent: false, traffic_light_position: None }
    };
    let weak = mado.clone();
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(titlebar),
            window_min_size: Some(SETTINGS_MIN),
            app_id: Some("Mado".into()),
            focus: true,
            // The top is the app's to drag by: left to AppKit, a press on
            // a control there moved the window.
            app_owns_titlebar_drag: true,
            ..Default::default()
        },
        move |window, cx| {
            window
                .observe_window_appearance(|window, cx| {
                    let appearance = Config::load().app.appearance;
                    crate::look::apply(&appearance, Some(window), cx);
                })
                .detach();
            crate::look::apply(&Config::load().app.appearance, Some(window), cx);
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            let view = cx.new(|cx| {
                if let Some(m) = weak.upgrade() {
                    cx.observe(&m, |_, _, cx| cx.notify()).detach();
                }
                // Each time the window comes in front: whether Accessibility
                // and launch at login are as they were, which the person may
                // have changed in System Settings meanwhile.
                let watched = weak.clone();
                cx.observe_window_activation(window, move |_, window, cx| {
                    if window.is_window_active() {
                        let _ = watched.update(cx, |m, cx| {
                            m.check_trust(cx);
                            m.check_system_tiling(cx);
                            m.at_login = crate::sys::launches_at_login();
                            cx.notify();
                        });
                    }
                })
                .detach();
                SettingsView { mado: weak.clone(), focus }
            });
            cx.new(|cx| gpui_component::Root::new(view, window, cx))
        },
    );
    match opened {
        Ok(handle) => {
            m.update(cx, |m, cx| {
                m.settings = Some(handle.into());
                m.check_trust(cx);
            });
            let _ = handle.update(cx, |_, window, _| {
                if std::env::var_os("MADO_GO").is_some() {
                    crate::sys::keep_on_top(window);
                }
                window.activate_window();
            });
            cx.activate(true);
        }
        Err(e) => eprintln!("mado: could not open the settings: {e}"),
    }
}

/// A group of the settings: its name over it, what belongs to the whole
/// group at the name's right, and its rows on one card.
fn group(title: Option<SharedString>, aside: Option<AnyElement>, rows: Vec<AnyElement>, theme: &gpui_component::Theme) -> AnyElement {
    let head = (title.is_some() || aside.is_some()).then(|| {
        h_flex()
            .min_h(px(20.))
            // In line with the rows' names and controls under it.
            .px(px(14.))
            .gap(px(12.))
            .items_center()
            .justify_between()
            .child(div().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).text_color(theme.muted_foreground).children(title))
            .children(aside)
    });
    v_flex().gap(px(6.)).children(head).child(card(rows, theme)).into_any_element()
}

/// The ground of a group: a card a shade off the page, with a hairline
/// between its rows.
fn card(rows: Vec<AnyElement>, theme: &gpui_component::Theme) -> Div {
    v_flex().w_full().rounded(CARD_R).border_1().border_color(theme.border).bg(card_ground(theme)).children(rows.into_iter().enumerate().map(|(i, row)| div().when(i > 0, |d| d.border_t_1().border_color(theme.border)).child(row)))
}

/// A card's ground: the muted shade thinned in the light appearance, the
/// sidebar's in the dark.
fn card_ground(theme: &gpui_component::Theme) -> Hsla {
    if theme.mode.is_dark() { theme.sidebar } else { theme.muted.opacity(0.4) }
}

/// A row of a card: its name, under it a line only where the name does
/// not say it all, and its control at the right. Every row is made here,
/// never laid out by hand.
fn line(title: SharedString, detail: Option<SharedString>, control: AnyElement, theme: &gpui_component::Theme) -> AnyElement {
    h_flex()
        .min_h(px(46.))
        .px(px(14.))
        .py(px(8.))
        .gap(px(16.))
        .items_center()
        .child(v_flex().flex_1().min_w_0().gap(px(1.)).child(div().text_size(px(13.)).child(title)).children(detail.map(|d| {
            // A line that names keys draws them on caps.
            let keys = d.chars().any(|c| matches!(c, '⌘' | '⌥' | '⌃' | '⇧'));
            div().text_size(px(11.5)).line_height(px(15.)).text_color(theme.muted_foreground).map(|el| if keys { el.child(crate::widgets::key_text(&d)) } else { el.child(d) })
        })))
        .child(div().flex_shrink_0().child(control))
        .into_any_element()
}

impl Mado {
    pub(crate) fn render_settings(&mut self, focus: FocusHandle, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        // The native window's ground is the page's, or it shows near white
        // under a dark page while the window is drawn or resized.
        if self.ground.get() != Some(theme.popover) {
            self.ground.set(Some(theme.popover));
            crate::sys::set_window_ground(window, theme.popover.to_rgb());
        }
        let section = self.settings_section;
        let rows = match section {
            "about" => self.about_rows(cx),
            "shortcuts" => self.shortcut_page(cx),
            _ => self.general_rows(cx),
        };
        let title = SECTIONS.iter().find(|(k, _, _)| *k == section).map(|(_, name, _)| t(name)).unwrap_or(t("General"));
        // What belongs to the whole page stands at its name's right: the
        // language on General.
        let aside = match section {
            "general" => Some(self.language_pill(cx)),
            _ => None,
        };
        let pick = Rc::new(|this: &mut Self, what: &'static str, id: SharedString, window: &mut Window, cx: &mut Context<Self>| this.settings_pick(what, id, window, cx));
        let close = Rc::new(|this: &mut Self, cx: &mut Context<Self>| {
            this.menus.close(cx);
            cx.notify();
        });
        let menu = if self.menus.open.is_some() || self.menus.gone.is_some() { self.menus.render(pick, close, cx) } else { None };
        div()
            .id("settings-window")
            .key_context(SETTINGS_CONTEXT)
            .track_focus(&focus)
            .size_full()
            .relative()
            .bg(theme.popover)
            .text_color(theme.foreground)
            .text_size(px(13.))
            // Escape puts away a list open here, and with none open closes
            // the window, as ⌘W does. While a shortcut is recorded the
            // keys go to the recording (`sys::record_keys`).
            .on_action(cx.listener(|this, _: &Escape, window, cx| {
                if this.menus.open.is_some() {
                    this.menus.close(cx);
                    cx.notify();
                } else {
                    window.remove_window();
                }
            }))
            .on_action(cx.listener(|_, _: &crate::CloseWindow, window, _| window.remove_window()))
            // On Shortcuts the arrows go from tile to tile, and Return
            // records the one chosen.
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if this.settings_section == "shortcuts" && this.recording.is_none() && this.menus.open.is_none() && ev.keystroke.modifiers.number_of_modifiers() == 0 && this.step_tile(&ev.keystroke.key, cx) {
                    cx.stop_propagation();
                }
            }))
            .on_action(cx.listener(|_, _: &OpenSettings, window, _| window.activate_window()))
            .child(
                h_flex()
                    .size_full()
                    .items_stretch()
                    // A new language is faded into, rail and page alike.
                    .with_animation(ElementId::NamedInteger("settings-lang".into(), self.lang_serial as u64), Animation::new(Duration::from_millis(240)).with_easing(ease_out_quint()), |d, t| d.opacity(t))
                    .child(self.settings_rail(cx))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .child(
                                drag_region(h_flex().h(TOP_H).flex_shrink_0().w_full().pl(PAGE_X).pr(PAGE_X + crate::csd::room(window, cx).1).items_center())
                                    .child(page_in(format!("title-{section}"), div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).child(title)))
                                    .when_some(aside, |d, a| d.child(div().flex_1()).child(page_in(format!("aside-{section}"), div().child(a)))),
                            )
                            // One pane: the settings have no gesture to route.
                            .child({
                                v_flex()
                                    .relative()
                                    .flex_1()
                                    .min_h_0()
                                    .child(v_flex().id("settings-body").flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.settings_scroll).px(PAGE_X).pt(px(2.)).pb(px(16.)).child(page_in(format!("page-{section}"), v_flex().gap(px(18.)).children(rows))))
                                    .vertical_scrollbar(&self.settings_scroll)
                            }),
                    ),
            )
            .children(menu)
            .children(crate::csd::chrome(window, TOP_H / 2. - px(12.), |window, _| window.remove_window(), cx))
            .into_any_element()
    }

    /// The rail of sections, under the traffic lights: an icon and a name
    /// a section, the chosen one on a plate. A section with something
    /// waiting wears a dot in the accent: General while Accessibility is
    /// not given, About while a newer version waits. Its empty room drags
    /// the window.
    fn settings_rail(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let current = self.settings_section;
        let waiting = |key: &str| match key {
            "general" => !self.trusted,
            "about" => self.update.found.is_some(),
            _ => false,
        };
        let dots: Vec<(&str, Option<(bool, u32)>, bool)> = ["general", "about"].iter().map(|k| (*k, self.motion.presence(&format!("dot-{k}"), waiting(k), cx), waiting(k))).collect();
        drag_region(v_flex().w(RAIL_W).h_full().flex_shrink_0().pt(TOP_H).px(px(10.)).gap(px(2.)).bg(if theme.mode.is_dark() { theme.sidebar } else { theme.muted.opacity(0.5) }).border_r_1().border_color(theme.border))
            .children(SECTIONS.iter().map(|(key, label, icon)| {
                let active = *key == current;
                let key: &'static str = key;
                let dot = dots.iter().find(|(k, _, _)| *k == key).and_then(|(_, moving, on)| {
                    (*on || moving.is_some()).then(|| {
                        let d = div().size(px(7.)).rounded_full().bg(theme.primary);
                        match moving {
                            Some((coming, n)) => {
                                let coming = *coming;
                                d.with_animation(ElementId::Name(format!("dot-{key}-{n}").into()), Animation::new(crate::widgets::FOLD_ANIM).with_easing(ease_out_quint()), move |d, t| d.opacity(if coming { t } else { 1. - t })).into_any_element()
                            }
                            None => d.into_any_element(),
                        }
                    })
                });
                h_flex()
                    .id(SharedString::from(format!("settings-nav-{key}")))
                    .h(px(30.))
                    .px(px(10.))
                    .gap(px(9.))
                    .items_center()
                    .rounded(px(8.))
                    .cursor_pointer()
                    .when(active, |d| d.bg(theme.sidebar_accent))
                    .when(!active, |d| d.hover(|s| s.bg(theme.sidebar_accent.opacity(0.6))))
                    // A press on a section is not a drag of the window.
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.settings_section != key {
                            this.settings_section = key;
                            this.settings_scroll.set_offset(point(px(0.), px(0.)));
                            this.menus.close(cx);
                            this.stop_recording(cx);
                            this.record_said = None;
                            let mut ui = crate::ui_state::UiState::load();
                            ui.settings_section = Some(key.to_string());
                            ui.save();
                        }
                        cx.notify();
                    }))
                    .child(Icon::default().path(*icon).with_size(px(15.)).text_color(if active { theme.foreground } else { theme.muted_foreground }))
                    .child(div().flex_1().text_size(px(13.)).when(active, |d| d.font_weight(FontWeight::MEDIUM)).text_color(if active { theme.foreground } else { theme.sidebar_foreground }).child(t(label)))
                    .children(dot)
            }))
            .into_any_element()
    }

    fn general_rows(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = cx.theme().clone();
        // Accessibility: given, said so; not given, Allow, the system's own
        // question, and once asked, a way to System Settings.
        let access = if self.trusted {
            h_flex().h(CONTROL_H).px(px(12.)).gap(px(6.)).items_center().rounded_full().bg(theme.success.opacity(0.1)).text_size(px(12.5)).text_color(theme.success).child(Icon::default().path("icons/check.svg").with_size(px(14.))).child(t("Allowed")).into_any_element()
        } else {
            let (icon, word) = if self.asked_trust { ("icons/external-link.svg", t("Open System Settings")) } else { ("icons/shield.svg", t("Allow")) };
            choice_pill("allow", icon, word, Press::Primary, false, &theme)
                .on_click(cx.listener(|this, _, window, cx| {
                    swallow_click(window, cx);
                    this.ask_trust(cx);
                }))
                .into_any_element()
        };
        let access_line = (!self.trusted).then(|| SharedString::from(t("Lets Mado move and size other apps’ windows.")));
        let login = self.motion.switch("login", self.at_login, Rc::new(|this: &mut Self, on, _, cx| this.set_launch_at_login(on, cx)), cx);
        let (shown, swapped) = (self.cfg.menu_bar.shown, self.cfg.menu_bar.swap);
        let in_bar = self.motion.switch("menu-bar", shown, Rc::new(move |this: &mut Self, on, _, cx| this.set_menu_bar(on, swapped, cx)), cx);
        let swap = self.motion.switch("swap", swapped, Rc::new(move |this: &mut Self, on, _, cx| this.set_menu_bar(shown, on, cx)), cx);
        let looks = div().w(CONTROL_W).child(self.motion.segmented(
            "appearance",
            vec![("system", "icons/device-desktop.svg", t("System").into()), ("light", "icons/sun.svg", t("Light").into()), ("dark", "icons/moon.svg", t("Dark").into())],
            match self.cfg.app.appearance.as_str() {
                "light" => "light",
                "dark" => "dark",
                _ => "system",
            },
            true,
            Rc::new(|this: &mut Self, key, window, cx| {
                this.cfg.app.appearance = key.to_string();
                let _ = Config::edit(|c| c.app.appearance = key.to_string());
                crate::look::apply(key, Some(window), cx);
                cx.notify();
            }),
            cx,
        ));
        let swap_line = if swapped { t("A click shows the menu; a right click, the settings.") } else { t("A click opens the settings; a right click, the menu.") };
        vec![
            group(None, None, vec![line(t("Accessibility").into(), access_line, access, &theme)], &theme),
            group(
                // The app's name stays as it is in every language.
                Some("Mado".into()),
                None,
                vec![
                    line(t("Open at login").into(), self.login_said.clone().map(SharedString::from), login, &theme),
                    line(t("Show in menu bar").into(), (!shown).then(|| SharedString::from(t("Open Mado again to see the settings."))), in_bar, &theme),
                    line(t("Swap clicks").into(), Some(swap_line.into()), swap, &theme),
                    line(t("Appearance").into(), None, looks.into_any_element(), &theme),
                ],
                &theme,
            ),
        ]
    }

    /// Snap Areas, on Actions under the palette: the switch at the right
    /// of its name, and one card, the three areas side by side, each its
    /// picture over its name and what it does, and macOS's own tiling. The areas are fixed
    /// (`snap::at`). Off, the areas fade to 40% over a moment.
    fn snap_group(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let live = self.cfg.snapping.on;
        let on = self.motion.switch("snapping", live, Rc::new(|this: &mut Self, on, _, cx| this.edit(move |c| c.snapping.on = on, cx)), cx);
        let fade = self.motion.presence("snap-areas", live, cx);
        let area = |what: Snapped, name: &'static str, does: &'static str| {
            v_flex()
                .flex_1()
                .min_w_0()
                .items_center()
                .gap(px(6.))
                .child(snap_picture(what, &theme))
                .child(v_flex().items_center().gap(px(1.)).child(div().text_size(px(13.)).text_center().child(t(name))).child(div().text_size(px(11.5)).line_height(px(15.)).text_center().text_color(theme.muted_foreground).child(t(does))))
        };
        let areas = h_flex()
            .px(px(14.))
            .pt(px(10.))
            .pb(px(8.))
            .gap(px(14.))
            .items_start()
            .child(area(Snapped::Top, phrase("Top edge"), phrase("Maximize")))
            .child(area(Snapped::Side, phrase("Left and right edges"), phrase("Halves; near a corner, the top or bottom half")))
            .child(area(Snapped::Corner, phrase("Corners"), phrase("A quarter of the screen")));
        const DIM: f32 = 0.4;
        let areas = match fade {
            Some((coming, n)) => areas.with_animation(ElementId::Name(format!("snap-areas-{n}").into()), Animation::new(crate::widgets::FOLD_ANIM).with_easing(ease_out_quint()), move |d, t| d.opacity(if coming { DIM + (1. - DIM) * t } else { 1. - (1. - DIM) * t })).into_any_element(),
            None => areas.when(!live, |d| d.opacity(DIM)).into_any_element(),
        };
        let mut rows = vec![areas];
        rows.extend(self.system_tiling_row(cx));
        group(Some(t("Snap Areas").into()), Some(on), rows, &theme)
    }

    /// macOS's own tiling, which takes the same drags: whether it is on,
    /// as a badge at the row's right, and a way to Desktop & Dock, where it
    /// is turned off, under a line that says to keep it off, whatever its
    /// state. Off is as Mado asks, a badge in the success color; on while
    /// Mado snaps too is not, a badge in the warning color, and the way
    /// there the primary pill. Amber, not red: Mado still works, each drag is only
    /// handled twice. On with snapping off, nothing meets, and the badge is
    /// plain. None where macOS has no such tiling.
    fn system_tiling_row(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let theme = cx.theme().clone();
        let on = self.system_tiling?;
        let clash = on && self.cfg.snapping.on;
        let (icon, word, ink, ground) = match (on, clash) {
            (false, _) => (Some("icons/check.svg"), t("Off"), theme.success, theme.success.opacity(0.1)),
            // Said as what is not yet done, not as a state that reads well.
            (true, true) => (Some("icons/triangle-alert.svg"), t("Still on"), theme.warning, theme.warning.opacity(0.12)),
            (true, false) => (None, t("On"), theme.muted_foreground, theme.foreground.opacity(0.06)),
        };
        let badge = h_flex().h(px(24.)).px(px(10.)).gap(px(5.)).items_center().rounded_full().bg(ground).text_size(px(12.)).text_color(ink).children(icon.map(|i| Icon::default().path(i).with_size(px(13.)))).child(word);
        let open = choice_pill("desktop-settings", "icons/external-link.svg", t("Open System Settings"), if clash { Press::Primary } else { Press::Quiet }, false, &theme)
            // The badge's size, so the two stand as a pair.
            .h(px(24.))
            .px(px(10.))
            .gap(px(5.))
            .text_size(px(12.))
            .on_click(|_, window, cx| {
                swallow_click(window, cx);
                crate::sys::open(crate::sys::DESKTOP_SETTINGS);
            });
        // Said in every state: off, it is why it should stay off.
        let said = Some(t("To avoid conflicts, turn off macOS tiling.").into());
        Some(line(t("macOS tiling").into(), said, h_flex().gap(px(10.)).items_center().child(badge).child(open).into_any_element(), &theme))
    }

    /// Actions: every action on one page, then the snap areas. Under Shortcuts' name, with clearing every shortcut and
    /// putting every one back at its right, a hero shows the chosen
    /// action on a small screen, its plate sliding to where the action
    /// puts a window, beside its name and its shortcut; under it the
    /// palette, a row a group of actions, a tile an action. A tile with no
    /// shortcut is drawn faint, so what is set reads at a glance.
    fn shortcut_page(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = cx.theme().clone();
        let hero = self.hero(cx);
        let rows: Vec<AnyElement> = families()
            .into_iter()
            .map(|(g, actions)| {
                h_flex()
                    .h(px(TILE_H))
                    .items_center()
                    .child(div().w(px(FAMILY_W)).flex_shrink_0().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).text_color(theme.muted_foreground).child(t(g.name())))
                    .child(h_flex().gap(px(6.)).children(actions.into_iter().map(|a| self.tile(a, cx))))
                    .into_any_element()
            })
            .collect();
        let palette = v_flex().w_full().rounded(CARD_R).border_1().border_color(theme.border).bg(card_ground(&theme)).px(px(14.)).py(px(6.)).children(rows);
        let pills = h_flex().gap(px(8.)).child(self.clear_pill(cx)).child(self.reset_pill(cx)).into_any_element();
        let head = h_flex().min_h(px(20.)).px(px(14.)).gap(px(12.)).items_center().justify_between().child(div().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).text_color(theme.muted_foreground).child(t("Shortcuts"))).child(pills);
        let shortcuts = v_flex().gap(px(6.)).child(head).child(hero).child(palette).into_any_element();
        vec![shortcuts, self.snap_group(cx)]
    }

    /// The chosen action, large: a small screen with its menu bar, the
    /// action's plate sliding from the last choice's place to this one's,
    /// and its name and its shortcut's field beside it.
    fn hero(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let action = self.picked;
        let (from, serial) = self.picked_from;
        let (aw, ah) = (SCREEN_W - SCREEN_PAD * 2., SCREEN_H - SCREEN_BAR - SCREEN_PAD * 2.);
        let place = move |a: Action| {
            let (x, y, w, h) = mado_core::layout::preview(a);
            (SCREEN_PAD + aw * x as f32, SCREEN_BAR + SCREEN_PAD + ah * y as f32, aw * w as f32, ah * h as f32)
        };
        let (to_r, from_r) = (place(action), place(from));
        let sign = match action {
            Action::Larger => Some("icons/plus.svg"),
            Action::Smaller => Some("icons/minus.svg"),
            Action::Restore => Some("icons/undo.svg"),
            Action::NextDisplay => Some("icons/chevron-right.svg"),
            Action::PreviousDisplay => Some("icons/chevron-left.svg"),
            _ => None,
        };
        // A tiling's windows, drawn as lines across the plate where they
        // meet, placed in fractions so they scale with it as it slides.
        let cells = mado_core::layout::preview_cells(action);
        let mut seams: Vec<(bool, f32, f32, f32)> = Vec::new();
        for (x, y, w, h) in &cells {
            if *x > 0. {
                seams.push((true, *x as f32, *y as f32, *h as f32));
            }
            if *y > 0. {
                seams.push((false, *y as f32, *x as f32, *w as f32));
            }
        }
        let plate = div()
            .absolute()
            .rounded(px(5.))
            .border_1()
            .border_color(theme.primary)
            .bg(theme.primary.opacity(0.22))
            .flex()
            .items_center()
            .justify_center()
            .children(sign.map(|p| Icon::default().path(p).with_size(px(16.)).text_color(theme.primary)))
            .children(seams.into_iter().map(|(across, at, from, len)| {
                let seam = div().absolute().bg(theme.primary);
                if across {
                    seam.left(relative(at)).top(relative(from)).h(relative(len)).w(px(1.))
                } else {
                    seam.top(relative(at)).left(relative(from)).w(relative(len)).h(px(1.))
                }
            }));
        let at = |d: Div, r: (f32, f32, f32, f32)| d.left(px(r.0)).top(px(r.1)).w(px(r.2)).h(px(r.3));
        let plate = if serial == 0 {
            at(plate, to_r).into_any_element()
        } else {
            plate
                .with_animation(ElementId::NamedInteger("hero-plate".into(), serial as u64), Animation::new(crate::widgets::SLIDE).with_easing(ease_out_quint()), move |d, t| {
                    let l = |a: f32, b: f32| a + (b - a) * t;
                    d.left(px(l(from_r.0, to_r.0))).top(px(l(from_r.1, to_r.1))).w(px(l(from_r.2, to_r.2))).h(px(l(from_r.3, to_r.3)))
                })
                .into_any_element()
        };
        let screen = div()
            .relative()
            .flex_shrink_0()
            .w(px(SCREEN_W))
            .h(px(SCREEN_H))
            .rounded(px(10.))
            .border_1()
            .border_color(theme.foreground.opacity(0.18))
            .bg(theme.popover)
            .overflow_hidden()
            .child(div().absolute().top_0().left_0().right_0().h(px(SCREEN_BAR)).bg(theme.foreground.opacity(0.06)))
            .child(plate);
        let said = self.record_said.as_ref().filter(|(a, _)| *a == action).map(|(_, s)| s.clone());
        let refused = self.refused.contains(&action).then(|| t("The system did not take this shortcut.").to_string());
        let line_under = said.or(refused);
        let field = self.shortcut_field(action, cx);
        let words = v_flex()
            .flex_1()
            .min_w_0()
            .gap(px(4.))
            // The name changes where it stands: the plate's slide is the
            // change's motion. Faded in from
            // nothing, the name blinked at every choice; eased in from
            // below, it moved the palette under the hero.
            .child(div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).child(t(action.name())))
            .child(div().pt(px(4.)).child(field))
            .child(
                div()
                    .min_h(px(15.))
                    .text_size(px(11.5))
                    .line_height(px(15.))
                    .text_color(theme.muted_foreground)
                    .map(|d| match line_under {
                        Some(l) if l.chars().any(|c| matches!(c, '⌘' | '⌥' | '⌃' | '⇧')) => d.child(crate::widgets::key_text(&l)),
                        Some(l) => d.child(l),
                        None => d,
                    }),
            );
        h_flex().w_full().gap(px(20.)).px(px(14.)).py(px(12.)).items_center().rounded(CARD_R).border_1().border_color(theme.border).bg(card_ground(&theme)).child(screen).child(words).into_any_element()
    }

    /// A tile of the palette: the action's picture, faint while it has no
    /// shortcut; chosen, on a wash of the accent that comes in over a
    /// moment. A press chooses it, and a press on the chosen one records.
    fn tile(&mut self, action: Action, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let chosen = self.picked == action;
        let shortcut = self.cfg.shortcut(action);
        let moving = self.motion.presence(&format!("tile-{}", action.key()), chosen, cx);
        let ground = (chosen || moving.is_some()).then(|| {
            let g = div().absolute().inset_0().rounded(px(8.)).bg(theme.primary.opacity(0.16)).border_1().border_color(theme.primary.opacity(0.7));
            match moving {
                Some((coming, n)) => g.with_animation(ElementId::Name(format!("tile-{}-{n}", action.key()).into()), Animation::new(crate::widgets::FOLD_ANIM).with_easing(ease_out_quint()), move |d, t| d.opacity(if coming { t } else { 1. - t })).into_any_element(),
                None => g.into_any_element(),
            }
        });
        let ink = if chosen {
            theme.primary
        } else if shortcut.is_some() {
            theme.foreground.opacity(0.85)
        } else {
            theme.muted_foreground.opacity(0.5)
        };
        let label: SharedString = match shortcut {
            Some(s) => format!("{} ({s})", t(action.name())).into(),
            None => t(action.name()).to_string().into(),
        };
        let hover = theme.foreground.opacity(0.06);
        div()
            .id(SharedString::from(format!("tile-{}", action.key())))
            .relative()
            .w(px(TILE_W))
            .h(px(TILE_H))
            .flex_shrink_0()
            .rounded(px(8.))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .managed_tooltip(move |window, cx| tip(label.clone(), window, cx))
            .on_click(cx.listener(move |this, _, window, cx| {
                swallow_click(window, cx);
                if this.picked == action {
                    if this.recording == Some(action) {
                        this.stop_recording(cx);
                    } else {
                        this.start_recording(action, cx);
                    }
                } else {
                    this.pick(action, cx);
                }
            }))
            .children(ground)
            .child(Icon::default().path(action_icon(action)).w(px(24.)).h(px(16.)).text_color(ink))
            .into_any_element()
    }

    /// An action's shortcut on caps in a field: a press records new keys,
    /// and the × at its end leaves the action with none.
    fn shortcut_field(&mut self, action: Action, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let recording = self.recording == Some(action);
        let shortcut = self.cfg.shortcut(action);
        let key = action.key();
        let inner: AnyElement = if recording {
            div().text_color(theme.muted_foreground).child(t("Press keys…")).into_any_element()
        } else {
            match shortcut {
                Some(s) => key_caps(&s.keys()).into_any_element(),
                None => div().text_color(theme.muted_foreground).child(t("None")).into_any_element(),
            }
        };
        let clear = (shortcut.is_some() && !recording).then(|| {
            let hover = theme.foreground.opacity(0.08);
            div()
                .id(SharedString::from(format!("clear-{key}")))
                .size(px(18.))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(hover))
                .managed_tooltip(|window, cx| tip(t("Clear"), window, cx))
                .on_click(cx.listener(move |this, _, window, cx| {
                    swallow_click(window, cx);
                    this.stop_recording(cx);
                    this.set_shortcut(action, None);
                    cx.notify();
                }))
                .child(Icon::default().path("icons/close.svg").with_size(px(12.)).text_color(theme.muted_foreground))
        });
        h_flex()
            .id(SharedString::from(format!("shortcut-{key}")))
            .w(FIELD_W)
            .h(CONTROL_H)
            .pl(px(10.))
            .pr(px(5.))
            .gap(px(6.))
            .items_center()
            .rounded(px(8.))
            .border_1()
            .border_color(if recording { theme.primary } else { theme.foreground.opacity(0.16) })
            .when(recording, |d| d.bg(theme.primary.opacity(0.06)))
            .text_size(px(12.5))
            .cursor_pointer()
            .hover(|s| s.border_color(theme.foreground.opacity(0.28)))
            .on_click(cx.listener(move |this, _, window, cx| {
                swallow_click(window, cx);
                if this.recording == Some(action) {
                    this.stop_recording(cx);
                } else {
                    this.start_recording(action, cx);
                }
            }))
            .child(div().flex_1().min_w_0().child(inner))
            .children(clear)
            .into_any_element()
    }

    /// The chosen tile, moved by an arrow: along the actions' order, or to
    /// the row above or below at the same place. Return or Space records
    /// it. Whether the key was one of these.
    fn step_tile(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let rows = families();
        let (r, c) = rows.iter().enumerate().find_map(|(r, (_, acts))| acts.iter().position(|a| *a == self.picked).map(|c| (r, c))).unwrap_or((0, 0));
        let all = Action::ALL;
        let at = all.iter().position(|a| *a == self.picked).unwrap_or(0);
        let next = match key {
            "left" => all[(at + all.len() - 1) % all.len()],
            "right" => all[(at + 1) % all.len()],
            "up" | "down" => {
                let r = if key == "up" { (r + rows.len() - 1) % rows.len() } else { (r + 1) % rows.len() };
                let acts = &rows[r].1;
                acts[c.min(acts.len() - 1)]
            }
            "enter" | "space" => {
                self.start_recording(self.picked, cx);
                return true;
            }
            _ => return false,
        };
        self.pick(next, cx);
        true
    }

    /// Every shortcut cleared, at the left of Restore Defaults: pale while
    /// no action has one.
    fn clear_pill(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let off = self.cfg.shortcuts_are_none();
        small_pill("clear-shortcuts", "icons/eraser.svg", t("Clear All"), Press::Quiet, off, &theme)
            .h(CONTROL_H)
            // A press on it is not a drag of the window's top.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .when(!off, |d| {
                d.on_click(cx.listener(|this, _, window, cx| {
                    swallow_click(window, cx);
                    this.stop_recording(cx);
                    this.clear_shortcuts(cx);
                }))
            })
            .into_any_element()
    }

    /// Every shortcut back to its default, at the right of Shortcuts'
    /// name: pale while they all are.
    fn reset_pill(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let off = self.cfg.shortcuts_are_defaults();
        small_pill("reset-shortcuts", "icons/undo.svg", t("Restore Defaults"), Press::Quiet, off, &theme)
            .h(CONTROL_H)
            // A press on it is not a drag of the window's top.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .when(!off, |d| {
                d.on_click(cx.listener(|this, _, window, cx| {
                    swallow_click(window, cx);
                    this.stop_recording(cx);
                    this.reset_shortcuts(cx);
                }))
            })
            .into_any_element()
    }

    /// About, the same in every product: the title card, set as a film's
    /// first one is, the pills to the source and the notes, and the updates.
    fn about_rows(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = cx.theme().clone();
        let hover = theme.link;
        let studio = div()
            .id("studio-link")
            .cursor_pointer()
            .text_color(theme.foreground)
            .text_decoration_1()
            .text_decoration_color(theme.foreground.opacity(0.35))
            .hover(move |s| s.text_color(hover).text_decoration_color(hover))
            .on_click(|_, _, _| crate::sys::open("https://github.com/alkaline-software"))
            .child("Alkaline Software")
            .into_any_element();
        let title = v_flex()
            .w_full()
            .items_center()
            .pt(px(30.))
            .pb(px(28.))
            .child(weave(&t("{studio} presents"), vec![("studio", studio)], h_flex().text_size(px(14.)).text_color(theme.muted_foreground).gap(px(0.))))
            .child(img("icon/icon-512.png").mt(px(18.)).size(px(88.)))
            .child(h_flex().mt(px(6.)).items_center().child(wordmark("Mado", 42., theme.primary, cx)).when_some(KANJI, |d, k| d.child(kanji(k, 42., theme.primary, cx))))
            .child(div().mt(px(4.)).text_size(px(12.5)).text_color(theme.muted_foreground).child(tf("Version {version}", &[("version", &update::current_version())])));
        let pills = h_flex()
            .w_full()
            .justify_center()
            .pt(px(4.))
            .gap(px(8.))
            .when_some(SITE, |d, url| d.child(small_pill("about-site", "icons/globe.svg", t("Website"), Press::Quiet, false, &theme).h(px(28.)).on_click(move |_, _, _| crate::sys::open(url))))
            .child(small_pill("about-source", "icons/github.svg", "GitHub", Press::Quiet, false, &theme).h(px(28.)).on_click(|_, _, _| crate::sys::open(&update::repo_page())))
            .child(small_pill("about-notes", "icons/file-text.svg", t("Release Notes"), Press::Quiet, false, &theme).h(px(28.)).on_click(|_, _, _| crate::sys::open(&update::release_page(update::current_version()))));
        let u = self.update.clone();
        let (icon, words, kind) = match (&u.found, u.downloading) {
            (_, Some(p)) => ("icons/download.svg", tf("Downloading… {percent}%", &[("percent", &p)]), Press::Primary),
            (Some(v), None) => ("icons/download.svg", tf("Update to {version}", &[("version", v)]), Press::Primary),
            (None, None) => ("icons/refresh.svg", t("Check for updates").to_string(), Press::Quiet),
        };
        let busy = u.checking || u.downloading.is_some();
        let button = choice_pill("update", icon, words, kind, busy, &theme).min_w(px(190.)).justify_center().when(!busy, |d| {
            d.on_click(cx.listener(|this, _, _, cx| {
                if this.update.found.is_some() {
                    this.install_update(cx);
                } else {
                    this.check_updates(cx);
                }
            }))
        });
        let said = u.said.clone().map(SharedString::from);
        let check = self.motion.switch(
            "check-updates",
            self.cfg.app.check_updates,
            Rc::new(|this: &mut Self, on, _, cx| {
                this.cfg.app.check_updates = on;
                let _ = Config::edit(|c| c.app.check_updates = on);
                cx.notify();
            }),
            cx,
        );
        let mut rows = vec![group(None, None, vec![title.into_any_element()], &theme), pills.into_any_element()];
        if !update::from_store() {
            rows.push(div().w_full().pt(px(12.)).child(group(None, None, vec![line(t("Updates").into(), said, button.into_any_element(), &theme), line(t("Check automatically").into(), None, check, &theme)], &theme)).into_any_element());
        }
        rows
    }

    /// The window's language, at the right of General's name: the
    /// system's, or a language by its flag and letters, in a list that
    /// drops down.
    fn language_pill(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let chosen = crate::i18n::chosen(&self.cfg.app.language);
        let (icon, label, picture) = match crate::i18n::CHOICES.iter().find(|(c, _, _)| *c == chosen) {
            Some((_, flag, letters)) => (*flag, letters.to_string(), true),
            None => ("icons/globe.svg", t("System").to_string(), false),
        };
        h_flex()
            .id("language-pill")
            .h(CONTROL_H)
            .px(px(12.))
            .gap(px(7.))
            .items_center()
            .rounded_full()
            .border_1()
            .border_color(theme.foreground.opacity(0.16))
            .text_size(px(12.5))
            .cursor_pointer()
            .hover(|s| s.bg(theme.foreground.opacity(0.06)))
            // A press on it is not a drag of the window's top.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                let at = ev.position() + point(px(-60.), px(16.));
                let chosen = crate::i18n::chosen(&this.cfg.app.language);
                let mut entries = vec![Entry::Choice { id: "system".into(), icon: "icons/globe.svg", label: t("System").into(), picture: false, chosen: chosen == "system" }, Entry::Rule];
                entries.extend(crate::i18n::CHOICES.iter().map(|(code, flag, letters)| Entry::Choice { id: (*code).into(), icon: flag, label: (*letters).into(), picture: true, chosen: chosen == *code }));
                this.menus.show("language", at, entries);
                cx.notify();
            }))
            .map(|d| if picture { d.child(img(icon).w(px(17.)).h(px(12.))) } else { d.child(Icon::default().path(icon).with_size(px(14.))) })
            .child(label)
            .child(Icon::default().path("icons/chevron-down.svg").with_size(px(12.)).text_color(theme.muted_foreground))
            .into_any_element()
    }

    fn settings_pick(&mut self, what: &'static str, id: SharedString, _window: &mut Window, cx: &mut Context<Self>) {
        if what == "language" {
            let code = id.to_string();
            self.cfg.app.language = code.clone();
            let _ = Config::edit(|c| c.app.language = code.clone());
            // On a Mac the app's own `AppleLanguages` is the setting, which
            // System Settings writes too.
            crate::sys::set_own_language(if code == "system" { None } else { crate::i18n::LANGS.iter().find(|(c, _, _)| *c == code).map(|(_, _, bundle)| *bundle).or(Some("en")) });
            crate::i18n::set(&code);
            crate::set_app_menus(cx);
            self.sync();
            self.lang_serial += 1;
        }
        self.menus.close(cx);
        cx.refresh_windows();
        cx.notify();
    }
}

/// The window's top and the rail's empty room drag the window.
fn drag_region(el: Div) -> Div {
    el.on_mouse_down(MouseButton::Left, |ev, window, _| {
        if ev.click_count == 2 {
            window.titlebar_double_click();
        }
    })
    .on_mouse_move(|ev, window, cx| {
        if ev.pressed_button == Some(MouseButton::Left) {
            window.start_window_move();
            // Dragged away from where Mado put it, it takes back its size.
            crate::mado::get(cx).update(cx, |m, cx| m.own_drag_began(cx));
        }
    })
}
