use quinn::prelude::*;

#[component]
fn Form() -> impl IntoView {
    let (name, set_name) = signal(String::new());

    view! {
        <div class="flex flex-col h-full bg-window-background dark:bg-window-background">
            <div class="flex flex-row items-center h-[52px] px-4">
                <span class="text-title3 text-label dark:text-label">"Form"</span>
            </div>
            <div class="h-[1px] bg-separator dark:bg-separator"></div>
            <div class="flex flex-col flex-1 items-center justify-center p-8">
                <div class="flex flex-col gap-3 p-6 w-[420px] rounded-window bg-system-background shadow-control dark:bg-system-background">
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"YOUR NAME"</span>
                    <h1 class="text-title1 text-label dark:text-label">
                        "Hello, "
                        {move || if name.get().is_empty() { "there".to_string() } else { name.get() }}
                    </h1>
                    <input
                        class="h-control-large rounded-control bg-control-background text-body text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator"
                        placeholder="Your name"
                        value=move || name.get()
                        on:input=move |value: String| { set_name.set(value); }
                    />
                    <div class="flex flex-row gap-2">
                        <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_name.set(String::new()); }>"Clear"</button>
                    </div>
                </div>
            </div>
        </div>
    }
}
