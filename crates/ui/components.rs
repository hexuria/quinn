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
            <div class="absolute left-[12px] top-[13px] w-[10px] h-[10px] rounded-full border border-muted bg-transparent"></div>
            <div class="absolute left-[20px] top-[21px] w-[2px] h-[2px] bg-muted"></div>
            <div class="absolute left-[22px] top-[23px] w-[2px] h-[2px] bg-muted"></div>
            <div class="absolute left-[24px] top-[25px] w-[2px] h-[2px] bg-muted"></div>
            <input class="w-full h-control pl-[36px] pr-3 rounded-control bg-subtle text-body text-primary border border-control dark:bg-subtle dark:text-primary dark:border-control" />
        </div>
    }
}

#[component]
fn NavRow(children: Children) -> impl IntoView {
    view! {
        <div class="flex flex-row items-center h-control w-full px-3 gap-2 rounded-control">
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
        <div class="w-[18px] h-[18px] rounded-[4px] bg-accent dark:bg-accent">
            <div class="absolute left-[4px] top-[9px] w-[2px] h-[2px] bg-on-accent dark:bg-on-accent"></div>
            <div class="absolute left-[6px] top-[11px] w-[2px] h-[2px] bg-on-accent dark:bg-on-accent"></div>
            <div class="absolute left-[8px] top-[9px] w-[2px] h-[3px] bg-on-accent dark:bg-on-accent"></div>
            <div class="absolute left-[10px] top-[7px] w-[2px] h-[3px] bg-on-accent dark:bg-on-accent"></div>
            <div class="absolute left-[12px] top-[5px] w-[2px] h-[3px] bg-on-accent dark:bg-on-accent"></div>
            <div class="absolute left-[14px] top-[4px] w-[2px] h-[2px] bg-on-accent dark:bg-on-accent"></div>
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
fn Slider(
    #[prop(default = "level")] signal: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="w-[240px] max-w-full h-control">
            <div class="absolute left-0 top-[16px] z-[-1] w-full h-[4px] rounded-full bg-divider dark:bg-divider"></div>
            <div class="absolute left-0 top-[16px] z-[-1] h-[4px] rounded-full bg-accent dark:bg-accent" bind:width="__slider__*2.24+8"></div>
            <div class="flex flex-row items-center h-control">
                <div class="h-[16px]" bind:width="__slider__*2.24"></div>
                <div class="w-[16px] h-[16px] rounded-full bg-surface border border-control dark:bg-surface dark:border-control"></div>
            </div>
            {children()}
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
