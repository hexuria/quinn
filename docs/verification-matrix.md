# Verification matrix

Checked on Linux with Xvfb, `MAKEPAD_GPU=gl`, and `LIBGL_ALWAYS_SOFTWARE=1`. The showcase process for the walkthrough was pid 141849, starttime 5901379, window `0x1c00002`. The recording is 22.03 seconds. The toolbar edit `Notes` → `Notes.` became visible 209 ms after `fsync` (frame 427 at 30 fps, fsync at 14.024 s). That edit was reverted. The committed toolbar string is `Notes`.

Before shots of the ff86f73 showcase are in `docs/images/audit/`. The shots below are this design system.

## Contrast

Relative luminance, WCAG 2.2. Normal text needs 4.5:1. A control boundary needs 3:1. The old secondary label composited to rgb(138, 138, 142) on white, which is 3.44:1.

| Pair | Ratio |
| --- | --- |
| Secondary `#475569` on white | 7.58 |
| Secondary `#475569` on canvas `#F8FAFC` | 7.24 |
| Secondary `#CBD5E1` on surface `#111827` | 11.95 |
| Muted `#5F6F85` on white | 5.12 |
| Muted `#5F6F85` on canvas | 4.90 |
| Muted `#94A3B8` on surface `#111827` | 6.92 |
| Muted `#94A3B8` on canvas `#0F172A` | 6.96 |
| Primary `#0F172A` on white | 17.85 |
| Primary `#F8FAFC` on surface `#111827` | 16.96 |
| White on accent `#1D4ED8` | 6.70 |
| `#0F172A` on accent `#93C5FD` | 9.90 |
| Control `#64748B` on white | 4.76 |
| Control `#94A3B8` on surface `#111827` | 6.92 |
| Danger `#B91C1C` on white | 6.47 |
| Danger `#B91C1C` on `#FEE2E2` | 5.30 |
| Danger `#FCA5A5` on `#111827` | 9.35 |
| Danger `#FCA5A5` on `#450A0A` | 8.51 |
| Success `#166534` on white | 7.13 |
| Warning `#92400E` on white | 7.09 |
| Selection `#1E3A8A` on `#DBEAFE` | 8.49 |
| Selection `#EFF6FF` on `#1E3A8A` | 9.52 |
| Disabled `#475569` on `#E2E8F0` | 6.15 |
| Disabled `#94A3B8` on `#1E293B` | 5.71 |
| Divider `#CBD5E1` on white | 1.48, decorative only |

## Viewports

`state.quinn_width` followed the X11 client width (logged 1440, 1280, 1024, 800, 640).

| Viewport | Theme | What showed |
| --- | --- | --- |
| 1440×900 | Light | Nav 224, collection, editor, inspector. Body wraps through "from last week". |
| 1440×900 | Dark | Same shell. Selection is `#1E3A8A`. Toast title is light on `#111827`. |
| 1280×800 | Light | Nav and collection still open. Inspector closes below 1440. |
| 1024×768 | Light | 64-wide rail. Icons sit in the rail. Labels and collection are hidden. |
| 800×600 | Light | 64-wide rail. Main column keeps the note. |
| 640×700 | Light | Nav width 0. Note fills the width and wraps. Toolbar clips Theme and New note. Library in the toolbar opens the drawer. |

Touch density (`set_density(2)`) was captured at 1440 in `docs/images/notes-touch.png`. Standard is density 1. Compact (0) is implemented by `set_density` and is not in the showcase UI.

## Controls that were exercised

| Control | Result |
| --- | --- |
| Navigation row | Selection fill is `#DBEAFE` / `#1E3A8A`. Unselected rows stay clear. Labels sit beside the icon. |
| Field | Full rounded `border-control` outline. Label is 8 units above the control (`gap-2`). |
| Checkbox | 18×18 glyph inside the row target. Click toggled the check. |
| Switch | 40×24 track. Click toggled the thumb. |
| Stepper | Horizontal 36×36 buttons. Copies went 2 → 3. Level went 50 → 55. |
| Slider | 4-unit track, 16-unit thumb. A click on the track set level to 86. Live drag is unsupported. Arrow keys are unsupported. The ± buttons step by 5. |
| Button row | Delete note, Bordered, and Borderless wrap instead of clipping. |
| Dialog | "Delete this note?" opens on the scrim with Delete and Cancel. It is placed at left 24, top 80. It does not center or flip. |
| Toast | Top-end, 16 units in, max width 360. Title readable in light and dark. |
| Theme | `toggle_theme()` repaints light and dark in the same process. |
| Hot reload | Same process. 209 ms from fsync to the extra period in the toolbar. |

## Stress cases run

Long note body wraps. The 60-character navigation specimen is in the gallery source. The three-line field error is in the gallery source. The action row wraps at 280. Empty and long table names were not given a separate fixture beyond the eight notes. Four-corner popovers were not photographed. Text at 150% and 200% cannot be set.

## Gates not met

These are unverified. The component is documented. The fallback is the visible control without that behavior.

| Gate | Consequence | Fallback |
| --- | --- | --- |
| Text scale 150% and 200% | Sizes are class constants. | Unverified. |
| Accessibility tree, names, roles | No platform accessibility API is called. | Visible label only. Unverified. |
| IME composition | Not exercised. | Unverified. |
| RTL | No direction utilities. | Unverified. |
| Touch hardware and 48px as a finger target | Density 2 only changes the pixel height. | Unverified on a device. |
| Mobile 390×844 and 320 CSS | No mobile target. | Unverified. |
| Display scale 1.25, 1.5, 2 | Not set on this X server. | Unverified. |
| Slider live drag | The handler language has no pointer-move. | Click segments and ± buttons. |
| Arrow keys on the slider | No key handler. | ± buttons. |
| 2px separated focus ring | Focus is the existing 1px border recolored to accent. | Unverified as a keyboard ring. |
| Focus trap, focus restore, Escape to dismiss | Not implemented. | Click Cancel or Dismiss. |
| Overlay edge flip | Dialog and toast use fixed offsets. | They stay inside this window. They do not flip. |
| Compact density in the UI | `set_density(0)` exists. | The showcase offers standard and touch only. |
