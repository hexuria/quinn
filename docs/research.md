# Research

Pins were taken on 2026-10-24 from the default branch each project publishes as current. `https://github.com/ricardopieper/rstml` returns HTTP 404; the parser moved to `https://github.com/rs-tml/rstml`.

| Project | Branch | Commit | Date / subject |
| --- | --- | --- | --- |
| [makepad/makepad](https://github.com/makepad/makepad) | `dev` | `44ecb822b48106fa76bde4dfd309e5a8a7ff1932` | 2026-10-21, RustRover `cargo check` fix (#1281) |
| [leptos-rs/leptos](https://github.com/leptos-rs/leptos) | `main` | `5ca3a9bf90cbcc0561dc4323f8018e18dce5906f` | 2026-10-21, relative trunk asset paths (#4961) |
| [josephg/leptos-native](https://github.com/josephg/leptos-native) | `main` | `00a81fe266345066eb29d6df74a0ffb67bdbef85` | 2026-07-27, cocoa canvas |
| [rs-tml/rstml](https://github.com/rs-tml/rstml) | `main` | `9fe0f4616a8224d37d64a53fa91722a08d736952` | 2026-09-26, release v0.13.1 |

`rstml` 0.13.1 on crates.io has no `repository` field. Its crate is not a Cargo workspace member of the git tree (`rstml/` sits beside `examples/` and `rstml-control-flow`, and the workspace members list does not include it), so the framework depends on `rstml = "=0.13.1"` and records the tag commit above. Leptos pins the same crates.io version (`leptos/Cargo.toml`).

Rust is pinned to **1.99.0** (`rustc 1.99.0 b940084d7 2026-09-28`), not `stable` (stable was 1.99.1 when this was written).

## Probe A — Makepad

Makepad 2.0 / Splash is the current `dev` tree. `live_design!` is archived (`docs/agents/splash.md`). Apps are `script_mod!` + `#[derive(Script, ScriptHook)]` + `app_main!`. The upstream counter is `examples/counter/src/main.rs`: state lives in `mod.state`, survives script reload because `AppMain::script_mod` only seeds it when `!vm.is_reload()`, and a click calls `Button::clicked` then `script_eval!` and `ui.main_view.render()`.

### What is public

- `WidgetUid(pub u64)` starts at 1 (`widgets/core/src/widget.rs`). `LiveId(pub u64)` (`libs/live_id/src/live_id.rs`).
- `WidgetRef::script_from_value` (`ScriptNew` in `platform/script/src/traits.rs`) does `script_new` + `script_apply(..., Apply::New, ...)`. `WidgetRef::script_apply` looks the value's type up in `WidgetRegistry` and constructs the Rust widget (`widgets/core/src/widget.rs`).
- `ScriptVm::eval(ScriptMod)` (`platform/script/src/vm.rs`) runs a source string. `ScriptMod` fields are public.
- `View.children: SmallVec<[(LiveId, WidgetRef); 2]>` is public (`widgets/core/src/view.rs`). Draw and `handle_event` walk that vec, not a hidden list.
- `CxWidgetExt::widget_tree_insert_child` is a public trait (`widgets/core/src/widget_tree.rs`). `insert_child` stores a weak ref. Same-name siblings replace each other unless the name is `LiveId(0)`.
- `WidgetRef::set_text`, `text`, `redraw`. `Label::set_text_color` no-ops when the color is unchanged. `Label.walk` and `Label.draw_text` are public. `Button` draw fields are private; `ButtonRef::clicked` and `TextInputRef::changed` / `set_text` / `set_empty_text` are public. `TextInput::set_text` clears undo history, so identical writes must be skipped.
- `View.layout`, `View.walk`, `View.show_bg`, `View.new_batch` are public. `RoundedView` is a script prototype of `View` (`widgets/core/src/view_ui.rs`) with a shader that has `color`, `border_size`, `border_radius`, `border_color`. Plain `DrawQuad` has no color.
- `Cx::redraw_all`, `Cx::quit`, `MatchEvent`. Linux paints with Vulkan if a device answers, else OpenGL ES (`platform/Cargo.toml`, `MAKEPAD_GPU`). `new_batch: true` is required when a background would otherwise cover text (`splash.md`).
- There is **no** public remove or reorder method. Removal is "drop the `WidgetRef` and delete it from `View.children`". The tree entry is weak and dies with the last strong ref.

`script_apply_eval` (`ScriptApply` in `traits.rs`) applies `Apply::Eval`, which does **not** rebuild children (`View::on_after_apply` skips eval). Creation must be `Apply::New`. Updates can be field writes or `script_apply_eval`.

`libs/html` and `widgets/families/rich_text` parse HTML into rich text. They are not a widget tree and are not used here.

No `ra_jit` / `ra-jit` symbol exists in the tree.

The executable probe is `probes/widget-lifecycle`. It performs create, insert, `set_text` (same uid), insert a sibling, remove, reorder, attach/fire/remove a Rust handler, and `redraw_all` while recording the `Window` uid.

Run on this machine (X11, `MAKEPAD_GPU=gl`, `LIBGL_ALWAYS_SOFTWARE=1`, Mesa llvmpipe, OpenGL ES 3.2) it logged `PROBE_OK`: window uid 3 stayed 3 across `redraw_all`, the first label uid was stable across `set_text`, the sibling uid survived reorder, the handler fired once and not after it was cleared, and the removed widget left `View.children`. A hand-built `ScriptMod` does **not** inherit the app module's `use mod.prelude.widgets.*`. `Fill` and `Fit` are identifiers spread from `mod.turtle` (`draw/src/turtle.rs`, `..me.Size`). Without that import, eval reports `variable Fill not found in scope` and the property stays at the prototype default; a number still becomes `Size::Fixed` (`ScriptHook for Size`). `lume` prefixes generated scripts with the import. `script_mod!` inside `quote!` also drops spaces, because the macro rebuilds source from token spans and `quote!` tokens share the call site, so `run!` emits the shell as a string with `#(0)` bound to `App::script_component`.

## Probe B — Leptos

`tachys/src/renderer/mod.rs` still declares `trait Renderer`, and the module comment says the generic renderer was removed before 0.7 because of compile times and linker errors. `pub type Rndr = dom::Dom`. DOM aliases and `web_sys` are hard-wired. `leptos_macro` expands `view!` to `::leptos::tachys::html::element::...`. Implementing `Renderer` does not reach Makepad.

`reactive_graph` 0.3.0-rc0 (edition 2024, rust-version 1.88) is a normal library:

- `signal()` returns `(ReadSignal<T>, WriteSignal<T>)` for `T: Send + Sync + 'static` (`reactive_graph/src/signal.rs`). `.get()` / `.set()` / `.update()` come from `traits`.
- `Owner` is an `Arc`. `Owner::new` parents to the current owner. `Owner::with` sets it for a closure. Children are `Weak`. Dropping the last `Owner` runs cleanups and drops arena nodes (`owner.rs`).
- `Effect` and `RenderEffect` schedule with `any_spawner::Executor::spawn_local` (`effect/render_effect.rs`). Makepad's UI thread has no executor.
- `ImmediateEffect::new` runs synchronously. It is a no-op unless the `effects` feature is on. `new_scoped` moves the effect into `on_cleanup`. `Dispose` for `ImmediateEffect` is an empty method that takes `self` by value, so the `Arc` drops and the effect stops (`effect/immediate.rs`). Dead subscribers fail a `Weak` upgrade and are not re-run.

`WidgetRef` is `Rc<RefCell<_>>`, so it is `!Send`. Effects therefore capture widget ids and push commands; the UI thread applies them with `&mut Cx`.

## Probe C — leptos-native and markup

leptos-native (`README.md`, MIT) is a fork: its own `view!` / `IntoView<R>`, Taffy layout, and platform façades (cocoa, gtk, uikit). It reuses upstream `reactive_graph` from crates.io. Greg Johnston asked for a fork rather than an upstream renderer. Adopting it would mean maintaining that fork and adding Taffy beside Makepad's turtle, which the milestone forbids unless turtle layout cannot express the subset. It can: `Flow::Down` / `Flow::Right`, `spacing`, `padding: Inset`, `Align { x, y }` as a leftover-space fraction, `Size::Fill` / `Fit` / `Fixed` (`draw/src/turtle.rs`).

rstml parses the token tree into `Node` / `NodeElement` / `KeyedAttribute`. `NodeName`'s `Display` joins `on:click` and `let:item` (`rstml/src/node/node_name.rs`). `parse2` is the entry point. That is the parser `view!` uses. It does not create DOM nodes.

## UI-thread experiment

`probes/widget-lifecycle` is the experiment for the eight operations. `lm-core`'s `MemorySurface` tests are the experiment for connecting that tree shape to `ImmediateEffect` (signal text, sibling identity, conditional cleanup, keyed identity, handler removal). The Makepad surface in `crates/makepad` is the same operations, called from the effect queue.
