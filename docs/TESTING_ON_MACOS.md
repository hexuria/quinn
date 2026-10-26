# Testing on macOS

The numbers in [benchmarks.md](benchmarks.md) were taken on Linux x86_64 with X11 and Mesa llvmpipe. macOS was not run. A `cargo check` on Linux is not a macOS window.

This is the checklist for a Mac that has the Xcode command line tools. `rust-toolchain.toml` pins Rust 1.99.0 and the `wasm32-unknown-unknown` target.

Do not export `MAKEPAD_GPU=gl` or `LIBGL_ALWAYS_SOFTWARE=1`. Those force the Linux software-GL path. On macOS, Makepad's windowing backend selects Metal on its own.

## Native examples

From the workspace root:

```bash
cargo run -p counter --release
cargo run -p conditional --release
cargo run -p keyed-list --release
cargo run -p form --release
cargo run -p tailwind-gallery --release
cargo run -p app-shell --release
cargo run -p async-dashboard --release
cargo run -p showcase --release
cargo run -p baseline-counter --release
cargo run -p probe-widget-lifecycle --release
```

What to look at:

- `counter`: the label starts at `Counter: 0`. Each click of Increment adds one. The button sits in the client area, not in the title band.
- `conditional`: toggling the switch removes the detail and brings it back. A remounted detail starts again; it does not keep the old count. The sibling counter is unchanged.
- `keyed-list`: reversing the rows reorders them. Beta's own counter survives the reorder and a removal of a different row.
- `form`: typing in the field updates the echoed line. Clearing the field empties both. This example uses `value=` and `on:input`, which rewrites `TextInput.text` on each render, so the caret may jump. The text should not disappear.
- `tailwind-gallery`: rows, wrap, a grid, a gradient, a shadow, an absolute label, and a short scrolling note. The window scrolls.
- `app-shell`: Home, Board, and Form switch the page and leave the typed name and the accent in place. The board has a grid, a gradient, a shadow, and a note you can scroll. The pinned label sits inside its short parent (`left-2 top-1`), not at a draw-space point. The card is a component with a title prop and children. Form shows a stored city. Move to Cebu changes that field only. The name field is `bind:value`: the echo updates and the caret stays where you clicked.
- `showcase`: a Notes-style window at 1440×900. Library selects Notes, Table, and Settings. The sidebar and inspector fill the window. Theme is a sun/moon toggle. New Note opens a sheet. Delete opens an alert. Table’s Name header reverses the rows; `let:index` stripes odd rows. Settings has a segmented control, a switch, a checkbox, a pop-up, a text field, push buttons, a joined stepper, a slider, and a progress bar. The sidebar is a `GaussRoundedView` tint of the window scene, not the desktop. Search echoes in the inspector; it does not filter the list.
- `async-dashboard`: the note shows "Loading the note", then "Ada's note". The city list shows "Loading cities", then Manila, Cebu, and Davao from `fetch_cities`. The feed line becomes "Could not load the feed". The boom line becomes "The job panicked" (the future panics; the window stays up). Place starts as "Manila desk". Save Cebu shows "Saving", then "Action: Saved Cebu". Save Davao replaces that with "Saved Davao". Race Cebu and then Race Davao, quickly, ends on "Davao won"; the slower Cebu result is dropped. Refetch sets the city to Cebu and the place line refreshes. Theme flips the page between the light and dark window backgrounds. Open modal paints a card over the page; Close removes it. A window narrower than 768px stacks the sidebar. `hover:` on the fill chip turns it system blue. The futures live in `examples/async-dashboard/src/jobs.rs`. Editing that file needs a rebuild. `QUINN_HTTP=1` with `--features net` GETs a public user list and falls back to the three cities if the request fails; without the variable the example never touches the network. The shell calls `quinn::drive_shell` after the widget handles the event. The earlier baked-job take (25 Oct 2026, about 7:45 PM Asia/Manila, pid 95336, starttime 3602412, window `0x2200002`, 325.3 ms and 231.7 ms) is not this binary.
- `baseline-counter`: `Count: 0` and Increment are inside the window body. The label and the button are light text on a dark control. A click changes the count.
- `probe-widget-lifecycle`: the process prints `PROBE_OK` and exits 0. `QUINN_PROBE_EXIT=1` makes a failed check a non-zero exit. The log path is `artifacts/probe-widget-lifecycle.log`.

`QUINN_EXIT=1` asks `counter`, `baseline-counter`, and `retain` to quit after the first presented frame. `QUINN_READY=/tmp/quinn-ready` writes `ready` from `post_draw_hook` at that same moment.

## Retention

```bash
QUINN_RETAIN=2000 QUINN_RETAIN_OUT=/tmp/retain.txt cargo run -p retain --release
```

Do not set `QUINN_EXIT=1`. The probe treats that as "quit after the first frame", and the retain job would not finish. The example quits itself when the report is written.

The file is one line starting `retain cycles=2000 bursts=2 path=script`. At rest (show closed, one row) Linux reported `widgets_rest=4`, `widgets_after=4`, `widgets_after_2=4`, `buttons_rest=1`, `buttons_after=1`, `host_children_rest=2`. The max during a burst was 9 widgets and 3 buttons, on both bursts. `rss_rest_kib` is after `quinn::reclaim_script_heap`. On Linux the release process started at 234,640 KiB and rested at 237,596 KiB, then 237,604 KiB after the second burst. The high water was 237,952 KiB and 238,564 KiB. macOS RSS will differ; the widget and button fields should not. There is no `nodes_after` or `handlers_after` on this path. Those belonged to the old reactive mount. `session_stats()` stays zero because that graph is not running. The unit test below still checks it.

The unit test does not need a window:

```bash
cargo test -p quinn-styles -p quinn-reactive -p quinn-core -p quinn-lower -p quinn --no-default-features
```

`retention_cycles_do_not_accumulate_nodes_or_handlers` is the memory-surface version of the same 2,000 cycles.

## Live reload and latency

```bash
cargo run -p counter --release -- --hot
cargo run -p conditional --release -- --hot
cargo run -p keyed-list --release -- --hot
cargo run -p form --release -- --hot
cargo run -p tailwind-gallery --release -- --hot
cargo run -p app-shell --release -- --hot
cargo run -p async-dashboard --release -- --hot
```

`./target/release/<bin> --hot` is the same loop without Cargo as the parent. Edit `view.rs` in that example, not `src/main.rs`. `view.rs` and `hot/ui.rs` are outside `src/`, so `cargo run` does not rebuild. On Linux, `cargo run -p counter --release -- --hot` kept pid 64535 and starttime 1913679 across five saves of `examples/counter/view.rs`. The log had one `Running` line. Saving `src/main.rs` still rebuilds and starts a new process.

`hot/ui.rs` is generated. Do not edit it. `QUINN_SKIP_LOWER=1` skips the startup rewrite so a hand-patched file survives; that is a debug hatch, not this loop.

What reloads in the same window: classes, static text, the click and input handlers, `<Show>`, `<For>` (including a `<Show>` inside a row), placeholders, component markup, grid classes, and gradient or shadow classes, when the lowerer accepts the file. `mod.state` and `mod.ctx` stay. What does not: a change under `src/`, a `view.rs` the lowerer rejects (the log says `quinn lower failed` and the old script stays), a new font asset in `app_main!` (that is a rebuild), and the baseline counter (edit its `script_mod!` in `main.rs` directly).

Leave the process up and set the state you care about first.

- Counter: click until the label reads `Counter: 5`. Change `gap-4` to `gap-8` and `"Increment"` to `"Add one"` in `examples/counter/view.rs`. The count stays 5. The new button still increments. The log contains `quinn lowered view into .../hot/ui.rs`, `hot reload applied`, and `script_reload pid=...`.
- Keyed list: click Beta a few times. Change the row's classes (`gap-4` to `gap-8`, or `text-lg` to `text-sm`). Beta's count and the order stay.
- Conditional: tick the sibling, leave the detail open, click the detail. Change the detail's heading or its classes. The sibling count, the open toggle, and the detail's clicks stay. A click after the reload still increments.
- Form: type a name. Change `"Hello, "` or `"Clear"` or `gap-4`. The field keeps the text. The heading follows it. This field uses `on:input`, so the caret may jump.
- App shell: open Board, type a name, and leave the accent as it is. Change the `Card` template text, `grid-cols-3` to `grid-cols-2`, and `to-stone-900` to another palette color. The route stays Board, the accent stays, and the typed name stays. `bind:value` does not rewrite the input, so the caret stays where you left it.
- Async dashboard: click Reload so the note says "Loading the note", then edit a static label in `examples/async-dashboard/view.rs` before that request finishes. The new label appears while the line is still loading, and "Note: Ada's note" arrives in that same tree. The future is not started a second time. Changing `src/jobs.rs` does not hot-reload. On Linux, 25 Oct 2026 at 9:44 PM Asia/Manila, pid 101729 starttime 4019518 window `0x2000002` stayed across `"Rail"` → `"Sidebar"` while `fetch_note` gen 3 was in flight. Save-to-visible was 328.1 ms. The note result was the next pixel change, 2428.0 ms after the same `fsync`, with the label already `Sidebar`. The later P3b take, 26 Oct 2026 at 4:45:30 AM Asia/Manila, pid 107429 starttime 4380723 window `0x2000002`, edited `"Sidebar"` → `"Rail"` while `Loading the pair` was showing for `["ada","2"]`. Save-to-visible was 253.5 ms. `Pair: pair ada p2` was the next pixel change, 1952.8 ms after the same `fsync`, and that generation was not cancelled. The X server reuses `0x2000002`; starttime is what identifies the process. Click Race Cebu and Race Davao close together and the label `Cancelled` counts the aborted future. Stamp sends a `Place` struct. `QUINN_HTTP=1` on a `--features net` binary replaces Manila / Cebu / Davao with the fetched user names; leave the variable unset for the offline list.
- Baseline: change `increment_button`'s `draw_bg.color` in `examples/baseline-counter/src/main.rs` and save. The button recolors. The count stays. That file is the script block itself. The log contains `hot reload applied`.

Confirm the process with `starttime` in `/proc/<pid>/stat` as well as the window id. The X server recycles window ids across processes. `xprop -id <id> -set WM_NAME` sets a title the frame will not immediately overwrite; `xdotool set_window --name` did not stick on the Linux run.

`script_eval!` must not sit in `hot/ui.rs`. Makepad counts those bodies and rejects the file when the counts disagree. Rust calls go through `mod.quinn_frame` (and the retain equivalents), which the lowerer assigns because `script_eval!` is a different script body and cannot see the view's local functions. `quinn::run!` does not emit a `script_mod!` the watcher can read.

`QUINN_LIVE=1` is not this path. It injects a synthetic script buffer into the running process and does not modify a file.

| Variable | Effect |
| --- | --- |
| `QUINN_LATENCY=80` | Eighty signal-to-next-frame samples, then quit. Optional `QUINN_LATENCY_OUT`. |
| `QUINN_LIVE=1` | In-process synthetic script reload on the first frame. Not a disk edit. |
| `QUINN_WIDGET_BENCH=200` | Create 200 labels and `set_text` them, seven times unless `QUINN_WIDGET_RUNS` is set. |

A change the lowerer does not understand is a rebuild only if you then edit Rust to express it. Time `cargo build -p counter --release` after touching `examples/counter/src/main.rs` when you are measuring a cold rebuild, not a view reload.

## Wasm

Install Makepad's wrapper from a checkout of `44ecb822` (`tools/cargo_makepad`), then:

```bash
cargo makepad wasm install-toolchain
cargo makepad wasm build --bindgen -p counter
```

Serve `target/makepad-wasm-app/debug/counter` with:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

Open the page, click Increment, and confirm the label reads `Counter: 1`. The first wasm build of this workspace needed `wasm-bindgen-cli` 0.2.129 (`--bindgen`). Without it, instantiation failed on a missing `__wbindgen_placeholder__` import.

## First click and typing

A button’s face and label stay the colors from its class after a click. Hover and press return to that face. They do not stick on the theme’s pale outset.

The first click in an unfocused window is swallowed unless Makepad’s `acceptsFirstMouse` returns `YES` for content. `patches/makepad-accepts-first-mouse.patch` does that. Apply it to the Makepad git checkout (`44ecb822`) and run `cargo clean -p makepad-platform` before the macOS build. Cargo keys the dependency on the git revision, so a dirty checkout is not rebuilt on its own. This repository’s Linux runs do not compile that file, so the patch is not proven here.

Typing with the keyboard, and IME `insertText:`, reaches `TextInput`. An accessibility client that sets `AXValue` or uses XCUITest `typeText` does not: the view does not publish `AXValue` or `AXSelectedText`. `save.version()` on the async dashboard reads 0 before the first Save.
