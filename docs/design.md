# Design

About `settings.rs`, `widgets.rs`, `menu.rs`, `look.rs`, `fonts.rs`, `assets.rs`, `i18n.rs`, `themes/mado.json`, `locales/words.json`, `assets/` and `scripts/icon/`: everything a person sees of Mado. The bar is the work that wins Awwwards, the Webbys and the FWA: minimal, calm, intuitive, finished in every state. Every change meets it unasked.

Mado has one window, the settings, and an item in the menu bar. What it does happens in other apps' windows, so most of what a person sees of it is a window going where they asked.

## Rules

- **Calm.** One accent, orchid, for what is on, chosen, or the one thing a page waits on (Allow, Update to x). A page of many of the same control (a field a shortcut) draws them quiet.
- **Everything fits.** Every section shows all it has without scrolling at the window's size; the page scrolls only for a window made shorter. A section that would not fit is drawn another way: the 20 actions are a palette of tiles under one hero, not 20 rows, and the snap areas stand side by side, not a row each.
- **Nothing jumps.** What appears, goes, opens, closes, grows, moves, fills or turns does so over a moment, with the pattern its neighbors use (Motion). A jump is a bug. A window Mado moves is the one exception: macOS moves it over a moment where its app has the command, Mado's own settings window is carried over the same moment, and elsewhere a window goes to its place at once (`docs/platform.md`).
- **No pause at the end of a motion.** It runs into its place and stops.
- **Words are few.** A row's name says what it is. A line under it only where the name does not say it all ("When a window goes to another display."), and never repeats what the control shows.
- **One way to draw each thing.** A setting is a row on a card; on or off is a switch; a choice among a few is a segmented control; a list of many drops down from a pill as wide as the widest controls; a few of many on or off are ticks; a button is a pill with an icon before its word; a key is a cap; a shortcut is a field of caps.
- **Things line up.** Names start on one line, controls end on another; every field, button and choice in a row is 28 tall.
- **Every language.** A label never depends on English being short: controls grow to their words. Check a change in French, German and Chinese.
- **Both appearances, every state.** Light and dark are designed together. A change is looked at in both, and in every state it has: allowed and not, asked, recording, refused, at defaults and not, an update waiting.
- **Readable.** Words read on their ground at 4.5 to 1 or better. A key is drawn as a key. Every control is reached by the keyboard.
- **Look before saying it is done** (`docs/probing.md`).

## The palette

The studio's neutrals, in `themes/mado.json`, one config per appearance:

| Role | Light | Dark |
|---|---|---|
| The page | `#FBFBFA` | `#1A1918` |
| Words | `#1F1E1D` | `#F3F1EA` |
| Muted | `#77746B` | `#A09D94` |
| Panel | `#F0EFED` | `#222120` |
| Border | `#DFDFDC` | `#32312E` |

A card is a shade off the page: the muted color at 40% in the light appearance, the sidebar's in the dark. The rail stands on the muted shade at half in the light appearance and the sidebar's in the dark. A color not in the theme file is a mistake.

### The accent

Mado wears **orchid** and has no setting for it: a menu bar app with one window wears one (`look::ACCENT`). Orchid is the studio's hue at 318 in OKLCH, at the studio's lightness for an accent:

| | Base | Hover | Active |
|---|---|---|---|
| Light | `#845293` | `#7E4C8D` | `#724181` |
| Dark | `#C498D2` | `#D1A4DF` | `#B78BC4` |

White is drawn on the light shade, the ink on the dark one. A link is the hover shade in the light appearance and the base in the dark. Orchid's dark shade is the icon's filled pane.

## The type

The chrome is the system's face. The name, on About, is Optima where the system has it, else Belleza (carried, with its license in `assets/fonts/`), the regular drawn twice half a pixel apart for a weight between (`widgets::wordmark`). Its kanji, 窓, stands after it in the system's Mincho at 0.78 of its size, in the accent (`KANJI` in `settings.rs`, `widgets::kanji`); it is on About alone.

Sizes, in points: a section's name 15 semibold; a row's name 13; text in a control 12.5; a group's name 12 semibold in the muted ink; a row's line 11.5 on 15 in the muted ink. Bold is 600.

## Icons in the window

- **Tabler's outline icons** under the names the toolkit asks for, each on one line, stroked at 1.75, in `assets/icons/` with `TABLER-LICENSE`. The rail: `settings` for General, `keyboard` for Actions, `info` for About. The snap areas' pointer (`pointer`, `pointer-halo`) is drawn here, the Mac's arrow, not Tabler's. The menu's Quit is `logout`.
- **Each action has its picture** (`scripts/actions/icons.py`, `assets/icons/actions/`): a small screen 24 by 16, its frame a 1.5 line with round corners, and the part the action puts a window in filled to the frame's inside, a hair apart from its neighbors (a tiling fills each window's part, a little further apart, so each reads as one); Larger, Smaller, Restore and the displays draw a sign in the middle instead; Restore's is the circular arrow of `undo`, the one its plate on the hero and Restore Defaults show, so the three read as one. One color, tinted by the window and by the menu. Rectangle's thumbnails are the model: a person who knows them reads these at once. A tile on Shortcuts and a line of the menu show it.
- **A key is drawn on a cap** (`widgets::key_caps` for a shortcut's keys, `key_text` for a line that names keys, `tip` for a hover's label): plates of one height, a sign (⌘ ⌥ ⇧ ⌃ ↩ ⇥ ← → ↑ ↓ ⌫) as an icon and a letter as a letter. A string with a sign in it is never handed to the window as it is: a row's line that names keys draws them on caps (`line` in `settings.rs`).

## The app icon

The studio's manner (the cream ground with a fifth of the accent in it, navy lines, one amber stroke and the violet spark), drawn in `scripts/icon/icon.html` and rendered by `scripts/make-icon.sh --menubar`.

- **The subject is a window from the front, its left half put in place**: a white sheet in a navy frame, its title bar, a mullion down its middle, and the left pane filled with orchid's dark shade, the half a shortcut put it in. One amber line of content stands in the right pane, with a navy one under it, and the spark in the title bar. Below 32 pixels the amber line and the spark go and the frame thickens.
- **Tried and dropped**: the window in thirds, its first third in orchid. At 32 pixels the three panes read as a table, and a third says less of Mado than a half. It is the page's second `VARIANT`.
- **The menu bar's image** is the window as one black silhouette with its right pane hollowed, so the filled half is the half put in place.

## The window

The settings are Mado's one window, laid out as the studio's settings are: 820 by 600 as it opens, the studio's size, never narrower, down to 460 tall where the page scrolls. The rail of sections at the left under the traffic lights, 190 wide, a row a section 30 tall with an icon and a name, the chosen one on a plate; the section's name 15 semibold in the top band, level with the traffic lights. Opened by the menu bar item, ⌘, and opening Mado again; opened again, it comes forward on the section asked for.

| Section | What it holds |
|---|---|
| General | Accessibility; Mado (Open at login, Show in menu bar, Swap clicks, Appearance). The language's list at the right of its name. |
| Actions | the two ways Mado moves a window, on one page that fits 820 by 600 whole in every language. Shortcuts: Clear All and Restore Defaults at the right of its name, then the hero (the chosen action on a small screen, its name and its shortcut) over the palette (a row a group of actions, a tile an action). Snap Areas under it: its switch at the right of its name, and one card. Second in the rail, next to General: it is what Mado is used by. A section of its own for Snap Areas was tried, and split what reads as one. |
| Snap Areas (on Actions) | the one switch at the right of its name, whether a drag snaps; one card, the areas and macOS tiling: macOS's own tiling of a window dragged to an edge. Its state is a badge at the row's right, before Open System Settings (it opens Desktop & Dock), both 24 tall: Off, as Mado asks, in the success color with a tick, the pill quiet; still on while Mado snaps, "Still on" in the warning color with `triangle-alert`, and the pill primary; on with snapping off, a plain "On". Under the name, in every state, a line says to turn it off to avoid conflicts: off, it says why it should stay off. The warning is amber, not red: Mado still works, a drag is only handled twice. It says "Still on", not "On", since "On" reads as a state that is well. Read again each time the window comes in front; no row before macOS 15. The areas stand side by side, each its picture over its name and what it does (the top edge, the sides, the corners): a small screen, 68 by 42, with its menu bar, the edge or the corner lit in the accent, the Mac's pointer just inside it (black on a white halo in both appearances), and where the window goes as the footprint shows it, a pale plate of the accent inset 4 from the screen's edges. A legend with the area drawn as a bar of the accent over a filled plate was tried, and the sides' and corners' read as a grid. Off, the areas fade to 40% over 200 ms |
| About | the title card, GitHub and Release Notes, Updates and Check automatically |

- **A dot in the accent** on a rail row says something waits there: General while Accessibility is not given, About while a newer version waits. It comes and goes in motion (`Motion::presence`).
- **Accessibility**: given, a quiet "Allowed" with a tick in the success color; not given, the line "Lets Mado move and size other apps' windows." and Allow, primary; after one press, Open System Settings in its place.
- **A shortcut's field** shows its keys on caps at its left and a small × at its right that leaves the action with none, with "Clear" as its hover label. Pressed, its edge takes the accent over a wash of it and it says "Press keys…". What a press says (why it cannot be a shortcut, which action it was taken from, that the system would not hold it) is the row's line.
- **Shortcuts** is every action on one page, so the person sees them together, as the screen positions they are. At the top, the hero: a small screen with its menu bar's strip, and on it a plate in the accent (a wash of it with an edge of it) where the chosen action puts a window, sliding there from the last choice's place over 220 ms; beside it, its name at 15 semibold (it changes where it stands, and the plate's slide is the change's motion: a name faded in from nothing blinked at every choice, and one eased in from below moved the palette under the hero), and its shortcut's field, 220 wide, with what a press says under it. Larger, Smaller, Restore and the displays draw their sign on the plate, and a tiling draws its windows' seams across it in the accent, placed in fractions so they scale as it slides. Under it, the palette, on one card: four rows (Edges, Corners, Size, Displays), each its name at the left, and a tile an action, 50 by 34, with the action's picture. A tile with no shortcut is drawn faint, so what is set reads at a glance; the chosen tile stands on a wash of the accent that comes and goes over 200 ms; a hover names the action with its keys on caps. A press on a tile chooses it, a press on the chosen one records; the arrows move from tile to tile (left and right along the actions, up and down between rows at the same place), and Return records. The hero's screen is 120 by 75; the action's group over its name was dropped, since the palette's row says it. Tried and dropped: a 46-point row an action, which took three pages and a switch to show what one page shows whole; and eight rows with the thirds, fourths, sixths and moves to an edge.
- **Clear All and Restore Defaults** stand at the right of the Shortcuts group's name in that order, small quiet pills: Clear All (`eraser`) leaves every action with no shortcut and is pale with no press while none has one; Restore Defaults (`undo`) is pale with no press while every shortcut is its default.
- **About** is the studio's: "Alkaline Software presents" at 14, the icon at 88, "Mado 窓" at 42 in the name's face and the accent, the version at 12.5 in the muted ink; under the card the pills, then the updates' card.
- **The language** is chosen from a list at the right of General's name: System first, then each by its flag and letters. A choice takes at once: the menus and the menu bar item's lines are named again, and the window fades into the new language.

## The menu bar

- **The item** is at the menu bar's right: the icon's window as a template image, 18 points at 1x and 2x, which the system tints.
- **A click opens the settings** where they were left; a right click, or Control and a click, shows the menu. Swap clicks turns the two around, for a person used to Rectangle, whose click shows the menu; Show in menu bar hides the item. With the item hidden, opening Mado again shows the settings.
- **The menu is Rectangle's, in the system's own menu**: every action with its picture and its shortcut, in the palette's four groups with a line between them; then Ignore and the app in front's name, with a tick while it is ignored; then Settings…, About Mado and Quit Mado. The studio's standard menu (the settings' sections) gives way here: a window manager's menu is where a window is moved by the pointer, and its people know Rectangle's.

## The footprint

Dragged over a snap area, a window's footprint shows where it will go: a veil over every app, nearly clear. The system's blur of what is behind it at part strength (FullScreenUI at 50% in the light appearance, HUDWindow at 35% in the dark, the panel drawn in the system's), a breath of the accent's pale lilac (8% light, 6% dark) and a hairline of the lilac at 1 point (75% light, 45% dark), rounded 12 as a card is. A plate of the accent and then a full frosted glass with a wash of the accent at 14% and an edge in the deep accent were tried, and read heavy and too purple over other apps. First shown it grows from the dragged window's frame to the area's as it fades in, as macOS's own tiling shows where a window will go; it slides to the next area, and fades out when the window is let go or leaves the areas (Motion). It ignores the pointer, so the drag goes on under it.

## Motion

Every motion eases with `ease_out_quint`.

| What | ms |
|---|---|
| The language's list and every list coming in, going out | 140, 110 |
| The footprint growing from the window and fading in, fading out | 280, 160 |
| The footprint sliding to the next area | 220 |
| Mado's own window carried to its place, on macOS tiling's curve (cubic-bezier 0.3, 1, 0.3, 1) | 350 |
| A switch, a dot that comes or goes | 200 |
| A segmented control's plate | 220 |
| A page's content and the section's name fading and rising in | 240 |
| The window fading into a new language | 240 |

- **The first sight plays nothing.** A change is played once, keyed on what changed.
- **Going is drawn from what it last showed.** The list is drawn a moment more as it fades.
- **Left as they are, on purpose**: the hero's name changing with the choice (the plate's slide is the motion), a shortcut's field changing to "Press keys…" and back (the field is the thing being typed into, and a fade would hide the first keys), a hover's ground.

## Languages

The window speaks English, Chinese (Simplified and Traditional), Japanese, Korean, French, German and Spanish, and follows the system by default. Only the window's own words change; Mado's name, the keys and what the system says stay as they are. Each action's name and each refusal comes from the core in English and is drawn through `t`; the test reads both lists. The action names follow the Mac's own words for its window tiling where it has them (Fill, Center).

## Checking

Shoot every state a change touches, in both appearances and in French or German, and look at it whole and close up (`docs/probing.md`). Then go round again until nothing is left to change.
