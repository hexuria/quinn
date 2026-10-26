# Component specifications

Import the library from an app view:

```rust
use quinn::prelude::*;
use quinn_ui::components::*;
```

`components.rs` is authoring source. Cargo does not compile it. The host passes `quinn_ui::SOURCE` into the lowerer (`load_hot_view_with`). A `view.rs` save reloads in the same process and still resolves these components.

Call-site `on:click`, `class`, `class:`, `bind:value`, and `placeholder` are forwarded. `variant`, `disabled`, `selected`, `invalid`, and `step` are props where a component declares them. `Button` and `Field` draw from templates in `crates/makepad/src/widgets.rs` (`QButton*`, `QInput*`): the template owns every face and state, light and dark, and editing it under `--hot` restyles every instance in the running window. A class on them may size and place them, not paint them; `class="bg-red-500"` on a `Button` is a lowering error. A text child becomes the button label. Element children fill `{children()}`. Several element children share the parent's row, gap, and alignment. A child that is only an absolute overlay is placed on the absolute layer, which is how the slider's hit targets cover the track.

Every color utility on these components has a `dark:` pair. A light-only face under `dark:text-*` made the toast title disappear.

Keyboard behavior below is the behavior this desktop host can stand behind. Anything else is marked unsupported.

## Button

Anatomy: one label, optional nothing else. Icons are a child only when you pass elements; a text child is the label.

Props: `variant` (`primary`, `secondary`, `ghost`, `danger` or `destructive`, default `primary`), `disabled` (bool, default false), `selected` (`move || signal.get()`, the segmented-control face), children.

Sizing: height is the control height, 28 / 36 / 48. Side padding is 8 / 12 / 16. Radius 6. Pass `class="w-control h-control"` for a square target. A fixed width (`w-[180px]`) wraps a long label and grows the button. The template's rect carries a 4-unit focus band on every side and a margin of -4, so layout sees only the face.

Spacing: the parent sets the gap between buttons. Use `gap-2` (8).

States: default, hover, pressed (darker than hover; hover holds while pressed), selected, disabled (solid `disabled-bg` / `disabled-fg`, no hover). A click does not take key focus. Loading is unsupported.

Keyboard: Tab moves a 2-unit focus ring, 2 units outside the face, from button to button.

Accessibility: the label is the visible text. No role or name API. Unsupported for screen readers.

Overflow: a long label is the button’s text run. Wrapping depends on the widget. The gallery also shows a bounded-width button. Do not shrink the font.

Responsive: the button does not change variant with width. Touch density raises the floor to 48.

```rust
<Button on:click=move |_| { set_saved.set(true); }>"Save"</Button>
<Button variant="secondary" disabled=true>"Waiting"</Button>
```

## Field

Anatomy: label, then the input. Gap 8 (`gap-2`). The label is required. Placeholder is extra.

Props: `label` (string), `placeholder` (string, default empty), `invalid` (bool, default false: the danger border and focus ring). `bind:value` is forwarded onto the input.

Sizing: the input is the control height and fills its parent. Border 1, `border-control`. Radius 6. Side padding 12. The text is centered at every density.

States: empty placeholder uses muted text (`#5F6F85` / `#94A3B8`). Focus is a 2-unit ring on the field's edge, accent or, when invalid, danger. Selection is an accent tint that fades with focus. Error text is a sibling caption, `text-caption text-danger`, under the field. Disabled is read-only with the disabled face. IME composition was not verified.

Keyboard: typing, arrows, and select-all were checked with real keystrokes. `bind:value` is controlled: each render hands the field its own text, and a shorter text (a cleared field) clamps the caret.

Accessibility: the visible label is the name. The placeholder does not replace it. No programmatic label association. Unsupported for screen readers.

Overflow: the field fills its parent width. A long value scrolls inside the input if the widget does; that scroll was not measured.

```rust
<Field label="Folder" placeholder="Folder name" bind:value=draft />
```

## Search

Anatomy: magnifier icon over the start of a `QInputSubtle` field (the tinted face), with 36 units of start padding. Same height, border, and states as Field. Placeholder defaults to `Search`.

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

Props: `step` (`move || signal.get()`, default 1). `bind:value` is forwarded.

The value moves by `step` (Shift and an arrow for ten steps) and snaps to its multiples. The showcase passes the Copies stepper: with Copies at 5, the right arrow moved Level from 50 to 55 to 60.

## Overlay

Anatomy: surface, 1px divider, radius 12, padding 16, shadow, width 360, `max-w-full`. Placed `right-[16px] top-[64px]`, so it clears the toolbar. Children are the message and the action.

A toast uses it. It does not queue, does not cap at three, and does not flip when it would leave the window. Edge flip is unsupported. The gallery pins four small surfaces 8px from the corners of a frame instead.

Dialogs in the showcase are not this component. They are a scrim plus a `rounded-lg` surface (radius 12) with a scrolling body (`max-h-[240px]`) and actions outside that scroll. Preferred widths 400 and 480. Focus is not trapped. Escape does not close them. Both are unsupported.

## Segmented control

Not a library component yet. The settings view builds it from three `h-control` cells of equal width inside a `bg-subtle` track. The selected cell is a `bg-surface` face with a divider border. The label is a transparent button, so the selected face does not move the text.

Scrollable tabs and a select fallback when labels do not fit are unsupported.
