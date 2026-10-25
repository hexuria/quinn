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

Makepad's `platform/` tree contains Linux (X11 and Wayland), Windows, macOS, iOS, Android, and a wasm bridge. This milestone **runs** the Linux X11 software-GL path (`MAKEPAD_GPU=gl`, `LIBGL_ALWAYS_SOFTWARE=1`, llvmpipe under Xvfb). Other targets are source-compatible in the sense that application code does not name a platform and Makepad is the backend, but they were not executed here. A successful `cargo check` is not treated as a running UI.

`cargo check --target wasm32-unknown-unknown` is attempted for the non-Makepad crates. The Makepad wasm target is a separate link of `makepad-wasm-bridge` and is recorded in the final report only if the command was actually run.

## Markup that works

Tags: `div`, `span`, `h1`, `h2`, `p`, `label`, `button`, `input`.

`div` → `RoundedView` (a `View` prototype). Text tags → `Label`, except `h1`/`h2` which use those prototypes. `button` → `Button`. `input` → `TextInput`.

Also: nested elements, string literals and `{expr}` / `{move || value}` text, `on:click`, `on:input` / `on:change`, `value=`, `placeholder="..."`, `class="..."`, `style="..."`, `<Show when=...>`, `<For each=... key=... let:item>`, and a custom component as a function call whose attributes are arguments in source order.

`#[component] fn Name(...) -> impl IntoView` creates an owner and returns `scope_owner`. It does not implement upstream prop helpers or children slots.

## Style subset

Utilities (later tokens win): `flex`, `flex-row`, `flex-col`, `gap-N` (N×4 px, `gap-px` = 1), `p-` / `px-` / `py-` / `pt-` / `pr-` / `pb-` / `pl-` and the `m*` forms on the same scale, `items-start|center|end`, `justify-start|center|end`, `w-full`, `w-fit`, `h-full`, `h-fit`, `text-xs` through `text-3xl`, `rounded`, `rounded-sm|md|lg|full`, `border`, `border-2`, and a short palette (`white`, `black`, `slate-900|600|100`, `stone-900|100`, `amber-700|600|100`) on `bg-`, `text-`, and `border-`.

Inline: `color`, `background`, `font-size`, `padding`, `margin`, `gap`, `radius`, `border`, `width`, `height` with `fill`, `fit`, or a pixel number.

Alignment is Makepad `Align { x, y }`: a fraction of leftover space, not per-item flex. For a column, `justify-*` is `y` and `items-*` is `x`. For a row they swap.

## Not supported

These are absent on purpose, not partially implemented:

- CSS inheritance, specificity, the cascade beyond source order, media queries, pseudo-classes, `em`/`rem`/`%` except `width: 100%` as fill
- `justify-between`, `items-stretch`, `flex-grow`, `flex-wrap`, grid, absolute/fixed positioning
- Hover/focus utility classes (Makepad still has its own button hover animation)
- HTML forms as a DOM (`<form>`, submit, labels-for), contenteditable, rich text, SVG, images
- Slots / `children` on custom components
- Upstream `#[component]` props syntax (`prop: impl Fn()` etc.)
- `RenderEffect`, resources, suspense, server functions, hydration
- A second layout engine

`TextInput::set_text` resets the caret and undo stack. The surface skips the write when the text is already equal. Typing therefore has to go through `on:input` into a signal whose effect writes the same string back.

## Public API versus Makepad internals

Used as public API: `script_from_value`, `ScriptVm::eval`, `ScriptMod`, `View.children`, `CxWidgetExt::widget_tree_insert_child`, `set_text` / `redraw` / `clicked` / `changed` / `set_empty_text`, `Label` walk and `draw_text`, `Cx::redraw_all`.

`script_apply_eval` is a public trait method. Passing a hand-built `ScriptMod` whose code is `__script_source__{...}` matches what the `script_apply_eval!` macro generates (`platform/script/derive/src/script.rs`). It is the update path for private fields (button face, input chrome, rounded-view color).

Editing `View.children` is relying on a public field, not an unstable private one, but it is not a dedicated "reorder" API. If Makepad later stops walking `children`, this is the line that has to move.
