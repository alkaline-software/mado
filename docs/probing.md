# Probing

How to look at a state of the window from a script, without the person's hands.

## Rules

- Probe with a second copy, never the copy the person is using: `MADO_HOME` at a scratch folder.
- Say what was checked and what was not. A synthetic click does not reach a click handler in a background window, so a picture of the state a click leads to does not show the click works.

## A copy

```
cargo build -p mado-app
MADO_HOME=<scratch>/home MADO_SETTINGS=shortcuts MADO_SHOT=<file.png> ./target/debug/Mado
```

| Variable | What it does |
|---|---|
| `MADO_HOME` | the copy's own settings and state; it also makes the copy a probe's: it holds no shortcut, has no menu bar item and ends no other Mado |
| `MADO_APPEARANCE` | `light` or `dark`, for this launch only; never written back |
| `MADO_LANG` | a language's code (`de`, `zh-Hans`), over the system's |
| `MADO_SETTINGS` | open the settings on that section (`general`, `shortcuts`, `about`; `snap` is `shortcuts`, Actions, where the snap areas are) |
| `MADO_GO` | steps, `;` between: `untrusted` and `asked` (Accessibility drawn as not given, and as after one Allow; no try overrides it), `pick:<action>` (that action chosen on Shortcuts), `record` (the chosen action's shortcut waiting for keys), `wrong` (the same, with why a press cannot be a shortcut), `updatefound` (a newer version waits: About's dot and button; nothing is fetched), `checking`, `uptodate`, `checkfailed`, `downloading`, `installfailed` (each state the Updates button comes to; nothing is fetched), `check` (the button's own press: a real check, fetched), `loginrefused` (Open at login refused by the system), `tiling` (macOS's own tiling drawn as on, whatever the system's is), `footprint` (the footprint over the left of the probe's own window, for a picture of the region with `screencapture -R`), `menu` (the language's list open), `run:<action>` (that action run a second after the settings open, 0.7 s apart; the probe copy is in front, so it moves its own window); any value keeps the window over the others |
| `MADO_SHOT` | a picture of the settings after they are drawn, then quit; `MADO_SHOT_FRAME=1` keeps the system's shadow and rim |

A process may take a picture of its own windows without Screen Recording access, which a terminal running a probe usually lacks.

## A press, checked

A press moves nothing but what it changes (`docs/design.md`, A press moves nothing), and a picture of each state alone does not show that: a row that grew by one line looks finished on its own. Each state a press leads to is shot beside the state before it, at the same section, language and appearance, and compared:

```
MADO_HOME=<scratch>/home MADO_SETTINGS=about MADO_GO=x MADO_SHOT=<dir>/rest.png ./target/debug/Mado
MADO_HOME=<scratch>/home MADO_SETTINGS=about MADO_GO=uptodate MADO_SHOT=<dir>/uptodate.png ./target/debug/Mado
Rscript scripts/press-diff.R <dir>/rest.png <dir>/uptodate.png
```

It prints, for the rail and the page apart, the box in points that holds every pixel that differs. The box must sit inside the control pressed (the Updates button is 224 by 28 at the right of its row), or in the rail for a dot. A box that reaches a row's or a card's edge, or anything under the control, is a press that moved the layout. A state a setting draws (Show in menu bar off, Swap clicks on, Snap Areas off) is shot from a home whose `config.json` says so. Done in English and in German and Japanese, whose words are longest; the widths that hold them were measured with the text system (`window.text_system().shape_line`) over every label each control can show, in every language.

## A corner, checked

A sharp corner is a few pixels at a box's corner and does not show in a picture of the whole window. Each rounded box's corners are cut from the probe's picture and enlarged eight times with no smoothing, in both appearances:

```
Rscript scripts/corners.R <dir>/actions.png <dir>/corners.png hero-tl=239,101 side-bl=489,429
```

Each `name=x,y` is a 20-point square from its top left, in points. A corner is round in the strip: nothing of what is inside the box (a strip, a lit edge, a row) shows square past the curve. Where something draws over every app (the footprint), the screen is pictured with `screencapture -x` while a probe shows it (`MADO_GO=footprint`), and its corners cut out the same way.

## A move, checked

`MADO_GO=run:left-half;run:left-half;run:restore` moves the probe's own settings window, and a debug build writes each move to `~/Library/Logs/mado-dev.log`: the action, the window, its frame before and after, and the frame asked for. This goes through Accessibility, the layout and the memory as a shortcut does, on the main thread (a window of Mado's own, `docs/platform.md`). A move of another app's window, on the background executor, a tiling (it leaves Mado's own windows alone, so a probe would move the person's), a drag to a snap area or away and the menu bar item's menu need the dev bundle with Accessibility given (`scripts/dev-app.sh`) and a person's hands: a probe copy holds no shortcut, watches no drag and has no item. Say which was checked.

Each step moves whatever window is in front when it runs. A probe that loses the front between steps (another app comes forward) moves the person's window: the log names each window's title, and a step that names another app's has moved it.

The settings window is at least 860 wide, so a half on a narrow screen shows the app refusing its size and `settle` keeping it at the screen's edge.

## Measuring

A speed-up starts from a number and ends with another. The stopwatch goes in a copy and comes out when it is done; none is kept.

- **At rest:** `top -pid <pid> -stats cpu,idlew,mem`, with the window closed and open. A product at rest is at about 0% with a few wakeups a second. What is left to find is in what it does at work: go through each path, and read what runs on a timer.
- **A frame:** the time from the top of `render` to a `canvas` painted last. A hover draws the window whole, so a frame asked for by a timer costs what a hover costs. A frame has about 16 ms.
- **A wait outside a frame:** a timer of 5 ms on the main thread that prints any gap over 30 ms. It finds a lock, a sound or a file that no frame shows.
- **A pointer with no hands:** a sweep through `window.dispatch_event` (`PlatformInput::MouseMove`, and a press and a release on each section), so the person's cursor stays where it is.
- **A wait that comes and goes:** the same thing done twice running, and done after a pause of several seconds. A system that has gone quiet takes a moment to wake.
- **A rate:** the app's CPU time and the window server's over the same seconds, doing nothing, as it was, and as it is. Frames cost the window server too, and with no baseline to take off, a saving does not show.
- **Work on the GPU:** a call that hands it work returns before the work is done, and the wait shows up under whatever comes next. Wait for it (`llama_synchronize`) before the clock stops.
- **What grows with use:** rows made up for the purpose at a size the person has and at several times it, never the person's own words.
- **Memory:** the physical footprint (`footprint <pid>`), never the resident size `top` and `ps` give. A window's drawing surfaces are IOSurfaces, which those leave out: a leak of 2 GB read there as 207 MB. `heap <pid>` counts the live objects of each Objective-C class (`GPUIView`, `CAMetalLayer`).
- **A window opened and closed** many times by a task in the copy (`remove_window`, then the settings opened again), the footprint read before and after. A closed window that keeps its surfaces shows only so.
- **Launch:** the time from `main` to the first frame painted, and to each step of start-up between. A bundle's launch differs from a bare binary's (the Dock icon, the app's own defaults), so it is timed in a copy of the dev bundle with the build put in it and signed ad hoc (`codesign -s - --force --deep`).
- **Nothing lost:** what the product gives is compared with the old build's, word for word, kept beside the new one.

## The core from a terminal

```
cargo run -p mado-core -- paths | config | shortcuts | update
cargo run -p mado-core -- place <action> <x,y,w,h> <area x,y,w,h>…
cargo run -p mado-core -- --try-update <version> <copy of Mado.app>
```
