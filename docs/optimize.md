# Optimization

About keeping Mado fast and light: the three aims of a pass, how one is done, and the log that makes each pass start where the last one stopped.

## Rules

- **A pass has three aims:** the best speed; the least use of the computer (CPU, memory, power, wakeups, disk, network, processes); and nothing the app does lost. The third is never traded for the other two.
- **Measured before and after,** with a stopwatch in a copy or a throwaway test, taken out when done (`docs/probing.md`, Measuring). The numbers go in the log.
- **Incremental.** A pass reads the log's last entry and looks only at what changed since the commit it names, plus anything it left for later. With no entry, the pass covers everything.
- **A row's call runs once a row a frame,** and a lookup made at every frame is answered from memory: the app's own writes go into the memory as they go to the disk, and the disk is looked at no more than every half second for another process's write.
- **A path is a name, not a look at the disk.** Whatever the path's root once did to the disk is done once a run.

## A pass

1. Read the log's last entry, or with none, take everything.
2. List what is new since its commit.
3. Find the hot paths: what `render` and each row call, clocks that wake with nothing to move, waits on the main thread, loops and what each turn costs, what grows with use.
4. Time each suspect.
5. Fix the cause where every caller goes through.
6. Time it again.
7. Check nothing is lost: `cargo test -p mado-core -p mado-app`, and a probe copy of each part changed.
8. Write the log entry: version and date, the commits covered, what was looked at, what was found with numbers before and after, what was left as it is and why, what was left for later.

## Log

Newest first.

### v0.1.3, 2026-11-24: the second pass

Covered `16ad847` to `4420fb0`: the settings at 860 wide, every press kept in its own control (the Updates button's states, fixed widths, the mark at Open at login) and the corner masks on Actions.

Looked at: what the changes add to a frame and to the window at rest. Nothing new runs on a timer, reads the disk, waits on the main thread or grows with use; the widths are numbers, not measured at a frame. Measured in dev builds, a frame from the top of `render` to a `canvas` painted last over 300 frames, and at rest with `top`, beside the v0.1.3 code built the same way.

Found: nothing to fix.

- A frame, median and 90th percentile: General 1.15 and 1.86 ms; Actions 1.72 and 2.82 ms with the corner masks, 1.66 and 2.69 ms without; About 0.71 and 1.38 ms with its mask, 0.94 and 1.46 ms without. The masks are within the noise between two runs.
- At rest with the settings open: 0.2 to 0.4% CPU and 0 to 1 idle wakeups a second on every section, the same for the v0.1.3 code. The first pass's 0.1 to 0.3% and none were read another way (a bundle, not a probe copy); measured side by side, nothing changed.

Left for later: what the first pass left, unchanged.

### v0.1.2, 2026-11-24: the first pass

Covered everything, up to `16ad847`.

Looked at: Mado at rest, the settings closed and open; a frame of each section; a press and each event of a drag on the drags' thread; the app in front changing; a shortcut's search of the app's menus for macOS's Move & Resize; the lookups a frame makes; launch, step by step; memory over 90 openings and closings of the settings. Measured in dev builds, a bundle's figures in a copy of the dev bundle.

Found and fixed:

- **A closed window kept its view, its Metal layer and an IOSurface the window's size.** In zed before `0eda7703`, a window's state held AccessKit's adapter, the adapter held the window's content view, and the content view held gpui's view, which held the state: no closed window was freed. Physical footprint after 90 openings and closings of the settings: 2,077 MB before, 58 MB after. A bare gpui window opened and closed five times left five `GPUIView`s and 40 MB of IOSurface before, none after. Fixed upstream in `0eda7703` ("Release the accessibility adapter when a window is dropped"); the pin moved there, the nearest revision with the fix, and gpui-component's inspector was brought to its API (`vendor/gpui-component/UPSTREAM.md`, 64). With the settings open, 49 to 58 MB; 186 MB once before, the old pin's.
- **A bundle decoded the Dock icon at every launch**, though it carries it as its `.icns`: 62 ms of launch, and 32 MB held for the app's life. Physical footprint at rest, the settings never opened: 50 MB before, 11 MB after.
- **Launch listed every installed font family** to learn whether Optima and a Mincho are there: 120 ms for 296 families. Asked for by name (`sys::has_font_family`), 0.8 ms; the same faces are found.
- Launch, from `main` to About's first frame in a bundle: 337 ms before, 166 ms after. The shortcuts are held about 180 ms sooner.

Left as it is:

- At rest: 0.0% CPU and no wakeups with the settings closed, 0.1 to 0.3% and no idle wakeups with them open.
- A frame: 0.4 to 0.9 ms on General and About, 1.0 to 1.9 ms on Actions. Mado's own part is microseconds (the window's language 3.7 µs, read from the system's cache of the defaults; the shortcuts' state 3.6 µs; the screens 16 µs); the rest is gpui's layout and paint.
- The app in front changing builds the menu's lines in 6 µs.
- A press anywhere costs the drags' thread 0.6 ms (the window server's topmost window, 0.2 to 0.3 ms, then its app's windows and its frame), and each drag event 0.04 ms (the window's frame). The window server gives a window's frame without asking its app, and its frames matched Accessibility's on every window shown, but it takes 0.1 ms.

Left for later:

- A shortcut in an app without macOS's Move & Resize searches all its menus again at every press: 4 ms each, 17 ms the first time (Spark Desktop). A miss could be kept until the app next comes to the front.
- Each command's item is searched for on its first use in each app: 2 to 24 ms, and 1 to 68 ms for an app's first, while it builds its menus for Accessibility. One walk could keep all seven.
- During a drag that does not move its window (a text selection), the window's frame is still read at every event, until the button goes up.
- Resident memory rises about 9 MB over the first 15 openings of the settings and about 30 KB an opening after, in malloc; no window is kept.
