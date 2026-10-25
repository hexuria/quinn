use quinn::prelude::*;

#[derive(Clone)]
struct Item {
    id: u64,
    label: String,
}

#[component]
fn Chip(item: Item) -> impl IntoView {
    let (open, set_open) = signal(false);
    let label = item.label;
    view! {
        <div class="flex flex-row items-center gap-2">
            <span class="text-sm">{label}</span>
            <button class="px-2 py-1" on:click=move |_| { set_open.update(|v| *v = !*v); }>"Note"</button>
            <Show when=move || open.get()>
                <span class="text-sm text-amber-700">"on"</span>
            </Show>
        </div>
    }
}

#[component]
fn Gallery() -> impl IntoView {
    let (count, set_count) = signal(0i32);
    let (tint, set_tint) = signal(false);
    let (items, set_items) = signal(vec![
        Item { id: 1, label: "Alpha".into() },
        Item { id: 2, label: "Beta".into() },
    ]);
    let (next_id, set_next) = signal(3u64);

    view! {
        <div class="flex flex-col gap-px p-2 bg-stone-100">
            <h1 class="text-lg">"Tailwind gallery"</h1>
            <div class="flex flex-row items-center gap-3">
                <span class="text-lg">"Count: " {move || count.get()}</span>
                <button class="px-3 py-1" on:click=move |_| { set_count.update(|n| *n += 2); }>"Add two"</button>
            </div>

            <span class="text-sm">"justify-between"</span>
            <div class="flex flex-row justify-between items-center bg-slate-100 p-2 rounded">
                <div class="w-[64px] h-[22px] bg-amber-700"><span class="text-white text-sm">"A"</span></div>
                <div class="w-[64px] h-[22px] bg-amber-600"><span class="text-sm">"B"</span></div>
                <div class="w-[64px] h-[22px] bg-stone-900"><span class="text-white text-sm">"C"</span></div>
            </div>

            <span class="text-sm">"flex-wrap"</span>
            <div class="flex flex-row flex-wrap gap-2 gap-y-4 bg-slate-100 p-2 rounded">
                <div class="w-[64px] h-[18px] bg-amber-100"><span class="text-sm">"w1"</span></div>
                <div class="w-[64px] h-[18px] bg-amber-100"><span class="text-sm">"w2"</span></div>
                <div class="w-[64px] h-[18px] bg-amber-100"><span class="text-sm">"w3"</span></div>
                <div class="w-[64px] h-[18px] bg-amber-100"><span class="text-sm">"w4"</span></div>
                <div class="w-[64px] h-[18px] bg-amber-100"><span class="text-sm">"w5"</span></div>
                <div class="w-[64px] h-[18px] bg-amber-100"><span class="text-sm">"w6"</span></div>
                <div class="w-[64px] h-[18px] bg-amber-100"><span class="text-sm">"w7"</span></div>
                <div class="w-[64px] h-[18px] bg-amber-100"><span class="text-sm">"w8"</span></div>
            </div>

            <span class="text-sm">"items-center"</span>
            <div class="flex flex-row items-center gap-2 bg-slate-100 p-2 rounded">
                <div class="w-[40px] h-[36px] bg-stone-900"><span class="text-white text-sm">"tall"</span></div>
                <div class="w-[40px] h-[14px] bg-amber-600"><span class="text-sm">"mid"</span></div>
            </div>

            <span class="text-sm">"flex-1"</span>
            <div class="flex flex-row gap-2">
                <div class="flex-1 h-[22px] bg-amber-600"><span class="text-sm">"grow"</span></div>
                <div class="flex-1 h-[22px] bg-stone-900"><span class="text-white text-sm">"grow"</span></div>
            </div>

            <span class="text-sm">"w-1/2"</span>
            <div class="flex flex-row bg-slate-100 rounded">
                <div class="w-1/2 h-[20px] bg-amber-700"><span class="text-white text-sm">"half"</span></div>
            </div>

            <span class="text-sm">"min / max"</span>
            <div class="flex flex-row items-center gap-2">
                <div class="w-[40px] min-w-[80px] h-[20px] bg-amber-600"><span class="text-sm">"min"</span></div>
                <div class="w-[120px] max-w-[48px] h-[20px] bg-stone-900"><span class="text-white text-sm">"max"</span></div>
            </div>

            <span class="text-sm">"px py vs p"</span>
            <div class="flex flex-row items-center gap-3">
                <div class="w-fit px-8 py-1 bg-amber-100 rounded"><span>"px py"</span></div>
                <div class="w-fit p-1 bg-amber-100 rounded"><span>"p"</span></div>
            </div>

            <span class="text-sm">"margin"</span>
            <div class="bg-slate-100 rounded">
                <div class="mx-8 my-1 bg-amber-700"><span class="text-white">"mx my"</span></div>
            </div>

            <span class="text-sm">"arbitrary"</span>
            <div class="flex flex-row items-center gap-2">
                <div class="w-[13px] h-[28px] bg-[#1e293b]"></div>
                <div class="bg-[#1e293b] p-[10px] rounded"><span class="text-[#fff]">"hex"</span></div>
            </div>

            <div class="flex flex-row items-center gap-2">
                <button class="px-3 py-1" on:click=move |_| { set_tint.update(|v| *v = !*v); }>"Tint"</button>
                <div class="bg-slate-100 px-4 py-1 rounded" class:bg-amber-600=move || tint.get()><span>"class:"</span></div>
            </div>

            <div class="flex flex-row items-center gap-2">
                <button class="bg-stone-900 hover:bg-amber-600 active:bg-amber-700 text-white px-3 py-1">"Hover me"</button>
                <button class="disabled disabled:bg-slate-100 disabled:text-slate-600 px-3 py-1">"Disabled"</button>
            </div>

            <span class="text-sm">"keyed show"</span>
            <For each=move || items.get() key=|item| item.id let:item>
                <Chip item=item />
            </For>
            <div class="flex flex-row flex-wrap gap-2">
                <button class="px-3 py-1" on:click=move |_| {
                    let id = next_id.get();
                    set_next.set(id + 1);
                    set_items.update(|rows| { rows.push(Item { id, label: format!("Item {id}") }); });
                }>"Add"</button>
                <button class="px-3 py-1" on:click=move |_| { set_items.update(|rows| { rows.pop(); }); }>"Pop"</button>
                <button class="px-3 py-1" on:click=move |_| { set_items.update(|rows| { rows.reverse(); }); }>"Reverse"</button>
                <button class="px-3 py-1" on:click=move |_| { set_items.update(|rows| { rows.swap(1, 2); }); }>"Swap"</button>
                <button class="px-3 py-1" on:click=move |_| { set_items.update(|rows| { rows.remove(1); }); }>"Remove"</button>
            </div>
        </div>
    }
}
