# Roadmap

What this tree is, and what it is not taking on next. Crate names on crates.io are unset on purpose. Every package has `publish = false`. Nothing here has been published.

## Done in this milestone

- `view!`, `#[component]`, and `run!` expand to a Makepad widget tree. There is no WebView and no second layout engine. `run!` is the string shell. The examples do not use it.
- Counter, conditional, keyed list, form, retain, and the Tailwind gallery lower that syntax into Makepad script (`mod.state`, `on_render`, `on_click`, `on_change`). A save of `view.rs` under `--hot` updates classes, static text, show/for structure, and handlers in the same process. `mod.state` keeps the count, the keyed map, the toggle, and the typed string.
- P0 from [gap-analysis.md](gap-analysis.md) is in: row wrap, `Distribute`, `RowAlign`, flex weight, min/max, `Size::Rel`, per-side padding and margin, `class:` / `style:` / arbitrary values, closed multi-statement handlers, `<Show>` inside a row, and real button hover, down, and disabled colors. `examples/tailwind-gallery` shows them. `examples/keyed-list` has a per-row note.
- P1 is in: fragments, `#[prop]` options, `children` / `ChildrenFn` as script functions, `provide_context` / `use_context` on `mod.ctx`, an in-app `<Router>` that is not `leptos_router`, `widgets.Grid`, `abs_pos`, clip and scroll, RobotoFlex weights and `leading-*`, gradients and shadows, nested script objects, and `bind:value` that does not rewrite the input. `examples/app-shell` is the multi-page shell. The gallery's last section shows the new utilities.
- The Linux X11 software-GL path runs the examples. The counter also runs as wasm under Chromium with SwiftShader.
- Startup, latency, and live script reload share `post_draw_hook` with a hand-written Makepad counter.

## Left open

- macOS, Windows, iOS, and Android have not been executed. [TESTING_ON_MACOS.md](TESTING_ON_MACOS.md) is the unchecked list.
- The script retain run (2,000 cycles, two bursts) keeps widgets at 4 and buttons at 1. RSS rests about 3 MiB above the start and does not climb again on the second burst. The old 400 MiB spike was the reactive `ScriptVm::eval` mount. The headless `MemorySurface` test still passes and is a different runtime.
- A `view.rs` edit the lowerer accepts hot-reloads, including a `<Show>` nested inside a `<For>` row. An edit in `src/` or a shape the lowerer rejects still needs a rebuild. `quinn::run!` shells are strings, so they are not watcher targets. The baseline counter's block is `examples/baseline-counter/src/main.rs`.
- The style language is the utility list in [compatibility.md](compatibility.md), including the P0 and P1 forms. Suspense, resources, and server functions are not started. P2 in the gap analysis is not started. Taffy is still not part of the plan. `mod.ctx` does not shadow, and the router does not speak URLs.
- `bind:value` leaves the input's `text` field alone, so the caret stays, and a sibling label still updates. `value=` with `on:input` still writes `text:` on each `render()`, and that caret can jump. `examples/form` is the second path.
