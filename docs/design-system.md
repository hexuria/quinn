# Quinn design system

The contract is [design-system-prd.md](design-system-prd.md). This page is the token reference the utilities actually implement. Component anatomy and props are in [components.md](components.md). The audit of the previous showcase is [audit-ff86f73.md](audit-ff86f73.md).

`quinn-styles` maps utility classes onto Makepad layout fields. An unknown class fails the lower (`unknown class: …`) and the previous script stays on screen. The Tailwind v3 hue palette remains, so older examples keep compiling. Apple system colors (`bg-system-blue`, `text-label`, `bg-window-background`) also remain as a compatibility leftover. New screens use the semantic names below. `accent` is the slate accent, not system blue. `accent-blue` is the old system blue.

The face is Inter, the variable font Makepad already ships. SF Pro is not bundled. There is no CSS inheritance: each text node sets its own color and size.

## Color

Light and dark are named pairs. Dark is not an inversion, and disabled is not a faded copy of the enabled control.

| Token | Light | Dark | Utilities |
| --- | --- | --- | --- |
| Canvas | `#F8FAFC` | `#0F172A` | `bg-canvas` |
| Surface | `#FFFFFF` | `#111827` | `bg-surface` |
| Subtle | `#F1F5F9` | `#1E293B` | `bg-subtle` |
| Raised | `#FFFFFF` | `#1E293B` | `bg-raised` |
| Primary text | `#0F172A` | `#F8FAFC` | `text-primary` |
| Secondary text | `#475569` | `#CBD5E1` | `text-secondary` |
| Muted text | `#5F6F85` | `#94A3B8` | `text-muted` |
| Divider | `#CBD5E1` | `#334155` | `bg-divider` |
| Control boundary | `#64748B` | `#94A3B8` | `border-control` |
| Accent | `#1D4ED8` | `#93C5FD` | `bg-accent`, `border-accent` |
| Text on accent | `#FFFFFF` | `#0F172A` | `text-on-accent` |
| Selection background | `#DBEAFE` | `#1E3A8A` | `bg-selection` |
| Selection foreground | `#1E3A8A` | `#EFF6FF` | `text-on-selection` |
| Danger | `#B91C1C` | `#FCA5A5` | `text-danger`, `border-danger` |
| Success | `#166534` | `#86EFAC` | `text-success`, `border-success` |
| Warning | `#92400E` | `#FDE68A` | `text-warning`, `border-warning` |
| Accent hover | `#1E40AF` | `#BFDBFE` | `hover:bg-accent-hover` |
| Accent pressed | `#1E3A8A` | `#DBEAFE` | `active:bg-accent-pressed` |
| Surface hover | `#F1F5F9` | `#1E293B` | `hover:bg-surface-hover` |
| Surface pressed | `#E2E8F0` | `#334155` | `active:bg-surface-pressed` |
| Disabled background | `#E2E8F0` | `#1E293B` | `disabled:bg-disabled-bg` |
| Disabled text | `#475569` | `#94A3B8` | `disabled:text-disabled-fg` |
| Danger background | `#FEE2E2` | `#450A0A` | `bg-danger-bg` |
| Success background | `#DCFCE7` | `#052E16` | `bg-success-bg` |
| Warning background | `#FEF3C7` | `#451A03` | `bg-warning-bg` |
| Focus | `#1D4ED8` | `#93C5FD` | focus stroke |
| Scrim | `#0F172A` at 50% | black at 60% | `bg-scrim` |

`dark:` reads `state.quinn_dark`. `toggle_theme()` flips it. Measured contrast for the pairs that ship is in [verification-matrix.md](verification-matrix.md).

Dividers are decorative. A control that must be identifiable uses `border-control`, which clears 3:1 against white and against `#111827`.

## Type

| Utility | Role | Size / line | Weight |
| --- | --- | --- | --- |
| `text-caption` | Caption, metadata | 12 / 16 | 400 |
| `text-control` | Control label | 14 / 20 | 500 |
| `text-body` | Body | 14 / 20 | 400 |
| `text-reading` | Reading | 16 / 24 | 400 |
| `text-section` | Section | 18 / 24 | 600 |
| `text-page` | Page title | 24 / 32 | 600 |
| `text-display` | Display | 32 / 40 | 600 |

`text-headline`, `text-title`, `text-title1` through `text-title3`, `text-callout`, `text-subheadline`, `text-footnote`, `text-caption2`, and `text-large-title` are the old macOS sizes. They still parse. New UI does not use them. `text-xs` through `text-3xl` are the Tailwind sizes.

Text scaling to 150% and 200% is not implemented. Sizes are class constants. That gate is unverified.

## Spacing, radius, density

Spacing numbers are multiplied by 4. `gap-2` is 8, `px-3` is 12, `p-6` is 24, `gap-8` is 32. That is the PRD scale (`space.1` = 4 through `space.12` = 48). The parent sets the gap between siblings. The component sets its own padding.

| Utility | Meaning |
| --- | --- |
| `rounded-control` | 6, controls |
| `rounded` | 8, grouped panels |
| `rounded-lg`, `rounded-sheet` | 12, overlays |
| `rounded-full` | pills, badges, switch, slider thumb |
| `h-control`, `w-control` | compact 28, standard 36, touch 48 |
| `min-h-control` | same floor, so a wrapping label can grow |
| `h-toolbar` | 48, or 56 when touch density is on |
| `px-control` | horizontal padding 8 / 12 / 16 |

`set_density(0)`, `set_density(1)`, and `set_density(2)` write `state.quinn_density`. Standard (1) is the default. Density is an application setting. It is not text scaling and it is not inferred from the window width.

## Shell

`bp800:`, `bp1120:`, and `bp1440:` are minimum window widths, read from `state.quinn_width`. The older `sm` `md` `lg` `xl` prefixes stay.

| Width | Default |
| --- | --- |
| 1440 and up | Navigation, collection, main, and inspector |
| 1120–1439 | Inspector width becomes 0; open it from Info |
| 800–1119 | Navigation is a 64-wide rail; collection width becomes 0 |
| Below 800 | Navigation width becomes 0; Library opens the drawer |

Pane widths at the wide step: navigation 224 (min 200, max 280), collection 280 (min 240, max 360), inspector 280 (min 260, max 360). Main content uses `bp800:min-w-[480px]`.

## Focus

Buttons and inputs paint `border_color_focus` with the accent. The stroke uses the control’s existing border width, so focus does not change the outer size. A 2-unit ring drawn outside the control, with 2 units of separation, is not available. Divs do not take a focus track.

## What this host does not do

Accessibility APIs, screen readers, IME, RTL, touch hardware, text scaling, display-scale matrices, and the 390×844 and 320 CSS viewports are not implemented. The verification matrix marks them unverified. Live slider dragging is not implemented: the handler language has click and `bind:width`, not pointer move. Arrow keys do not move the thumb. The minus and plus buttons do.
