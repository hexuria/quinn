use quinn::prelude::*;

#[derive(Clone)]
struct Item {
    id: u64,
    label: String,
}

#[component]
fn NoteRow(item: Item) -> impl IntoView {
    let label = item.label;
    view! {
        <div class="flex flex-col">
            <div class="flex flex-row items-center h-[36px] px-3 gap-3">
                <span class="text-body text-label dark:text-label">{label}</span>
                <div class="flex-1"></div>
                <span class="text-callout text-label-secondary dark:text-label-secondary">"Note"</span>
            </div>
            <div class="h-[1px] bg-separator dark:bg-separator"></div>
        </div>
    }
}

#[component]
fn Showcase() -> impl IntoView {
    let (notes, set_notes) = signal(true);
    let (table, set_table) = signal(false);
    let (settings, set_settings) = signal(false);
    let (sheet, set_sheet) = signal(false);
    let (alert, set_alert) = signal(false);
    let (toast, set_toast) = signal(false);
    let (checked, set_checked) = signal(true);
    let (check_off, set_check_off) = signal(false);
    let (switched, set_switched) = signal(true);
    let (switch_off, set_switch_off) = signal(false);
    let (menu, set_menu) = signal(false);
    let (choice, set_choice) = signal("On My Mac".to_string());
    let (query, set_query) = signal(String::new());
    let (draft, set_draft) = signal(String::new());
    let (step1, set_step1) = signal(false);
    let (step2, set_step2) = signal(true);
    let (step3, set_step3) = signal(false);
    let (step4, set_step4) = signal(false);
    let (day, set_day) = signal(true);
    let (week, set_week) = signal(false);
    let (month, set_month) = signal(false);
    let (rows, set_rows) = signal(vec![
        Item { id: 1, label: "Grocery list".into() },
        Item { id: 2, label: "Kitchen remodel".into() },
        Item { id: 3, label: "Saturday market".into() },
        Item { id: 4, label: "Paint colors".into() },
    ]);

    view! {
        <div class="flex flex-col h-full bg-window-background dark:bg-window-background">
            <div class="flex flex-row items-center h-[52px] px-4 gap-3">
                <span class="text-title3 text-label dark:text-label">"Notes"</span>
                <div class="flex-1"></div>
                <input
                    class="w-[280px] h-control-large rounded-control bg-system-fill text-body text-label border border-separator dark:bg-system-fill dark:text-label dark:border-separator"
                    bind:value=query
                    placeholder="Search"
                />
                <div class="flex-1"></div>
                <button class="px-3 h-control-large rounded-control text-body text-system-blue border-0 bg-transparent dark:text-system-blue" on:click=move |_| { toggle_theme(); }>"Theme"</button>
                <button class="px-3 h-control-large rounded-control text-body bg-system-blue text-white dark:bg-system-blue" on:click=move |_| { set_sheet.set(true); }>"New Note"</button>
            </div>
            <div class="h-[1px] bg-separator dark:bg-separator"></div>
            <div class="flex flex-col md:flex-row flex-1 items-stretch">
                <div class="flex flex-col w-full md:w-[240px] bg-sidebar dark:bg-sidebar p-3 gap-1">
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"LIBRARY"</span>
                    <div class="w-full h-[28px]">
                        <button
                            class="w-full h-[28px] pl-[32px] rounded-control text-body text-label border-0 bg-transparent dark:text-label"
                            class:bg-system-blue=move || notes.get()
                            class:text-white=move || notes.get()
                            on:click=move |_| {
                                set_notes.set(true);
                                set_table.set(false);
                                set_settings.set(false);
                            }
                        >"Notes"</button>
                        <div class="absolute left-2 top-[5px] w-[18px] h-[18px] rounded-control-sm bg-system-orange"></div>
                    </div>
                    <div class="w-full h-[28px]">
                        <button
                            class="w-full h-[28px] pl-[32px] rounded-control text-body text-label border-0 bg-transparent dark:text-label"
                            class:bg-system-blue=move || table.get()
                            class:text-white=move || table.get()
                            on:click=move |_| {
                                set_notes.set(false);
                                set_table.set(true);
                                set_settings.set(false);
                            }
                        >"Table"</button>
                        <div class="absolute left-2 top-[5px] w-[18px] h-[18px] rounded-control-sm bg-system-blue"></div>
                    </div>
                    <div class="w-full h-[28px]">
                        <button
                            class="w-full h-[28px] pl-[32px] rounded-control text-body text-label border-0 bg-transparent dark:text-label"
                            class:bg-system-blue=move || settings.get()
                            class:text-white=move || settings.get()
                            on:click=move |_| {
                                set_notes.set(false);
                                set_table.set(false);
                                set_settings.set(true);
                            }
                        >"Settings"</button>
                        <div class="absolute left-2 top-[5px] w-[18px] h-[18px] rounded-control-sm bg-system-gray"></div>
                    </div>
                </div>
                <div class="w-full md:w-[1px] h-[1px] md:h-full bg-separator dark:bg-separator"></div>
                <div class="flex flex-col flex-1 p-6 gap-4 overflow-y-auto bg-system-background dark:bg-system-background">
                    <Show when=move || toast.get()>
                        <div class="flex flex-row items-center gap-3 px-3 h-[36px] rounded-control bg-system-fill dark:bg-system-fill">
                            <span class="text-body text-label dark:text-label">"Moved to Recently Deleted"</span>
                            <div class="flex-1"></div>
                            <button class="px-2 h-control rounded-control text-body text-system-blue border-0 bg-transparent dark:text-system-blue" on:click=move |_| { set_toast.set(false); }>"Dismiss"</button>
                        </div>
                    </Show>
                    <Show when=move || notes.get()>
                        <div class="flex flex-col gap-4">
                            <span class="text-large-title text-label dark:text-label">"Kitchen"</span>
                            <span class="text-footnote text-label-secondary dark:text-label-secondary">"Edited just now"</span>
                            <span class="text-body text-label dark:text-label">"Pick up citrus, olive oil, and the blue linen. The cabinet paint is still the sample from last week."</span>
                            <span class="text-body text-label-secondary dark:text-label-secondary">"Call the joiner before Thursday. Leave the island where it is."</span>
                            <span class="text-footnote text-label-secondary dark:text-label-secondary">"THIS WEEK"</span>
                            <div class="flex flex-row items-end gap-2 h-[88px]">
                                <div class="w-[18px] h-[28px] rounded-control-sm bg-system-blue"></div>
                                <div class="w-[18px] h-[44px] rounded-control-sm bg-system-blue"></div>
                                <div class="w-[18px] h-[36px] rounded-control-sm bg-system-blue"></div>
                                <div class="w-[18px] h-[64px] rounded-control-sm bg-system-blue"></div>
                                <div class="w-[18px] h-[52px] rounded-control-sm bg-system-blue"></div>
                                <div class="w-[18px] h-[40px] rounded-control-sm bg-system-blue"></div>
                                <div class="w-[18px] h-[24px] rounded-control-sm bg-system-blue"></div>
                            </div>
                        </div>
                    </Show>
                    <Show when=move || table.get()>
                        <div class="flex flex-col gap-3">
                            <span class="text-title2 text-label dark:text-label">"All Notes"</span>
                            <div class="rounded-window bg-system-background-secondary dark:bg-system-background-secondary">
                                <div class="flex flex-row items-center h-[28px] px-3">
                                    <button class="px-2 h-control-small rounded-control text-footnote text-label-secondary border-0 bg-transparent dark:text-label-secondary" on:click=move |_| {
                                        set_rows.update(|rows| { rows.reverse(); });
                                    }>"Name"</button>
                                    <div class="flex-1"></div>
                                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"Kind"</span>
                                </div>
                                <div class="h-[1px] bg-separator dark:bg-separator"></div>
                                <For each=move || rows.get() key=|item| item.id let:item>
                                    <NoteRow item=item />
                                </For>
                            </div>
                        </div>
                    </Show>
                    <Show when=move || settings.get()>
                        <div class="flex flex-col gap-4">
                            <span class="text-title2 text-label dark:text-label">"Settings"</span>
                            <div class="flex flex-col md:flex-row gap-6">
                                <div class="flex flex-col gap-4">
                                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"SEGMENTED CONTROL"</span>
                                    <div class="flex flex-row gap-1 p-1 rounded-control bg-system-fill dark:bg-system-fill w-fit">
                                        <button
                                            class="px-3 h-control rounded-control-sm text-body text-label border-0 bg-transparent dark:text-label"
                                            class:bg-system-background=move || day.get()
                                            class:shadow-control=move || day.get()
                                            on:click=move |_| { set_day.set(true); set_week.set(false); set_month.set(false); }
                                        >"Day"</button>
                                        <button
                                            class="px-3 h-control rounded-control-sm text-body text-label border-0 bg-transparent dark:text-label"
                                            class:bg-system-background=move || week.get()
                                            class:shadow-control=move || week.get()
                                            on:click=move |_| { set_day.set(false); set_week.set(true); set_month.set(false); }
                                        >"Week"</button>
                                        <button
                                            class="px-3 h-control rounded-control-sm text-body text-label border-0 bg-transparent dark:text-label"
                                            class:bg-system-background=move || month.get()
                                            class:shadow-control=move || month.get()
                                            on:click=move |_| { set_day.set(false); set_week.set(false); set_month.set(true); }
                                        >"Month"</button>
                                    </div>
                                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"SWITCH"</span>
                                    <div class="flex flex-row items-center gap-3">
                                        <Show when=move || switch_off.get()>
                                            <div class="flex flex-row items-center w-[44px] h-[26px] rounded-full bg-system-fill px-1 dark:bg-system-fill">
                                                <div class="w-[18px] h-[18px] rounded-full bg-white"></div>
                                                <button class="absolute inset-0 border-0 bg-transparent" on:click=move |_| { set_switched.set(true); set_switch_off.set(false); }>" "</button>
                                            </div>
                                        </Show>
                                        <Show when=move || switched.get()>
                                            <div class="flex flex-row items-center justify-end w-[44px] h-[26px] rounded-full bg-system-green px-1 dark:bg-system-green">
                                                <div class="w-[18px] h-[18px] rounded-full bg-white"></div>
                                                <button class="absolute inset-0 border-0 bg-transparent" on:click=move |_| { set_switched.set(false); set_switch_off.set(true); }>" "</button>
                                            </div>
                                        </Show>
                                        <span class="text-body text-label dark:text-label">"iCloud"</span>
                                    </div>
                                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"CHECKBOX"</span>
                                    <div class="flex flex-row items-center gap-3">
                                        <Show when=move || check_off.get()>
                                            <button class="w-[18px] h-[18px] rounded-control-sm bg-control-background border border-separator dark:bg-control-background dark:border-separator" on:click=move |_| { set_checked.set(true); set_check_off.set(false); }>" "</button>
                                        </Show>
                                        <Show when=move || checked.get()>
                                            <button class="w-[18px] h-[18px] rounded-control-sm bg-system-blue border-0 text-white text-caption dark:bg-system-blue" on:click=move |_| { set_checked.set(false); set_check_off.set(true); }>"x"</button>
                                        </Show>
                                        <span class="text-body text-label dark:text-label">"Show in menu bar"</span>
                                    </div>
                                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"POP-UP"</span>
                                    <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator w-fit" on:click=move |_| { set_menu.update(|v| *v = !*v); }>
                                        {choice}
                                    </button>
                                    <Show when=move || menu.get()>
                                        <div class="flex flex-col w-[220px] p-1 rounded-control bg-system-background shadow-window dark:bg-system-background">
                                            <button class="px-3 h-control-large rounded-control-sm text-body text-label border-0 bg-transparent dark:text-label" on:click=move |_| { set_choice.set("On My Mac".to_string()); set_menu.set(false); }>"On My Mac"</button>
                                            <button class="px-3 h-control-large rounded-control-sm text-body text-label border-0 bg-transparent dark:text-label" on:click=move |_| { set_choice.set("iCloud".to_string()); set_menu.set(false); }>"iCloud"</button>
                                            <button class="px-3 h-control-large rounded-control-sm text-body text-label border-0 bg-transparent dark:text-label" on:click=move |_| { set_choice.set("Recently Deleted".to_string()); set_menu.set(false); }>"Recently Deleted"</button>
                                        </div>
                                    </Show>
                                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"TEXT FIELD"</span>
                                    <input
                                        class="h-control-large rounded-control bg-control-background text-body text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator"
                                        bind:value=draft
                                        placeholder="Folder name"
                                    />
                                </div>
                                <div class="flex flex-col gap-4">
                                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"PUSH BUTTONS"</span>
                                    <div class="flex flex-row gap-2">
                                        <button class="px-3 h-control-large rounded-control text-body text-system-red border-0 bg-transparent dark:text-system-red" on:click=move |_| { set_alert.set(true); }>"Delete Note"</button>
                                        <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_toast.set(true); }>"Bordered"</button>
                                        <button class="px-3 h-control-large rounded-control text-body text-system-blue border-0 bg-transparent dark:text-system-blue">"Borderless"</button>
                                    </div>
                                    <button class="px-3 h-control-large rounded-control text-body disabled disabled:bg-system-fill disabled:text-label-tertiary w-fit">"Disabled"</button>
                                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"STEPPER"</span>
                                    <div class="flex flex-row items-center gap-2">
                                        <button class="w-[28px] h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_step1.set(true); set_step2.set(false); set_step3.set(false); set_step4.set(false); }>"−"</button>
                                        <Show when=move || step1.get()><span class="text-body text-label dark:text-label">"1"</span></Show>
                                        <Show when=move || step2.get()><span class="text-body text-label dark:text-label">"2"</span></Show>
                                        <Show when=move || step3.get()><span class="text-body text-label dark:text-label">"3"</span></Show>
                                        <Show when=move || step4.get()><span class="text-body text-label dark:text-label">"4"</span></Show>
                                        <button class="w-[28px] h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_step1.set(false); set_step2.set(false); set_step3.set(false); set_step4.set(true); }>"+"</button>
                                        <button class="px-3 h-control rounded-control text-callout bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_step1.set(true); set_step2.set(false); set_step3.set(false); set_step4.set(false); }>"25%"</button>
                                        <button class="px-3 h-control rounded-control text-callout bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_step1.set(false); set_step2.set(true); set_step3.set(false); set_step4.set(false); }>"50%"</button>
                                        <button class="px-3 h-control rounded-control text-callout bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_step1.set(false); set_step2.set(false); set_step3.set(true); set_step4.set(false); }>"75%"</button>
                                        <button class="px-3 h-control rounded-control text-callout bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_step1.set(false); set_step2.set(false); set_step3.set(false); set_step4.set(true); }>"Full"</button>
                                    </div>
                                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"PROGRESS"</span>
                                    <div class="w-full h-[8px] rounded-full bg-system-fill dark:bg-system-fill">
                                        <Show when=move || step1.get()>
                                            <div class="h-[8px] w-1/4 rounded-full bg-system-blue"></div>
                                        </Show>
                                        <Show when=move || step2.get()>
                                            <div class="h-[8px] w-1/2 rounded-full bg-system-blue"></div>
                                        </Show>
                                        <Show when=move || step3.get()>
                                            <div class="h-[8px] w-3/4 rounded-full bg-system-blue"></div>
                                        </Show>
                                        <Show when=move || step4.get()>
                                            <div class="h-[8px] w-full rounded-full bg-system-blue"></div>
                                        </Show>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </Show>
                </div>
                <div class="w-full md:w-[1px] h-[1px] md:h-full bg-separator dark:bg-separator"></div>
                <div class="flex flex-col w-full md:w-[280px] p-4 gap-3 bg-system-background-secondary dark:bg-system-background-secondary">
                    <span class="text-footnote text-label-secondary dark:text-label-secondary">"INSPECTOR"</span>
                    <Show when=move || notes.get()>
                        <span class="text-headline text-label dark:text-label">"Kitchen"</span>
                    </Show>
                    <Show when=move || table.get()>
                        <span class="text-headline text-label dark:text-label">"All Notes"</span>
                    </Show>
                    <Show when=move || settings.get()>
                        <span class="text-headline text-label dark:text-label">"Settings"</span>
                    </Show>
                    <div class="h-[1px] bg-separator dark:bg-separator"></div>
                    <span class="text-callout text-label-secondary dark:text-label-secondary">"Search"</span>
                    <span class="text-body text-label dark:text-label">{query}</span>
                    <span class="text-callout text-label-secondary dark:text-label-secondary">"Account"</span>
                    <span class="text-body text-label dark:text-label">{choice}</span>
                    <span class="text-callout text-label-secondary dark:text-label-secondary">"Folder"</span>
                    <span class="text-body text-label dark:text-label">{draft}</span>
                </div>
            </div>
            <Portal>
                <Show when=move || sheet.get()>
                    <div class="absolute inset-0">
                        <button class="absolute inset-0 z-40 border-0 bg-[#1c191780] hover:bg-[#1c191780] text-white" on:click=move |_| { set_sheet.set(false); }>" "</button>
                        <div class="absolute left-[510px] top-[160px] z-50 w-[420px] p-5 rounded-sheet bg-system-background shadow-window dark:bg-system-background">
                            <span class="text-title3 text-label dark:text-label">"New Note"</span>
                            <span class="text-callout text-label-secondary dark:text-label-secondary">"Saved on this Mac. You can find it in the library."</span>
                            <input
                                class="h-control-large rounded-control bg-control-background text-body text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator"
                                placeholder="Title"
                            />
                            <div class="flex flex-row gap-2">
                                <button class="px-3 h-control-large rounded-control text-body bg-system-blue text-white dark:bg-system-blue" on:click=move |_| { set_sheet.set(false); set_toast.set(true); }>"Save"</button>
                                <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_sheet.set(false); }>"Cancel"</button>
                            </div>
                        </div>
                    </div>
                </Show>
                <Show when=move || alert.get()>
                    <div class="absolute inset-0">
                        <div class="absolute inset-0 z-40 bg-[#1c191780]"></div>
                        <div class="absolute left-[520px] top-[240px] z-50 w-[400px] p-5 rounded-sheet bg-system-background shadow-window dark:bg-system-background">
                            <span class="text-title3 text-label dark:text-label">"Delete this note?"</span>
                            <span class="text-body text-label-secondary dark:text-label-secondary">"The note moves to Recently Deleted. You can recover it for 30 days."</span>
                            <div class="flex flex-row gap-2">
                                <button class="px-3 h-control-large rounded-control text-body bg-system-red text-white dark:bg-system-red" on:click=move |_| { set_alert.set(false); set_toast.set(true); }>"Delete"</button>
                                <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_alert.set(false); }>"Cancel"</button>
                            </div>
                        </div>
                    </div>
                </Show>
            </Portal>
        </div>
    }
}
