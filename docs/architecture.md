# Architecture

`quinn` is the crate applications import. A running example does not mount a Rust signal graph. `quinn-lower` reads `view.rs` (Leptos-looking `view!`, `#[component]`, and `signal()` / `get` / `set` / `update`) and writes one Makepad `script_mod!` to `hot/ui.rs`. State is `mod.state`. Clicks are `Button.on_click`. Typing is `TextInput.on_change`. `on_render` rebuilds children when `render()` runs.

`view.rs` sits next to `src/`. Cargo does not compile it. `hot/` is gitignored and is not a Rust module either. Saving `view.rs` does not restart `cargo run`.

```
view.rs   view! / #[component] / signal()     (not compiled)
        │
        ▼
quinn-lower   parse → script text
        │
        ▼
hot/ui.rs   script_mod!     mod.state, on_render, on_click, on_change
        │
        ▼
Makepad ScriptVm + widgets     RoundedView, H1, Button, TextInput
```

The Rust builders (`el`, `show`, `for_each`, `Owner`, `MemorySurface`) are still in `quinn-core`. Unit tests and `crates/quinn/tests/markup.rs` use them. The example binaries call `boot_script` and do not call `startup`, so they never mount that graph. `reactive_graph` stays linked because `quinn::signal` is the type of a component body when that body is compiled (the tests). It is not the UI runtime.

## Script state

A missing `mod.state` is an error, not nil. The seed is `if ok{mod.state} == nil { mod.state = { ... } }`. `mod` is the VM's modules object, shared by every script body, so a reload that replaces the view's scope keeps the object. `let state = mod.state` then reads it.

`script_eval!` from Rust is a different body. It sees `mod` and `ui`, not the functions defined in `hot/ui.rs`. The lowerer publishes the ones Rust calls: `mod.quinn_frame`, `mod.quinn_retain_step`, `mod.quinn_retain_reset`, `mod.quinn_set_width`, and, when the view has jobs, `mod.quinn_job_count`, `mod.quinn_job_delay`, `mod.quinn_job_gen`, and `mod.quinn_finish`.

## Show, For, and inputs

`<Show>` is not an inline `if` in `on_render`. A statement `if` whose branches are different shapes unbalances the script stack. The lowerer emits two functions, `quinn_show_<signal>_<n>_on` (returns the child widget) and `_off` (returns `0`, which a view skips). `on_render` calls `quinn_show_<signal>_<n>()`. That function records whether the condition was already true and, on a fresh mount, zeroes the signals that belong to the child. A sibling's fields are not touched. Hiding clears the child's fields so a remount does not keep the old clicks.

`<For>` emits `fn quinn_row(...)` and `for item in state.items { quinn_row(item.id, item.label) }`. Anonymous children match by index, so a row's own count cannot live on the widget. The first integer signal on the row lives on `state.counts` under the string key `"k"+id`, which survives reorder, slot reuse, and reload. Other row signals and a row show's live flag live on `state.r_<name>["k"+id]`. Pushing an id calls `quinn_row_reset`, so a removed row that is added again starts clean. Pop does not clear that id. There is no script `reverse`; the lowerer emits `quinn_reverse`. Swap and remove-by-id are `quinn_swap_key` and `quinn_remove_key`.

A `<Show>` inside a row is its own function pair, `row_quinn_show_<signal>_<n>(id, …)`. Those sites are not the host show cursor. `examples/keyed-list` and `examples/tailwind-gallery` both use one.

`TextInput.on_change` is invoked with `vm.call`, which has no widget uid, so `ui.host` from that closure cannot see the app. `on_render` runs on the host, captures it (`let root = ui`), and the closure calls `root.render()`. `Button.on_click` goes through `widget_to_script_call`, which flood-searches from the button, so buttons still call `ui.host.render()`. `render()` applies children with `Apply::Reload`. An input that sets `text:` (the `value=` / `on:input` path) is rewritten, so the caret can jump. `bind:value` omits `text:`, so the field is left alone and a sibling that reads the signal still updates.

## Hot reload

With `--hot`, a thread in the process watches `view.rs` (about 80 ms, then 40 ms) and rewrites `hot/ui.rs` when the lowerer accepts the file. Makepad's watcher keys the block by file, line, and column. The header above `script_mod!` stays a fixed height. On `Apply::Reload` the shell calls `ui.host.render()`, so the new `on_render` runs and label text updates. `request_style_reload()` is the wrong follow-up: it forces `ScriptReapply`, which keeps the old label string.

A shape the lowerer does not parse is logged (`quinn lower failed`) and the previous `hot/ui.rs` stays. `QUINN_SKIP_LOWER=1` skips the rewrite on startup. That is a debug hatch for a hand-patched `hot/ui.rs`, not the loop to use.

Recommended:

```bash
cargo run -p counter --release -- --hot
# edit examples/counter/view.rs
```

The same command works for `conditional`, `keyed-list`, `form`, `retain`, `tailwind-gallery`, `app-shell`, and `async-dashboard`. `./target/release/<bin> --hot` is the same watcher without Cargo as a parent. Saving `src/main.rs` or `src/ops.rs` still rebuilds and, under `cargo run`, starts a new process.

Client jobs are not script async. `drive_shell` in `quinn-makepad` polls on startup, live edit, geometry, timers, and clicks. It arms `Cx::start_timeout` from `mod.quinn_job_delay` (seconds) and, when the timer fires, calls `mod.quinn_finish`, which writes the phase and calls `render()`. `state.quinn_width` and `state.quinn_dark` live in the nil-guarded state object, so a reload keeps them. A `<Portal>` is emitted after the page view. Suspense, transition, and error gates are functions whose both branches are calls, same as `<Show>`.

## What still needs a rebuild

- Anything in `src/` (the shell, `ops.rs`, a new probe).
- A `view.rs` edit the lowerer rejects.
- The baseline counter, which is hand-written Makepad script, not a `view.rs`.

## Headless graph

`#[component]` in compiled Rust expands to `Owner::new()`, runs the body inside `owner.with`, and returns `scope_owner(owner, view)`. Signals created in the body are registered on that owner. Unmount drops the node, which drops the owner, which runs `on_cleanup` and drops the effect. `ImmediateEffect` is the only effect. It pushes a command (`Text`, `Style`, `Show`, `List`) onto a thread-local queue. `Runtime::flush` applies the queue. Tests use `MemorySurface`, which never reuses an id. That is how `retention_cycles_do_not_accumulate_nodes_or_handlers` still runs without a window.

The script examples do not have that owner. Unmount is "the widget is not returned from `on_render`", and the child's fields on `mod.state` are cleared by the show function. Keyed identity is the id on `state.counts`, not a Rust node id. `quinn-lower`'s `Sim` is the test for those rules.

## Shell

Each example's `src/main.rs` is an `app_main!` shell. `script_mod` calls `load_hot_view`, which lowers `view.rs` on the first eval and `eval`s `hot/ui.rs`. The host is the `RoundedView` named `host`. `boot_script` checks that view exists and installs the frame probe. `script_eval!` in `main.rs` would make Makepad's extractor reject `hot/ui.rs` if it lived in the same file; the evals that must stay in Rust are in `examples/counter/src/ops.rs`, or they call `mod.*` from `main.rs` only when that file is not the watched script.

## What is intentionally not a second system

No Taffy, no CSS cascade, no DOM `Renderer`, no second GPU backend, no JS runtime. Class strings are parsed in `quinn-styles` into the fields Makepad already lays out.
