# Gap analysis

P0, P1, and P2 from section 4 are implemented. Status is against the running UI: `view.rs` lowered to Makepad script (`hot/ui.rs`, `mod.state`, `on_render`). The headless Rust graph (`quinn-core`, `MemorySurface`, `reactive_graph`) is named when it can do something the window cannot.

Sources checked in this tree and in the pinned Makepad checkout `44ecb822`:

- Quinn authoring: `crates/macros/src/lib.rs`, `crates/lower/src/parse.rs`, `crates/lower/src/emit.rs`, `crates/styles/src/lib.rs`, `crates/reactive/src/lib.rs`
- Quinn deps: `Cargo.lock` has no `taffy`. `crates/makepad/Cargo.toml` depends on `makepad-widgets` at `44ecb822` with default features off.
- Turtle: `draw/src/turtle.rs` (`Walk`, `Layout`, `Flow`, `Size`, `Align`, `Distribute`, `RowAlign`)
- Grid widget (not the turtle): `widgets/core/src/grid.rs`
- Text and draw: `draw/src/shader/draw_text.rs` (`TextStyle`, `FontMember.weight`), `widgets/core/src/button.rs` (hover/down animator uniforms)
- Leptos 0.8.0 release notes (https://github.com/leptos-rs/leptos/releases/tag/v0.8.0). 0.8 keeps the 0.7 authoring model. The additions that matter here are store fields implementing `IntoClass` / `IntoStyle` / `AttributeValue`, `Suspend::new` accepting `IntoFuture`, and websocket server functions. Quinn's `reactive_graph` pin is `5ca3a9bf` (`docs/compatibility.md`), the 0.7-era graph, and the examples do not execute it.

`docs/research.md` describes an earlier turtle (row, column, spacing, padding, `Align`, `Fill`/`Fit`/`Fixed`). The pin is past that. Wrap, overlay, `abs_pos`, fill weight/shrink/basis, `Distribute`, min/max, relative sizes, and a `Grid` widget are in `44ecb822`. Quinn now emits the P0 slice of that (wrap on a row, `Distribute`, `RowAlign`, fill weight/shrink/basis, min/max, `Size::Rel`), the P1 slice (`widgets.Grid`, clip, scroll), and the P2 slice (parent-relative `Flow::Overlay` frames, stacking order). `abs_pos` is not emitted. `align-content` is still not a turtle field.

## 1. Leptos syntax parity

Legend: **supported** means a `view.rs` the lowerer accepts and a running example can use. **partial** means a slice works, or only the headless graph has it. **missing** means the lowerer rejects it or it has no runtime.

### `view!` and components

| Feature | Status | On a script runtime |
| --- | --- | --- |
| Elements, string literals, `{move \|\| signal.get()}` text | Partial | The lowerer accepts a closed set of text shapes (`Part` in `crates/lower/src/model.rs`): literals, one signal, `if bool`, `if empty`. A general closure is rejected. Widening the expression parser and emitting script expressions is local to `quinn-lower`. |
| Fragments and `<>...</>` | Supported | A fragment is flattened into the parent. It is not a widget. |
| Nested components | Supported | Any `#[component]` the lowerer can parse is a call. The lowerer does not special-case names. Generics are not monomorphized. |
| `children` / slots | Supported | `{children()}` is `fn quinn_slot_N`. `{children()(expr)}` is a `ChildrenFn` called with that expr (`arg` inside the slot). A script function returns one widget, so the returned widget is anonymous (`name :=` does not yield). Several children share one `Fit` column. Inside a `<For>` row the children stay inlined in the row function. |
| `#[prop(optional)]`, `default`, `into` | Supported | Omitted optional props are `""`. `default = "..."` fills when the attribute is missing. `into` is accepted; literals and signals lower. There is no Rust `Into` conversion in the script. |
| Generic components | Missing | Script functions are untyped. A generic component lowers only if monomorphized at the call site into a concrete script function. The lowerer does not do that. Medium, and only for types the seed can represent. |

### Signals

| Feature | Status | On a script runtime |
| --- | --- | --- |
| `signal()` + `.get()` / `.set()` / `.update()` | Partial | Parsed for `let (name, set_name) = signal(...)` with int, bool, `String`, and `vec![Item ...]` (`parse.rs`). The window stores the value on `mod.state` and assigns it. There is no subscriber. A write calls `render()`. `.with()` is not parsed. |
| `RwSignal`, split read/write, `ReadSignal` | Missing | The re-export is whatever `reactive_graph` prelude has, and only compiled tests can call it. Script would be the same `mod.state` field with two names. Small once expressions exist. |
| `Memo`, derived closures | Missing | Text `if` is the only derived form, and it is recomputed inside `on_render`, not cached. A memo is a script function of `state` called from `on_render`. No dependency tracking. Medium for a real expression language; small if the set of expressions stays closed. |
| `Effect` / `RenderEffect` | Partial | Headless code has `ImmediateEffect` (`crates/reactive/src/lib.rs`) because Makepad has no async executor (`docs/decisions.md`). The window has no effect. A script effect is "the setter writes state and calls `render()`". An effect that must run without rebuilding the tree needs a script function invoked from the setter, not a subscriber list. Medium, and it will not match Leptos's "track what you read". |
| Resources, `async`, `Suspense`, `Transition`, `Action` | Partial | `LocalResource::new`, `Resource::new`, and `Action::new` name a compiled async fn. A tokio worker polls that future. `SignalToUI` delivers the `Result` onto `mod.state` phase fields. `<Suspense>` / `<Transition>` are script gates. Not `Suspend::new(IntoFuture)`, and not a server. The fn body is Rust, so editing it needs a rebuild. |
| `on_cleanup` | Partial | Re-exported and used by the headless owner. Script unmount is "this `on_render` did not return the widget", and `<Show>` zeroes the child's fields (`emit.rs` `emit_show_fn`). There is no user `on_cleanup` hook. A script callback stored on `mod` and called from the show's off branch is small. |

### Control flow

| Feature | Status | On a script runtime |
| --- | --- | --- |
| `<Show when>` | Partial | Works at the host and inside a `<For>` row. Two functions: `_on` returns the child, `_off` returns `0`. A statement `if` in `on_render` unbalances the script stack, which is why it is functions. Fallback is the off branch, not a `<Show fallback>`. A row show is `row_quinn_show_<signal>_<n>(id, …)` and its flag lives on `state.r_<flag>["k"+id]`. Removing a row does not clear that id; pushing the id again calls `quinn_row_reset`. |
| `<For each key let>` | Partial | Keyed counts live on `state.counts["k"+id]` because anonymous children match by index (`docs/architecture.md`). The row template is one element. Add, pop, and reverse are hard-coded actions. A general `each()` closure is not. Indexed `For` without a key would be the same loop and would lose per-row state on reorder, which is the turtle's behavior, not a bug to paper over. |
| `<Suspense>` / `<Transition>` | Partial | A gate function. Loading with an empty value shows the fallback. `<Transition>` keeps the previous value on screen beside the fallback. The value is whatever the compiled future returned. |
| `<ErrorBoundary>` | Partial | Shows the fallback when a child job's phase is `"error"`, including `Err` from the future and a panic caught around `poll`. It does not catch a script panic or a failed `on_render`. |
| `<Portal>` | Partial | Lifted to a later sibling of the app view under `flow: Overlay`, so it paints above the page. Not a DOM mount and not a second OS window. |
| `<DynamicChild>` / `<{expr}>` | Missing | The macro has no dynamic tag. Script would store a tag name on `state` and branch in `on_render` with the same function-per-branch pattern as `<Show>`. Medium, and only for tags the lowerer knows. |

### Attributes

| Feature | Status | On a script runtime |
| --- | --- | --- |
| `class="..."` and `style="..."` | Partial | Static strings, `class:token=move \|\| flag.get()`, and `style:prop=move \|\| if flag.get() { "a" } else { "b" }`. Unknown utilities are ignored. `class:` toggles one utility against the static style. `style:` requires two literals; a runtime string is a compile error. |
| `prop:` / `attr:` | Missing | There is no DOM attribute map. A known prop (`placeholder`, `value`) is already a dedicated attribute. Anything else needs a named script field on that widget. Per-widget allowlist, small for each widget, not a general `attr:` namespace. |
| `on:` with event types | Partial | `on:click`, `on:input`, and `on:change` only. A click body may be several statements: set or update a signal with `+ - * /`, boolean not, or a string, and push, pop, reverse, swap, or remove a list by item id. Anything else is a compile error (`cannot lower`). `MouseEvent` does not exist. Makepad gives the button no event object; `TextInput` passes the string. |
| `bind:value` | Supported | `bind:value=name` or `bind:value=profile.name` omits `text:` on the `TextInput`, so `Apply::Reload` does not rewrite the field and the caret stays. `on_change` still assigns the signal and calls `root.render()`, and a sibling that reads the signal updates. `value=` plus `on:input` (`examples/form`) still writes `text:` and the caret can jump. |
| `node_ref` | Missing | Script widgets already have names (`h1 := H1`). A ref is that name, read back with `script_eval!` as `ui.host.h1`. Exposing it as a Rust `NodeRef` crosses back into the shell and does not hot-reload. Small as a script name; a Rust ref is the wrong direction. |
| Spread `{..attrs}` | Missing | No attribute bag. Would be a script object merged into the widget literal. Medium, and only keys the lowerer knows survive. |

### Context, router, stores, server functions

| Feature | Status | On a script runtime |
| --- | --- | --- |
| `provide_context` / `use_context` | Partial | `mod.ctx` is one process-global object. The seed is `if ok{mod.ctx.key} == nil`, so the first value wins and a hot reload does not reset it. A nested provider does not shadow. It is not a Leptos reactive scope. `{accent}` reads `mod.ctx.accent`. A field read such as `{theme.ink}` reads `mod.ctx.Theme.ink` when that local is the context key. |
| Router (`Routes`, `Route`, nested routes, `A`, params, navigate) | Partial | In-app only. `state.route` is a string. `<A href>` and `navigate("/form")` assign it and call `render()`. Each route is `fn quinn_route_N` choosing `_on` or `_off` (the off branch returns `0`). This is not `leptos_router`: no URL bar, no history, no params, no nested route matching. |
| Stores (`reactive_stores`, 0.8 `IntoClass` for store fields) | Missing | A store field is a nested script object (`state.user.name`). Updates are assignments plus `render()`. Fine-grained field tracking is the subscriber list script does not have. Medium for nested objects; large if a field write must skip unrelated widgets. |
| Server functions, `#[server]`, 0.8 websocket server functions | Out of scope | They compile to an HTTP or websocket endpoint and assume a Leptos server and, for hydration, a DOM. Quinn has no server, no request codec, and no hydration. A native app that calls an HTTP API is an ordinary Rust client writing `mod.state` when the response arrives (the resource bridge). Reimplementing `server_fn` would drag in the Leptos server stack this project refused. |

## 2. Tailwind on the turtle

Quinn's parser is a closed utility list (`crates/styles/src/lib.rs`). "Supported" means that list emits something today. "Feasible" means `44ecb822` can do it and Quinn does not emit it. "New capability" means it is not a turtle walk, not the `Grid` widget, and not an existing draw uniform.

`flex` / `flex-row` emit `flow: Right`. `flex-col` emits `flow: Down`. `justify-start|center|end` is `Align{x y}` on the main axis. `items-center|start|end` on a row is `RowAlign` (`turtle.rs` line 3009), not `Align.y`. `Align` is still one fraction of leftover space (`turtle.rs` around line 2939) and is what a column uses for `items-*`.

| Category | Status | Maps to | What it would take |
| --- | --- | --- | --- |
| Layout: row and column | Supported | `Layout.flow` `Flow::Right` / `Flow::Down` (`turtle.rs` 2959) | Already emitted. |
| Layout: wrap | Supported on a row | `Flow::Right { wrap: true }`, `Layout.wrap_spacing` (`flow_right_wrap`, line 2785) | Emitted. `Flow::Down` has no wrap, so a column does not wrap. |
| Layout: flex grow / shrink / basis | Supported | `Size::Fill { weight, basis, shrink, min, max }` (`turtle.rs` 311). Bare `Fill` is still weight 100, shrink 0. `flex-1` / `grow` / `shrink` / `basis-*` emit the fields. | Done for the main axis. |
| Layout: space-between / around / evenly | Supported | `Distribute` (`turtle.rs` 3036): `SpaceBetween`, `SpaceAround`, `SpaceEvenly`. | Emitted. Main-axis slack between walks, not a second flex pass. |
| Layout: per-item cross alignment | Supported for start/center/end | `RowAlign::Center` / `Bottom` / `Top` | Emitted on rows. Stretch is cross-axis `Fill`, because `RowAlign` has no stretch. |
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
| States: hover, focus, active, disabled | Partial | Button `color_hover`, `color_down`, `color_focus`, `color_disabled`. A `div` with `hover:` gets an `Animator` that swaps `draw_bg.color`, because `RoundedView`'s shader reads `self.color` and `View` plays `hover.on` / `down.on`. `focus:` is `color_focus` on `Button` and `TextInput` only. `View` does not play `focus.on`. |
| Responsive breakpoints | Partial | `sm:` 640, `md:` 768, `lg:` 1024, `xl:` 1280 read `state.quinn_width`. `dark:` reads `state.quinn_dark`. `drive_shell` sets the width from `WindowGeomChange`. `toggle_theme()` flips the theme. Not a media query. |
| Dark mode | Feasible as a signal | No color scheme in the turtle. | `state.dark` picked by the shell, two colors in the class string. Small once `class:` exists. It will not follow the OS unless the shell reads it. |
| Arbitrary values `w-[13px]`, `bg-[#abc]` | Missing | The inline parser already accepts `width: 13px` and `#rrggbb` (`parse_inline`). | Accept `w-[13px]` in `apply_token`. Small. `bg-[url(...)]` is not a color and stays missing. |

Tailwind, used as a class string that means "row, column, gap, padding, type scale, a flat color, a radius", fits the turtle. Tailwind used as "the CSS layout spec, including grid areas, arbitrary selectors, and variants" does not fit in the parser. Wrap, grow, and a column grid are emitted. Overlay as a flow is not.

## 3. Layout engine

Quinn does not use Taffy.

- `Cargo.lock` has no `taffy` dependency.
- `docs/architecture.md`: "No Taffy".
- `docs/decisions.md` records leptos-native's Taffy and rejects it because the turtle covered the milestone's subset.
- Emitted layout is flow, spacing, wrap spacing, padding and margin as `Inset` or a number, `Align`, `Distribute`, `RowAlign`, and walk size (`emit.rs`).

The turtle is the layout engine. It walks children in one pass and resolves `Size` (`draw/src/turtle.rs`, the comment at `Layout` line 2733: "Specifies how walks should be laid out with respect to each other").

What the turtle cannot do, from the fields that exist and the ones that do not:

| CSS / Tailwind | In the pin? |
| --- | --- |
| `flex-wrap` | Yes. Emitted as `Flow::Right { wrap: true }` and `wrap_spacing`. Not on `Flow::Down`. |
| `flex-grow` / `flex-shrink` / `flex-basis` | Yes. Emitted as `Fill { weight, shrink, basis }` on the main axis. |
| `justify-between` and the space-* distributions | Yes. Emitted as `Distribute`. |
| `align-items` for mixed heights | Yes. Emitted as `RowAlign` on a row. Stretch is cross-axis `Fill`. |
| `align-content` (packing wrapped lines on the cross axis) | No field on `Layout`. `wrap_spacing` is a constant gap, not `align-content: space-between`. |
| CSS grid | Not a turtle flow. `Walk.cell` is ignored unless a grid consumer reads it. `widgets/core` `Grid` is a separate widget with `fr` tracks, spans, and named areas. No subgrid, no `minmax()` as a first-class track (tracks are px, percent, fr, or a size expression). |
| Absolute positioning with a containing block and all four insets | `abs_pos` is one point. There is no `position: absolute; left: 0; right: 0` rule. |
| `position: fixed` against the viewport | No. A fixed layer would be a child of the root turtle, or `abs_pos` in root coordinates. The pin does not have a fixed containing block. |
| `z-index` | No. |
| Multi-line flex `align-content`, baseline grid, masonry | No. |

Adding Taffy is not justified for wrap, grow, space-between, or grid. Those already exist in this pin, and the lowerer emits the P0 and P1 slice of them. A second engine would:

- Run a full measure/layout pass beside the turtle, then push rectangles back onto widgets the turtle also wants to walk. One of the two must win, or widgets get laid out twice.
- Hot reload stays a script `render()` only while positions are turtle fields. A Taffy pass has to run after every `render()` or the new tree keeps the old rectangles. That is another step on the UI thread on every keystroke that already calls `render()`.
- Binary size and the compile the milestone has been measuring go up by the Taffy crate and the glue. The published incremental rebuild of `counter` is 1.30 s median with Makepad already built (`docs/benchmarks.md`). Taffy itself is small next to Makepad; the cost that matters is the second layout on the hot path, not the compile.
- It does not help script hot reload. Taffy does not understand `class=` strings. The lowerer would still parse classes, then feed numbers to Taffy instead of to `spacing` and `flow`.

Extending the turtle upstream is justified only for the holes: `align-content` of wrapped lines, a real inset box (left and right both set), and `z-index`. Those are Makepad changes, then a Quinn emitter change. They are not a reason to vendor Taffy. `grid-cols-*` already lowers to `widgets.Grid`.

## 4. Roadmap

Sizes are scope, not a schedule. Small: parser and emitter, turtle fields that already exist. Medium: new script patterns, tests, and a widget the lowerer does not emit yet. Large: a runtime beside the script VM, or an upstream turtle change.

### P0 — done

Shipped in `quinn-styles`, `quinn-lower`, and `examples/tailwind-gallery`. `examples/keyed-list` has a `<Show>` inside each row.

| Item | What landed |
| --- | --- |
| Wrap, `Distribute`, `RowAlign`, fill weight/shrink/basis, min/max, `Size::Rel` | `flex-wrap` sets `Flow.Right { wrap, row_align }` and `wrap_spacing` from `gap-y` (or `gap`). `justify-between/around/evenly` set `Distribute` and do not also set `Align`. `items-center/start/end` on a row are `RowAlign`. `flex-1`, `grow`, `shrink`, `basis-*` become `Fill { weight, shrink, basis }` on the parent's main axis unless that axis is an explicit px or `Rel`. `min-*` / `max-*` are walk bounds (`max-w-full` is `"100%"`). `w-1/2` and the other fractions are `Rel { base: Base.Parent }`. `w-screen` / `h-screen` are `Base.Vw` / `Base.Vh`. |
| Per-side padding and margin | `px` `py` `pt` `pr` `pb` `pl` and the `m*` forms emit `Inset { left top right bottom }` when the sides differ, and a bare number when they match. `p-4` no longer replaces a later `px-*` inside one class string; later tokens still win per side. |
| `class:` / `style:` and arbitrary values | `class:token=move \|\| flag.get()` is a script `if` around the properties that token changes. `style:prop` must be `if signal.get() { "literal" } else { "literal" }`. `w-[13px]`, `bg-[#1e293b]`, `text-[#fff]`, `p-[10px]` and the other arbitrary spacing, size, and color forms parse. 3-digit hex expands. |
| Closed handler bodies | Several statements lower to script: arithmetic, boolean not, string assign, and list `push` / `pop` / `reverse` / `swap(id, id)` / `remove(id)` / `retain(\|row\| row.id != id)`. A handler outside that set fails with `cannot lower`. |
| `<Show>` inside `<For>` | Row shows are separate functions (`row_quinn_show_…`) and do not consume the host show cursor. The first integer row signal stays `state.counts["k"+id]` (`quinn_row_bump`). Every other row signal and the show's live flag is `state.r_<name>["k"+id]`. `quinn_row_reset` runs on push, including a re-add of an id that left. Pop does not clear the id, so a removed row's count is still there until that id is pushed again. |
| Button hover, down, disabled | `hover:bg-*` / `hover:text-*` set `color_hover`. `active:` sets `color_down`. `disabled:` sets `color_disabled`. A bare `disabled` also sets `enabled: false` and `draw_bg.disabled` / `draw_text.disabled` to `1`, because `enabled: false` alone does not start the disabled animator. The face color is not copied into hover, down, or focus. |

Limits that are still true after P0:

- `self-start` / `self-center` / `self-end` are a wrapper `RoundedView`. The turtle has no per-item `RowAlign`, and `Walk.cell` is read by the `Grid` widget only.
- `items-stretch` and `self-stretch` set `Fill` on the cross size when the class did not set that axis. `RowAlign` has no stretch variant.
- `flex-wrap` is a row. `Flow::Down` has no `wrap` field, so `flex-col flex-wrap` stays a column and does not wrap.
- `hover:` and `active:` are button animator colors. A `div` does not run that animator, so `hover:bg-*` on a `RoundedView` does not change the face.
- `focus:` is not mapped. The theme focus color is left alone.
- `align-content`, a four-side inset stretch, `z-index`, and Taffy are still out. Taffy is still not recommended.

### P1 — done

Shipped in `quinn-styles`, `quinn-lower`, `examples/app-shell`, and the bottom of `examples/tailwind-gallery`. `examples/app-shell` is the multi-page shell. It is not `leptos_router`.

| Item | What landed |
| --- | --- |
| Fragments, `#[prop]`, children | `<>...</>` flattens. `#[prop(optional)]`, `default`, and `into` parse. `{children()}` and `{children()(expr)}` are script functions (`quinn_slot_N`). The function's return value is an anonymous widget. Several children share a `Fit` column. Components are looked up by name. `Card` is not special-cased. |
| `mod.ctx` | `provide_context` / `use_context` seed `mod.ctx` once (`ok{mod.ctx.key}`). The value survives `render()` and a hot reload. |
| In-app router | `<Router>`, `<Routes>`, `<Route path>`, `<A href>`, and `navigate(path)` are `state.route` plus `quinn_route_N` / `_on` / `_off`. |
| Grid | `grid grid-cols-N gap-*` emits `widgets.Grid` with `columns`, `column_gap`, `row_gap`, and `cell: CellPlacement{col_span: N}`. |
| Absolute and overflow | `absolute` plus `left-*` / `top-*` (including `left-[16px]` / `top-[176px]`) sets `abs_pos: vec2(x, y)`. `overflow-hidden` sets `clip_x` / `clip_y`. `overflow-y-auto` / `overflow-x-auto` set `scroll_bars: ScrollBars{...}`. |
| Type | `font-medium` 500, `font-semibold` 600, `font-bold` 700 emit a `TextStyle` on RobotoFlex (`FontMember.weight`). `leading-*` sets `line_spacing`. The example binaries declare `ROBOTO_FLEX_FONT_ASSET` in `app_main!`. |
| Gradient and shadow | `bg-gradient-to-r from-* to-*` sets `draw_bg.color`, `color_2`, and `gradient_fill_horizontal`. `shadow` / `shadow-md` / `shadow-lg` select `RoundedShadowView` (`shadow_radius`, `shadow_offset`, `shadow_color: #0006`). A gradient without a shadow stays `RoundedView`. |
| Stores | `signal(Profile { name, city })` seeds `state.profile: {name, city}`. `set_profile.update(\|p\| { p.city = "Cebu".into(); })` assigns `state.profile.city`. `{profile.city}` reads it. |
| `bind:value` | The input omits `text:`. `on_change` writes the signal and calls `root.render()`. The sibling label updates. The caret stays where it was clicked. |

Limits that were still true after P1. P2 changes context shadowing, `#[prop(into)]`, parent-relative `absolute`, `hover:` on a `div`, `focus:` on buttons and inputs, and `z-index`. See the P2 section for the current rule.

- `mod.ctx` is one object for the process. After P1 a nested `provide_context` did not shadow. P2 assigns and restores around the widget. The root seed is still nil-guarded, so editing the root provider literal does not reset a value that is already there.
- The router is not `leptos_router`. There is no URL bar, no history API, and no route params.
- A script function returns one widget. The widget a slot or route returns is anonymous, because `name := Widget` does not yield the widget. Named children inside that widget keep their names. Children inside a `<For>` row are inlined, not a separate slot function.
- `#[prop(into)]` unwraps a string literal's `.into()` / `.to_string()`. A path that is not a signal is an error. It is not a Rust `Into` impl.
- `widgets.Grid` draws with `DrawQuad`. A gradient, a shadow, or `scroll_bars` on the grid div itself is not emitted. Put those on a child `RoundedView` / `RoundedShadowView`. Implicit rows are a fixed pixel size (the tallest explicit child height, otherwise 40). This Grid rejects `auto` tracks, so there is no content-sized row. Every implicit row shares that one size, and the default `align_items` stretches the cell to it.
- P1 emitted `abs_pos`, which is one point in draw space. P2 stopped emitting it. `left` / `top` / `right` / `bottom` / `inset-0` are padding and align on a parent-sized overlay frame. `overflow-hidden` still sets `clip_x` / `clip_y`.
- The stock `RoundedView` shader has gradient uniforms and no shadow uniforms. A shadow selects `RoundedShadowView`, which has the same gradient uniforms. `shadow_color` is `#0006`.
- Weights use RobotoFlex. The theme face named `font_bold` is IBM Plex SemiBold, a static file, so Quinn does not switch to it for `font-bold`. The example must list `ROBOTO_FLEX_FONT_ASSET` or the glyphs come out as boxes.
- `bind:value` is the path that skips `text:`. `value=` with `on:input` still writes `text:` and can still move the caret.
- Column wrap, `align-content`, and Taffy stay out. `hover:` on a `div`, `focus:` on a button or input, and `z-index` landed in P2.

### P2 — in, with the limits below

`examples/async-dashboard` is the screen. `drive_shell` (end of `handle_event`) is the executor. Script stays synchronous.

| Item | What landed |
| --- | --- |
| `LocalResource` / `Resource` / `Action` | The body is `delay_ms(N); Ok("literal")` or `Err("literal")`. Fields: `{name}_phase`, `{name}_value`, `{name}_error`, `{name}_gen`. An action also has `{name}_pending` and `{name}_arg`. `Resource` stores `{name}_source` and `quinn_sync_jobs()` restarts it when that signal changes, without clearing the previous value. |
| `<Suspense fallback>` / `<Transition fallback>` | `quinn_gate_N`. Loading and an empty value show the fallback. A transition with a value already set shows the fallback and the child. |
| `<ErrorBoundary fallback>` | `quinn_err_N` when a child job's phase is `"error"`. |
| `<Portal>` | When the tree has one, `host` is `flow: Overlay` and the page is `app`. Portal children are later siblings, so they paint on top. A closed `<Show>` returns `0` and does not eat hits. A `<Show>` does not reset the signal it is watching, so a portal can stay open. Signals written inside the branch still reset when that branch opens. |
| `sm:` / `md:` / `lg:` / `xl:` / `dark:` | `state.quinn_width` (seeded, then `WindowGeomChange`) and `state.quinn_dark` (`toggle_theme()`). Both sit inside the `mod.state` nil-guard, so a hot reload keeps a resized width and a toggled theme. |
| Parent-relative position | An `absolute` child is wrapped in a `Fill` / `Fill` `flow: Overlay` frame. Padding is `left` / `top` / `right` / `bottom`. `align` is `1` only when that edge is set and the opposite edge is not. `inset-0` is zero padding plus `w-full` / `h-full` on the child. `abs_pos` is not emitted. |
| `z-index` | `z-0` through `z-50`, `z-N`, `z-[n]`. Siblings are painted in order. Negative `z` is before the in-flow box; the rest are after. There is no turtle z field. |
| Nested `provide_context` | The provider assigns `mod.ctx.key` and restores the previous value around the widget. The root seed stays nil-guarded. |
| `#[prop(into)]` | `"Ada".into()` / `.to_string()` becomes the literal. A path that is not a signal is an error. |
| `hover:` on a `div` | `cursor: MouseCursor.Arrow` plus an `Animator` that writes `draw_bg.color`. The down track's resting color is the base color, so the first frames are not the hover swatch. `focus:` sets `draw_bg.color_focus` on `Button` and `TextInput`. |

Limits that are still true after P2, except where P3a below replaces the job body:

- `<ErrorBoundary>` does not catch a script error. A failed `on_render` still discards the frame. A panic inside a job future is an error phase (P3a); a panic in the frame is not.
- Server functions, websocket server functions, hydration, and `leptos_router` are out. No server.
- `align-content` is not implemented. Turtle `align` is per item, and `wrap_spacing` is a constant gap. Quinn does not vendor or patch Makepad for this.
- An `absolute` child of a `Grid` is positioned inside the cell. The grid is not rewritten into an overlay, because that would drop the columns.
- `focus:` on a `div` does nothing. `View` plays hover and down, not focus.
- The overlay frame that positions an `absolute` child does not set `cursor`, so it does not steal hits. The child does when it has an animator (`hover:`) or is a button.
- `Flow::Overlay` uses the parent's align for every overlay child. Per-child align is the frame's align, computed from which edges were set.
- Taffy stays out.

Practical parity is P0 plus P1 plus this P2 slice, with P3a running the futures. It is not a Leptos 0.8 port. The runtime stays `mod.state`, `mod.ctx`, and `render()`. `drive_shell` reads script numbers with `as_number`, because an integer literal is not an `f64`.

### P3a — real async fns

`view.rs` is still not compiled. The lowerer reads one function name and one argument expression out of the closure. `quinn::job("fetch_note", ...)` registers the future. `examples/async-dashboard/src/jobs.rs` is that registry. A two-thread tokio runtime in `quinn-makepad` polls it. Completion pushes a `JobValue` and calls `SignalToUI::set_ui_signal`. On `Event::Signal`, `mod.quinn_apply` writes `phase` / `value` / `error` only when `state.{name}_gen` is still that generation and `quinn_job_live(name)` is 1.

| Item | What landed |
| --- | --- |
| Call shape | `LocalResource::new(move \|\| fetch_note(id.get()))`, `Resource::new(\|\| city.get(), \|city\| async move { fetch_place(city).await })`, `Action::new(\|place: &String\| { let p = place.clone(); async move { save_place(p).await } })`. One argument. `let p = place.clone()` is an alias of the closure parameter. |
| `dispatch` / `refetch` | `dispatch(arg)` stores the argument and bumps `gen`. The future receives that argument. `refetch()` sets phase `"loading"`, bumps `gen`, and keeps the previous value. |
| Source tracking | Signals read by the resource source (and by `signal.get()` in a local body) are `deps`. `quinn_sync_jobs` compares them to `{name}_source` and restarts without clearing the value. |
| Races | `STARTED` is `(binding, gen)` on the UI thread and is not cleared by hot reload. A newer gen makes `quinn_job_gen` non-zero, so a new task starts. The old result fails the gen check and is dropped. A binding removed by the reload fails `quinn_job_live` and is dropped. |
| Values | `String`, numbers, bools, and serde structs / `Vec` become script values through JSON (`parse_json`). A list of objects is `state.{name}_value`, which `<For each=move \|\| cities.get()>` walks. `{city.name}` is the row field. |
| Panics | `poll` runs inside `catch_unwind`. The payload becomes `Err("panic: ...")` and the phase is `"error"`. |
| HTTP | Feature `net` plus `QUINN_HTTP=1` GETs `https://jsonplaceholder.typicode.com/users` with ureq on `spawn_blocking`. Failure, or a build without the variable, uses Manila / Cebu / Davao. |

Limits that are still true after P3a:

- The async fn is compiled Rust. Changing `jobs.rs` needs a rebuild. Changing markup or classes in `view.rs` hot-reloads, and an in-flight future is not started a second time.
- The closure may call one function with one argument. There is no `.await` of an inline block that is not a named fn, and no multi-argument call.
- `JobValue` is the value language (string, number, bool, list, object). It is not an arbitrary Rust type stored in the script heap.
- `<ErrorBoundary>` still does not catch a script panic or a panic outside the job future.
- No cancellation token is dropped into the future. A stale gen is ignored when it finishes; the worker still runs it.
- Server functions, websockets, and hydration are out. The HTTP path is one optional GET.

A `<For>` that is the only child of a suspense or boundary function is wrapped in a `RoundedView`. A bare `for` emits into the surrounding widget literal; a function has none, so the rows would otherwise not mount.

The P2 take, before these futures, was measured 25 Oct 2026, about 7:45 PM Asia/Manila: pid 95336, starttime 3602412, window `0x2200002`, markup 325.3 ms, class 231.7 ms. That binary baked the results. The P3a take, 25 Oct 2026 at 9:44 PM Asia/Manila, pid 101729, starttime 4019518, window `0x2000002`, was 328.1 ms from `fsync` of `"Rail"` → `"Sidebar"` to the first changed frame, with the note still loading. The fetched note was the next change, at 2428.0 ms, in that reloaded tree. Details are in [benchmarks.md](benchmarks.md).
