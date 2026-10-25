# Decisions

## UI runtime — Makepad script, not a signal graph

Chosen: option (c). `view!`, `#[component]`, and `signal()` / `get` / `set` / `update` stay as the authoring syntax. On the counter they lower into Makepad script (`mod.state`, `on_render`, `on_click`) in `examples/counter/src/ui_script.rs`. `reactive_graph` is not the counter's runtime. It still runs the conditional, keyed-list, form, and retain examples, and the `quinn-core` tests, until that lowerer covers `<Show>`, `<For>`, and inputs.

### What Splash / ScriptVm actually has

Checked on Makepad `44ecb822` (dev). There is no signal, effect, or observable type in `platform/script`.

- State is a heap object. The upstream counter (`examples/counter/src/main.rs`) does `let state = mod.state` and `mod.state.counter += 1`. `AppMain::script_mod` seeds that object only when `!vm.is_reload()` (`platform/script/src/vm.rs` `is_reload` / `with_reload`). Splash snapshots `mod.*` across a style reapply and puts it back; `style_reapply_keeps_body_module_state` in `widgets/core/src/splash.rs` bumps `mod.state.count` to 1, restyles, and the redefined `fn bump` still reaches 2.
- Handlers are script closures. `Button::on_click` is a `ScriptFnRef`. `Button::handle_event` calls it from `FingerUp` via `widget_to_script_call` (`widgets/core/src/button.rs`). The upstream counter happens to use the Rust `clicked` path plus `script_eval!`. Both exist. `CheckBox` and the glass panel have `on_click` too.
- Display updates are explicit. `View::on_render` rebuilds children when something calls `render()`. The upstream counter says render is not automatic. `render()` applies children with `Apply::Reload` (the template wins, including text, and children are matched by `LiveId`). `render_style` uses `Apply::ScriptReapply`, which keeps imperative strings such as `Label` text (`Apply::preserves_runtime_state` in `platform/script/src/apply.rs`, `String::script_apply` in `prims.rs`).
- `--hot` re-reads literal `script_mod!` blocks (`platform/src/live_reload.rs`). `Apply::Reload` is the walk when the DSL changed. `ScriptReapply` re-walks the cached template and will not pick up a new label string.

### Why not (a)

(a) keeps `reactive_graph` for bindings and handlers and puts only markup in script. A `{move || ...}` or `on:click` is a compiled Rust closure. Saving the file cannot replace it. That is the narrow hot-reload result already measured for draw colors. The owner wants the Leptos-looking syntax because it is easier to read, and wants Makepad's own reload path under it. `reactive_graph` is the wrong runtime for that.

### Why not (b)

(b) puts UI-local state in script and app state in Rust signals, with a bridge. The counter's count, its label, and its click are one piece of UI. A bridge means two writes, two cleanups, and a Rust closure that a script reload does not replace. That is how a handler goes stale. Script handlers are replaced because `Apply::Reload` writes the new `on_click` onto the widget. Rust `clicked` listeners are not.

### What (c) gives, and what it does not

Hot reload covers the lowered script: classes, static text, the `on_click` body, and `on_render`. `mod.state` survives because the seed lives in `seed.rs` and skips itself on reload. After a reload the counter calls `ui.host.render()` so the new `on_render` runs under `Apply::Reload`. `request_style_reload()` is the wrong follow-up here. It forces `ScriptReapply`, which keeps the old label text.

State preservation is the script object, not a Rust `Owner`. Keyed widget identity is the `name :=` id. Anonymous `for` children are positional; a row count that must survive reorder or reload has to live on `mod.state` under a stable key, which is also what survives a widget slot being reused. The lowerer does not emit `<For>` or `<Show>` yet. Those examples stay on the Rust graph, where the existing tests still check owner cleanup and keyed node ids.

Script is dynamic. The first compile still typechecks the Rust syntax. A hot edit is checked by the script parser (`validate_extracted_script_mod`); a shape the lowerer does not know is logged and not written. There is no fine-grained subscriber list. A click writes the field and calls `render()`, which is what Makepad's own counter does.

One measured save on this machine: process 53851, X window `0x1c00002`, starttime unchanged. The label read `Counter: 5` with button `Increment`. Editing `gap-4` to `gap-8` and `Increment` to `Add one` in `examples/counter/src/main.rs` rewrote `ui_script.rs`. The log was `hot reload applied 1 override(s)` for that file at line 6, then `script_reload pid=53851`. The same window showed `Counter: 5` and `Add one`. Another click showed `Counter: 6`. The wait from the save to that log line was about 200 ms and includes the lowerer's poll, so it is not a paint median.

## Earlier choice — `reactive_graph` instead of forking Leptos

This was the choice against Tachys and leptos-native. It is not the UI runtime choice above. `quinn-reactive` still re-exports `reactive_graph` at `5ca3a9bf90cbcc0561dc4323f8018e18dce5906f` with the `effects` feature. `quinn-macros` parses with rstml 0.13.1 and emits `quinn` builders. `#[component]` is a small attribute that installs an `Owner`. It is not the upstream attribute: it does not support props macros, slots, or `impl IntoView` magic beyond "the function body is the view". Attribute order on a custom tag is argument order.

### Why not A (upstream Tachys / `leptos_macro`)

`tachys/src/renderer/mod.rs` sets `type Rndr = dom::Dom` and documents that the generic renderer was removed. The macro expansion names `tachys::html` and `web_sys`. A Makepad backend would be a standing fork of `leptos_macro` and `tachys`. That is the opposite of a small integration.

### Why not B (leptos-native, swap the backend)

`leptos-native` at `00a81fe266345066eb29d6df74a0ffb67bdbef85` is already a fork, and its layout is Taffy. The README records that upstream asked for a fork. Replacing cocoa/gtk/uikit with Makepad would keep Taffy and the fork. Makepad's turtle covers the subset this milestone draws (row, column, gap, padding, margin, alignment as a leftover fraction, fixed/fit/fill sizes), so a second layout engine is not justified.

## Makepad as the only renderer

Widgets are constructed with `ScriptVm::eval` + `WidgetRef::script_from_value` so shaders and prototypes (`RoundedView`, `H1`, `Button`, `TextInput`) apply. `div` is `RoundedView` because a plain `View`'s `DrawQuad` has no color. Text tags are `Label` / `H1` / `H2`. There is no public remove API; `View.children` is public and is the vec draw walks, so removal and reorder edit that vec and drop the `WidgetRef`. The widget-tree edge is a weak ref (`widget_tree.rs` `insert_child`).

`WidgetRef` is not `Send`. The reactive graph stays `Send`. The bridge is an id plus a thread-local command queue flushed on the UI thread.

## Sync effects, not `RenderEffect`

`RenderEffect` needs `any_spawner`. Wiring an executor into Makepad's event loop would be a second scheduler. `ImmediateEffect` already runs on the setter's thread and stops when dropped. The `effects` feature is required; without it `new` returns a no-op (`effect/immediate.rs`).

## Style subset

`quinn-styles` parses a fixed utility vocabulary and a small inline grammar into a `Style` struct. Tag defaults live in `Tag::defaults` (dark ink `#1c1917` on the light page, light label on a dark button, input height 36px). Unknown utilities are ignored. See `docs/compatibility.md` for what is not CSS.

## Crate split

| Crate | Role |
| --- | --- |
| `quinn-styles` | Pure parser. No Makepad, no Leptos. |
| `quinn-reactive` | Pinned `reactive_graph` re-export. |
| `quinn-core` | Owners, commands, `Surface`, `MemorySurface`, tests. |
| `quinn-macros` | `view!`, `#[component]`, `run!`. |
| `quinn-makepad` | `MakepadSurface`, startup, actions. Optional. |
| `quinn` | The import path. `native` (default) pulls `quinn-makepad`. |

`cargo test -p quinn --no-default-features` never links Makepad. That is what CI runs.

## Live reload

Splash reload of `script_mod` preserves `mod.state` in the baseline counter (`if !vm.is_reload()`). `Apply::ScriptReapply` keeps runtime widget state; a full script reload rebuilds the shell. The counter's `view!` is lowered to `ui_script.rs` and reloads in-process; see the UI runtime section. The conditional, keyed-list, and form examples are still compiled Rust. Changing those, or any of their signal logic, needs a rebuild and a new process. A release rebuild after `touch examples/counter/src/main.rs`, with Makepad already built, took 1.30 s median (three wall-clock runs, 1.25–1.32 s). One of those rebuilds plus a new process reaching `QUINN_READY` took 1.44 s. Reactive style and text updates apply to the existing `WidgetRef` without recreating it. ra-jit is not in the Makepad tree and is not a prerequisite.

Disk hot reload is Makepad's `--hot` watcher, started from `app_main!`. It re-reads `script_mod!` blocks from the Rust file. `script_eval!` in that same file makes the matcher reject the edit (`runtime has N, file has 1`). `quinn::run!` builds its shell with `ScriptVm::eval` of a string, so the watcher has no block to extract. The counter keeps the shell in `examples/counter/src/chrome.rs` as a real `script_mod!`. The baseline's evals live in `script_ops.rs`.

`Apply::Reload` stores the new script and leaves existing draw colors alone. Both windows call `request_style_reload()` when a file edit arrives as `Reload`, and the next tick uses `Apply::ScriptReapply`. The count stays. The color changes. An `on_render` label keeps the text it already has. Measured write-to-red-pixel medians were 85 ms for the counter page and 136 ms for the baseline button, same PID and same X window id the whole time. See [benchmarks.md](benchmarks.md).

`QUINN_LIVE=1` is a separate harness. It queues a synthetic buffer inside the already-running process. It does not edit the disk. The 74–85 ms figures from that harness are not the disk-watcher numbers.

## Script heap after unmount

Creating a widget calls `ScriptVm::eval`, which allocates a scope object. The paint loop's `ScriptVm::gc` does not return that capacity to the allocator, so a burst of mounts leaves a high-water RSS even after the `WidgetRef`s are gone. `quinn::reclaim_script_heap` runs `gc_and_compact` and, on Linux, `malloc_trim`. After 2,000 native cycles the high water was about 528 MiB and the resting RSS was 129 MiB, from a 117 MiB start. A second burst high-watered at the same level and rested at 133 MiB. Node, widget, and handler counts were unchanged. The retain example calls this between bursts so the resting set is what gets recorded.
