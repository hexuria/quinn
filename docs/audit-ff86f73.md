# Audit at ff86f73

Commit: `ff86f73585944fa8f48b3ba5d65d8364a9b1743b`  
Source of truth after this audit: [design-system-prd.md](design-system-prd.md)  
Screenshots of that commit (1440×900, Linux, llvmpipe): `docs/images/audit/`

The PRD supersedes the Apple HIG visual system wherever they differ. These notes record what that commit actually shipped, not what the HIG pass intended.

## Screenshot-backed issues

| # | Where | What the shot and the source show | Contract it misses |
| --- | --- | --- | --- |
| 1 | Settings, text field | `docs/images/audit/settings-light.png`. The field is a 28px box inside another 28px box (`examples/showcase/view.rs`, the “Folder name” input). The 1px border is the control’s outer edge, so the lower stroke sits on the clip of that fixed height. Placeholder ink is `label-tertiary` at alpha 77. | Field border must be complete. Placeholder does not replace the label, and it must stay readable. |
| 2 | Settings, control sizes | Same shot. Segmented cells are 72×22, buttons and the popup are 28px tall, the stepper halves are 18px, the checkbox glyph is 14px, the switch is 44×26, the slider hit row is 20px. `h-control` in `quinn-styles` is 22px. | Standard controls are 36px tall. Stepper buttons are full targets, side by side. Checkbox glyph is 18. Switch visual is 40×24. Slider track is 4, thumb is 16, hit target is the density minimum. |
| 3 | Settings, labels | Group captions (`text-subheadline`, 11px, `label-secondary`) sit in a `gap-4` (16px) stack with the control. The field caption is not tied to the input by an 8px label-to-control gap. | Label-to-control gap is 8. The parent owns sibling gaps. |
| 4 | Notes and Settings, secondary text | Captions use `text-label-secondary`. Light value is `rgba(60,60,67,0.60)`. Composited on white that is rgb(138,138,142). Contrast against white is **3.44:1**. Against `#1C1C1E` the dark pair is **5.94:1**. Measured with WCAG relative luminance, not estimated from the screenshot. 3.44:1 fails 4.5:1 for normal text. The dark pair passes. | Secondary text is `#475569` / `#CBD5E1`. Those pairs are 7.58:1 on white and 11.95:1 on `#111827`. |
| 5 | Notes body | `docs/images/audit/notes-light.png` shows the kitchen sentence through “sample”. The signal text continues “from last week.” The editor span is `text-body` with no width (`view.rs` `{note_body}`), so the label’s fit width clips the line instead of wrapping. | Reading text wraps inside the column. Clipping is not the wrap policy. |
| 6 | Shell | Panes are `md:w-[220px]`, `md:w-[280px]` is not declared, inspector is `md:w-[260px]`. Breakpoints are 768, not 800 / 1120 / 1440. No min or max pane widths. Below the `md` breakpoint the panes stack, but there is no single-pane back action and no 64px rail. | Nav 224 (200–280), collection 280 (240–360), inspector 280 (260–360), main min 480, collapse at 1440 / 1120 / 800. |
| 7 | Color | Faces use `system-blue` `#007AFF`, `window-background`, `label-secondary`, and one-off hex on the scrim. | Semantic tokens only. Accent is `#1D4ED8` / `#93C5FD`. |
| 8 | Library | Every control is inlined in `examples/showcase/view.rs`. Nothing is imported. | A component module apps import, with documented props, in the Leptos syntax. |
| 9 | Gallery | Settings is a control pile, not a reference gallery. No state specimens, content stress, layout stress, density switch, or copyable snippets. | Section 12 of the PRD. |

Notes body in the still frame can look complete when the line happens to fit the capture width. The walkthrough clip and the fit-width span are the defect. The field outline is the same class of bug: a fixed box the size of the stroke.

## Capability map

Filled from this repository at ff86f73. “Supported targets” means what the Makepad desktop host on Linux actually does.

| Design concept | Existing Quinn component/prop | Supported targets | Gap or fallback |
| --- | --- | --- | --- |
| Density | No density prop. `h-control` is a fixed 22px. `h-control-mini` 16, `small` 18, `large` 28. | Desktop window only. | Add `h-control` / `min-h-control` / `w-control` driven by `state.quinn_density` (0 compact, 1 standard, 2 touch) and `set_density(n)`. Text scaling is not this signal. |
| Semantic variant | No variant prop. Color is a class string (`bg-system-blue`, `text-system-red`). `disabled` is a class that sets the button animator. | Classes lower on every example. | `Button variant="primary\|secondary\|ghost\|danger"` appended by the lowerer. Disabled stays a real face color, not a faded control. |
| Layout gap / alignment | `gap-*` (n×4), `px-*`, `items-center`, `justify-*`, flex row/column. | Desktop. | Use the spacing scale as-is (space.2 = `gap-2` = 8). No new spacing unit. |
| Overflow policy | `overflow-hidden` clips. `overflow-y-auto` scrolls. Label flow wraps only once the node has a width. | Desktop. | Give reading text `w-full`. Do not use clip to hide a pane across breakpoints. |
| Accessible label / state | Text content is the visible label. No role, name, or value API. `placeholder` is `empty_text`. | Desktop draw and click. Screen readers unverified. | Visible label on every field. State is also shape (switch thumb, check glyph), not color alone. AX exposure stays unsupported. |
| Theme | `dark:` reads `state.quinn_dark`. `toggle_theme()`. | Desktop, same process. | Keep it. Point `dark:` at the slate semantic colors. |
| Shell width | `sm` 640, `md` 768, `lg` 1024, `xl` 1280 via `state.quinn_width`. | Desktop resize. | Add `bp800:` `bp1120:` `bp1440:`. |
| Component call | `#[component]` in the same `view.rs`. Props: string, int, bool, children. `class` on a call is ignored. `on:click` on a component is an error. | Same file only. | `quinn-ui` source merged by the lowerer. Forward `on:click`, `class`, `class:`, `bind:value`, `placeholder`. |
| Slider value | `bind:width="signal*factor"`. Click sets the signal. No pointer-move in the handler language. | Click and buttons. | Track clicks plus full-height keyboard buttons. Live drag unverified. |
| Focus | Button and input `border_color_focus`. Divs have no focus track. | Pointer. Keyboard ring unverified. | Accent stroke on the existing 1px border so focus does not change outer size. A 2px separated ring is unsupported. |
| Text scale 150/200% | Font size is a class constant. | Not supported. | Unverified. Do not claim it. |
| IME, RTL, touch hardware, mobile 390×844, display scale | No APIs. | Not supported on this host. | Unverified. Desktop viewports are the ones this host can run. |
| Overlay | Root `absolute` children lift onto a transparent host layer. `Portal` exists. | Desktop. | Toast and dialogs use that path. Edge flip is unsupported; specimens are placed with 16px clearance. |
