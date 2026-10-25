use quinn::prelude::*;

#[component]
fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);
    let set_probe = set_count;
    let read_probe = count;
    quinn::set_frame_driver(move || {
        set_probe.update(|n| *n += 1);
    });
    quinn::set_inspector(move || read_probe.get_untracked().to_string());

    view! {
        <div class="flex flex-col gap-4 p-4">
            <h1>"Counter: " {move || count.get()}</h1>
            <button on:click=move |_| {
                set_count.update(|n| *n += 1);
            }>"Increment"</button>
        </div>
    }
}

quinn::run!("Counter", 480, 360, view! { <Counter /> });
