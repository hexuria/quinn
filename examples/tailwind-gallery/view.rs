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
        <div class="flex flex-row items-center gap-2 h-[32px]">
            <span class="text-body text-label dark:text-label">{label}</span>
            <button class="px-3 h-control rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_open.update(|v| *v = !*v); }>"Note"</button>
            <Show when=move || open.get()>
                <span class="text-body text-system-orange dark:text-system-orange">"on"</span>
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
        <div class="flex flex-col h-full bg-window-background dark:bg-window-background overflow-y-auto">
            <div class="flex flex-row items-center h-[52px] px-4 gap-3">
                <span class="text-headline text-label dark:text-label">"Layout"</span>
                <span class="text-body text-label dark:text-label">"Count: " {move || count.get()}</span>
                <button class="px-3 h-control-large rounded-control text-body bg-system-blue text-white dark:bg-system-blue" on:click=move |_| { set_count.update(|n| *n += 2); }>"Add two"</button>
                <button class="px-3 h-control-large rounded-control text-body text-system-blue border-0 bg-transparent dark:text-system-blue" on:click=move |_| { toggle_theme(); }>"Theme"</button>
            </div>
            <div class="h-[1px] bg-separator dark:bg-separator"></div>
            <div class="flex flex-col md:flex-row gap-4 p-4">
                <div class="flex flex-col gap-3">
                    <div class="flex flex-col md:flex-row gap-2">
                        <span class="text-callout text-label-secondary dark:text-label-secondary">"Stacked until md"</span>
                        <span class="text-callout text-label dark:text-label md:font-bold">"Bold from md"</span>
                    </div>
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"JUSTIFY BETWEEN"</span>
                    <div class="flex flex-row justify-between items-center bg-system-background p-3 rounded-window shadow-control dark:bg-system-background">
                        <div class="flex flex-row items-center w-[64px] h-[32px] rounded-control bg-system-blue"><span class="text-body text-white">"A"</span></div>
                        <div class="flex flex-row items-center w-[64px] h-[32px] rounded-control bg-system-orange"><span class="text-body text-label">"B"</span></div>
                        <div class="flex flex-row items-center w-[64px] h-[32px] rounded-control bg-label"><span class="text-body text-white">"C"</span></div>
                    </div>
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"FLEX WRAP"</span>
                    <div class="flex flex-row flex-wrap gap-2 gap-y-4 bg-system-background p-3 rounded-window shadow-control dark:bg-system-background">
                        <div class="flex flex-row items-center w-[64px] h-[32px] rounded-control bg-system-fill"><span class="text-body text-label dark:text-label">"w1"</span></div>
                        <div class="flex flex-row items-center w-[64px] h-[32px] rounded-control bg-system-fill"><span class="text-body text-label dark:text-label">"w2"</span></div>
                        <div class="flex flex-row items-center w-[64px] h-[32px] rounded-control bg-system-fill"><span class="text-body text-label dark:text-label">"w3"</span></div>
                        <div class="flex flex-row items-center w-[64px] h-[32px] rounded-control bg-system-fill"><span class="text-body text-label dark:text-label">"w4"</span></div>
                        <div class="flex flex-row items-center w-[64px] h-[32px] rounded-control bg-system-fill"><span class="text-body text-label dark:text-label">"w5"</span></div>
                        <div class="flex flex-row items-center w-[64px] h-[32px] rounded-control bg-system-fill"><span class="text-body text-label dark:text-label">"w6"</span></div>
                        <div class="flex flex-row items-center w-[64px] h-[32px] rounded-control bg-system-fill"><span class="text-body text-label dark:text-label">"w7"</span></div>
                        <div class="flex flex-row items-center w-[64px] h-[32px] rounded-control bg-system-fill"><span class="text-body text-label dark:text-label">"w8"</span></div>
                    </div>
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"ITEMS CENTER"</span>
                    <div class="flex flex-row items-center gap-2 bg-system-background p-3 rounded-window shadow-control dark:bg-system-background">
                        <div class="flex flex-row items-center w-[64px] h-[48px] rounded-control bg-label"><span class="text-body text-white">"tall"</span></div>
                        <div class="flex flex-row items-center w-[64px] h-[32px] rounded-control bg-system-orange"><span class="text-body text-label">"mid"</span></div>
                    </div>
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"FLEX 1"</span>
                    <div class="flex flex-row gap-2">
                        <div class="flex flex-row items-center flex-1 h-[32px] rounded-control bg-system-orange"><span class="text-body text-label">"grow"</span></div>
                        <div class="flex flex-row items-center flex-1 h-[32px] rounded-control bg-label"><span class="text-body text-white">"grow"</span></div>
                    </div>
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"HALF"</span>
                    <div class="flex flex-row bg-system-background rounded-window dark:bg-system-background">
                        <div class="flex flex-row items-center w-1/2 h-[32px] rounded-control bg-system-blue"><span class="text-body text-white">"half"</span></div>
                    </div>
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"MIN / MAX"</span>
                    <div class="flex flex-row items-center gap-2">
                        <div class="flex flex-row items-center w-[40px] min-w-[80px] h-[32px] rounded-control bg-system-orange"><span class="text-body text-label">"min"</span></div>
                        <div class="flex flex-row items-center w-[120px] max-w-[48px] h-[32px] rounded-control bg-label"><span class="text-body text-white">"max"</span></div>
                    </div>
                </div>
                <div class="flex flex-col gap-3">
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"PADDING"</span>
                    <div class="flex flex-row items-center gap-3">
                        <div class="w-fit px-8 py-2 bg-system-fill rounded-control"><span class="text-body text-label dark:text-label">"px py"</span></div>
                        <div class="w-fit p-2 bg-system-fill rounded-control"><span class="text-body text-label dark:text-label">"p"</span></div>
                    </div>
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"MARGIN"</span>
                    <div class="bg-system-background rounded-window dark:bg-system-background">
                        <div class="mx-8 my-2 bg-system-blue rounded-control"><span class="text-body text-white">"mx my"</span></div>
                    </div>
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"ARBITRARY"</span>
                    <div class="flex flex-row items-center gap-2">
                        <div class="w-[13px] h-[32px] rounded-control-sm bg-[#1e293b]"></div>
                        <div class="bg-[#1e293b] p-[10px] rounded-control"><span class="text-body text-[#fff]">"hex"</span></div>
                    </div>
                    <div class="flex flex-row items-center gap-2">
                        <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_tint.update(|v| *v = !*v); }>"Tint"</button>
                        <div class="bg-system-fill px-4 py-2 rounded-control" class:bg-system-blue=move || tint.get()><span class="text-body text-label dark:text-label">"class:"</span></div>
                    </div>
                    <div class="flex flex-row items-center gap-2">
                        <button class="px-3 h-control-large rounded-control text-body bg-label hover:bg-system-orange active:bg-system-blue text-white">"Hover me"</button>
                        <button class="px-3 h-control-large rounded-control text-body disabled disabled:bg-system-fill disabled:text-label-tertiary">"Disabled"</button>
                    </div>
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"KEYED SHOW"</span>
                    <For each=move || items.get() key=|item| item.id let:item>
                        <Chip item=item />
                    </For>
                    <div class="flex flex-row flex-wrap gap-2">
                        <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| {
                            let id = next_id.get();
                            set_next.set(id + 1);
                            set_items.update(|rows| { rows.push(Item { id, label: format!("Item {id}") }); });
                        }>"Add"</button>
                        <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_items.update(|rows| { rows.pop(); }); }>"Pop"</button>
                        <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_items.update(|rows| { rows.reverse(); }); }>"Reverse"</button>
                        <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_items.update(|rows| { rows.swap(1, 2); }); }>"Swap"</button>
                        <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_items.update(|rows| { rows.remove(1); }); }>"Remove"</button>
                    </div>
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"GRID"</span>
                    <div class="grid grid-cols-3 gap-2">
                        <div class="col-span-2 h-[48px] rounded-control bg-gradient-to-r from-system-blue to-system-indigo shadow-control">
                            <span class="text-body text-white font-bold">"wide"</span>
                        </div>
                        <div class="h-[48px] rounded-control bg-system-fill shadow-control">
                            <span class="text-body text-label dark:text-label font-medium">"side"</span>
                        </div>
                    </div>
                    <div class="h-[32px]">
                        <div class="absolute left-1 top-0">
                            <span class="text-caption text-label-secondary dark:text-label-secondary leading-tight">"pinned"</span>
                        </div>
                    </div>
                    <div class="h-[72px] overflow-y-auto bg-system-background rounded-window shadow-control dark:bg-system-background">
                        <div class="flex flex-col px-3 py-2">
                            <span class="text-body text-label dark:text-label leading-relaxed">"Scroll the gallery note."</span>
                            <span class="text-body text-label dark:text-label leading-relaxed">"It runs past the box."</span>
                            <span class="text-body text-label dark:text-label leading-relaxed">"The bar has somewhere to go."</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
