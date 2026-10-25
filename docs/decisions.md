# Decisions

## Option C — `reactive_graph` + a focused `view!`

Chosen. `quinn-reactive` re-exports `reactive_graph` at `5ca3a9bf90cbcc0561dc4323f8018e18dce5906f` with the `effects` feature. `quinn-macros` parses with rstml 0.13.1 and emits `quinn` builders. `#[component]` is a small attribute that installs an `Owner`. It is not the upstream attribute: it does not support props macros, slots, or `impl IntoView` magic beyond "the function body is the view". Attribute order on a custom tag is argument order.

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

Splash reload of `script_mod` preserves `mod.state` in the baseline counter (`if !vm.is_reload()`). `Apply::ScriptReapply` keeps runtime widget state; a full script reload rebuilds the shell. `view!` is compiled Rust. Changing it, or any signal logic, needs a rebuild and a new process. A release rebuild after `touch examples/counter/src/main.rs`, with Makepad already built, took 1.30 s median (three wall-clock runs, 1.25–1.32 s). One of those rebuilds plus a new process reaching `QUINN_READY` took 1.44 s. Reactive style and text updates apply to the existing `WidgetRef` without recreating it. ra-jit is not in the Makepad tree and is not a prerequisite.

Disk hot reload is Makepad's `--hot` watcher, started from `app_main!`. It re-reads `script_mod!` blocks from the Rust file. `script_eval!` in that same file makes the matcher reject the edit (`runtime has N, file has 1`). `quinn::run!` builds its shell with `ScriptVm::eval` of a string, so the watcher has no block to extract. The counter keeps the shell in `examples/counter/src/chrome.rs` as a real `script_mod!`. The baseline's evals live in `script_ops.rs`.

`Apply::Reload` stores the new script and leaves existing draw colors alone. Both windows call `request_style_reload()` when a file edit arrives as `Reload`, and the next tick uses `Apply::ScriptReapply`. The count stays. The color changes. An `on_render` label keeps the text it already has. Measured write-to-red-pixel medians were 85 ms for the counter page and 136 ms for the baseline button, same PID and same X window id the whole time. See [benchmarks.md](benchmarks.md).

`QUINN_LIVE=1` is a separate harness. It queues a synthetic buffer inside the already-running process. It does not edit the disk. The 74–85 ms figures from that harness are not the disk-watcher numbers.

## Script heap after unmount

Creating a widget calls `ScriptVm::eval`, which allocates a scope object. The paint loop's `ScriptVm::gc` does not return that capacity to the allocator, so a burst of mounts leaves a high-water RSS even after the `WidgetRef`s are gone. `quinn::reclaim_script_heap` runs `gc_and_compact` and, on Linux, `malloc_trim`. After 2,000 native cycles the high water was about 528 MiB and the resting RSS was 129 MiB, from a 117 MiB start. A second burst high-watered at the same level and rested at 133 MiB. Node, widget, and handler counts were unchanged. The retain example calls this between bursts so the resting set is what gets recorded.
