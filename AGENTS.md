# AGENTS.md

## What this is

**Mado** (窓, "window") is a macOS menu bar app that moves and sizes windows, Rectangle's way: from the keyboard (⌃⌥← puts the window in front on the left half, ⌃⌥U in the top left corner, ⌃⌥↩ over the whole screen, ⌃⌥⌘→ on the next display), from its menu, and by dragging a window to an edge or a corner of the screen. Every window on a screen can be tiled in rows, columns or a grid; a window Mado placed takes back its size when dragged away; and Restore puts a window back where it was before Mado moved it. Its actions, snap areas and settings follow Rectangle's, read from its source and made smaller (`docs/placing.md`). It is an Alkaline Software product: Rust and GPUI (Zed's toolkit) with the vendored gpui-component, a core crate with no window and an app crate with the window. Only the Mac version is published; the code keeps the studio's Windows and Linux paths, and there Mado moves nothing.

The one idea: **the core decides, the app carries out.** Where every action puts a window, which screen a window is on, what Restore goes back to and what a drag away gives back are pure functions in `mado-core` (`layout.rs`), tested without a Mac. The app reads the screens and the window, asks `Memory::plan`, sets the frame, reads back what the window took and tells `Memory::done`. A change of behavior starts as a core test (`docs/placing.md`).

## The standard

Mado's design is held to the studio's bar: minimal, calm, intuitive, finished in every state. `docs/design.md` is that standard in Mado's terms (its orchid accent, its icon, its one window, the menu bar item, motion, languages), and every change a person can see meets it unasked. Three of its rules hold in every session:

- **Nothing jumps.** What appears, goes, opens, closes, grows or moves does so over a moment, with the pattern its neighbors use (`widgets.rs`, `menu.rs`).
- **Words are few.** A row's name says what it is; a line under it only where the name does not say it all.
- **Look before saying it is done:** at all the change shows, hides or moves, in both appearances and in French or German (`docs/probing.md`).

## Rules that hold everywhere

Each is here because breaking it crashed the app or brought back a fixed bug. The doc in brackets has the story.

- **The main thread never waits on another app.** Every Accessibility question about another app's window runs on the background executor or the drags' own thread, each with a 0.25 s timeout (`sys::limit_waits`); the global mouse monitor only hands events on. (`docs/platform.md`)
- **Mado's own window is moved on the main thread, outside any update** (`sys::place_own`): Accessibility answers the app's own process in that process and sets the frame through AppKit, which off the main thread stops the app, and inside `this.update` leaves the window's old drawing stretched over its new frame. (`docs/platform.md`)
- **The shortcuts are the system's hot keys**, never an event tap: no permission, and no key but the shortcuts is ever seen. They are given back while a shortcut is recorded, so the keys reach the window. (`docs/platform.md`)
- **Whether Accessibility is given is whether a question can be asked** (`sys::trusted`), never `AXIsProcessTrusted` alone, and Allow first forgets the system's old entry. It is asked when the settings open or come back, after Allow and at the system's notices; never on a timer. (`docs/platform.md`)
- **One Mado runs at a time**; a probe copy with its own `MADO_HOME` ends none and holds no shortcut. (`docs/platform.md`)
- **Settings close by `on_window_closed`**, never `on_window_should_close`, which ⌘W never asks. (`docs/platform.md`)
- **One shortcut does one thing.** Given to an action, it is taken from the one that had it, and the row says so (`Config::set_shortcut`). (`docs/placing.md`)
- **Every word a person sees goes through `i18n::t`** (or `tf`) and is in `locales/words.json` in every language, the core's action names and refusals included; `cargo test -p mado-app` fails otherwise. (`docs/design.md`, Languages)
- **Every button is a pill with an icon** (`widgets::choice_pill`, `small_pill`), and every key shown is on caps (`widgets::key_caps`, `key_text`, `tip`).
- **Every icon the code names is carried** in `assets/icons/`; `cargo test -p mado-app` fails otherwise.
- **Every child process starts through `mado_core::process::command`**, never `Command::new`, and every call to the system lives in `sys.rs` or its modules, each with a body for every system.
- **One copy of gpui.** The pin is in `Cargo.lock`, never a `rev` of our own. Changes to the toolkit go in `vendor/gpui-component/`, marked `(Studio addition.)` and listed in its `UPSTREAM.md`. Build from the root, never inside `vendor/`.
- **Nothing personal is in the repository**: no signing identity, no credential, no path under one person's home.

## Before you edit, read

| You are about to touch | Read first |
|---|---|
| Anything a person sees: a row, a control, a color, the settings, the icon, the menu bar item, motion, a language | `docs/design.md` |
| What an action does, where a window goes, a second press, Restore, a snap area, the shortcuts and their config (`mado-core`, `mado.rs`) | `docs/placing.md` |
| A call to macOS: Accessibility, the hot keys, the key recorder, the screens, the drags and the footprint, the menu bar item, the app in front, the Dock, login (`sys.rs`, `sys/`) | `docs/platform.md` |
| Looking at a state from a script, measuring a speed | `docs/probing.md` |
| Making something faster or lighter | `docs/performance.md` |
| A release, a script under `scripts/release-*`, CI | `docs/release.md` |

A subagent handed work on a file is told to read its doc too.

## Layout

```
Cargo.toml                 the workspace and the one version; the zed revision is pinned in Cargo.lock
crates/mado-core/          everything without a window
  src/action.rs              the 20 actions in four groups: key, name, group, default shortcut
  src/layout.rs              where a window goes: plan, the tilings, settle, Restore, a drag away
  src/snap.rs                the snap areas: which action a cursor at an edge or a corner asks for
  src/shortcut.rs            modifiers and a Mac key code; config text "ctrl-alt-left"; display "⌃⌥←"
  src/config.rs              ~/.mado/config.json: shortcuts, snap areas, ignored apps, the menu bar item
  src/paths.rs process.rs    the home folder and atomic writes; child processes
  src/update.rs              the newest release, its disk image, and putting it in place
  src/bin/mado-core.rs       paths | config | shortcuts | place | update | --try-update
  tests/core.rs
crates/mado-app/           the window
  src/main.rs                start-up, the hot keys' and the menu bar's channels, the menus, the probe's variables
  src/mado.rs                the app's one `Mado`: shortcuts, moves, the menu's lines, Ignore, Accessibility, login, updates, probe steps
  src/settings.rs            the settings window: General, Actions (the shortcuts' hero and palette, the snap areas), About
  src/widgets.rs             the shared drawing: choice_pill, key_caps, key_text, tip, page_in, Motion
  src/menu.rs                the app's own list (the language), in and out in motion
  src/look.rs fonts.rs       the appearance and the orchid accent; the faces carried
  src/assets.rs i18n.rs      the icons built in; the window's words in eight languages
  src/sys.rs                 every call to the system, a body for each; the Mac's in sys/
  src/sys/                   ax.rs, place.rs, hotkeys.rs, keys.rs, snap.rs, footprint.rs, status_item.rs
  src/csd.rs ui_state.rs     the window's own buttons where the system draws none; what is remembered
  assets/ themes/ locales/   icons, each action's picture, the app icon, the menu bar image; the palette; the words
  macos/                     the Info.plist additions (LSUIElement, the languages)
vendor/                    gpui-component and block, each with an UPSTREAM.md
scripts/                   dev-app.sh, relaunch.sh, make-icon.sh and icon/, make-readme-gif.sh and readme/, actions/icons.py, dmg/, release-*.sh
docs/                      design.md, placing.md, platform.md, probing.md, performance.md, release.md
```

## Running it

```
cargo build -p mado-app && scripts/relaunch.sh   # quit the running Mado, start the new build as an app
scripts/dev-app.sh                               # start the build as target/debug/dev.noindex/Mado.app
```

The dev build runs as a bundle of its own (`com.alkalinesoftware.mado.dev`), signed with the developer's identity when the keychain has one, so the Accessibility given to it outlives a rebuild. The release (`com.alkalinesoftware.mado`) is a separate entry in Privacy & Security.

One dev build at a time. After a change to the app, build it and run `scripts/relaunch.sh` as the last command; a change to documents alone needs no relaunch. A release ends with nothing compiled left: `scripts/release-clean.sh` deletes `target/` and `dist/` (`docs/release.md`).

## Writing things down

- Each document has one job. This file: what Mado is, the rules every session follows, where to read next. `docs/<part>.md`: how one part works and why, with its traps. `docs/release.md`: how to cut a release. `CHANGELOG.md`: what each release changed, for the person installing it. `docs/probing.md`: how to look at the window from a script. `docs/performance.md`: how Mado is kept fast and light, with the log of every pass. `README.md`: a short introduction for GitHub. Nothing else is kept as a file: a plan or a list of tasks goes stale within weeks.
- A fact goes where its code is: how a part works and its traps in that part's doc, under `## Rules` when breaking it crashes the app or brings back a bug. This file gets a line only for a rule every session follows, or a row in its table. Keep it under 300 lines.
- What is so now and why, not how it got there. An approach tried and dropped gets one sentence, only where someone might try it again.
- Plain words, short declarative sentences, the reason after the rule. No hard wraps: a paragraph or a list item is one line. American spelling. No em dashes. Name the place, not a line number (`docs/design.md`, Motion; `widgets::key_caps`).
- A lesson learned goes in these docs, never in an agent's private memory.

## Testing

```
cargo test -p mado-core -p mado-app
```

The core's tests need no Mac and no window: the layout's are frames in and frames out. `tests/core.rs` points `MADO_HOME` at a temp folder (`isolated()`), so no test reads or writes the person's settings. The app's tests hold the languages' table to the phrases the code and the core ask for, and the icons to the ones the code names. `rust.yml` runs them on every system the studio builds for.
