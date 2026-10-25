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

`run!` has to sit at the crate root. It expands to a Makepad `app_main!` shell. Rust 1.99.0 is pinned in `rust-toolchain.toml`.

## Test the reactive core

This does not need a GPU or a display:

```bash
cargo test -p quinn-styles -p quinn-reactive -p quinn-core -p quinn --no-default-features
```

## Run a native example

Linux, software OpenGL (llvmpipe) under Xvfb:

```bash
export MAKEPAD_GPU=gl
export LIBGL_ALWAYS_SOFTWARE=1
xvfb-run -a cargo run -p counter --release
```

The same invocation works for `conditional`, `keyed-list`, `form`, `baseline-counter`, `retain`, and `probe-widget-lifecycle`.

Without Xvfb, drop `xvfb-run -a` if `DISPLAY` is already a real X server. On macOS, leave both GPU variables unset and follow [docs/TESTING_ON_MACOS.md](docs/TESTING_ON_MACOS.md).

`QUINN_EXIT=1` asks `counter`, `baseline-counter`, and `retain` to quit after the first presented frame. `QUINN_READY=/tmp/ready` writes `ready` from Makepad's `post_draw_hook`, after that frame's draw lists are recorded and before swap. The lifecycle probe writes `artifacts/probe-widget-lifecycle.log` and quits with a failure status when `QUINN_PROBE_EXIT=1` if any of the eight checks failed.

Other harness variables, read by `counter` and `baseline-counter`:

| Variable | Effect |
| --- | --- |
| `QUINN_LATENCY=80` | Eighty signal-to-next-presented-frame samples, then quit |
| `QUINN_LIVE=1` | Apply one Makepad script/style reload and record how long it took |
| `QUINN_WIDGET_BENCH=200` | Time creating 200 labels and updating their text |
| `QUINN_RETAIN=2000` | On `retain` only: two bursts of mount/unmount cycles |

`QUINN_RETAIN=2000 QUINN_EXIT=1 cargo run -p retain --release` prints node, widget, handler, and RSS counts. Nodes and handlers return to the resting tree. RSS during a burst climbs with the script heap; `quinn::reclaim_script_heap` compacts it before the resting sample.

## Layout of the workspace

- `crates/styles` — utility / inline style subset
- `crates/reactive` — upstream `reactive_graph`
- `crates/core` — owners, surface, lifecycle tests
- `crates/macros` — `view!`, `#[component]`, `run!`
- `crates/makepad` — the only renderer
- `crates/quinn` — what applications import
- `examples/` — counter, conditional, keyed list, form, retain, and a Makepad baseline. The baseline keeps the button and the label inside a `height: Fill` client view, with light text on a dark button, so neither sits in the title band.
- `probes/widget-lifecycle` — the Makepad tree experiment
- `docs/` — research, architecture, decisions, compatibility, benchmarks, roadmap, macOS checklist

## Docs

- [Research](docs/research.md)
- [Architecture](docs/architecture.md)
- [Decisions](docs/decisions.md)
- [Compatibility](docs/compatibility.md)
- [Benchmarks](docs/benchmarks.md)
- [Roadmap](docs/roadmap.md)
- [Testing on macOS](docs/TESTING_ON_MACOS.md)
