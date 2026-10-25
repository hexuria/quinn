# Roadmap

What this tree is, and what it is not taking on next. Crate names on crates.io are unset on purpose. Every package has `publish = false`. Nothing here has been published.

## Done in this milestone

- `view!`, `#[component]`, and `run!` expand to a Makepad widget tree. There is no WebView and no second layout engine.
- Signals, conditional mount, and keyed lists update the existing widgets. Unmount drops the owner, the handlers, and the `WidgetRef`.
- The Linux X11 software-GL path runs the examples. The counter also runs as wasm under Chromium with SwiftShader.
- Startup, latency, and live script reload share `post_draw_hook` with a hand-written Makepad counter.

## Left open

- macOS, Windows, iOS, and Android have not been executed. [TESTING_ON_MACOS.md](TESTING_ON_MACOS.md) is the unchecked list.
- Resting RSS after `reclaim_script_heap` still creeps a few megabytes per thousand native mount cycles. Widget and handler counts do not. The leftover is in Makepad's script heap after compact, not in the Quinn node map.
- `view!` is compiled Rust. A Rust change is a rebuild and a new process. `--hot` reloads a `script_mod!` block from disk in the same window. `ScriptReapply` updates draw colors and keeps text the widget already has. It does not hot-swap Rust closures. `quinn::run!` shells are strings, so they are not watcher targets; the counter's chrome is a real `script_mod!`.
- The style language is the fixed utility list in [compatibility.md](compatibility.md). Slots, `children` on custom components, suspense, resources, and server functions are not started.
- `TextInput::set_text` still resets the caret. The surface skips the write when the string is unchanged. A richer input would need a Makepad API that edits text without clearing undo.
