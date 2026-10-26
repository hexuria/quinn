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

## NavItem

Anatomy: one `QNavItem` widget: a 16-unit icon and the label on a row face. Below 1120 the icon sits over a 12-unit caption in a 56-unit cell; from 1120 the icon sits beside a 14-unit label in a control-height row with 12 of side padding.

Props: `icon` (one of the shipped icons), `selected` (`move || signal.get()`), the label as a text child, `on:click`.

States: clear at rest, a tint under the pointer, the selection fill with the selection ink when selected (icon included). Focus is a 2-unit ring on the row's edge, from the keyboard only.

Overflow: a long label ends in an ellipsis beside the icon. Makepad wraps text only in a wrapping flow, and there the label would drop below the icon.

Keyboard: Tab reaches each row. Arrow-key roving is unsupported.

```rust
<NavItem icon="notes" selected=move || notes.get() on:click=move |_| { set_notes.set(true); }>"Notes"</NavItem>
```

## Checkbox

Anatomy: one `QCheckbox` widget: an 18-unit box (radius 4) at the start, gap 8, the label. The whole row is the target.

Props: `checked` (`move || signal.get()`), the label as a text child, `on:click` (flip the signal there).

States: unchecked is a surface box with `border-control`; checked is the accent box with a drawn check in the on-accent ink; hover deepens either. Focus is a 2-unit ring around the box, from the keyboard. Mixed is unsupported.

Keyboard: Tab reaches it and Space toggles it (checked with the control hook's key events).

```rust
<Checkbox checked=move || checked.get() on:click=move |_| { set_checked.update(|value| *value = !*value); }>"Show in menu bar"</Checkbox>
```

## Switch

Anatomy: one `QSwitch` widget: a 40×24 track with a 16-unit thumb, gap 8, the label. On parks the thumb at the end on the accent track; off parks it at the start on a light track with `border-control`. The label is text, so the value is not color alone.

Props: `checked`, the label as a text child, `on:click`.

Keyboard: Tab reaches it and draws the ring around the track.

```rust
<Switch checked=move || switched.get() on:click=move |_| { set_switched.update(|value| *value = !*value); }>"Sync notes"</Switch>
```

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

## Panel

Anatomy: the `QPanel` template, a raised surface: face, 1-unit hairline, radius 12, and a soft shadow, light and dark. A class sizes, places, and spaces it (`gap-4 w-[480px] p-6`); it does not color it. A panel takes the pointer, so a click on its empty space does not reach what is under it.

Dialogs, the navigation drawer, and toasts sit on it. Dialogs are centered over a scrim that also takes the pointer; Tab stays inside and Escape closes the top one.

## Overlay and Toast

`Overlay` only places its child: `right-[16px] top-[64px]`, width 360, `max-w-full`, so it clears the toolbar. `Toast` is the panel inside it: a check, the message, and Dismiss. `on:click` on `Toast` is the Dismiss action.

It does not queue, does not cap at three, and does not flip when it would leave the window.

```rust
<Overlay>
    <Toast message=toast_msg on:click=move |_| { set_toast.set(false); } />
</Overlay>
```

## Menu and MenuItem

`Menu` is a `QMenuPanel` (the panel with radius 8) placed under the control before it, flipping above it near the bottom edge. Each `MenuItem` is a `QMenuItem` row: clear, tinted under the pointer, a keyboard ring on its edge. While a menu is open it gets every event before the page, so an item over a field takes the click.

```rust
<Button variant="secondary" on:click=move |_| { set_menu.update(|value| *value = !*value); }>{choice}</Button>
<Show when=move || menu.get()>
    <Menu>
        <MenuItem on:click=move |_| { set_choice.set("iCloud".to_string()); set_menu.set(false); }>"iCloud"</MenuItem>
    </Menu>
</Show>
```

## SortHeader

A column header that acts on click: the `QHeader` template, caption type in the secondary ink, tinted under the pointer. `sorted` marks the column the list is ordered by: its caption takes the primary ink and shows the direction after the label, a chevron up for A to Z and down for Z to A (`descending`). The handler sorts with `sort_by` and sets the flags.

```rust
<SortHeader sorted=move || sorted.get() descending=move || sort_desc.get() on:click=move |_| {
    if sort_asc.get() {
        set_rows.update(|rows| { rows.sort_by(|a, b| b.label.cmp(&a.label)); });
        set_sort_asc.set(false);
        set_sort_desc.set(true);
    } else {
        set_rows.update(|rows| { rows.sort_by(|a, b| a.label.cmp(&b.label)); });
        set_sort_asc.set(true);
        set_sort_desc.set(false);
    }
    set_sorted.set(true);
}>"Name"</SortHeader>
```

## Segmented control

Anatomy: `Segmented` is the tinted track (`p-1`, `bg-subtle`); each `Segment` is a `QSegment` widget. Unselected segments are the track, tinted under the pointer; the selected one is a raised surface with a hairline. Equal widths come from the call site (`class="w-[88px]"`).

Props: `Segment` takes `selected` (`move || signal.get()`), the label, and `on:click`.

Keyboard: Tab reaches each segment and draws the ring outside its face. Arrow-key movement inside the group is unsupported. Scrollable tabs and a select fallback when labels do not fit are unsupported.

```rust
<Segmented>
    <Segment class="w-[88px]" selected=move || day.get() on:click=move |_| { set_day.set(true); set_week.set(false); }>"Day"</Segment>
    <Segment class="w-[88px]" selected=move || week.get() on:click=move |_| { set_day.set(false); set_week.set(true); }>"Week"</Segment>
</Segmented>
```
