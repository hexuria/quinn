// Quinn components. This file is not a Rust module. The lowerer merges it
// with an app view.rs that contains `use quinn_ui::components::*;`.

#[component]
fn Button(
    #[prop(default = "primary")] variant: String,
    #[prop(optional, default = false)] disabled: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <button class="h-fit min-h-control px-control py-1 rounded-control border border-accent bg-accent text-on-accent text-control hover:bg-accent-hover active:bg-accent-pressed disabled:bg-disabled-bg disabled:text-disabled-fg dark:border-accent dark:bg-accent dark:text-on-accent dark:hover:bg-accent-hover dark:active:bg-accent-pressed dark:disabled:bg-disabled-bg dark:disabled:text-disabled-fg">
            {children()}
        </button>
    }
}

#[component]
fn Field(
    #[prop(into)] label: String,
    #[prop(optional, into, default = "")] placeholder: String,
) -> impl IntoView {
    view! {
        <div class="flex flex-col w-full gap-2">
            <span class="text-control text-primary dark:text-primary">{label}</span>
            <input class="w-full h-control px-3 rounded-control bg-surface text-body text-primary border border-control dark:bg-surface dark:text-primary dark:border-control" />
        </div>
    }
}

#[component]
fn Search(
    #[prop(optional, into, default = "Search")] placeholder: String,
) -> impl IntoView {
    view! {
        <div class="w-[240px] max-w-full h-control">
            <input class="w-full h-control pl-[36px] pr-3 rounded-control bg-subtle text-body text-primary border border-control dark:bg-subtle dark:text-primary dark:border-control" />
            <div class="absolute inset-0 flex flex-row items-center pl-3">
                <Icon name="search" class="w-[16px] text-muted dark:text-muted" />
            </div>
        </div>
    }
}

#[component]
fn NavRow(children: Children) -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center w-full h-[56px] gap-1 rounded-control bp1120:flex-row bp1120:justify-start bp1120:h-control bp1120:px-3 bp1120:gap-2">
            {children()}
            <button class="absolute inset-0 border-0 bg-transparent">" "</button>
        </div>
    }
}

#[component]
fn CheckRow(
    #[prop(into)] label: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="flex flex-row items-center gap-2 h-control">
            {children()}
            <span class="text-body text-primary dark:text-primary">{label}</span>
            <button class="absolute inset-0 border-0 bg-transparent">" "</button>
        </div>
    }
}

#[component]
fn CheckOn() -> impl IntoView {
    view! {
        <div class="flex flex-row items-center justify-center w-[18px] h-[18px] rounded-[4px] bg-accent dark:bg-accent">
            <Icon name="check" class="w-[14px] text-on-accent dark:text-on-accent" />
        </div>
    }
}

#[component]
fn CheckOff() -> impl IntoView {
    view! {
        <div class="w-[18px] h-[18px] rounded-[4px] bg-surface border border-control dark:bg-surface dark:border-control"></div>
    }
}

#[component]
fn SwitchOn(
    #[prop(into)] label: String,
) -> impl IntoView {
    view! {
        <div class="flex flex-row items-center gap-2 h-control">
            <div class="flex flex-row items-center justify-end w-[40px] h-[24px] rounded-full bg-accent px-1 dark:bg-accent">
                <div class="w-[16px] h-[16px] rounded-full bg-surface dark:bg-surface"></div>
            </div>
            <span class="text-body text-primary dark:text-primary">{label}</span>
            <button class="absolute inset-0 border-0 bg-transparent">" "</button>
        </div>
    }
}

#[component]
fn SwitchOff(
    #[prop(into)] label: String,
) -> impl IntoView {
    view! {
        <div class="flex flex-row items-center gap-2 h-control">
            <div class="flex flex-row items-center w-[40px] h-[24px] rounded-full bg-subtle border border-control px-1 dark:bg-subtle dark:border-control">
                <div class="w-[16px] h-[16px] rounded-full bg-surface border border-control dark:bg-surface dark:border-control"></div>
            </div>
            <span class="text-body text-primary dark:text-primary">{label}</span>
            <button class="absolute inset-0 border-0 bg-transparent">" "</button>
        </div>
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

#[component]
fn Slider() -> impl IntoView {
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

#[component]
fn Overlay(children: Children) -> impl IntoView {
    view! {
        <div class="absolute right-[16px] top-[64px] z-50 flex flex-col gap-2 w-[360px] max-w-full p-4 rounded-lg bg-surface border border-divider shadow-lg dark:bg-surface dark:border-divider">
            {children()}
        </div>
    }
}
