// Quinn components. This file is not a Rust module. The lowerer merges it
// with an app view.rs that contains `use quinn_ui::components::*;`.

/// The look is the `QButton` templates in `quinn-makepad/src/widgets.rs`.
/// `variant` picks one; `selected` is the segmented-control state.
#[component]
fn Button(
    #[prop(default = "primary")] variant: String,
    #[prop(optional, default = false)] disabled: bool,
    #[prop(optional, default = false)] selected: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <button widget="QButton">
            {children()}
        </button>
    }
}

/// The field's look is the `QInput` template in `quinn-makepad/src/widgets.rs`.
/// `invalid=true` gives it the danger border and ring.
#[component]
fn Field(
    #[prop(into)] label: String,
    #[prop(optional, into, default = "")] placeholder: String,
    #[prop(optional, default = false)] invalid: bool,
) -> impl IntoView {
    view! {
        <div class="flex flex-col w-full gap-2">
            <span class="text-control text-primary dark:text-primary">{label}</span>
            <input widget="QInput" class="w-full" />
        </div>
    }
}

#[component]
fn Search(
    #[prop(optional, into, default = "Search")] placeholder: String,
) -> impl IntoView {
    view! {
        <div class="w-[240px] max-w-full h-control">
            <input widget="QInputSubtle" class="w-full pl-[36px]" />
            <div class="absolute inset-0 flex flex-row items-center pl-3">
                <Icon name="search" class="w-[16px] text-muted dark:text-muted" />
            </div>
        </div>
    }
}

/// A row in the navigation rail: an icon over a caption, or beside the label
/// from 1120 up and below 800, where the rail is the full-width first screen. The look is the `QNavItem` template; `selected` is the
/// current page.
#[component]
fn NavItem(
    #[prop(into)] icon: String,
    #[prop(optional, default = false)] selected: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <button widget="QNavItem" class="w-full h-[56px] flex flex-col items-center justify-center gap-1 text-caption bp1120:flex-row bp1120:justify-start bp1120:h-fit bp1120:min-h-control bp1120:py-2 bp1120:px-3 bp1120:gap-2 bp1120:text-body max-bp800:flex-row max-bp800:justify-start max-bp800:h-fit max-bp800:min-h-control max-bp800:py-3 max-bp800:px-4 max-bp800:gap-3 max-bp800:text-body">
            {children()}
        </button>
    }
}

/// A switch with its label: the `QSwitch` template. `checked` is the state;
/// the click handler flips it.
#[component]
fn Switch(
    #[prop(optional, default = false)] checked: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <button widget="QSwitch">
            {children()}
        </button>
    }
}

/// A checkbox with its label: the `QCheckbox` template.
#[component]
fn Checkbox(
    #[prop(optional, default = false)] checked: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <button widget="QCheckbox">
            {children()}
        </button>
    }
}

/// A segmented control: a tinted track of `Segment`s.
#[component]
fn Segmented(children: Children) -> impl IntoView {
    view! {
        <div class="flex flex-row items-center p-1 rounded-control bg-subtle w-fit dark:bg-subtle">
            {children()}
        </div>
    }
}

/// One segment: the `QSegment` template. `selected` raises its face.
#[component]
fn Segment(
    #[prop(optional, default = false)] selected: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <button widget="QSegment">
            {children()}
        </button>
    }
}

#[component]
fn Stepper(children: Children) -> impl IntoView {
    view! {
        <div class="flex flex-row items-center gap-2">
            {children()}
        </div>
    }
}

/// `step=move || signal.get()` makes a drag or an arrow key move by the
/// signal's value.
#[component]
fn Slider(#[prop(optional, default = 1)] step: i64) -> impl IntoView {
    view! {
        <input type="range" min="0" max="100" step="1" class="w-full" />
    }
}

#[component]
fn Specimen(
    #[prop(into)] title: String,
    #[prop(into)] note: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="flex flex-col w-full gap-4 p-6 rounded-[8px] bg-surface border border-divider dark:bg-surface dark:border-divider">
            <div class="flex flex-col w-full gap-1">
                <span class="text-control text-primary dark:text-primary">{title}</span>
                <span class="text-caption text-secondary w-full dark:text-secondary">{note}</span>
            </div>
            {children()}
        </div>
    }
}

#[component]
fn InfoGroup(
    #[prop(into)] title: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="flex flex-col w-full gap-1">
            <span class="text-control text-primary pb-1 dark:text-primary">{title}</span>
            {children()}
        </div>
    }
}

#[component]
fn InfoRow(
    #[prop(into)] label: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="flex flex-row items-center w-full min-h-[28px] gap-3">
            <span class="text-caption text-secondary w-[88px] dark:text-secondary">{label}</span>
            <div class="flex flex-row items-center flex-1 min-w-0 gap-2">
                {children()}
            </div>
        </div>
    }
}

/// Places its child in the top end corner, 16 from the edges, at most 360
/// wide.
#[component]
fn Overlay(children: Children) -> impl IntoView {
    view! {
        <div class="absolute right-[16px] top-safe top-[64px] z-50 flex flex-col w-[360px] max-w-full">
            {children()}
        </div>
    }
}

/// A raised surface: the `QPanel` template. The class sizes, places, and
/// spaces it.
#[component]
fn Panel(children: Children) -> impl IntoView {
    view! {
        <div widget="QPanel" class="flex flex-col">
            {children()}
        </div>
    }
}

/// A confirmation: check, message, and Dismiss on a panel. `on:click` is the
/// Dismiss action.
#[component]
fn Toast(#[prop(into)] message: String) -> impl IntoView {
    view! {
        <div widget="QPanel" class="flex flex-row items-center w-full gap-3 p-4">
            <Icon name="check" class="w-[16px] text-success dark:text-success" />
            <span class="text-body text-primary flex-1 min-w-0 dark:text-primary">{message}</span>
            <Button variant="ghost">"Dismiss"</Button>
        </div>
    }
}

/// A menu under the control before it: the `QMenuPanel` template.
#[component]
fn Menu(children: Children) -> impl IntoView {
    view! {
        <div widget="QMenuPanel" class="flex flex-col w-[220px] p-1" data-popover="below">
            {children()}
        </div>
    }
}

/// One menu item: the `QMenuItem` template.
#[component]
fn MenuItem(
    #[prop(optional, default = false)] selected: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <button widget="QMenuItem" class="w-full h-[32px]">
            {children()}
        </button>
    }
}

/// A list surface with a rounded border that its rows reach: the rows sit on
/// the surface and a `QFrame` over them strokes the border and masks the
/// corners with the page color.
#[component]
fn Table(children: Children) -> impl IntoView {
    view! {
        <div class="flex flex-col rounded-[8px] bg-surface overflow-y-auto dark:bg-surface">
            {children()}
            <div widget="QFrame" class="absolute inset-0"></div>
        </div>
    }
}

/// A column header that sorts on click: the `QHeader` template. `sorted`
/// marks the column the list is ordered by; `descending` points its chevron
/// up for A to Z, down for Z to A.
#[component]
fn SortHeader(
    #[prop(optional, default = false)] sorted: bool,
    #[prop(optional, default = false)] descending: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <button widget="QHeader">
            {children()}
        </button>
    }
}
