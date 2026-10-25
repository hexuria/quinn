use quinn::prelude::*;

#[derive(Clone)]
struct Item {
    id: u64,
    label: String,
}

#[component]
fn Row(item: Item) -> impl IntoView {
    let (count, set_count) = signal(0i32);
    let (open, set_open) = signal(false);
    let label = item.label;
    view! {
        <div class="flex flex-row gap-4 items-center">
            <span class="text-lg">{label}</span>
            <button on:click=move |_| { set_count.update(|n| *n += 1); }>
                {move || count.get()}
            </button>
            <button on:click=move |_| { set_open.update(|v| *v = !*v); }>"Note"</button>
            <Show when=move || open.get()>
                <h2 class="text-sm text-amber-700">"row note"</h2>
            </Show>
        </div>
    }
}

#[component]
fn KeyedList() -> impl IntoView {
    let (items, set_items) = signal(vec![
        Item {
            id: 1,
            label: "Alpha".into(),
        },
        Item {
            id: 2,
            label: "Beta".into(),
        },
        Item {
            id: 3,
            label: "Gamma".into(),
        },
    ]);
    let (next_id, set_next) = signal(4u64);

    view! {
        <div class="flex flex-col gap-3 p-4">
            <h1>"Keyed rows"</h1>
            <For each=move || items.get() key=|item| item.id let:item>
                <Row item=item />
            </For>
            <div class="flex flex-row gap-2">
                <button on:click=move |_| {
                    let id = next_id.get();
                    set_next.set(id + 1);
                    set_items.update(|rows| {
                        rows.push(Item { id, label: format!("Item {id}") });
                    });
                }>"Add"</button>
                <button on:click=move |_| {
                    set_items.update(|rows| { rows.pop(); });
                }>"Remove last"</button>
                <button on:click=move |_| {
                    set_items.update(|rows| { rows.reverse(); });
                }>"Reverse"</button>
            </div>
        </div>
    }
}
