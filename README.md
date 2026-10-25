# quinn

Leptos-style components, rendered by Makepad. There is no WebView and no second layout engine.

```rust
use quinn::prelude::*;

#[component]
fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);

    view! {
        <div class="flex flex-col gap-4 p-4">
            <h1>"Counter: " {move || count.get()}</h1>
            <button on:click=move |_| {
                set_count.update(|n| *n += 1);
            }>"Increment"</button>
        </div>
    }
}

quinn::run!("Counter", 480, 360, view! { <Counter /> });
```

The syntax above is what you write in `view.rs`. The running examples lower it into a Makepad `script_mod!` (`mod.state`, `on_render`, `on_click`, `on_change`) so `--hot` can reload the view in the same process. `view.rs` is not a Rust module. `run!` still expands to an `app_main!` shell for a compiled component, and the headless tests still use `reactive_graph`. The example windows do not. Rust 1.99.0 is pinned in `rust-toolchain.toml`.

## Test the reactive core

This does not need a GPU or a display:

```bash
cargo test -p quinn-styles -p quinn-reactive -p quinn-core -p quinn-lower -p quinn --no-default-features
```

## Run a native example

Linux, software OpenGL (llvmpipe) under Xvfb:

```bash
export MAKEPAD_GPU=gl
export LIBGL_ALWAYS_SOFTWARE=1
cargo run -p counter --release -- --hot
```

Edit `examples/counter/view.rs` while that process is up. The same command works for `conditional`, `keyed-list`, `form`, `retain`, `tailwind-gallery`, `app-shell`, `async-dashboard`, and `showcase`. Saving `src/main.rs` still rebuilds. `baseline-counter` and `probe-widget-lifecycle` are hand-written Makepad programs; they have no `view.rs`. `showcase` is the macOS window (sidebar, toolbar, content, inspector) and is the place to look at the design system. `async-dashboard` and the other themed examples call `quinn::drive_shell` so resources, the dark flag, and the window width update without a rebuild.

`xvfb-run -a cargo run -p counter --release` still starts a window when you are not reloading. Drop `xvfb-run -a` if `DISPLAY` is already a real X server. On macOS, leave both GPU variables unset and follow [docs/TESTING_ON_MACOS.md](docs/TESTING_ON_MACOS.md).

`QUINN_EXIT=1` asks `counter`, `baseline-counter`, and `retain` to quit after the first presented frame. `QUINN_READY=/tmp/ready` writes `ready` from Makepad's `post_draw_hook`, after that frame's draw lists are recorded and before swap. The lifecycle probe writes `artifacts/probe-widget-lifecycle.log` and quits with a failure status when `QUINN_PROBE_EXIT=1` if any of the eight checks failed.

Other harness variables, read by `counter` and `baseline-counter`:

| Variable | Effect |
| --- | --- |
| `QUINN_LATENCY=80` | Eighty signal-to-next-presented-frame samples, then quit |
| `QUINN_LIVE=1` | Inject one synthetic script reload in-process. This does not edit a file. Disk reload is `--hot` |
| `QUINN_WIDGET_BENCH=200` | Time creating 200 labels and updating their text |
| `QUINN_RETAIN=2000` | On `retain` only: two bursts of mount/unmount cycles |

`QUINN_RETAIN=2000 cargo run -p retain --release` prints the script census and RSS, then quits. Do not set `QUINN_EXIT=1` with it: that flag quits on the first frame. Widgets and buttons return to the resting tree (4 widgets, 1 button, 2 host children at rest for the current view). `session_stats()` stays empty because the Rust graph is not mounted.

## Layout of the workspace

- `crates/styles` — utility / inline style subset. Apple’s macOS type scale and system colors, plus the Tailwind v3 palette. See [docs/design-system.md](docs/design-system.md).
- `crates/reactive` — upstream `reactive_graph`
- `crates/core` — owners, surface, lifecycle tests
- `crates/lower` — lowers `view.rs` (`Show`, `For`, inputs, clicks) into Makepad script
- `crates/macros` — `view!`, `#[component]`, `run!`
- `crates/makepad` — the only renderer
- `crates/quinn` — what applications import
- `examples/` — counter, conditional, keyed list, form, retain, Tailwind gallery, and a Makepad baseline. The gallery is the P0 utilities (wrap, distribute, row align, flex weight, fractions, per-side spacing, arbitrary values, `class:`, button hover, and a show inside a row). The baseline keeps the button and the label inside a `height: Fill` client view, with light text on a dark button, so neither sits in the title band.
- `probes/widget-lifecycle` — the Makepad tree experiment
- `docs/` — research, architecture, decisions, compatibility, benchmarks, roadmap, macOS checklist

## Docs

- [Design system](docs/design-system.md)
- [Research](docs/research.md)
- [Architecture](docs/architecture.md)
- [Decisions](docs/decisions.md)
- [Compatibility](docs/compatibility.md)
- [Benchmarks](docs/benchmarks.md)
- [Roadmap](docs/roadmap.md)
- [Testing on macOS](docs/TESTING_ON_MACOS.md)
