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

`quinn-core`'s `retention_cycles_do_not_accumulate_nodes_or_handlers` runs 2,000 conditional mount/unmount and keyed add/remove cycles on `MemorySurface`. Node count, alive widgets, and click handlers return to the settled baseline. Cleanups run.

The native run is `examples/retain` with `QUINN_RETAIN=N`. Each cycle toggles a conditional `Detail` and pushes or pops a keyed `Row`, then `flush_now`. Two bursts of N cycles run back to back. Counts are taken at rest, with the show closed and a single row left.

`ScriptVm::eval` allocates a scope object per widget. Makepad's paint-loop `gc()` fills free lists and does not return that capacity, so RSS during a burst climbs until something compacts the heap. `quinn::reclaim_script_heap` calls `ScriptVm::gc_and_compact` and, on Linux, `malloc_trim`. The table below is after that call.

| Cycles per burst | RSS before | High water, burst 1 | Rest | High water, burst 2 | Rest | Nodes / widgets at rest | Handlers |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 500 | 116,984 KiB | 219,764 KiB | 120,552 KiB | 221,392 KiB | 121,536 KiB | 7 → 7, max 12 | 1 → 1 → 1 |
| 2,000 | 116,920 KiB | 527,848 KiB | 129,468 KiB | 532,456 KiB | 133,308 KiB | 7 → 7, max 12 | 1 → 1 → 1 |

Host children stay at 1. Cleanups go 1 → 501 → 1,001 for 500 cycles, and 1 → 2,001 → 4,001 for 2,000. The second burst's high water matches the first, so the 400 MiB spike is the script heap for that burst, not a widget map that keeps growing. Resting RSS is not perfectly flat: about 12 MiB remains after the first 2,000 cycles and about 4 MiB more after the second. Widget, node, and handler counts do not follow that curve.

## Live script reload

`QUINN_LIVE=1` queues a `script_mod` file change through Makepad's watcher and calls `request_style_reload`. The clock starts after the queue and stops on the next `post_draw_hook` whose `reload_generation` moved. Rust `view!` source is not live-patched; a Rust change is the rebuild-and-restart number above.

The counter harness clicks seven times, then replaces the shell color `#xf4f0e6` with `#xff3300`. The baseline harness recolors the label and rewrites its text. Applied runs:

| App | Elapsed | State after reload |
| --- | --- | --- |
| `counter` | 78.2–85.2 ms | signal still `"7"`, 3 nodes, 3 widgets |
| `baseline-counter` | 74.4–81.4 ms | label still `"Count: 7"` |

## Wasm

`cargo-makepad wasm build --bindgen -p counter` (dev profile) wrote `target/makepad-wasm-app/debug/counter` in 1m 07s of compile time plus packaging. The package was served with `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`. Headless Chromium with SwiftShader loaded it, and a click moved the counter from 0 to 1.

`Instant::now` panics on Makepad's wasm target. The probe only reads the clock when `QUINN_LATENCY` or `QUINN_LIVE` is set, so a normal counter load does not call it.
