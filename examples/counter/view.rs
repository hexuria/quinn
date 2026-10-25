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
        <div class="flex flex-col gap-4 p-4">
            <h1>"Counter: " {move || count.get()}</h1>
            <button on:click=move |_| {
                set_count.update(|n| *n += 1);
            }>"Increment"</button>
        </div>
    }
}
