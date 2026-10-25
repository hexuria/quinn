# Decisions

## UI runtime — Makepad script, not a signal graph

Chosen: option (c), for every example. `view!`, `#[component]`, and `signal()` / `get` / `set` / `update` stay as the authoring syntax. `quinn-lower` turns that syntax into Makepad script (`mod.state`, `on_render`, `on_click`, `on_change`) in `hot/ui.rs`. The counter, conditional, keyed list, form, and retain examples all boot that script. They do not call `startup`, so they do not mount an `Owner`.

`reactive_graph` is still a dependency, for two narrow reasons:

1. A component body that Rust actually compiles (`quinn-core` tests, `crates/quinn/tests/markup.rs`) typechecks through `quinn::signal`. `view.rs` is not a Rust module, so the running examples never execute those closures.
2. `MemorySurface` still checks owner cleanup, keyed node ids, and handler maps without a window. `quinn-lower`'s `Sim` is the same rules for the script that actually runs. Both stay green. The script sim is the source of truth for the UI.

Nothing in the example event loop reads or writes a `reactive_graph` signal.

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

Hot reload covers whatever `view.rs` the lowerer can emit: classes, static text, the `on_click` / `on_change` bodies, `<Show>`, `<For>`, and inputs. `mod.state` survives because the seed is `if ok{mod.state} == nil` and `mod` outlives the reloaded scope. After a reload the shell calls `ui.host.render()` so the new `on_render` runs under `Apply::Reload`. `request_style_reload()` is the wrong follow-up. It forces `ScriptReapply`, which keeps the old label text.

State preservation is the script object, not a Rust `Owner`. A `<Show>` that turns off clears the child's fields and returns `0` instead of the child widget, so the next mount starts clean and the sibling's fields stay. A row count lives on `state.counts["k"+id]` because anonymous `for` children are positional. A reload replaces `on_click` because `Apply::Reload` writes the new closure onto the widget.

`TextInput` fires `on_change` with `vm.call`, which has no widget uid. `ui.host` from that closure cannot see the app (`widget 'host' not found`, or `widget has no uid` if a top-level function calls it). `on_render` captures the host (`let root = ui`) and the closure calls `root.render()`. Buttons still use `ui.host.render()`; `widget_to_script_call` flood-searches from the button.

Script is dynamic. A hot edit the lowerer does not know is logged and not written. There is no fine-grained subscriber list. A click writes the field and calls `render()`.

Measured on 25 Oct 2026, about 12:53–1:02 PM Asia/Manila, Linux X11, llvmpipe, release binaries. Each row is one process. `starttime` is field 22 of `/proc/<pid>/stat`. The X server reused window id `0x1c00002` across processes, so the pid and the starttime are the identity. Save-to-visible medians are in [benchmarks.md](benchmarks.md).

- Counter, `cargo run -p counter --release -- --hot`. pid 64535, starttime 1913679. At count 5, `gap-4` → `gap-8` and `Increment` → `Add one`. The log stayed a single `Running` line, then `script_reload pid=64535`. The same window showed `Counter: 5` and `Add one`. A click showed `Counter: 6`.
- Keyed list, pid 65587, starttime 1942631. Beta was 2, order Alpha, Beta, Gamma. The row's `gap-4` → `gap-8` and `text-lg` → `text-sm`. Beta stayed 2 and the order stayed.
- Conditional, pid 66075, starttime 1953109. Sibling ticks were 2, the detail was open with 1 click. The detail heading and its panel class changed (`Detail stays`, white panel). Sibling stayed 2, the toggle still read `Hide detail`, detail clicks stayed 1, and a click after the reload made 2.
- Form, pid 66471, starttime 1960143. The field held `Ada`. `Hello, ` → `Hi, `, `Clear` → `Reset`, `gap-4` → `gap-8`. The heading became `Hi, Ada` and the field still showed `Ada`.

## Earlier choice — `reactive_graph` instead of forking Leptos

This was the choice against Tachys and leptos-native. It is not the UI runtime choice above. `quinn-reactive` still re-exports `reactive_graph` at `5ca3a9bf90cbcc0561dc4323f8018e18dce5906f` with the `effects` feature. `quinn-macros` parses with rstml 0.13.1 and emits `quinn` builders. `#[component]` is a small attribute that installs an `Owner`. It is not the upstream attribute: it does not support props macros, slots, or `impl IntoView` magic beyond "the function body is the view". Attribute order on a custom tag is argument order.

### Why not A (upstream Tachys / `leptos_macro`)

`tachys/src/renderer/mod.rs` sets `type Rndr = dom::Dom` and documents that the generic renderer was removed. The macro expansion names `tachys::html` and `web_sys`. A Makepad backend would be a standing fork of `leptos_macro` and `tachys`. That is the opposite of a small integration.

### Why not B (leptos-native, swap the backend)

`leptos-native` at `00a81fe266345066eb29d6df74a0ffb67bdbef85` is already a fork, and its layout is Taffy. The README records that upstream asked for a fork. Replacing cocoa/gtk/uikit with Makepad would keep Taffy and the fork. Makepad's turtle covers the subset this milestone draws (row, column, gap, padding, margin, alignment as a leftover fraction, fixed/fit/fill sizes), so a second layout engine is not justified.

## Makepad as the only renderer

The running examples construct widgets from the lowered `script_mod!`. `div` is `RoundedView` because a plain `View`'s `DrawQuad` has no color. Text tags are `Label` / `H1` / `H2`. `button` is `Button`. `input` is `TextInput`. Removal is "on_render did not return that child".

The headless surface still builds widgets with `ScriptVm::eval` + `WidgetRef::script_from_value`. `View.children` is public and is the vec draw walks, so that surface's removal and reorder edit the vec. The widget-tree edge is a weak ref (`widget_tree.rs` `insert_child`). `WidgetRef` is not `Send`. The reactive graph stays `Send`. That bridge is an id plus a thread-local command queue flushed on the UI thread. The example event loops do not flush it.

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

Splash reload of `script_mod` preserves `mod.state` because the seed checks `ok{mod.state}` and `mod` is the VM modules object. `Apply::ScriptReapply` keeps runtime widget state; a full script reload re-runs `on_render` under `Apply::Reload`. Every example's `view.rs` reloads in-process. A release rebuild after `touch examples/counter/src/main.rs`, with Makepad already built, took 1.30 s median (three wall-clock runs, 1.25–1.32 s). That number is a Rust rebuild, which is what a `src/` edit still costs. One of those rebuilds plus a new process reaching `QUINN_READY` took 1.44 s. ra-jit is not in the Makepad tree and is not a prerequisite.

Disk hot reload is Makepad's `--hot` watcher, started from `app_main!`. It re-reads literal `script_mod!` blocks. The block it re-reads is `hot/ui.rs`, which the lowerer rewrites when `view.rs` changes. `script_eval!` in that same file makes the matcher reject the edit (`runtime has N, file has 1`). `quinn::run!` builds its shell with `ScriptVm::eval` of a string, so the watcher has no block to extract. The examples do not use `run!`. The baseline's evals live in `script_ops.rs`.

The color-only rows in [benchmarks.md](benchmarks.md) are the earlier `ScriptReapply` path (`request_style_reload()`). Draw colors updated and text the widget already held stayed. The view path does not do that. It calls `render()` under `Apply::Reload`, so the label string and the handlers update, and `mod.state` keeps the count, the row map, the toggle, and the typed text. Write-to-red-pixel medians on the old path were 85 ms for the counter page and 136 ms for the baseline button.

`QUINN_LIVE=1` is a separate harness. It queues a synthetic buffer inside the already-running process. It does not edit the disk. The 74–85 ms figures from that harness are not the disk-watcher numbers.

## Script heap after unmount

The previous reactive mount called `ScriptVm::eval` per widget. The paint loop's `ScriptVm::gc` does not return that capacity, so 2,000 of those cycles high-watered near 528 MiB and rested near 129 MiB from a 117 MiB start. That table is historical. The script retain example does not eval a widget per cycle. `render()` rebuilds the host's children, and `quinn::reclaim_script_heap` (`gc_and_compact`, and on Linux `malloc_trim`) runs between bursts. On 25 Oct 2026 (release, llvmpipe, `QUINN_RETAIN=2000`) resting widgets stayed 4, buttons stayed 1, host children stayed 2, and the second burst's high water was 238,564 KiB against a 234,640 KiB start. The Rust node and handler maps stay at zero because that graph is not mounted. See [benchmarks.md](benchmarks.md).
