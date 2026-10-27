# Compatibility

## Licenses

| Piece | License | Notes |
| --- | --- | --- |
| This workspace | MIT OR Apache-2.0 | `publish = false` |
| Makepad `dev` @ `44ecb822` | MIT (Makepad B.V. 2023, root `LICENSE`); widget crates also `MIT OR Apache-2.0` | Git dependency, default features off so the tweaker and the other families are not linked |
| `reactive_graph` @ `5ca3a9bf` | MIT (Greg Johnston 2022) | Git dependency, feature `effects` |
| rstml 0.13.1 | MIT (stoically, vldm) | crates.io `=0.13.1`, matching git `9fe0f461` |
| leptos-native @ `00a81fe2` | MIT | Investigated, not depended on |

No GPUI, NSH, Electron, WebView, or JavaScript runtime is a dependency.

## Platforms

Makepad's `platform/` tree contains Linux (X11 and Wayland), Windows, macOS, iOS, Android, and a wasm bridge. This milestone **runs** the Linux X11 software-GL path (`MAKEPAD_GPU=gl`, `LIBGL_ALWAYS_SOFTWARE=1`, Mesa llvmpipe, OpenGL ES 3.2). On this VM that was display `:1` (XFCE) and, for the headless smoke runs, Xvfb. Vulkan windowing is skipped on X11 because Makepad wants Wayland for Vulkan (`X11 session: Vulkan windowing needs Wayland; rendering with OpenGL ES`). macOS, Windows, and iOS were not executed. A successful `cargo check` is not a running UI.

Android has been executed twice on this pin. A Makepad `makepad-example-counter` APK was installed on an Android 14 x86_64 emulator; the host packages and that toolchain are in [SETUP_LINUX_AND_ANDROID.md](SETUP_LINUX_AND_ANDROID.md). Quinn's showcase release APK (`android:debuggable` false, ABI `x86_64`, package `dev.makepad.showcase`) was installed on a Pixel emulator running Android 15 on 27 Oct 2026 (about 11:28 AM Asia/Manila) and drew the Kitchen note. The other Quinn examples still read `view.rs` from disk and were not packaged. That build is [ANDROID.md](ANDROID.md).

`cargo-makepad wasm build --bindgen -p counter` produced `target/makepad-wasm-app/debug/counter`. Served with `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`, that package loaded in headless Chromium (SwiftShader) and a click moved the counter from 0 to 1. An earlier `cargo check` of the framework crates for `wasm32-unknown-unknown` (12.12s) and of `baseline-counter` (15.50s) did not produce a browser session. `bumpalo` was not in the offline cache, so the first framework check needed network once. The wasm runtime needs `wasm-bindgen-cli` 0.2.129; without `--bindgen` instantiation fails on `__wbindgen_placeholder__`.

## Markup that works

Tags: `div`, `span`, `h1`, `h2`, `p`, `label`, `button`, `input`.

`div` → `RoundedView` (a `View` prototype), or `RoundedShadowView` when the class has `shadow`, or `widgets.Grid` when the class has `grid-cols-*`. Text tags → `Label`, except `h1`/`h2` which use those prototypes. `button` → `Button`. `input` → `TextInput`. `<A href>` → `Button` that sets `state.route`.

Also: nested elements, fragments (`<>...</>`), string literals and `{expr}` / `{move || value}` text, `on:click`, `on:input` / `on:change`, `value=`, `bind:value`, `placeholder="..."`, `class="..."`, `style="..."`, `<Show when=...>`, `<For each=... key=... let:item>`, `<Router>` / `<Routes>` / `<Route path>`, `<Suspense fallback>`, `<Transition fallback>`, `<ErrorBoundary fallback>`, `<Portal>`, and a custom component as a function call. `#[prop(optional)]`, `default`, and `into` are accepted. `into` unwraps `"..." .into()` / `.to_string()` to the literal; a path that is not a signal is an error. `children: Children` and `children: ChildrenFn` are script functions. `toggle_theme()` flips `state.quinn_dark`. `save.dispatch("Cebu")` and `save.pending()` are the action. `{note}` reads `state.note_value`.

`#[component] fn Name(...) -> impl IntoView`, when Rust compiles it, creates an owner and returns `scope_owner`. The example `view.rs` files are not compiled. The lowerer reads the same syntax and emits script. A `<Show>` nested inside a `<For>` row is its own function pair and does not use the host show cursor. The in-app router is `state.route`. It is not `leptos_router`.

## Style subset

Utilities (later tokens win): `flex`, `flex-row`, `flex-col`, `flex-wrap`, `flex-nowrap`, `flex-1`, `flex-none`, `grow`, `shrink`, `shrink-0`, `grow-N`, `shrink-N`, `basis-N`, `gap-N`, `gap-x-N`, `gap-y-N` (N×4 px, `gap-px` = 1), `p-` / `px-` / `py-` / `pt-` / `pr-` / `pb-` / `pl-` and the `m*` forms on the same scale, `items-start|center|end|stretch`, `self-start|center|end|stretch`, `justify-start|center|end|between|around|evenly`, `w-full`, `w-fit`, `w-screen`, `h-full`, `h-fit`, `h-screen`, `w-1/2` and the other `n/d` fractions, `min-w-*`, `max-w-*`, `min-h-*`, `max-h-*` (`full` is `"100%"`), `text-xs` through `text-3xl`, the macOS scale `text-large-title`, `text-title1` / `text-title`, `text-title2`, `text-title3`, `text-headline`, `text-body`, `text-callout`, `text-subheadline`, `text-footnote`, `text-caption`, `text-caption2`, `font-medium`, `font-semibold`, `font-bold`, `leading-none|tight|snug|normal|relaxed|loose`, `rounded`, `rounded-sm|md|lg|full`, `rounded-control`, `rounded-control-sm`, `rounded-control-lg`, `rounded-window`, `rounded-sheet`, `h-control-mini`, `h-control-small`, `h-control` / `h-control-regular`, `h-control-large`, `border`, `border-0`, `border-2`, `ring`, `shadow`, `shadow-sm`, `shadow-md`, `shadow-lg`, `shadow-control`, `shadow-window`, `grid`, `grid-cols-N`, `col-span-N`, `absolute`, `inset-0`, `left-*`, `top-*`, `right-*`, `bottom-*`, `z-0`, `z-10`, `z-20`, `z-30`, `z-40`, `z-50`, `z-N`, `overflow-hidden`, `overflow-x-auto`, `overflow-y-auto`, `overflow-auto`, `bg-gradient-to-r|l|t|b`, `from-*`, `to-*`, `disabled`, the Tailwind v3 hues (50–950) plus `white`, `black`, and `transparent`, and the Apple names in [design-system.md](design-system.md) (`system-blue` / `accent` / `accent-blue`, the other system colors, `label`, `label-secondary`, `label-tertiary`, `label-quaternary`, `separator`, `system-background`, `window-background`, `sidebar`, `control-background`, and the fills) on `bg-`, `text-`, `border-`, `from-`, and `to-`. An unknown token fails `lower_file` with `unknown class:`.

Shell prefixes, not media queries: `sm:` (640), `md:` (768), `lg:` (1024), `xl:` (1280) compare `state.quinn_width`. `dark:` compares `state.quinn_dark == 1`. They can nest (`dark:md:`). The root `<div class>` is the host, so `dark:bg-*` on it wraps `draw_bg.color`.

Arbitrary values: `w-[13px]`, `h-[13px]`, `min-w-[8px]`, `max-w-[40px]`, `min-h-[0px]`, `max-h-[16px]`, `p-[10px]` and the per-side `p*` / `m*` forms, `gap-[8px]`, `basis-[16px]`, `left-[8px]`, `right-[8px]`, `top-[8px]`, `bottom-[8px]`, `z-[12]`, `bg-[#1e293b]`, `text-[#fff]`, `text-[13px]`. A 3-digit hex expands. An 8-digit hex keeps the alpha (`#RRGGBBAA`). Opaque colors stay `#xRRGGBB`.

`class:token=move || flag.get()` toggles that one utility. `style:prop=move || if flag.get() { "120px" } else { "40px" }` switches a literal. A runtime string in `style:` is a compile error.

On a row, `items-*` is `RowAlign` and `justify-start|center|end` is `Align` on the main axis. `justify-between|around|evenly` is `Distribute` and does not also set `Align`. `items-stretch` gives children `Fill` on the cross axis when their class did not set that size. `self-start|center|end` wraps the child; `self-stretch` is cross-axis `Fill`. `flex-wrap` is `Flow.Right { wrap: true }` plus `wrap_spacing` from `gap-y`. A column cannot wrap: `Flow::Down` has no wrap field.

`hover:bg-*`, `hover:text-*`, `active:bg-*`, `active:text-*`, `disabled:bg-*`, and `disabled:text-*` set those states. When a state is omitted, the button copies its face and label into `color_hover`, `color_down`, `color_focus`, and `color_disabled`, so a click cannot leave the theme’s pale outset on screen. `reset_hover_on_click` cuts the hover track after the click. The focus ring is `border_color_focus` in the accent (`#1D4ED8` light, `#93C5FD` dark) on the script path. The host patch path, which has no theme state, uses `#1D4ED8`. A bare `disabled` sets `enabled: false` and the disabled mix to 1. `hover:` on a `div` emits an `Animator` that swaps `draw_bg.color`. A `div` does not play a focus track. Text is Inter, not SF Pro. See [design-system.md](design-system.md).

Inline: `color`, `background`, `font-size`, `padding`, `margin`, `gap`, `radius`, `border`, `width`, `height` with `fill`, `fit`, or a pixel number.

## Not supported

These are absent on purpose, not partially implemented:

- CSS inheritance, specificity, the cascade beyond source order, real media queries, `em`/`rem`
- `align-content`, `fixed`, and a turtle z field. `z-*` is paint order only. `inset-0` fills the parent; it is not a CSS inset of a positioned box with both left and right stretching a shrink-wrapped child
- `focus:` on a plain `div`
- HTML forms as a DOM (`<form>`, submit, labels-for), contenteditable, rich text, SVG, images
- `leptos_router` (URL, history, params, nested matching). The in-app `<Router>` is `state.route`.
- A Rust `Into` impl for `#[prop(into)]`. Nested `provide_context` does shadow for the subtree and then restores. The root seed does not reset on reload.
- `RenderEffect`, an inline future that is not a chain of named `quinn::job` fns, server functions, hydration. A job body may be several registered calls (`let a = f(x).await?; g(a).await`), a tuple source, a method call, or a `Serialize` struct argument. `if` / `match` / loops / arithmetic / nested closures in that body are a compile error that names the shape. See P3b in [gap-analysis.md](gap-analysis.md)
- A second layout engine
- Gradient, shadow, or scroll bars on a `Grid` div. `Grid` draws a `DrawQuad`. Put those classes on a child view.

`bind:value` writes `text: state.<signal>` on the `QuinnTextInput`. `Apply::Reload` resets every field the script leaves out, so an input without `text:` was emptied on each render. Giving the field its own text back leaves the caret where it is, and a render that shortens the text (clearing a focused field) clamps the caret instead of panicking. A sibling that reads the signal still updates. `value=` with `on:input` still writes `text:` on each `root.render()`, so that path can still move the caret. The string survives either way. The headless surface still skips `set_text` when the string is already equal; the running examples do not use that surface.

## Public API versus Makepad internals

Used as public API: `script_from_value`, `ScriptVm::eval`, `ScriptMod`, `View.children`, `CxWidgetExt::widget_tree_insert_child`, `set_text` / `redraw` / `clicked` / `changed` / `set_empty_text`, `Label` walk and `draw_text`, `Cx::redraw_all`.

`script_apply_eval` is a public trait method. Passing a hand-built `ScriptMod` whose code is `__script_source__{...}` matches what the `script_apply_eval!` macro generates (`platform/script/derive/src/script.rs`). It is the update path for private fields (button face, input chrome, rounded-view color).

Editing `View.children` is relying on a public field, not an unstable private one, but it is not a dedicated "reorder" API. If Makepad later stops walking `children`, this is the line that has to move.

## Measurements

The apples-to-apples numbers are in [benchmarks.md](benchmarks.md). Both the Quinn counter and the Makepad baseline report ready from `Cx::post_draw_hook`, immediately before `eglSwapBuffers`. On this machine the release counter reached that point in a median of 147 ms (nine runs, 136–168 ms) and the baseline in 123 ms (120–132 ms). A release incremental rebuild of `counter` is 1.30 s median; the baseline is 1.39 s. Stripped release binaries are 20,502,896 bytes (`counter`) and 20,178,256 bytes (`baseline-counter`).

`MemorySurface`, measured inside the debug unit tests: 1,000 signal updates plus flush took 4.312 ms (4.3 µs each). Creating 1 div and 200 labels took 621 µs. Those figures are not GPU present latency. Signal-to-presented-frame latency is in the benchmarks note (Quinn p50 1.58 ms, baseline p50 0.74 ms, 80 samples each).
