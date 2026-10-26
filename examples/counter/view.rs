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
        <div class="flex flex-col h-full bg-canvas dark:bg-canvas">
            <div class="flex flex-row items-center h-toolbar px-4 gap-2 bg-surface dark:bg-surface">
                <span class="text-section text-primary dark:text-primary">"Counter"</span>
                <div class="flex-1"></div>
                <button class="h-control px-3 rounded-control text-control text-accent border-0 bg-transparent dark:text-accent" on:click=move |_| { toggle_theme(); }>"Theme"</button>
            </div>
            <div class="h-[1px] bg-divider dark:bg-divider"></div>
            <div class="flex flex-col flex-1 items-center justify-center p-6">
                <div class="flex flex-col gap-4 p-6 w-[420px] max-w-full rounded-[8px] bg-surface border border-divider dark:bg-surface dark:border-divider">
                    <span class="text-caption text-secondary dark:text-secondary">"Count"</span>
                    <h1 class="text-page text-primary dark:text-primary">"Counter: " {move || count.get()}</h1>
                    <span class="text-body text-secondary dark:text-secondary">"Each click adds one. The window also ticks while it is open."</span>
                    <div class="flex flex-row gap-2">
                        <button class="h-control px-3 rounded-control text-control bg-accent text-on-accent border border-accent dark:bg-accent dark:text-on-accent dark:border-accent" on:click=move |_| {
                            set_count.update(|n| *n += 1);
                        }>"Increment"</button>
                    </div>
                </div>
            </div>
        </div>
    }
}
