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
        <div class="flex flex-col odd:bg-system-fill">
            <div class="flex flex-row gap-3 items-center h-[36px] px-2">
                <span class="text-body text-label dark:text-label">{label}</span>
                <div class="flex-1"></div>
                <button class="px-3 h-control rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_count.update(|n| *n += 1); }>
                    {move || count.get()}
                </button>
                <button class="px-3 h-control rounded-control text-body text-system-blue border-0 bg-transparent dark:text-system-blue" on:click=move |_| { set_open.update(|v| *v = !*v); }>"Note"</button>
            </div>
            <Show when=move || open.get()>
                <h2 class="text-callout text-label-secondary dark:text-label-secondary px-2">"row note"</h2>
            </Show>
            <div class="h-[1px] bg-separator dark:bg-separator"></div>
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
        <div class="flex flex-col h-full bg-window-background dark:bg-window-background">
            <div class="flex flex-row items-center h-[52px] px-4 gap-2">
                <span class="text-headline text-label dark:text-label">"Keyed rows"</span>
                <div class="flex-1"></div>
                <button class="px-3 h-control-large rounded-control text-body bg-system-blue text-white dark:bg-system-blue" on:click=move |_| {
                    let id = next_id.get();
                    set_next.set(id + 1);
                    set_items.update(|rows| {
                        rows.push(Item { id, label: format!("Item {id}") });
                    });
                }>"Add"</button>
                <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| {
                    set_items.update(|rows| { rows.pop(); });
                }>"Remove last"</button>
                <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| {
                    set_items.update(|rows| { rows.reverse(); });
                }>"Reverse"</button>
            </div>
            <div class="h-[1px] bg-separator dark:bg-separator"></div>
            <div class="flex flex-col p-4 w-[640px]">
                <div class="rounded-window bg-system-background shadow-control dark:bg-system-background">
                    <For each=move || items.get() key=|item| item.id let:item let:index>
                        <Row item=item />
                    </For>
                </div>
                <span class="text-footnote text-label-secondary dark:text-label-secondary">"Each row keeps its own count when the list is reversed."</span>
            </div>
        </div>
    }
}
