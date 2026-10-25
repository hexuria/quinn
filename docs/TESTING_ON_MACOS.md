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

Build the release binaries, then run them directly so Cargo is not watching the tree:

```bash
cargo build -p baseline-counter -p counter --release
./target/release/baseline-counter --hot
./target/release/counter --hot
```

`cargo run -- --hot` works for a color edit that does not rewrite a source file Cargo is compiling. The counter's watcher rewrites `examples/counter/src/ui_script.rs`. Under `cargo run` that rewrite starts a new process.

Click until the count reads 5. Leave the process up.

- Baseline: change `increment_button`'s `draw_bg.color` in `examples/baseline-counter/src/main.rs` and save. The button recolors. The count stays 5. The log contains `hot reload applied`, then `hot_reload_followup pid=...`.
- Counter: change the `view!` in `examples/counter/src/main.rs` (`gap-4` to `gap-8`, or `"Increment"` to `"Add one"`) and save. The same window updates. The count stays 5. The new button still increments. The log contains `quinn lowered view into .../ui_script.rs`, `hot reload applied` for that file, and `script_reload pid=...`.

The title bar can be set with `xdotool set_window --name` to the PID and the X id. Both should be unchanged after the save. Confirm the process with `starttime` in `/proc/<pid>/stat` as well as the window id. The X server recycles window ids across processes.

`script_eval!` must not sit in the file you edit. Makepad counts those bodies and rejects the file when the counts disagree. The counter's seed and its Rust script calls live in `seed.rs` and `ops.rs`. `quinn::run!` does not emit a `script_mod!` the watcher can read.

`QUINN_LIVE=1` is not this path. It injects a synthetic script buffer into the running process and does not modify a file.

| Variable | Effect |
| --- | --- |
| `QUINN_LATENCY=80` | Eighty signal-to-next-frame samples, then quit. Optional `QUINN_LATENCY_OUT`. |
| `QUINN_LIVE=1` | In-process synthetic script reload on the first frame. Not a disk edit. |
| `QUINN_WIDGET_BENCH=200` | Create 200 labels and `set_text` them, seven times unless `QUINN_WIDGET_RUNS` is set. |

A change to a Rust handler that the lowerer does not understand, or to conditional / keyed-list / form, is a rebuild and a new process. The counter's lowered `view!` is not: with `--hot` it stays one process. Time `cargo build -p counter --release` after touching `examples/counter/src/main.rs` only when you are measuring a cold rebuild.

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
