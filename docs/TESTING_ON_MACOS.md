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
cargo run -p baseline-counter --release
cargo run -p probe-widget-lifecycle --release
```

What to look at:

- `counter`: the label starts at `Counter: 0`. Each click of Increment adds one. The button sits in the client area, not in the title band.
- `conditional`: toggling the switch removes the detail and brings it back. A remounted detail starts again; it does not keep the old count.
- `keyed-list`: reversing the rows reorders the existing rows. A row's own counter survives the reorder.
- `form`: typing in the field updates the echoed line. Clearing the field empties both.
- `baseline-counter`: `Count: 0` and Increment are inside the window body. The label and the button are light text on a dark control. A click changes the count.
- `probe-widget-lifecycle`: the process prints `PROBE_OK` and exits 0. `QUINN_PROBE_EXIT=1` makes a failed check a non-zero exit. The log path is `artifacts/probe-widget-lifecycle.log`.

`QUINN_EXIT=1` asks `counter`, `baseline-counter`, and `retain` to quit after the first presented frame. `QUINN_READY=/tmp/quinn-ready` writes `ready` from `post_draw_hook` at that same moment.

## Retention

```bash
QUINN_RETAIN=2000 QUINN_EXIT=1 QUINN_RETAIN_OUT=/tmp/retain.txt cargo run -p retain --release
```

The file should show `nodes_after=7`, `widgets_after=7`, `handlers_after=1`, and the same counts with the `_2` suffix after the second burst. `rss_rest_kib` is after `quinn::reclaim_script_heap`. `rss_high_kib` is the script-heap spike before that compact and will be much larger. On Linux, 2,000 cycles rested near 129 MiB from a 117 MiB start, then 133 MiB after the second burst. macOS RSS will differ; the count fields should not.

The unit test does not need a window:

```bash
cargo test -p quinn-styles -p quinn-reactive -p quinn-core -p quinn --no-default-features
```

`retention_cycles_do_not_accumulate_nodes_or_handlers` is the memory-surface version of the same 2,000 cycles.

## Live reload and latency

These env vars work on the release `counter` and `baseline-counter` binaries:

| Variable | Effect |
| --- | --- |
| `QUINN_LATENCY=80` | Eighty signal-to-next-frame samples, then quit. Optional `QUINN_LATENCY_OUT`. |
| `QUINN_LIVE=1` | Queue one Makepad script/style reload after seven increments. Optional `QUINN_LIVE_OUT`. |
| `QUINN_WIDGET_BENCH=200` | Create 200 labels and `set_text` them, seven times unless `QUINN_WIDGET_RUNS` is set. |

`QUINN_LIVE` on the counter changes the shell background from `#xf4f0e6` to `#xff3300` and leaves the signal at `7`. On the baseline it recolors the label and leaves the text `Count: 7`.

A Rust edit is not a live reload. Time `cargo build -p counter --release` after touching `examples/counter/src/main.rs`, then the next `QUINN_READY`.

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
