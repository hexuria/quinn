# Quinn Design System — Product Requirements Document

Version: 1.0  
Scope: Design system, component design, layout behavior, and showcase quality  
Audience: The agent implementing Quinn and applications built with Quinn

## 1. Mission

Build a coherent, production-quality design system for Quinn, a GUI framework for cross-platform applications authored with the project's existing Leptos-style syntax.

A developer must be able to compose Quinn components and get consistent spacing, alignment, readable typography, complete interaction states, and predictable resizing without manually correcting every screen.

This is a design-system task. Preserve the existing rendering architecture, reactive model, application behavior, and authoring syntax. Make only the component styling and layout changes required to satisfy this document. Do not turn this into a renderer rewrite, framework migration, dependency cleanup, or unrelated code refactor.

## 2. Evidence and boundaries

The supplied 20-second showcase demonstrates a Notes application, a table, a Settings component gallery, an inspector, light and dark themes, and notification feedback.

Visible concerns include a text field with an incomplete lower outline around 00:04, inconsistent apparent control sizes, loose relationships between labels and controls, and secondary text that appears weak against its background. Contrast must be measured before declaring a standards failure. These observations identify visual symptoms, not their underlying code causes.

The supplied repository URL required authentication during preparation of this PRD. Existing component names, props, rendering capabilities, and platform support have not been verified. The implementing agent must inspect the actual repository before mapping this specification to code.

All dimensions and component names below are proposed design contracts, not claims about APIs already implemented.

## 3. Product outcomes

The design system must deliver:

- Consistent components across application screens and the component gallery.
- Zero unintended clipping, overlap, inaccessible actions, or truncated control borders.
- Layouts that adapt to their available container width and content.
- Complete keyboard, pointer, touch, focus, validation, and disabled behavior where applicable.
- Light and dark themes with semantic colors and verified contrast.
- Familiar platform behavior without maintaining unrelated visual systems.
- Documented, runnable examples using Quinn's existing Leptos syntax.

The Notes demo is a reference application. Do not make generic components depend on Notes-specific dimensions, content, or colors.

## 4. Design direction

Use a restrained desktop productivity aesthetic: neutral surfaces, strong typography, compact but comfortable spacing, restrained borders, and one primary accent.

Hierarchy comes from typography, grouping, spacing, and surface separation. Avoid wrapping every group in a card. Avoid decorative gradients, oversized rounding, and shadows on ordinary controls.

Use rounded rectangles for buttons and inputs. Reserve pills for badges and controls whose shape conveys a specific function.

Keep selection, keyboard focus, hover, and pressed states visually distinct. A selected item is not necessarily the item with keyboard focus.

## 5. Foundations and tokens

### 5.1 Coordinate system

All dimensions are logical units: device-independent units on native targets and CSS pixels on web targets, mapped by the existing backend.

Never infer logical sizes directly from video pixels without verifying capture scale. Preserve logical geometry across display scales. Border alignment may use device-pixel snapping without changing layout size.

### 5.2 Spacing

Allowed spacing tokens:

| Token | Value | Primary use |
|---|---:|---|
| space.0 | 0 | Joined controls |
| space.1 | 4 | Closely related metadata |
| space.2 | 8 | Icon-to-label; label-to-control |
| space.3 | 12 | Control padding; small groups |
| space.4 | 16 | Related controls; compact panel padding |
| space.5 | 20 | Medium component padding |
| space.6 | 24 | Form groups; normal panel padding |
| space.8 | 32 | Section separation |
| space.10 | 40 | Major section separation |
| space.12 | 48 | Large page separation |

Defaults: page padding 24; narrow-page padding 16; section gap 32; field-group gap 24; label-to-control gap 8; helper/error gap 4; action gap 8.

The parent owns spacing between siblings. The component owns internal padding. Do not combine child margins and parent gaps to create accidental double spacing.

Use explicit named exceptions for borders, optical alignment, and platform constraints. Do not introduce arbitrary spacing values to repair individual screenshots.

### 5.3 Typography

Use the platform UI font with a documented fallback on each target. Do not assume identical font metrics across operating systems.

| Role | Font size / line height | Weight |
|---|---|---|
| Caption / metadata | 12 / 16 | Regular |
| Control label | 14 / 20 | Medium |
| Body | 14 / 20 | Regular |
| Reading content | 16 / 24 | Regular |
| Section heading | 18 / 24 | Semibold |
| Page title | 24 / 32 | Semibold |
| Display / empty-state heading | 32 / 40 | Semibold |

These are base sizes, not fixed clipping boxes. Respect user text scaling. Default controls must grow when the scaled line box and padding exceed their nominal height.

Use sentence case. Use tabular numerals for aligned numerical columns. Never reduce font size to make a label fit.

### 5.4 Density and geometry

| Contract | Compact | Standard desktop | Touch |
|---|---:|---:|---:|
| Button/input minimum height | 28 | 36 | 48 |
| Single-line navigation row minimum height | 28 | 36 | 48 |
| Horizontal control padding | 8 | 12 | 16 |
| Icon size | 16 | 16–20 | 20–24 |
| Minimum interactive target | 28 × 28 | 32 × 32 | 48 × 48 |

Standard desktop is the default. Compact is explicit opt-in for dense applications; never silently shrink controls to fit a window. Touch mode follows input needs or an explicit application setting, not window width alone.

Icon-only buttons use square targets. A small checkbox glyph or slider thumb can have a larger interactive region, but neighboring hit regions must not overlap.

Corner radius: controls 6; grouped panels 8; overlays 12; badges fully rounded. Border width: 1 logical unit. Focus indicator: 2-unit ring with 2-unit separation, or an equivalent visible inset treatment when required.

No hover, focus, selection, or error state may change outer component geometry.

### 5.5 Semantic colors

Use tokens, never scattered raw colors. The following is the baseline palette; verify every actual foreground/background pairing before release.

| Semantic role | Light | Dark |
|---|---|---|
| Canvas | #F8FAFC | #0F172A |
| Surface | #FFFFFF | #111827 |
| Subtle surface | #F1F5F9 | #1E293B |
| Raised surface | #FFFFFF | #1E293B |
| Primary text | #0F172A | #F8FAFC |
| Secondary text | #475569 | #CBD5E1 |
| Muted text | #5F6F85 | #94A3B8 |
| Decorative divider | #CBD5E1 | #334155 |
| Control boundary | #64748B | #94A3B8 |
| Accent | #1D4ED8 | #93C5FD |
| Text on accent | #FFFFFF | #0F172A |
| Selection background | #DBEAFE | #1E3A8A |
| Selection foreground | #1E3A8A | #EFF6FF |
| Danger foreground | #B91C1C | #FCA5A5 |
| Success foreground | #166534 | #86EFAC |
| Warning foreground | #92400E | #FDE68A |

Add named hover, pressed, disabled, focus, and semantic background tokens. Do not derive dark mode by inversion or reduce an entire component's opacity to represent disabled state.

Decorative dividers are not adequate substitutes for boundaries necessary to identify controls. State meaning must also use text, shape, iconography, or semantics.

## 6. Mandatory layout contract

Every component must declare:

1. Intrinsic or preferred size.
2. Minimum and maximum size.
3. Whether it fills, wraps, or sizes to content.
4. Whether each axis may shrink.
5. Behavior for long content.
6. Overflow and scroll ownership.
7. Focus-ring and overlay clearance.
8. Behavior under text scaling and narrow containers.

Use content-sized layout by default. Fixed height is appropriate only for deliberately bounded content, and must not clip scaled text.

Required rules:

- Borders and padding are included in declared outer dimensions.
- Text and icons align through shared alignment rules, not arbitrary offsets.
- Leading icons retain their size; text takes available space.
- Side-by-side fields share control baselines. Errors appear below their controls.
- Each constrained text area can shrink and has an explicit wrap or truncation policy.
- Each scrollable region can shrink within its parent and reach its final item.
- Each axis has an intentional scroll owner. Independent application panes may scroll separately.
- Form labels, validation, dialog messages, and primary actions wrap or reflow; they do not silently ellipsize.
- Ellipsis is allowed for navigation names, list previews, and designated table columns. Full values remain available through an accessible interaction.
- Truncation never replaces a required accessible name.

Do not use hidden overflow as a general repair for layout defects. Clipping is acceptable only for documented cases such as an avatar crop or a scrolling viewport. Popovers and focus indicators must not accidentally inherit that clipping.

Do not implement ordinary layout with absolute coordinates, negative margins, spacer strings, manual line breaks, or visual transforms that move content away from its hit target.

## 7. Layout primitives

Reuse existing equivalent primitives wherever possible. The names below describe responsibilities, not mandatory new APIs.

| Primitive | Required contract |
|---|---|
| Stack | Vertical flow; explicit gap and alignment; content-driven height |
| Inline | Horizontal flow; explicit gap, alignment, and wrap policy |
| Grid | Explicit column minimums; collapses when those minimums cannot fit |
| Container | Width cap and responsive padding; no accidental fixed content height |
| Surface | Background, border, radius, and padding; no hidden sibling margins |
| Field | Associated label, control, description, and error |
| Scroll area | Bounded viewport; reachable content; explicit scroll ownership |
| Split pane | Minimum pane sizes, resize affordance, and collapse behavior |
| Overlay | Viewport-aware placement outside inappropriate ancestor clipping |

Correct the primitive or component contract before adding page-specific compensation.

## 8. Application shell and responsive behavior

Support top bar, navigation, optional collection pane, main content, and optional inspector.

Default desktop pane contracts:

- Navigation: preferred 224, minimum 200, maximum 280.
- Collection/list: preferred 280, minimum 240, maximum 360.
- Inspector: preferred 280, minimum 260, maximum 360.
- Main content: minimum 480 when participating in a multi-pane layout.
- Top bar: minimum 48 on desktop and 56 on touch, plus applicable safe-area insets.

All values include pane padding and borders. Pane allocation must actually fit the available window content area. Persisted user widths must be clamped again after resizing.

| Available app width | Default shell behavior |
|---|---|
| 1440+ | Navigation, collection, main, and inspector may coexist when needed |
| 1120–1439 | Inspector becomes an explicit drawer |
| 800–1119 | Navigation becomes a 64-unit rail; collection opens as a drawer or replaces main |
| Below 800 | Single active pane with explicit navigation/back actions |

These are default transition points, not permission to violate minimum sizes. Collapse earlier if text scaling, localization, user resizing, or platform insets require it.

Settings grids respond to their own container width: use two columns only when both 280-unit columns, a 24-unit gap, and container padding fit. Otherwise use one column.

Top-bar layout priority: navigation/back, title, primary action, then secondary actions. Search may collapse to an accessible button that opens a search surface. Move secondary actions into overflow before overlap occurs. Never hide the only route to an action.

Respect host-provided window controls, menu placement, drag regions, safe areas, and system gestures. Do not render duplicate window controls. Interactive controls must not belong to a draggable title-bar region.

## 9. Component specifications

Every shipped component needs documented anatomy, dimensions, content limits, variants, keyboard behavior, accessibility semantics, and applicable states. Unimplemented behavior must be marked unsupported.

### 9.1 Buttons and icon buttons

Anatomy: container, optional leading icon, label, optional trailing icon or loading indicator.

Variants: primary, secondary, ghost, destructive. Keep shape, density, and padding consistent across variants. A page action group normally has one primary action.

Labels remain readable. Use wrapping or layout reflow before clipping. Loading preserves button size and prevents duplicate activation. Icon-only buttons require accessible names and discoverable explanations.

States: default, hover, pressed, focus-visible, disabled, loading. Destructive intent and focus must remain visible together.

### 9.2 Fields, text inputs, and text areas

Anatomy: label, optional required/optional indication, control, helper text, error.

Inputs fill their allocated width. Leading/trailing adornments reserve their own space. Input text, caret, selection, border, and focus indicator must remain within their intended bounds.

Placeholder text supplements a label; it never replaces one. Errors use text plus a visual state. Multiline errors expand the field instead of overlapping the next field.

Single-line editing may scroll internally. Text areas grow to a documented maximum and then scroll. Read-only and disabled states are distinct. Preserve platform text selection, copy/paste, and input-method composition behavior.

### 9.3 Search

Anatomy: search icon, input, optional clear action, optional shortcut hint.

Clear is an accessible action with a full target. Hide shortcut hints before reducing usable input space. Display empty, loading, populated, and no-results states where suggestions exist.

### 9.4 Checkbox, radio, and switch

Use 18-unit checkbox/radio glyphs within the density's full target. Align the glyph with the first text line when a label wraps; make the associated label interactive.

Checkbox: unchecked, checked, mixed, disabled, focused, and invalid when applicable. Radio: one selected option per group. Switch: explicit on/off semantics and a stable label; use for an immediately applied setting.

Standard switch visual size: 40 × 24 inside the larger target. Do not use color alone to indicate its value.

### 9.5 Select and combobox

A select chooses from predefined options; a combobox also supports text entry/filtering. Do not treat either as an action menu.

Trigger uses input geometry and reserves trailing space for its indicator. Popup minimum width matches the trigger unless constrained by the viewport. Long options wrap or provide an accessible full-value view.

Specify selection, active option, disabled option, no-results state, keyboard movement, dismissal, and focus restoration. A failed query must not appear as an empty list without explanation.

### 9.6 Segmented control and tabs

Segmented controls choose a related value; tabs change the associated content panel. Choose the correct interaction semantics.

Segments share a minimum height and consistent internal padding. For short peer labels, equal widths are preferred. Selected styling must not shift labels.

When labels cannot fit, change the value selector to a select or use a documented scrollable tab strip. Never shrink text or clip the last option.

### 9.7 Slider, stepper, and progress

Slider: label, track, thumb, and optional value. Standard track 4 units, visual thumb 16 units, interaction region at least the density minimum. Expose min/max/current value and support keyboard adjustment. Keep labels and value readouts aligned.

Stepper: prefer a horizontal decrement/value/increment group using normal control heights. Each button has its own full target. Define bounds, invalid entry, and disabled-at-limit states. Do not use two tiny stacked buttons with inaccessible targets.

Progress: use a 4-unit standard track; determinate and indeterminate are separate states. Include a label or accessible description. Never imply an exact percentage for unknown progress.

### 9.8 Navigation, lists, and tables

Navigation row: leading icon slot, flexible label, optional badge, optional trailing affordance. Standard padding 12 horizontally; icon-label gap 8. Use distinct selection and focus treatments.

Collection row: title, metadata, optional preview. Default minimum height 64 when title and preview are present; allow growth. Metadata must not collide with the title. Only the preview and designated long names may truncate.

Table: header minimum height 36; body row minimum height 40; cell padding 12 horizontally and 8 vertically. Rows grow for wrapped content. Align numbers to the end and text to the start. Keep column headings aligned with their cells.

Give columns explicit sizing and overflow behavior. A wide table may scroll horizontally within its own region; the entire page must not acquire unintended horizontal scrolling. Distinguish an ordinary table from an interactive grid with cell navigation.

### 9.9 Panels, forms, and settings groups

A panel has optional heading, description, body, and actions. Default padding 24; compact inspector padding 16.

Settings use consistent rows: label/description and control in two columns when space permits, stacked otherwise. Related settings use a shared group heading and a 24-unit group gap.

A component gallery is different from a settings form: each example should have a title, description, bounded preview, visible state controls, and usage guidance.

### 9.10 Menus, tooltips, popovers, dialogs, and drawers

Overlay placement must account for window bounds, safe areas, and the on-screen keyboard. Maintain an 8-unit edge clearance where possible; flip or shift placement near edges. Long surfaces scroll internally.

Menus: leading indicator/icon, label, optional shortcut, optional submenu affordance. Standard item minimum height 32; touch minimum 48.

Tooltips: concise explanations on hover and focus; no essential actions or required information only available through a tooltip.

Dialogs: preferred widths 360/480/640; maximum available width minus 32. Header and actions remain reachable; long bodies scroll. Actions reflow on narrow screens.

Modal dialogs/drawers isolate focus and restore it on close. Nonmodal inspectors do not trap focus. Escape dismisses the top dismissible layer. Clicking outside must not silently discard unsaved work.

Overlay ordering: base content, sticky content, nonmodal popups, modal scrim, modal surface, then popups owned by that modal. Notifications must respect the active modal context.

### 9.11 Notifications and content states

Toasts appear in a consistent safe corner with 16-unit edge spacing and a maximum width of 360, constrained by available width. Show at most three; queue the remainder. They must not cover essential current-task controls.

Use inline feedback for persistent errors. Actionable notifications must remain available long enough to use; do not make a short-lived toast the only way to undo an important action.

Define empty, loading, error, success, and unavailable states for lists and panels. State messages explain what happened and the next available action. Loading must not collapse the surrounding layout.

## 10. Interaction and accessibility

Apply WCAG 2.2 AA requirements to web output and use equivalent measurable visual criteria for native output, supplemented by each platform's accessibility APIs.

Normal text needs at least 4.5:1 contrast; qualifying large text at least 3:1. Necessary control boundaries and state indicators need at least 3:1 against adjacent colors. Measure actual states and surfaces.

Use 48-unit touch targets as Quinn's product standard. This is deliberately larger than WCAG's 24-CSS-pixel minimum, which has defined exceptions.

Requirements:

- Complete essential workflows with keyboard alone.
- Visible focus, logical focus order, and no accidental focus traps.
- Accessible names, roles, values, states, labels, and errors.
- Platform-equivalent accessibility exposure for custom native controls.
- Standard keyboard patterns for each semantic widget.
- Focus remains visible when scrolling and opening overlays.
- Text scaling to 200% without losing content or actions.
- Web reflow at 320 CSS pixels, with necessary two-dimensional content such as tables handled separately.
- Mixed-script, accented text, emoji, CJK, and IME input without glyph clipping.
- Logical start/end alignment; verify RTL where supported.
- Reduced-motion behavior and no essential hover-only functionality.

Use brief 120–180 ms state transitions. Motion must not delay activation or hide the final state.

## 11. Preserve Leptos authoring

Use the repository's existing Leptos-style component composition, reactive values, event conventions, and child composition.

Before changing anything, document the current canonical examples. Do not invent a parallel JSX/TSX API, introduce JavaScript authoring, or assume browser DOM/CSS support exists in Quinn's native backend.

For each design concept, record:

| Design concept | Existing Quinn component/prop | Supported targets | Gap or fallback |
|---|---|---|---|
| Density | Fill from repository | Verify | Document |
| Semantic variant | Fill from repository | Verify | Document |
| Layout gap/alignment | Fill from repository | Verify | Document |
| Overflow policy | Fill from repository | Verify | Document |
| Accessible label/state | Fill from repository | Verify | Document |

Prefer existing props and theme mechanisms. Add only the minimal missing design-facing capability. Every published example must compile in this repository.

If a design requirement reveals an engine limitation, document the affected component, target, visible consequence, and bounded fallback. Do not silently claim support or launch an unrelated architecture rewrite.

## 12. Showcase requirements

Retain the Notes, Table, and Settings flows and add a dedicated component reference gallery.

The gallery must include:

- Foundations: typography, spacing, surfaces, semantic colors, density.
- Controls: every component already shipped, including all controls visible in the walkthrough.
- State specimens: applicable default, hovered, pressed, focused, selected, disabled, loading, and error states.
- Content stress: short, long, multiline, empty, and localized content.
- Layout stress: narrow containers, resizeable panes, edge-positioned overlays, and long scrollable content.
- Light/dark and density selectors.
- Copyable, verified Leptos usage examples.

Do not imply unsupported states are working because a static specimen looks correct.

Apply the system to the actual demo: repair the clipped field, normalize control geometry, align labels and actions, enforce pane minimums, and provide clear selected/focused states.

## 13. Validation and release gates

Verify rendered results, not compilation alone.

Required viewport checks: 1440 × 900, 1280 × 800, 1024 × 768, 800 × 600, 390 × 844, and a 320-unit narrow-content case. Native desktop minimum-window constraints must be documented; narrow web/mobile layouts remain required for those supported targets.

Check light and dark themes, standard and touch densities, and text at 100%, 150%, and 200%. Check representative 1×, 1.25×, 1.5×, and 2× display scales on applicable targets. Include compact density if shipped.

Run every existing supported target. Record unavailable targets as unverified, not passed. Mobile requirements are acceptance contracts for mobile support, not evidence that mobile backends already exist.

Mandatory stress cases:

- A 60-character navigation label and a long unbroken identifier.
- A three-line field error and a two-line button label.
- A translated action label twice the English length.
- An empty list and a long scrollable list.
- Popovers at all four viewport edges.
- A dialog with content longer than the window.
- Rapid theme changes, resize operations, and selection changes.
- Keyboard-only use; screen-reader checks on available targets.
- On-screen keyboard with the last field focused on supported mobile targets.

Release gates:

| Area | Acceptance criterion |
|---|---|
| Clipping | No unintentionally cut text, glyphs, borders, controls, or focus indicators |
| Alignment | Intended shared edges/baselines agree within 1 logical unit, excluding documented optical adjustments |
| Spacing | Shared spacing follows tokens; exceptions are named and justified |
| Sizing | Minimums and content-growth contracts hold in every required fixture |
| Overflow | Every offscreen item/action remains reachable; no unintended page-level horizontal scrolling |
| States | Every shipped applicable state works and has a reviewed specimen |
| Themes | Actual foreground/background pairings pass the required contrast checks |
| Authoring | Usage examples compile with the existing Quinn syntax |
| Scope | No unrelated architecture changes or application-feature changes |
| Evidence | Reviewed screenshots and a target/viewport/state verification matrix are provided |

Screenshots alone do not prove hit targets, keyboard operation, accessibility, or scroll reachability. Check those behaviors directly.

## 14. Execution order and deliverables

1. Audit the current components and demo. Produce a screenshot-backed issue list and capability map.
2. Establish semantic tokens, typography, density, and layout contracts.
3. Validate the foundation with a button, field, navigation row, and overlay in both themes and constrained layouts.
4. Apply the system to all existing components and complete their state contracts.
5. Apply it to Notes, Table, and Settings; build the reference gallery.
6. Run visual and interaction checks; fix shared causes; record remaining gaps.

Required deliverables: design-system documentation, token reference, component specification sheets, working gallery, corrected showcase, verified Leptos examples, before/after evidence, and a platform verification matrix.

For each component specification, include anatomy; props mapped to existing APIs; sizing; spacing; states; keyboard behavior; accessibility; overflow; responsive behavior; and examples.

Completion means a developer can compose a new screen from Quinn's documented components without adding ad hoc spacing or clipping fixes. “Looks better at one window size” is not completion.

## References

These references inform accessibility and Leptos terminology. They do not certify Quinn's current implementation.

- Leptos basic components: https://book.leptos.dev/view/01_basic_component.html
- WCAG 2.2: https://www.w3.org/TR/WCAG22/
- WAI interaction patterns: https://www.w3.org/WAI/ARIA/apg/patterns/
- Target size: https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html
- Reflow: https://www.w3.org/WAI/WCAG21/Understanding/reflow.html

