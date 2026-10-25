use quinn::prelude::*;

/// Authoring source. This file is not a Rust module: Cargo does not compile
/// it, so saving it does not restart `cargo run`. The lowerer writes
/// `hot/ui.rs`, and a process started with `--hot` reloads that script.
#[component]
fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);
    quinn::set_frame_driver(move || {
        set_count.update(|n| *n += 1);
    });
    quinn::set_inspector(move || count.get_untracked().to_string());

    view! {
        <div class="flex flex-col h-full bg-window-background dark:bg-window-background">
            <div class="flex flex-row items-center h-[52px] px-4 gap-3">
                <span class="text-title3 text-label dark:text-label">"Counter"</span>
                <div class="flex-1"></div>
                <button class="px-3 h-control-large rounded-control text-body text-system-blue border-0 bg-transparent dark:text-system-blue" on:click=move |_| { toggle_theme(); }>"Theme"</button>
            </div>
            <div class="h-[1px] bg-separator dark:bg-separator"></div>
            <div class="flex flex-col flex-1 items-center justify-center p-8">
                <div class="flex flex-col gap-3 p-6 w-[420px] rounded-window bg-system-background shadow-control dark:bg-system-background">
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"COUNT"</span>
                    <h1 class="text-large-title text-label dark:text-label">"Counter: " {move || count.get()}</h1>
                    <span class="text-body text-label-secondary dark:text-label-secondary">"Each click adds one. The window also ticks while it is open."</span>
                    <div class="flex flex-row gap-2">
                        <button class="px-3 h-control-large rounded-control text-body bg-system-blue text-white dark:bg-system-blue" on:click=move |_| {
                            set_count.update(|n| *n += 1);
                        }>"Increment"</button>
                    </div>
                </div>
            </div>
        </div>
    }
}
