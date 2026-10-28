# Grids

A macOS window manager in Rust: every Rectangle action plus the Rectangle Pro feature set, as a
menu bar app with no Swift runtime resident. Settings is a separate SwiftUI app inside the
bundle that only runs while its window is open.

## Build and install

```
scripts/build.sh            # build/Grids.app (daemon + Settings + test window)
scripts/build.sh --install  # replace /Applications/Grids.app and launch it
```

Needs Rust and the Command Line Tools. Swift builds against the MacOSX26.5 SDK: the CLT ship no
SwiftUI macro plugin for the 27 SDK. The bundle is signed with the first "Apple Development"
identity in your keychain (override with `GRIDS_SIGN`; ad hoc if there is none), so the Accessibility grant survives rebuilds.

On first launch Grids imports Rectangle's (or Rectangle Pro's) shortcuts, gaps, repeat mode
and snap settings. Quit Rectangle so the two don't compete for the same keys.

## Rectangle parity

`python3 scripts/parity.py` audits the catalog against every `WindowAction` raw value in
Rectangle Pro 3.95's shipped binary (255 actions). It exits non-zero if a Pro action is
neither implemented nor one of the eight internal mode/sentinel values. Both Pro's raw
camelCase names and the `Landscape`/`Portrait` variants are accepted by the URL API and the
first-run importer, so a shortcut bound in either Rectangle edition keeps working.

Approximations worth knowing: `restore-to-screen` returns a window to the screen of its
pre-action frame; `reasonable-size` is 80% of the usable area centred; `left/right-todo`
use Pin Mode with the front app; `alternate-left-half` is a left half. Rectangle's cursor
mode actions (`drag`, `resize`, `dial`, `reticle`, `manual`, `winMod`) are not shortcut
actions and are covered by Grids' own Move & Resize and Quick Throw gestures.

## Layout

| Path | What |
| --- | --- |
| `src/actions.rs` | Action catalog and pure geometry for every single-window action (unit tested) |
| `src/engine.rs` | Dispatch, repeats, gaps, history (Restore/Last), displays, Spaces, custom entries |
| `src/multi.rs` | Tiles, cascades, app-wide moves, Reveal Desktop Edge, Tidy |
| `src/snap.rs` | Drag-to-snap edges/corners, snap targets, snap panel, unsnap restore, neighbour resize |
| `src/throw.rs` | Window Throw + Long Throw, Quick Throw, Move & Resize |
| `src/stash.rs`, `src/pin.rs`, `src/layouts.rs` | Stash, Pin Mode, Layouts and their triggers |
| `src/tap.rs`, `src/hotkeys.rs`, `src/touch.rs` | Event taps, Carbon hotkeys, trackpad gesture |
| `src/ax.rs`, `src/screens.rs`, `src/overlay.rs` | Accessibility windows, displays, layer-drawn overlays |
| `src/watchers.rs` | Display/wake/app events, config reload, display-layout restore, Dock adjust |
| `apple/Settings/` | SwiftUI Settings app; edits `~/Library/Application Support/Grids/config.json` |

The config is one JSON document; the daemon writes every key on start and reloads when Settings
posts `app.grids.reload` or when the file changes (a kqueue watch, so iCloud edits from
another Mac land too).

## Staying light

- No timers while idle. Config changes come from a kqueue watch; the per-display window
  snapshot is taken when displays announce a reconfiguration, before sleep, and on app
  switches at most every 30 s.
- Event taps are as narrow as their job. Clicks pass through a filtering tap only when a
  feature can swallow one (title-bar double-click, click-for-menu, throw button); otherwise
  every tap is listen-only and never holds input up. Mouse-moved and scroll are tapped only
  while a throw, move/resize or stash needs them.
- The status menu is built when it opens and emptied after it closes.
- Only the AppKit headers in use are compiled, so CloudKit, CoreData, CoreImage, CoreVideo and
  Metal are not linked. The daemon is built for size (`opt-level = "z"`): action time is
  Accessibility round trips, and measured the same at `opt-level = 3`.
- Settings borrows the host's icon; the icon is quantised (`pngquant`, `oxipng`) and packed
  verbatim by `scripts/pack-icns.py`.

## URL API

```
open -g "grids://execute-action?name=left-half"
open -g "grids://execute-action?name=left-half&window=<CGWindowID>"   # explicit target
open -g "grids://execute-layout?name=Work"
open -g "grids://execute-custom?name=Centre%2060"
open -g "grids://settings?name=layouts"
open -g "grids://debug"     # writes debug.json: windows moved, history, screens, menu probe
```

`rectangle-pro://` URLs work unchanged, including Rectangle Pro's action names.

## Tests

- `cargo test`: action geometry.
- `open -g -n build/Grids.app --args --selftest`: runs every single-window action,
  repeats, Restore/Last, custom entries and Stash against a throwaway window. The result lands
  in `~/Library/Application Support/Grids/selftest.txt`. It must run as Grids.app:
  on macOS 27, a process borrowing the terminal's Accessibility trust gets placeholder windows.
- `tests/live.sh`, `tests/features.sh`: drive the running daemon with synthetic input against
  `build/obj/TestWindow.app`. Hotkey steps run only while the test window is frontmost, and
  cursor steps only where it is the topmost window, because those actions hit whatever window
  is there. Run them when you are not using the Mac. `tests/latency.sh` times keypress to move.

## Measured (Apple M1, macOS 27)

| | Grids | Rectangle 2.0.3 |
| --- | --- | --- |
| App size | 1.6 MB (0.8 MB daemon) | 11 MB |
| Idle memory (phys_footprint / RSS) | 11 MB / 43 MB | 16 MB / 67–70 MB |
| Linked libraries | 12 | (Swift, SwiftUI, Combine, Sparkle…) |
| Idle timers | none | |
| Keypress to window moved | ~37 ms median | ~39 ms median |
| Engine time per action (self-test) | 2.2–9 ms median, by system load | n/a |

An empty AppKit status-item app measures 10 MB footprint on the same machine, so Grids's
own share is about 1 MB. Keypress-to-move time is mostly the window server and the target
app redrawing, so the two tie there.

## Not yet exercised live

The taps were split (listen-only drags and keys, on-demand scroll) after the last live run:
rerun `tests/live.sh` and `tests/features.sh` while away from the Mac.


Covered by code and unit or self tests, but not driven end to end on hardware: trackpad and
mouse-button throw triggers, Next/Previous Space, display connect/disconnect and wake layout
triggers, restoring a display's windows on reconnect, Dock adjustment, iCloud sync, multi-window
commands (tiles, cascades, Tidy, Reveal Desktop Edge), click-for-menu, resizing neighbours after
an edge drag.
