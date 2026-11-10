# Placing

About `mado-core`'s `action.rs`, `layout.rs`, `snap.rs`, `shortcut.rs` and `config.rs`, and the parts of `mado.rs` that carry them out: what each action does, where it puts a window, what Restore and a drag do, and how a shortcut is kept. The behavior is Rectangle's, read from its source and written again here, made smaller: four groups of actions, no settings for how a window sits, and tilings of every window that Rectangle does not have. Where Mado differs, this says so.

## Rules

- **Frames are the Accessibility API's points**: the origin at the main screen's top left, y going down. The screens are flipped into them once (`sys::place::screens`), so a frame read from a window is set back as it is and the core never thinks about AppKit's upward y.
- **Edges are rounded, not sizes** (`Rect::round`). Rounding x and the width each on their own put a bottom half a point past the screen and left a point between two halves.
- **A window counts as Mado's only while it is where Mado put it** (`Rect::near`, two points of slack). The last frame kept is the one read back from the window, not the one asked for: a terminal sizes itself to its cells, and the frame asked for would never match.
- **A move is noted as what it placed** (`Plan::action`): Next Display places the window the same way on the next screen, a half staying a half.
- **An action pressed again does the same thing again**, which leaves the window where it is. A half pressed again stepped to a third and a quarter once (Behavior's Press again for the next size); it gave a window sizes no drag or other action gives, which apps laid out badly, and it is gone, as are Rectangle's thirds, fourths and sixths and its seven repeat modes.
- **A window sits on the area's edges.** Gaps between windows, Maximize again to restore and moving the cursor with a window to another display were Behavior's settings once; they are gone with it, and a config's `windows` is passed over.
- **Restore goes back to where the window was before Mado last found it moved.** A window moved by hand since Mado placed it has that place to go back to, not the one before. A drag that ends in a snap area keeps the place noted as the drag began (`done` with `restore` false).
- **What is remembered is bounded**: 256 windows, the oldest moved let go first. The memory is held for the app's life, and a window long closed would otherwise stay in it.
- **One shortcut does one thing.** `Config::set_shortcut` takes it from the action that had it and says which. Rectangle lets two actions share a shortcut and cycles them; Mado does not, since a row could not say which of the two a press does. A shortcut equal to the action's default is not written, so a new default reaches a person who never changed it; "" is kept as none.

## The actions

`Action::ALL` is the order the settings and the menu show them in, and an action's place in it is the number its hot key is registered under. Each has a key the config names it by, an English name (the key of its other languages), a group, and a default shortcut. The defaults are Rectangle's recommended set, all on ⌃⌥:

| Group | Actions | Defaults |
|---|---|---|
| Edges | Left, Right, Top, Bottom Half; Tile in Rows, Tile in Columns | ← → ↑ ↓; none |
| Corners | Top Left, Top Right, Bottom Left, Bottom Right; Tile in a Grid | U I J K; none |
| Size | Maximize, Almost Maximize, Maximize Height, Larger, Smaller, Center, Restore | ↩, none, ⇧↑, = -, C, ⌫ |
| Displays | Next, Previous Display | ⌘→ ⌘← |

Rectangle's thirds, fourths, sixths, Center Half, Move to Edge, its Spectacle set, its other Extra actions and its Todo mode are not here.

## Where a window goes

`Memory::plan` takes the window's id, the action, the window's frame, and every screen (its frame and the area a window may use), and gives a `Plan`: the frame, the screen, what it is placed as, and whether it went to another screen. None means nothing to do.

- **The screen** is the one holding most of the window, else the nearest (`screen_of`). **The screens' order** for Next Display is in rows from the top, then left to right (`by_place`), Rectangle's.
- **A part of the area** (`part`): the halves, the corners and Maximize. A half is a half of a portrait screen too, as Rectangle has it.
- **Center** keeps the size, made to fit; **Almost Maximize** is nine tenths of the area each way, centered; **Maximize Height** keeps the width and its place across.
- **Larger and Smaller** change the size by 30 points, half at each side. An edge within 5 points of the area's edge stays there, so a window against the left grows to the right. Smaller stops at a quarter of the area each way.
- **Next and Previous Display**: a window Mado placed is placed the same way on the next screen (a left half stays a left half); any other keeps its place in proportion (`carry`). Rectangle keeps a window's size and its insets instead; Mado's way keeps a half a half on a screen of another size.

## Tiled

Tile in Rows, Tile in Columns and Tile in a Grid place every window on one screen at once, not the one in front (`layout::tile`, `sys::place::tile`). Rectangle has rows and columns among its Extra actions; the grid is Mado's.

- **The screen** is the one the window in front is on, else the one under the pointer.
- **The windows** are those the window server shows in the Space in front (`CGWindowListCopyWindowInfo`), each an app's standard window, not minimized and not full screen. Mado's own are left where they are: they move on the main thread alone, and the tiling runs off it.
- **Rows** run down the area, each the full width; **columns** across it, each the full height. **A grid** is as near a square as the count allows, more columns than rows on a landscape screen (the turn of it on a portrait one), and its last row takes what is left across the whole width, so the windows fill the screen: three are two above and one below, five are three and two.
- **Which window goes where** follows where each stands: rows by how high, columns by how far left, a grid in reading order by the band of rows each is in. Tiled again, every window keeps its cell, so a second press changes nothing.
- **Each window is noted as a move**, so Restore puts the one in front back.

## Dragged away

- **A window Mado placed takes back its size when dragged away**, always, with no setting: by a shortcut, the menu, a tiling or a snap, and still where it put it or at the size it gave. A window moved off its place by its app as a snap's drag ended keeps the size Mado gave it; taken for one never placed, it noted its snapped size as the one to go back to, and the next drag away gave that size back. Once the cursor is 30 points from where it pressed (`layout::AWAY`), the window takes the size Restore would give it, shifted so the cursor stays on it (`Memory::dragged`). A nudge shorter than that leaves it placed. Any other window notes where the drag began, as where Restore goes back to.
- **It needs no snap areas**: the drags are watched with snapping off too (`docs/platform.md`, Snap areas). Mado's own settings window takes back its size and snaps the same way, watched by the app itself, and moves over the same moment as any app's (`docs/platform.md`).

## Snap areas

`snap::at` takes the cursor's point and the screens, and gives the screen and the action, or none. The areas are fixed; Snap Areas, on the settings' Actions, has one switch, whether a drag snaps at all.

- **The areas** are Rectangle's: an edge 5 points deep, a corner 25 points square, and the 145 points of a side next to a corner that end's half. The point is tested against each screen's whole frame, its far edges included, in the order the screens come.
- **What each area does** is fixed: the top edge maximizes; a side gives its half, and near a corner the top or the bottom half; a corner its quarter. The bottom edge is no area. Each was a choice once (the top Maximize or Halves, the bottom too, the sides with or without their ends), and a list a row read as clutter; what an older version wrote for them is left alone.
- **A portrait screen** takes its sides as its top and bottom halves.

Rectangle's thirds, fourths and sixths along an edge, its snap modifiers, early snapping, haptics and footprint blur are not here.

## The shortcuts

- **A shortcut** is the Mac's modifier flags and a virtual key code (the US layout's position). The config writes it as text, `ctrl-alt-left`, `ctrl-alt--` for minus.
- **It needs ⌘, ⌥ or ⌃**, or it would take a key from typing in every app. A function key may stand alone.
- **Recording**: a press on a row's field waits for keys (`Mado::start_recording`). The hot keys are given back meanwhile. Esc alone gives up; Delete alone leaves the action with none; a press that cannot be a shortcut says why under the name and keeps waiting.
- **Ignored apps**: while an app Mado leaves alone is in front (the menu's Ignore), no shortcut is held, so its keys reach it, and no drag snaps.

## The config

`~/.mado/config.json` (`MADO_HOME` overrides): `app` (appearance, language, check_updates), `menu_bar` (shown, swap), `snapping` (on), `ignored` (bundle ids) and `shortcuts` (action key to text). Every field has a default. A field no version reads now (`windows`, an older version's gaps and sizes) is passed over as the file loads and left out the next time it is written. `Config::edit` starts from the file, so an override from the environment is never written back.

## From a terminal

```
cargo run -p mado-core -- shortcuts
cargo run -p mado-core -- place left-half 200,200,600,400 0,25,1440,875
```
