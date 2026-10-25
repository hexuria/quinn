# Benchmarks

Host: Linux x86_64, kernel 6.12.94, rustc 1.99.0 (`b940084d7`), 4 cores, XFCE on X11 display `:1`, Mesa llvmpipe (LLVM 20.1.2), OpenGL ES 3.2. Every native run set `MAKEPAD_GPU=gl` and `LIBGL_ALWAYS_SOFTWARE=1`. Release profile is Cargo's default `optimized` (no extra LTO). Makepad is `44ecb822`.

Both apps write `QUINN_READY` from `Cx::post_draw_hook`. On X11 that hook runs at the end of `call_draw_event`, after the frame's draw lists are recorded and before `eglSwapBuffers` in the same paint. Startup, latency, and live-reload timings share that marker. A log line from GL init is not the ready point.

## Clean release build

Three clean release builds of each example. Each log starts by compiling Makepad. Cargo's own `Finished` line:

| Run | `counter` | `baseline-counter` |
| --- | --- | --- |
| 1 | 1m 19s | 1m 11s |
| 2 | 1m 14s | 1m 10s |
| 3 | 1m 15s | 1m 10s |
| Median | 1m 15s | 1m 10s |

Quinn links `quinn-styles`, `quinn-reactive`, `quinn-core`, `quinn-macros`, `quinn-makepad`, and `quinn` on top of the same Makepad graph. That is the few extra seconds.

## Incremental release rebuild

`touch` on `examples/<app>/src/main.rs`, then `cargo build -p <app> --release`, with Makepad already built. Wall clock of the cargo process, three runs:

| App | Samples | Median |
| --- | --- | --- |
| `counter` | 1.25s, 1.30s, 1.32s | 1.30s |
| `baseline-counter` | 1.38s, 1.39s, 1.40s | 1.39s |

One rebuild of `counter` followed by a fresh process reaching `QUINN_READY` took 1.31s + 0.129s = 1.44s.

## Binary size

Release binaries after the incremental rebuilds above. `strip` was applied to a copy.

| Binary | Unstripped | Stripped |
| --- | --- | --- |
| `counter` | 27,628,056 bytes | 20,502,896 bytes |
| `baseline-counter` | 27,089,328 bytes | 20,178,256 bytes |

## Startup to first presented frame

Nine release processes each. Time is spawn to the `QUINN_READY` file appearing. Every run exited 0 after `QUINN_EXIT=1`.

| App | Min | Median | Max |
| --- | --- | --- | --- |
| `counter` | 136.2 ms | 147.3 ms | 167.7 ms |
| `baseline-counter` | 120.3 ms | 123.2 ms | 132.2 ms |

## Idle, 3 seconds after ready

One window, sleeping (`state S`), sampled from `/proc/<pid>/stat` against the aggregate `/proc/stat` line. RSS is `VmRSS` and did not move during the window.

| App | Process ticks | Machine ticks | CPU | RSS |
| --- | --- | --- | --- | --- |
| `counter` | 2 | 1,198 | 0.17% | 232,116 KiB |
| `baseline-counter` | 2 | 1,195 | 0.17% | 211,136 KiB |

A tick is 10 ms of scheduled time, so this window only resolves CPU to about 0.08 percentage points. Both apps used two ticks.

## Signal to the next presented frame

`QUINN_LATENCY=80`. `t0` is inside the previous frame's `post_draw_hook`, immediately before the update. `t1` is entry to the next `post_draw_hook`, after the updated frame was recorded and before that frame's swap. Quinn writes the counter signal and flushes. The baseline runs `script_eval!` to increment `mod.state.counter`.

| App | n | min | p50 | p95 | max |
| --- | --- | --- | --- | --- | --- |
| `counter` | 80 | 1.11 ms | 1.58 ms | 3.28 ms | 87.5 ms |
| `baseline-counter` | 80 | 0.43 ms | 0.74 ms | 2.95 ms | 63.2 ms |

The max is the first sample on both apps (llvmpipe warming the window). Later Quinn samples sit between 1.1 ms and 4.6 ms.

## Widget create and text update

In-process, release, 200 labels, 7 runs. Create is `ScriptVm::eval` plus insert. Update is `set_text` on those labels. `examples/counter` and `examples/baseline-counter` both call the same Makepad widget path for this bench (`QUINN_WIDGET_BENCH=200`).

| App | Create min / median / max | Update min / median / max |
| --- | --- | --- |
| `counter` | 24.1 / 26.7 / 29.4 ms | 46 / 48 / 75 µs |
| `baseline-counter` | 22.2 / 26.8 / 32.2 ms | 42 / 44 / 87 µs |

Per label, the median create is about 134 µs on both.

## Retention

`quinn-core`'s `retention_cycles_do_not_accumulate_nodes_or_handlers` runs 2,000 conditional mount/unmount and keyed add/remove cycles on `MemorySurface`. Node count, alive widgets, and click handlers return to the settled baseline. Cleanups run. That test is the headless Rust graph. It is not what `examples/retain` does now.

The native run is `examples/retain` with `QUINN_RETAIN=N`, release, script path. Each cycle calls `mod.quinn_retain_step`, which toggles `state.open` and pushes or pops one keyed row, then `ui.host.render()`. One step per draw, because `render()` is applied by the script task pump at the end of the event. Two bursts of N cycles run back to back. The census is `census_host` on the next draw: widgets under the host, buttons, and the host's direct children. `session_stats()` stays empty. That struct counts the Rust runtime, which this example does not mount. `quinn::reclaim_script_heap` runs before each resting RSS sample.

Taken 25 Oct 2026, about 12:50 PM Asia/Manila, `./target/release/retain`, `QUINN_RETAIN=2000`, `QUINN_RETAIN_OUT` set, no `QUINN_EXIT` (that flag quits on the first frame and would cancel the job). The process exited 0. No script errors.

| Cycles per burst | RSS before | High water, burst 1 | Rest | High water, burst 2 | Rest | Widgets rest / max / after | Buttons rest / max / after | Host children |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2,000 | 234,640 KiB | 237,952 KiB | 237,596 KiB | 238,564 KiB | 237,604 KiB | 4 / 9 / 4, and the same on burst 2 | 1 / 3 / 1, and the same on burst 2 | 2 → 2 → 2 |

Rest is the closed show plus one row (heading, row view, span, button). The max is that row, a second row, and the mounted detail. The second burst does not raise the widget or button counts. RSS climbs about 3 MiB on the first burst and the second rest is 8 KiB above the first rest.

The table below is the previous reactive mount, kept so the 400 MiB spike is not mistaken for this path. That run called `ScriptVm::eval` per widget and `flush_now`. Do not compare its node or handler columns to the script census.

| Cycles per burst | RSS before | High water, burst 1 | Rest | High water, burst 2 | Rest | Nodes / widgets at rest | Handlers |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 500 | 116,984 KiB | 219,764 KiB | 120,552 KiB | 221,392 KiB | 121,536 KiB | 7 → 7, max 12 | 1 → 1 → 1 |
| 2,000 | 116,920 KiB | 527,848 KiB | 129,468 KiB | 532,456 KiB | 133,308 KiB | 7 → 7, max 12 | 1 → 1 → 1 |

## Live script reload

Two different things were easy to mix up.

`QUINN_LIVE=1` does not watch the disk and does not restart the process. On the first presented frame it writes a synthetic `script_mod` buffer with `queue_file_change` and calls `request_style_reload`. Earlier timings of that path, from the queue to the next `post_draw_hook`, were 78–85 ms on the counter (signal still `"7"`) and 74–81 ms on the baseline (label still `"Count: 7"`). The screen recording that looked like a destroyed window was not that path. It started `baseline-counter`, quit it, then started `counter`. Two processes, two windows, cut together.

Makepad's real desktop path is the `--hot` flag. `app_main!` then starts the file watcher (`hot reload watching N script_mod source files`). A write to a watched Rust file is read back, matched against compiled `script_mod!` blocks, and applied in the same process.

The watcher rejects a file when the number of compiled script bodies for that path differs from the number of `script_mod!` blocks in the file. `script_eval!` bodies count. With those evals in `examples/baseline-counter/src/main.rs` the log was `runtime has 3, file has 1` and the pixels did not change, while the PID and X window stayed. Those evals now live in `script_ops.rs`, so `main.rs` has one block. `quinn::run!` evaluates its shell from a string, which the extractor cannot see.

The color rows below are from that `ScriptReapply` follow-up. `request_style_reload()` makes the next tick re-apply with `Apply::ScriptReapply`. Draw colors update. Text the widget already holds stays. Changing `"Count: "` to `"Total: "` did not change the on-screen string. Changing `draw_bg.color` did. The examples no longer do this. `quinn-lower` writes `view.rs` into `hot/ui.rs`, and a reload calls `render()` under `Apply::Reload`, so markup, classes, and handlers update. `mod.state` keeps the count, the keyed map, the toggle, and the typed string.

Five disk edits in one process each, timed from `write()` returning to a frame that contains the new red (`#xff3300`). Historical. The files they name (`chrome.rs`, the color edit in `main.rs`) are not the current view path:

| App | What changed | PID / X id held | Visible recolor |
| --- | --- | --- | --- |
| `baseline-counter --hot` | button `draw_bg.color` in `main.rs` | pid 49286, starttime 1166678, window `0x1c00002` for all five | 89, 119, 136, 136, 195 ms (median 136) |
| `counter --hot` | page `draw_bg.color` in `chrome.rs` (that file is gone; this row is the earlier color-only run) | pid 48220, starttime 1155946, window `0x1c00002` for all five | 72, 79, 85, 100, 111 ms (median 85) |

The baseline samples scanned the whole window for red pixels, so that median includes a few milliseconds of polling. The counter samples read one pixel. A separate single take of each app, with the title bar set to the PID and window id, kept both across the edit: baseline pid 49423 / `0x1e00002`, counter pid 49597 / `0x1e00002`. Both still showed 5 after the recolor. `/proc/<pid>/stat` starttime did not move, so the process was not replaced.

### View reload, save to a changed frame

25 Oct 2026, about 12:53–1:02 PM Asia/Manila. Release binaries, `MAKEPAD_GPU=gl`, `LIBGL_ALWAYS_SOFTWARE=1`, display `:1`, Mesa llvmpipe. The clock starts when `write()` of `view.rs` returns and stops on the first 30 fps raw x11grab frame whose pixels differ. Five forward edits per example, one process each. `starttime` is field 22 of `/proc/<pid>/stat` and did not change. Window id `0x1c00002` was reused by the X server across processes; the pid is what stayed. The counter was `cargo run -p counter --release -- --hot` (one `Running` line in the log). The other three were `./target/release/<name> --hot`.

| Example | Edit in `view.rs` | What stayed | PID / starttime | Samples (ms) | Median |
| --- | --- | --- | --- | --- | --- |
| counter | `gap-4` → `gap-8`, `Increment` → `Add one` | count 5, then a click made 6 | 64535 / 1913679 | 262.6, 295.5, 263.6, 296.2, 263.4 | 263.6 |
| keyed-list | row `gap-4` → `gap-8`, `text-lg` → `text-sm` | Beta at 2, order Alpha, Beta, Gamma | 65587 / 1942631 | 295.1, 263.8, 261.5, 229.2, 230.8 | 261.5 |
| conditional | detail heading `Detail is mounted` → `Detail stays`, panel `bg-amber-100` → `bg-white`, `gap-2` → `gap-4` | sibling ticks 2, still open, detail clicks 1 (a later click made 2) | 66075 / 1953109 | 229.2, 218.7, 231.7, 296.3, 197.0 | 229.2 |
| form | `Hello, ` → `Hi, `, `Clear` → `Reset`, `gap-4` → `gap-8` | field text `Ada`, heading followed it (`Hi, Ada`) | 66471 / 1960143 | 229.9, 262.6, 264.5, 262.6, 262.2 | 262.6 |

The lowerer's poll is inside these numbers. They are not the old ~200 ms log-line sample, and they are not the red-pixel medians above.

### Async dashboard

25 Oct 2026, about 7:45 PM Asia/Manila. `./target/release/async-dashboard --hot`. One process: pid 95336, starttime 3602412, window `0x2200002`. The title bar showed that pid, starttime, and window id for the whole take. Before the edits the note was already `Ada's note`, dark mode was on, and the portal was open. One uncut 30 fps raw x11grab, 88 frames, 2.933 s. Not stitched. The clock starts when `write()` and `fsync` of `examples/async-dashboard/view.rs` return, and stops on the first received frame whose pixels differ.

| Edit | What stayed | ms |
| --- | --- | --- |
| `"Modal"` → `"Still open"` | dark theme, open portal, loaded note, `Saved Cebu` | 325.3 |
| `w-[280px]` → `w-[420px]` | the same, with the title already `Still open` | 231.7 |

Closing the modal later in that same take showed `Note: Ada's note`, `Place: Manila desk`, `Could not load the feed`, and `Action: Saved Cebu` on the dark page. Both edits logged `script_reload pid=95336 window_stays=1`. `view.rs` was restored after the take. starttime did not change.

## Wasm

`cargo-makepad wasm build --bindgen -p counter` (dev profile) wrote `target/makepad-wasm-app/debug/counter` in 1m 07s of compile time plus packaging. The package was served with `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`. Headless Chromium with SwiftShader loaded it, and a click moved the counter from 0 to 1.

`Instant::now` panics on Makepad's wasm target. The probe only reads the clock when `QUINN_LATENCY` or `QUINN_LIVE` is set, so a normal counter load does not call it.
