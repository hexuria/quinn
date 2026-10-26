# Component specifications

Import the library from an app view:

```rust
use quinn::prelude::*;
use quinn_ui::components::*;
```

`components.rs` is authoring source. Cargo does not compile it. The host passes `quinn_ui::SOURCE` into the lowerer (`load_hot_view_with`). A `view.rs` save reloads in the same process and still resolves these components.

Call-site `on:click`, `class`, `class:`, `bind:value`, and `placeholder` are forwarded. `variant` and `disabled` are props. A text child becomes the button label. Element children fill `{children()}`. Several element children share the parent's row, gap, and alignment. A child that is only an absolute overlay is placed on the absolute layer, which is how the slider's hit targets cover the track.

Every color utility on these components has a `dark:` pair. A light-only face under `dark:text-*` made the toast title disappear.

Keyboard behavior below is the behavior this desktop host can stand behind. Anything else is marked unsupported.

## Button

Anatomy: one label, optional nothing else. Icons are a child only when you pass elements; a text child is the label.

Props: `variant` (`primary`, `secondary`, `ghost`, `danger` or `destructive`, default `primary`), `disabled` (bool, default false), children.

Sizing: `min-h-control` and `h-fit`, so the floor is 28 / 36 / 48 and a taller label can grow. `px-control` is 8 / 12 / 16. Radius 6. Pass `class="w-control h-control"` for a square target.

Spacing: the parent sets the gap between buttons. Use `gap-2` (8).

States: default, hover (`hover:bg-accent-hover` or the surface equivalents), pressed (`active:`), disabled (solid `disabled-bg` / `disabled-fg`, not a fade). Focus is the accent border stroke. Loading is unsupported.

Keyboard: the control is a button. Tab order and a visible keyboard focus ring were not observed on this host. Unsupported until a key test shows the ring.

Accessibility: the label is the visible text. No role or name API. Unsupported for screen readers.

Overflow: a long label is the button’s text run. Wrapping depends on the widget. The gallery also shows a bounded-width button. Do not shrink the font.

Responsive: the button does not change variant with width. Touch density raises the floor to 48.

```rust
<Button on:click=move |_| { set_saved.set(true); }>"Save"</Button>
<Button variant="secondary" disabled=true>"Waiting"</Button>
```

## Field

Anatomy: label, then the input. Gap 8 (`gap-2`). The label is required. Placeholder is extra.

Props: `label` (string), `placeholder` (string, default empty). `bind:value` is forwarded onto the input.

Sizing: the input is `h-control` and `w-full`. Border width 1, color `border-control`, drawn on a 36px (or density) box that is not nested in a shorter clip. Radius 6. Horizontal padding 12.

States: empty placeholder uses muted text (`#5F6F85` / `#94A3B8`). Focus uses the accent stroke. Error is a sibling caption, `text-caption text-danger`, 4px under the field (`gap-1`), plus `border-danger` on an input when you need the error face. Disabled and read-only are unsupported on Field. IME composition is unsupported.

Keyboard: the input accepts pointer text. Caret, selection, and IME were not verified.

Accessibility: the visible label is the name. The placeholder does not replace it. No programmatic label association. Unsupported for screen readers.

Overflow: the field fills its parent width. A long value scrolls inside the input if the widget does; that scroll was not measured.

```rust
<Field label="Folder" placeholder="Folder name" bind:value=draft />
```

## Search

Anatomy: magnifier glyph drawn as geometry, then the input. Same height and border as Field. Placeholder defaults to `Search`.

Props: `placeholder`. `bind:value` forwarded.

A clear button and a shortcut hint are unsupported. Empty, loading, and no-results suggestion lists are unsupported.

## NavRow

Anatomy: a row, `h-control`, padding 12, gap 8, then whatever children you pass (icon slot, label). A transparent button covers the row and receives `on:click`.

Props: children. Selection is `class:bg-selection` and `class:text-on-selection` on the label. Those are different from focus.

Sizing: width fills the parent. The label needs a width (`w-[148px]` or similar) or it sizes to its text and can paint past the rail. Below 1120 the showcase sets the label width to 0 so the 64px rail keeps the icon.

Overflow: a 60-character label in a 160px-wide span wraps. Ellipsis is unsupported; full text stays in the row.

Keyboard: activation is the row button. Arrow-key roving tabindex is unsupported.

## CheckRow, CheckOn, CheckOff

Anatomy: 18px glyph, gap 8, label, row height `h-control`. The row is the target, larger than the glyph.

`CheckOn` is the filled accent box with a light check. `CheckOff` is a surface box with `border-control`. Mixed, disabled, and invalid checks are unsupported. The parent swaps On and Off with `<Show>`. The label is not a separate hit target; the row is.

Keyboard: the row is a button. Space to toggle was not verified.

## SwitchOn, SwitchOff

Anatomy: 40×24 track, 16px thumb, inside an `h-control` row. On parks the thumb at the end on `bg-accent`. Off parks it at the start on `bg-subtle` with `border-control`. The label is text, so the value is not color alone.

Props: `label`. `on:click` toggles the parent signals. Disabled is unsupported.

Keyboard: same as CheckRow. Arrow keys do not apply.

## Stepper

Anatomy: a horizontal row, `gap-2`. Children are the decrement button, the value, and the increment button. Use `Button` with `class="w-control h-control"` so each target is the density minimum, not a stacked 18px pair.

The component does not own the number. The parent clamps:

```rust
<Stepper>
    <Button variant="secondary" class="w-control h-control" on:click=move |_| { set_step.set(clamp(step.get() - 1, 1, 8)); }>"−"</Button>
    <span class="text-body text-primary w-[32px]">{step}</span>
    <Button variant="secondary" class="w-control h-control" on:click=move |_| { set_step.set(clamp(step.get() + 1, 1, 8)); }>"+"</Button>
</Stepper>
```

At the limit, `clamp` keeps the value. The button does not draw a disabled-at-limit face. Unsupported: a disabled end state, typed entry, and keyboard focus on the value.

## Slider

Anatomy: 240px wide, `h-control` tall. Track is 4px, `bg-divider`, with an accent fill. Thumb is 16px, surface with `border-control`. Children sit on top of the track; the showcase uses eight click segments. Minus and plus buttons beside the slider repeat the value for keyboard.

Props: `signal` (default `"level"`). The fill and the thumb spacer bind that signal. The parent declares `let (level, set_level) = signal(50i64)`.

Live dragging is unsupported. The handler language has no pointer-move. Clicking a segment sets the value. Arrow keys do not move the thumb. The minus and plus buttons step by 5 and clamp to 0…100.

## Overlay

Anatomy: surface, 1px divider, radius 12, padding 16, shadow, width 360, `max-w-full`. Placed `right-[16px] top-[64px]`, so it clears the toolbar. Children are the message and the action.

A toast uses it. It does not queue, does not cap at three, and does not flip when it would leave the window. Edge flip is unsupported. The gallery pins four small surfaces 8px from the corners of a frame instead.

Dialogs in the showcase are not this component. They are a scrim plus a `rounded-lg` surface (radius 12) with a scrolling body (`max-h-[240px]`) and actions outside that scroll. Preferred widths 400 and 480. Focus is not trapped. Escape does not close them. Both are unsupported.

## Segmented control

Not a library component yet. The settings view builds it from three `h-control` cells of equal width inside a `bg-subtle` track. The selected cell is a `bg-surface` face with a divider border. The label is a transparent button, so the selected face does not move the text.

Scrollable tabs and a select fallback when labels do not fit are unsupported.
