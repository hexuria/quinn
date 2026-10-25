use quinn::prelude::*;

#[component]
fn Detail() -> impl IntoView {
    let (clicks, set_clicks) = signal(0);
    view! {
        <div class="flex flex-col gap-2 p-4 rounded-window bg-system-background shadow-control dark:bg-system-background">
            <h2 class="text-title3 text-label dark:text-label">"Detail is mounted"</h2>
            <span class="text-callout text-label-secondary dark:text-label-secondary">"Closing this card throws the count away. Opening it starts again."</span>
            <button class="px-3 h-control-large rounded-control text-body bg-system-blue text-white dark:bg-system-blue" on:click=move |_| { set_clicks.update(|n| *n += 1); }>
                "Detail clicks: " {move || clicks.get()}
            </button>
        </div>
    }
}

#[component]
fn Conditional() -> impl IntoView {
    let (open, set_open) = signal(true);
    let (ticks, set_ticks) = signal(0);

    view! {
        <div class="flex flex-col h-full bg-window-background dark:bg-window-background">
            <div class="flex flex-row items-center h-[52px] px-4">
                <span class="text-title3 text-label dark:text-label">"Conditional"</span>
            </div>
            <div class="h-[1px] bg-separator dark:bg-separator"></div>
            <div class="flex flex-col gap-4 p-6 w-[520px]">
                <h1 class="text-title2 text-label dark:text-label">"Sibling ticks: " {move || ticks.get()}</h1>
                <div class="flex flex-row gap-2">
                    <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_ticks.update(|n| *n += 1); }>
                        "Tick sibling"
                    </button>
                    <button class="px-3 h-control-large rounded-control text-body text-system-blue border-0 bg-transparent dark:text-system-blue" on:click=move |_| { set_open.update(|open| *open = !*open); }>
                        {move || if open.get() { "Hide detail" } else { "Show detail" }}
                    </button>
                </div>
                <Show when=move || open.get()>
                    <Detail />
                </Show>
            </div>
        </div>
    }
}
