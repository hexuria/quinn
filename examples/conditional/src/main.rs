use quinn::prelude::*;

#[component]
fn Detail() -> impl IntoView {
    let (clicks, set_clicks) = signal(0);
    view! {
        <div class="flex flex-col gap-2 p-4 rounded bg-amber-100">
            <h2>"Detail is mounted"</h2>
            <button on:click=move |_| { set_clicks.update(|n| *n += 1); }>
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
        <div class="flex flex-col gap-4 p-4">
            <h1>"Sibling ticks: " {move || ticks.get()}</h1>
            <button on:click=move |_| { set_ticks.update(|n| *n += 1); }>
                "Tick sibling"
            </button>
            <button on:click=move |_| { set_open.update(|open| *open = !*open); }>
                {move || if open.get() { "Hide detail" } else { "Show detail" }}
            </button>
            <Show when=move || open.get()>
                <Detail />
            </Show>
        </div>
    }
}

quinn::run!("Conditional", 520, 420, view! { <Conditional /> });
