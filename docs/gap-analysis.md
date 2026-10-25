# Gap analysis

Analysis only. Nothing here is implemented. Status is against the running UI: `view.rs` lowered to Makepad script (`hot/ui.rs`, `mod.state`, `on_render`). The headless Rust graph (`quinn-core`, `MemorySurface`, `reactive_graph`) is named when it can do something the window cannot.

Sources checked in this tree and in the pinned Makepad checkout `44ecb822`:

- Quinn authoring: `crates/macros/src/lib.rs`, `crates/lower/src/parse.rs`, `crates/lower/src/emit.rs`, `crates/styles/src/lib.rs`, `crates/reactive/src/lib.rs`
- Quinn deps: `Cargo.lock` has no `taffy`. `crates/makepad/Cargo.toml` depends on `makepad-widgets` at `44ecb822` with default features off.
- Turtle: `draw/src/turtle.rs` (`Walk`, `Layout`, `Flow`, `Size`, `Align`, `Distribute`, `RowAlign`)
- Grid widget (not the turtle): `widgets/core/src/grid.rs`
- Text and draw: `draw/src/shader/draw_text.rs` (`TextStyle`, `FontMember.weight`), `widgets/core/src/button.rs` (hover/down animator uniforms)
- Leptos 0.8.0 release notes (https://github.com/leptos-rs/leptos/releases/tag/v0.8.0). 0.8 keeps the 0.7 authoring model. The additions that matter here are store fields implementing `IntoClass` / `IntoStyle` / `AttributeValue`, `Suspend::new` accepting `IntoFuture`, and websocket server functions. Quinn's `reactive_graph` pin is `5ca3a9bf` (`docs/compatibility.md`), the 0.7-era graph, and the examples do not execute it.

`docs/research.md` describes an earlier turtle (row, column, spacing, padding, `Align`, `Fill`/`Fit`/`Fixed`). The pin is past that. Wrap, overlay, `abs_pos`, fill weight/shrink/basis, `Distribute`, min/max, relative sizes, and a `Grid` widget are in `44ecb822`. Quinn does not emit them.

## 1. Leptos syntax parity

Legend: **supported** means a `view.rs` the lowerer accepts and a running example can use. **partial** means a slice works, or only the headless graph has it. **missing** means the lowerer rejects it or it has no runtime.

### `view!` and components

| Feature | Status | On a script runtime |
| --- | --- | --- |
| Elements, string literals, `{move \|\| signal.get()}` text | Partial | The lowerer accepts a closed set of text shapes (`Part` in `crates/lower/src/model.rs`): literals, one signal, `if bool`, `if empty`. A general closure is rejected. Widening the expression parser and emitting script expressions is local to `quinn-lower`. |
| Fragments and `<>...</>` | Missing | `view!` in the macros expands a fragment to `quinn::fragment` (`crates/macros/src/lib.rs`). The lowerer returns "only elements are supported" (`parse.rs`). Script can emit sibling widgets with no wrapper. Small. |
| Nested components | Partial | `Detail` and `Row` are inlined when the lowerer understands the call (`examples/conditional/view.rs`, `examples/keyed-list/view.rs`). Arbitrary components, generics, and `impl Fn()` props are not. Each new component shape is more parser, not more runtime, until props and children exist. |
| `children` / slots | Missing | The macro errors with "child slots on custom components are not in this milestone" (`macros/src/lib.rs`). Script has no slot. A child would be a function the parent calls from `on_render`, the same way `quinn_row` already works. Medium: the lowerer has to pass that function through. |
| `#[prop(optional)]`, `default`, `into` | Missing | `#[component]` ignores its attribute and wraps the body in an `Owner` (`macros/src/lib.rs`). The lowerer takes function arguments as positional names (`parse.rs` `parse_comp`). Defaults are a parser default in the script seed. `into` is a Rust trait bound and does not exist in script; the lowerer would coerce the few types it already has (int, bool, string, item). Medium. |
| Generic components | Missing | Script functions are untyped. A generic component lowers only if monomorphized at the call site into a concrete script function. The lowerer does not do that. Medium, and only for types the seed can represent. |

### Signals

| Feature | Status | On a script runtime |
| --- | --- | --- |
| `signal()` + `.get()` / `.set()` / `.update()` | Partial | Parsed for `let (name, set_name) = signal(...)` with int, bool, `String`, and `vec![Item ...]` (`parse.rs`). The window stores the value on `mod.state` and assigns it. There is no subscriber. A write calls `render()`. `.with()` is not parsed. |
| `RwSignal`, split read/write, `ReadSignal` | Missing | The re-export is whatever `reactive_graph` prelude has, and only compiled tests can call it. Script would be the same `mod.state` field with two names. Small once expressions exist. |
| `Memo`, derived closures | Missing | Text `if` is the only derived form, and it is recomputed inside `on_render`, not cached. A memo is a script function of `state` called from `on_render`. No dependency tracking. Medium for a real expression language; small if the set of expressions stays closed. |
| `Effect` / `RenderEffect` | Partial | Headless code has `ImmediateEffect` (`crates/reactive/src/lib.rs`) because Makepad has no async executor (`docs/decisions.md`). The window has no effect. A script effect is "the setter writes state and calls `render()`". An effect that must run without rebuilding the tree needs a script function invoked from the setter, not a subscriber list. Medium, and it will not match Leptos's "track what you read". |
| Resources, `async`, `Suspense`, `Transition`, `Action` | Missing | Script is synchronous. 0.8's `Suspend::new(IntoFuture)` and `Action` still need a future. A Rust task that writes `mod.state` and calls `ui.host.render()` when it finishes is a bridge beside the VM, the thing option (c) avoided for UI state. Large. Websocket server functions from 0.8 are the same bridge plus a socket. |
| `on_cleanup` | Partial | Re-exported and used by the headless owner. Script unmount is "this `on_render` did not return the widget", and `<Show>` zeroes the child's fields (`emit.rs` `emit_show_fn`). There is no user `on_cleanup` hook. A script callback stored on `mod` and called from the show's off branch is small. |

### Control flow

| Feature | Status | On a script runtime |
| --- | --- | --- |
| `<Show when>` | Partial | Works at the host: two functions, `_on` returns the child, `_off` returns `0` (`emit.rs`). A statement `if` in `on_render` unbalances the script stack, which is why it is functions. Fallback is the off branch, not a `<Show fallback>`. A `<Show>` inside a `<For>` row is parsed and emitted wrong: the row function uses an empty show cursor (`emit.rs`). Fixing that is medium (show sites per row, resets keyed by row id). |
| `<For each key let>` | Partial | Keyed counts live on `state.counts["k"+id]` because anonymous children match by index (`docs/architecture.md`). The row template is one element. Add, pop, and reverse are hard-coded actions. A general `each()` closure is not. Indexed `For` without a key would be the same loop and would lose per-row state on reorder, which is the turtle's behavior, not a bug to paper over. |
| `<Suspense>` / `<Transition>` | Missing | Needs the resource bridge above. The widget can be a `<Show>` on a loaded flag. The async part is the large piece. |
| `<ErrorBoundary>` | Missing | A script error aborts the render (`View::render` discards a failed `on_render`). Catching that and drawing a fallback is a host change around `script_eval` / `render`, not a component the lowerer can fake. Medium, and it will not catch a Rust panic in the shell. |
| `<Portal>` | Missing | 0.7/0.8 `Portal` moves DOM into another node. The turtle has `Flow::Overlay` and `Walk.abs_pos` (`turtle.rs`). A portal is an overlay child or a second `Window`, not a DOM mount point. Medium for overlay; a second OS window is a shell change. |
| `<DynamicChild>` / `<{expr}>` | Missing | The macro has no dynamic tag. Script would store a tag name on `state` and branch in `on_render` with the same function-per-branch pattern as `<Show>`. Medium, and only for tags the lowerer knows. |

### Attributes

| Feature | Status | On a script runtime |
| --- | --- | --- |
| `class="..."` and `style="..."` | Partial | Static strings only. Unknown utilities are ignored (`crates/styles/src/lib.rs`). `class:` and `style:` (the reactive forms) are rejected as unknown attributes (`parse.rs`). A boolean class is `if state.flag { "bg-white" } else { "" }` inside the class string the lowerer already parses, then the same `parse_classes` result. Small. |
| `prop:` / `attr:` | Missing | There is no DOM attribute map. A known prop (`placeholder`, `value`) is already a dedicated attribute. Anything else needs a named script field on that widget. Per-widget allowlist, small for each widget, not a general `attr:` namespace. |
| `on:` with event types | Partial | `on:click`, `on:input`, and `on:change` only, and the closure body must be one of increment, toggle, clear, pop, reverse, push, or row increment (`parse.rs`). `MouseEvent` does not exist. Makepad gives the button no event object; `TextInput` passes the string. New gestures (keyboard, pointer position) need a Makepad handler that exists on that widget, then a lowerer pattern. Medium per gesture. |
| `bind:value` | Partial | The form is `value=` plus `on:input` writing the same signal (`examples/form/view.rs`). `on_change` calls `root.render()` because `TextInput` uses `vm.call` with no widget uid (`docs/decisions.md`). The string survives. The caret can jump because `render()` rewrites `text`. A true bind that does not call `render` on each keystroke needs a way to set the heading without reapplying the input. Medium, and it is a Makepad text API question. |
| `node_ref` | Missing | Script widgets already have names (`h1 := H1`). A ref is that name, read back with `script_eval!` as `ui.host.h1`. Exposing it as a Rust `NodeRef` crosses back into the shell and does not hot-reload. Small as a script name; a Rust ref is the wrong direction. |
| Spread `{..attrs}` | Missing | No attribute bag. Would be a script object merged into the widget literal. Medium, and only keys the lowerer knows survive. |

### Context, router, stores, server functions

| Feature | Status | On a script runtime |
| --- | --- | --- |
| `provide_context` / `use_context` | Missing | Headless `Owner` could hold it; the window has no owner. `mod.ctx` is a script object any function can read. Small. It will not be typed and it will not scope itself to a subtree unless the lowerer passes it as an argument, which is the slot work above. |
| Router (`Routes`, `Route`, nested routes, `A`, params, navigate) | Missing | 0.8 made `PossibleRouteMatch` dyn-safe and changed route generation; it is still a URL router. A native window has no URL. An in-app stack is `state.route` plus `<Show>`. Medium. Real paths, queries, and history belong on wasm only, and they are a shell around that signal, not a layout feature. Large if it has to match `leptos_router`. |
| Stores (`reactive_stores`, 0.8 `IntoClass` for store fields) | Missing | A store field is a nested script object (`state.user.name`). Updates are assignments plus `render()`. Fine-grained field tracking is the subscriber list script does not have. Medium for nested objects; large if a field write must skip unrelated widgets. |
| Server functions, `#[server]`, 0.8 websocket server functions | Out of scope | They compile to an HTTP or websocket endpoint and assume a Leptos server and, for hydration, a DOM. Quinn has no server, no request codec, and no hydration. A native app that calls an HTTP API is an ordinary Rust client writing `mod.state` when the response arrives (the resource bridge). Reimplementing `server_fn` would drag in the Leptos server stack this project refused. |

## 2. Tailwind on the turtle

Quinn's parser is a closed utility list (`crates/styles/src/lib.rs`). "Supported" means that list emits something today. "Feasible" means `44ecb822` can do it and Quinn does not emit it. "New capability" means it is not a turtle walk, not the `Grid` widget, and not an existing draw uniform.

`flex` / `flex-row` emit `flow: Right`. `flex-col` emits `flow: Down` (`emit.rs` `flow_line`). `items-*` and `justify-*` both become `align: Align{x y}` (`align_line`). That `Align` is one fraction of leftover space added to every walk (`turtle.rs` `Align`, around line 2939). It is not per-item `align-items`. `RowAlign` (`turtle.rs` line 3009) is the per-item cross-axis and is what `items-center` on a row should use. Quinn never emits it.

| Category | Status | Maps to | What it would take |
| --- | --- | --- | --- |
| Layout: row and column | Supported | `Layout.flow` `Flow::Right` / `Flow::Down` (`turtle.rs` 2959) | Already emitted. |
| Layout: wrap | Feasible | `Flow::Right { wrap: true }`, `Layout.wrap_spacing` (`flow_right_wrap`, line 2785) | Parser plus one script field. No new engine. |
| Layout: flex grow / shrink / basis | Feasible | `Size::Fill { weight, basis, shrink, min, max }` (`turtle.rs` 311). Quinn's `Fill` is the bare identifier, default weight 100, shrink 0. | Emit the fields. Equal `w-full` siblings already share weight 100, which is a crude `flex-grow: 1` for every fill child, not a per-item factor. |
| Layout: space-between / around / evenly | Feasible | `Distribute` (`turtle.rs` 3036): `SpaceBetween`, `SpaceAround`, `SpaceEvenly`. Quinn has no `justify-between`. | One `Layout.distribute` field. This is main-axis slack between walks, not a second flex pass. |
| Layout: per-item cross alignment | Feasible, current mapping is wrong for mixed heights | `RowAlign::Center` / `Bottom` / `Baseline` | Stop using `Align.y` as `items-center`. Emit `flow: Right { row_align: Center, wrap: false }`. |
| Layout: grid | Feasible, not on the turtle | `widgets/core/src/grid.rs`: `columns` / `rows` as `TrackLen::Px`, `Pct`, `Fr`, gaps, `auto_flow`, `areas`, `justify_items`, `align_items`. `Walk.cell` is ignored by other flows (`turtle.rs` line 125). | Lower a grid container to `Grid { ... }`. The widget is in `makepad-widgets`, which Quinn already links. Subgrid and masonry are not on that struct. |
| Layout: absolute / inset | Feasible for a point, not the CSS inset box | `Walk.abs_pos: Option<Vec2d>` (`turtle.rs` 96). Tests place a walk at a point (around line 831). | `left`/`top` can set `abs_pos`. All four insets (stretch between left and right) are not one field. That is a size computation in the lowerer, or a missing layout rule. |
| Layout: z-index | New capability | Paint order is child order. `Flow::Overlay` stacks walks (`flow_overlay`, line 2803). No z field was found on `Walk` or `Layout`. | Reorder children, or overlay. A numeric z-index that sorts independently of source order is new. |
| Layout: overflow | Feasible | `Layout.clip_x` / `clip_y` default true (line 2738). `Layout.scroll`. | `overflow-hidden` is the default clip. `overflow-visible` is `clip_*: false`. Scroll is the scroll vector, not a browser scrollport with momentum. |
| Spacing: padding, margin, gap | Supported | `Layout.padding`, `Walk.margin`, `Layout.spacing`. Scale is N×4 px (`styles` `spacing`). | Per-side padding is parsed. `emit.rs` `uniform_padding` only emits padding when all four sides match, so `px-4` without `py-4` is dropped. Emitting `Inset` per side is small. |
| Sizing: full / fit / px | Supported | `Size::Fill`, `Size::Fit`, `Size::Fixed` | `w-full`, `w-fit`, `h-full`, `h-fit`, and inline px. |
| Sizing: min / max, aspect | Feasible | `Walk.min_width` / `max_width` / `min_height` / `max_height`, `Walk.aspect` (`turtle.rs` 112) | Parser plus script fields. |
| Sizing: fractions, screen | Feasible for parent and viewport | `Size::Rel { base: Parent \| Vw \| Vh \| Cqw \| Cqh, factor }` (`turtle.rs` 340, `Base` at 2865) | `w-1/2` is `Rel { base: Parent, factor: 0.5 }`. `w-screen` is `Vw`. Container query units need `Layout.container_id`. Quinn emits none of these. |
| Typography: font size | Supported | `TextStyle.font_size` (`draw_text.rs` 3538). `text-xs` through `text-3xl` map to 12–30 px. | Done. |
| Typography: leading | Feasible | `TextStyle.line_spacing` (line 3540), `top_drop` | One field. |
| Typography: weight, family | Feasible, limited | `FontMember.weight` is the OpenType `wght` axis (`draw_text.rs` 3558). `TextStyle.font_family` is a list of font resources, not a CSS font stack. | Weight means shipping a variable font or one file per weight and selecting the member. Family means a resource path, not `font-sans`. |
| Typography: tracking, alignment, truncate | Partial / feasible | `DrawText` layout takes an `Align` and can measure an ellipsis (`draw_text.rs` `draw_walk`, the `"…"` layout around line 2255). Tracking (letter-spacing) was not found on `TextStyle`. | Truncate and text align can call the existing draw API. Tracking needs a shader or glyph offset Quinn does not have. |
| Colors, opacity | Partial | Palette is 11 names (`styles` `palette`). `Rgba.a` exists and `hex_script` drops alpha (`#xRRGGBB`). | More names are data. Opacity is passing alpha through if the draw uniform has it. `draw_text` colors are `vec4`. Small. |
| Backgrounds, gradients | Feasible for a shader, not a utility | `DrawText` has `gradient_fill_horizontal` and mixes two colors (`draw_text.rs` around 452). `draw_vector.rs` samples a gradient texture. `RoundedView` uses a flat `draw_bg.color`. | A two-stop background is a draw uniform on the view's shader, copied from the text gradient or the vector sampler. Arbitrary `bg-[url(...)]` and mesh gradients are out. |
| Borders, radius | Supported | `draw_bg.border_size`, `border_color`, `border_radius`. `rounded`, `rounded-sm/md/lg/full`, `border`, `border-2`. | Per-corner radius and per-side width are not in the style struct. The draw quad would have to grow those uniforms. |
| Shadows | Feasible in a shader | `draw_vector.rs` `shadow_rounded_rect` (gaussian SDF). Not wired to `RoundedView`. | A shadow utility is a shader on the view, or an extra draw behind it. It is not layout. Medium because the view prototype Quinn emits has no shadow input. |
| Transforms | New capability for CSS transforms | No `transform` field on `Walk`. Overlay and `abs_pos` move a walk. Scale and rotate are draw-matrix work (`draw_text.rs` has a mat helper near line 3530 used by glyphs, not a widget transform). | `translate` can be `abs_pos` or a margin. `scale` / `rotate` need a draw transform inherited by children. That is new. |
| Transitions, animation | Feasible for widget animators | `Button` ramps `hover` and `down` with `anim_time` (`widgets/core/src/button.rs` around 362). `View` has an `Animator` (`view.rs`). | A duration utility can drive an animator Quinn does not currently set. CSS `@keyframes` and transition-all are not that animator. |
| States: hover, focus, active, disabled | Feasible on widgets that already have them | Button `color_hover`, `color_down`, `disabled` (`button.rs`). Quinn copies one color into all four button states so the face does not change (`emit.rs` button colors). | Stop flattening those uniforms. `hover:` as a class means a second color in the widget's animator, not a CSS pseudo-class on a `div`. A plain `RoundedView` hover needs the view animator turned on. Medium. |
| Responsive breakpoints | New capability as CSS, feasible as a signal | No media-query type in the turtle. `Size::Rel` viewport units are not breakpoints. | `state.wide` set from the window size in the shell, then the same `if` the lowerer uses for text. The shell does not report size today. Medium, and it is not `md:`. |
| Dark mode | Feasible as a signal | No color scheme in the turtle. | `state.dark` picked by the shell, two colors in the class string. Small once `class:` exists. It will not follow the OS unless the shell reads it. |
| Arbitrary values `w-[13px]`, `bg-[#abc]` | Missing | The inline parser already accepts `width: 13px` and `#rrggbb` (`parse_inline`). | Accept `w-[13px]` in `apply_token`. Small. `bg-[url(...)]` is not a color and stays missing. |

Tailwind, used as a class string that means "row, column, gap, padding, type scale, a flat color, a radius", fits the turtle. Tailwind used as "the CSS layout spec, including grid areas, arbitrary selectors, and variants" does not fit in the parser, and a slice of it (grid, wrap, grow, overlay) fits Makepad only by emitting widgets and fields Quinn ignores today.

## 3. Layout engine

Quinn does not use Taffy.

- `Cargo.lock` has no `taffy` dependency.
- `docs/architecture.md`: "No Taffy".
- `docs/decisions.md` records leptos-native's Taffy and rejects it because the turtle covered the milestone's subset.
- Emitted layout is `flow: Down` or `flow: Right`, `spacing`, `padding`, and `Align{x y}` (`emit.rs`).

The turtle is the layout engine. It walks children in one pass and resolves `Size` (`draw/src/turtle.rs`, the comment at `Layout` line 2733: "Specifies how walks should be laid out with respect to each other").

What the turtle cannot do, from the fields that exist and the ones that do not:

| CSS / Tailwind | In the pin? |
| --- | --- |
| `flex-wrap` | Yes. `Flow::Right { wrap: true }`. Quinn does not emit it. |
| `flex-grow` / `flex-shrink` / `flex-basis` | Yes, as `Size::Fill`'s `weight`, `shrink`, and `basis`. Not per-child in Quinn's output. |
| `justify-between` and the space-* distributions | Yes, as `Distribute`. Not emitted. |
| `align-items` for mixed heights | Yes, as `RowAlign`. Quinn's `Align` fraction is a weaker substitute. |
| `align-content` (packing wrapped lines on the cross axis) | No field on `Layout`. `wrap_spacing` is a constant gap, not `align-content: space-between`. |
| CSS grid | Not a turtle flow. `Walk.cell` is ignored unless a grid consumer reads it. `widgets/core` `Grid` is a separate widget with `fr` tracks, spans, and named areas. No subgrid, no `minmax()` as a first-class track (tracks are px, percent, fr, or a size expression). |
| Absolute positioning with a containing block and all four insets | `abs_pos` is one point. There is no `position: absolute; left: 0; right: 0` rule. |
| `position: fixed` against the viewport | No. A fixed layer would be a child of the root turtle, or `abs_pos` in root coordinates. The pin does not have a fixed containing block. |
| `z-index` | No. |
| Multi-line flex `align-content`, baseline grid, masonry | No. |

Adding Taffy is not justified for wrap, grow, space-between, or grid. Those already exist in this pin and are unused by the lowerer. A second engine would:

- Run a full measure/layout pass beside the turtle, then push rectangles back onto widgets the turtle also wants to walk. One of the two must win, or widgets get laid out twice.
- Hot reload stays a script `render()` only while positions are turtle fields. A Taffy pass has to run after every `render()` or the new tree keeps the old rectangles. That is another step on the UI thread on every keystroke that already calls `render()`.
- Binary size and the compile the milestone has been measuring go up by the Taffy crate and the glue. The published incremental rebuild of `counter` is 1.30 s median with Makepad already built (`docs/benchmarks.md`). Taffy itself is small next to Makepad; the cost that matters is the second layout on the hot path, not the compile.
- It does not help script hot reload. Taffy does not understand `class=` strings. The lowerer would still parse classes, then feed numbers to Taffy instead of to `spacing` and `flow`.

Extending the turtle upstream is justified only for the holes: `align-content` of wrapped lines, a real inset box (left and right both set), and `z-index`. Those are Makepad changes, then a Quinn emitter change. They are not a reason to vendor Taffy. Using `Grid` for `grid-cols-*` is a Quinn emitter change against a widget that already hot-reloads as script.

## 4. Roadmap

Sizes are scope, not a schedule. Small: parser and emitter, turtle fields that already exist. Medium: new script patterns, tests, and a widget the lowerer does not emit yet. Large: a runtime beside the script VM, or an upstream turtle change.

### P0 — the utilities and syntax the current examples are already asking for

| Item | Size | Why first |
| --- | --- | --- |
| Emit turtle features the pin already has: wrap, `Distribute`, `RowAlign`, fill weight/shrink/basis, min/max, `Size::Rel` for fractions and viewport | Small | Corrects `items-center` and unlocks `flex-wrap`, `justify-between`, `w-1/2`, `flex-grow` without a new engine. |
| Per-side `Inset` padding and margin (today non-uniform padding is dropped) | Small | `px-4` is parsed and then thrown away. |
| `class:` / `style:` as script `if` expressions, plus arbitrary `w-[13px]` and hex colors | Small | Static classes already hot-reload. Dynamic ones are the same parser. |
| Wider but still closed handler bodies (set a field, toggle a sibling, not only inc/pop/reverse) | Medium | `on:click` rejects anything that is not one of seven actions. This is the difference between the examples and a form that is not hard-coded. |
| `<Show>` inside a row, with resets keyed by row id | Medium | The known emitter bug. Keyed identity is already `state.counts`. |
| Stop copying one color into button hover/down/disabled; map `hover:` and `disabled:` onto the uniforms `Button` already animates | Small | The widget does this. Quinn erases it. |

### P1 — practical parity for an app that is more than the four examples

| Item | Size | Notes |
| --- | --- | --- |
| Fragments, `#[prop]` defaults, and children as script functions | Medium | Slots are the large half. Props without children are the small half. |
| `mod.ctx` for provide/use | Small | Subtree scope needs the children work. |
| In-app navigation: `state.route` and `<Show>` | Medium | Not `leptos_router`. A URL bar is wasm-only and can sit on the same signal later. |
| `grid-cols-*` / gap / span lowered to `widgets.Grid` | Medium | Do not add Taffy for this. |
| `abs_pos` for `left`/`top`, clip and scroll for overflow | Medium | Four-inset stretch stays out until the turtle grows it. |
| Font weight and line spacing via `TextStyle` / `FontMember.weight` | Small | Requires the font files. |
| Two-stop gradient and a shadow on `RoundedView` | Medium | Shader uniforms, not layout. |
| Nested script objects as a stand-in for stores | Medium | Full store-field tracking is P2. |
| `bind:value` that updates a sibling without resetting the caret | Medium | Blocked on a text update that is not `Apply::Reload` of `text`. |

### P2 — do not pretend these are layout work

| Item | Size | Notes |
| --- | --- | --- |
| Resources, `Suspense`, `Transition`, `Action` | Large | A Rust task writing `mod.state` and calling `render`. Script stays synchronous. |
| Server functions and 0.8 websocket server functions | Out of scope | No server and no hydration. Call HTTP from the shell if a screen needs data. |
| `ErrorBoundary` | Medium | Host wrapper around a failed `render`, not a Leptos error hook. |
| `Portal` | Medium | `Flow::Overlay` or a second window. Not a DOM portal. |
| Breakpoints and dark mode | Medium | Signals set by the shell. Not media queries. |
| `align-content`, inset stretch, `z-index` | Large | Upstream turtle. Revisit only after P0 emits what the turtle can already do. |
| Taffy | Not recommended | See section 3. Reconsider only if `Grid` plus the turtle holes still cannot express a screen we have to ship. |

Practical parity is P0 plus the P1 navigation, grid, and children items. It is not a Leptos 0.8 port. The authoring syntax can grow. The runtime stays `mod.state` and `render()`.
