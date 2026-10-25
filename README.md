# lume

Leptos-style components, rendered by Makepad. There is no WebView and no second layout engine.

```rust
use lume::prelude::*;

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

lume::run!("Counter", 480, 360, view! { <Counter /> });
```

`run!` has to sit at the crate root. It expands to a Makepad `app_main!` shell. Rust 1.99.0 is pinned in `rust-toolchain.toml`.

## Test the reactive core

This does not need a GPU or a display:

```bash
cargo test -p lm-styles -p lm-reactive -p lm-core -p lume --no-default-features
```

## Run a native example

Linux, software OpenGL (llvmpipe) under Xvfb:

```bash
export MAKEPAD_GPU=gl
export LIBGL_ALWAYS_SOFTWARE=1
xvfb-run -a cargo run -p counter --release
```

The same invocation works for `conditional`, `keyed-list`, `form`, `baseline-counter`, and `probe-widget-lifecycle`.

Without Xvfb, drop `xvfb-run -a` if `DISPLAY` is already a real X server.

`LUME_EXIT=1` asks a lume example to quit after the first frame. `LUME_READY=/tmp/ready` writes `ready` once the tree is mounted. The lifecycle probe writes `artifacts/probe-widget-lifecycle.log` and quits with a failure status when `LUME_PROBE_EXIT=1` if any of the eight checks failed.

## Layout of the workspace

- `crates/styles` — utility / inline style subset
- `crates/reactive` — upstream `reactive_graph`
- `crates/core` — owners, surface, lifecycle tests
- `crates/macros` — `view!`, `#[component]`, `run!`
- `crates/makepad` — the only renderer
- `crates/lume` — what applications import
- `examples/` — counter, conditional, keyed list, form, and an unmodified-shape Makepad baseline
- `probes/widget-lifecycle` — the Makepad tree experiment
- `docs/` — research, architecture, decisions, compatibility

## Docs

- [Research](docs/research.md)
- [Architecture](docs/architecture.md)
- [Decisions](docs/decisions.md)
- [Compatibility](docs/compatibility.md)
