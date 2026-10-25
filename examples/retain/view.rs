use quinn::prelude::*;

#[derive(Clone)]
struct Item {
    id: u64,
    label: String,
}

#[component]
fn Detail() -> impl IntoView {
    view! {
        <div class="p-2 rounded-control bg-system-background">
            <button class="px-3 h-control-large rounded-control text-body bg-system-blue text-white">"Detail"</button>
        </div>
    }
}

#[component]
fn Row(item: Item) -> impl IntoView {
    let label = item.label;
    view! {
        <div class="flex flex-row gap-2 items-center">
            <span class="text-body text-label">{label}</span>
            <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator">"row"</button>
        </div>
    }
}

#[component]
fn Retain() -> impl IntoView {
    let (open, set_open) = signal(true);
    let (items, set_items) = signal(vec![Item {
        id: 1,
        label: "Alpha".into(),
    }]);
    let (next_id, set_next) = signal(2u64);
    quinn::retain_probe();

    view! {
        <div class="flex flex-col gap-3 p-4 h-full bg-window-background">
            <h1 class="text-title2 text-label">"Retention"</h1>
            <Show when=move || open.get()>
                <Detail />
            </Show>
            <For each=move || items.get() key=|item| item.id let:item>
                <Row item=item />
            </For>
        </div>
    }
}
