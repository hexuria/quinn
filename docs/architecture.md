# Architecture

`quinn` is the crate applications import. A `view!` body expands to `el` / `show` / `for_each` builders in `quinn-core`. Those builders mount onto a `Surface`. The only production surface is Makepad. Tests use `MemorySurface`, which records creates, text writes, and child order and never reuses an id.

```
view! / #[component]  (quinn-macros, rstml)
        │
        ▼
el / show / for_each / scope_owner   (quinn-core)
        │  ImmediateEffect (quinn-reactive → reactive_graph)
        ▼
Surface::create / set_text / set_style / set_order / remove
        │
        ▼
MakepadSurface  →  WidgetRef via script_from_value(Apply::New)
                   View.children + widget_tree_insert_child
```

## Identity

A mounted node id **is** `WidgetUid.0` (or a monotonic id on `MemorySurface`). The runtime does not keep a parallel widget. It keeps, per id:

- the `Owner` created for that node
- extra owners handed to `scope_owner` (the `#[component]` owner, so signals in the function body live until the root node of that component unmounts)
- click and input listeners
- the `Show` / `For` driver, if this node is a host

`#[component]` expands to `Owner::new()`, runs the body inside `owner.with`, and returns `scope_owner(owner, view)`. Signals created in the body are registered on that owner. Unmount drops the node, which drops the owner, which runs `on_cleanup` and drops the effect `Arc`.

## Updates

`ImmediateEffect::new_scoped` is the only effect. It runs on the thread that wrote the signal, which is the UI thread for Makepad events and the test thread for unit tests. The effect pushes a command (`Text`, `Style`, `Show`, `List`) onto a thread-local queue. It does not touch `Cx`. `Runtime::flush` applies the queue, up to 32 passes, so a show that mounts a child can queue that child's text in the same turn.

`set_text` and `set_style` no-op when the value is unchanged. That is what keeps a text field's caret alive: the `value` effect writes the same string the input just produced, and the surface skips `TextInput::set_text`.

Commands carry ids, not `WidgetRef`, because effects are `Send + Sync` and `WidgetRef` is not.

## Lists and conditionals

`<Show>` mounts a host `div` and, when `when()` is true, mounts the child factory. The factory runs from `flush`, not from inside the effect, so reads inside the child subscribe to the child's effect rather than to the show effect. Turning the condition off unmounts that child and leaves siblings alone. Turning it on mounts a new child.

`<For>` does the same with a host. The effect is the only reader of `each()`. It stores the `Vec` and queues a reconcile. Reconcile keeps the node id for a key that is still present, mounts a view for a new key, unmounts missing keys, then `set_order` rewrites `View.children`. An existing row is not rebuilt, so a per-row signal created inside that row survives a reorder. Fields of the list item captured at creation are not pushed into an existing row; a row that needs live item data should read a signal.

## Threading

`MakepadSurface` methods take no `Cx`. `startup` / `on_actions` call `with_cx`, which stores `*mut Cx` in a thread-local for the duration of the call and clears it afterwards. That pointer is only dereferenced on the UI thread while the original borrow is live. Listeners run, then `flush` runs, still inside `with_cx`. A listener must not re-enter `on_actions`.

## Shell

`run!` expands, in the application crate, to the same shape as `examples/counter`: `app_main!`, `script_mod!`, `Script` / `ScriptHook`, `MatchEvent`. The shell is one `RoundedView` named `host` with a light background (`#xf4f0e6`) and `new_batch: true`. Rust mounts into that view on `Event::Startup`, which `app_main!` delivers after `script_mod` has built the widgets (`platform/src/app_main.rs`).

`script_mod!` cannot be emitted from `macro_rules` because Makepad's `#(expr)` interpolation uses `#`, which `macro_rules` treats as syntax. `run!` is a proc macro and splices a `#` token.

## What is intentionally not a second system

No Taffy, no CSS cascade, no DOM `Renderer`, no second GPU backend, no JS runtime. Class strings are parsed in `quinn-styles` into the fields Makepad already lays out.
