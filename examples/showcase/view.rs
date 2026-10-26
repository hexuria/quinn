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
        <div class="flex flex-col odd:bg-system-fill">
            <div class="flex flex-row items-center h-[32px] px-3 gap-3">
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
    let (step, set_step) = signal(2i64);
    let (level, set_level) = signal(50i64);
    let (day, set_day) = signal(true);
    let (week, set_week) = signal(false);
    let (month, set_month) = signal(false);
    let (dark_on, set_dark_on) = signal(false);
    let (light_on, set_light_on) = signal(true);
    let (sel_kitchen, set_sel_kitchen) = signal(true);
    let (sel_other, set_sel_other) = signal(false);
    let (chart_on, set_chart_on) = signal(true);
    let (note_title, set_note_title) = signal("Kitchen".to_string());
    let (note_when, set_note_when) = signal("Edited just now".to_string());
    let (note_body, set_note_body) = signal(
        "Pick up citrus, olive oil, and the blue linen. The cabinet paint is still the sample from last week.".to_string(),
    );
    let (note_more, set_note_more) = signal(
        "Call the joiner before Thursday. Leave the island where it is.".to_string(),
    );
    let (rows, set_rows) = signal(vec![
        Item { id: 1, label: "Grocery list".into() },
        Item { id: 2, label: "Kitchen remodel".into() },
        Item { id: 3, label: "Saturday market".into() },
        Item { id: 4, label: "Paint colors".into() },
    ]);

    view! {
        <div class="flex flex-col h-full bg-window-background dark:bg-window-background">
            <div class="flex flex-row items-center h-[52px] px-4 gap-3">
                <span class="text-headline text-label dark:text-label">"Notes"</span>
                <div class="flex-1"></div>
                <div class="w-[240px] h-[28px]">
                    <div class="absolute left-[8px] top-[7px] w-[10px] h-[10px] rounded-full border border-label-tertiary bg-transparent dark:border-label-tertiary"></div>
                    <div class="absolute left-[16px] top-[15px] w-[2px] h-[2px] bg-label-tertiary dark:bg-label-tertiary"></div>
                    <div class="absolute left-[18px] top-[17px] w-[2px] h-[2px] bg-label-tertiary dark:bg-label-tertiary"></div>
                    <div class="absolute left-[20px] top-[19px] w-[2px] h-[2px] bg-label-tertiary dark:bg-label-tertiary"></div>
                    <input
                        class="w-full h-[28px] pl-[28px] rounded-control bg-system-fill text-body text-label border-0 dark:bg-system-fill dark:text-label"
                        bind:value=query
                        placeholder="Search"
                    />
                </div>
                <div class="flex-1"></div>
                <div class="w-[28px] h-[28px]">
                    <Show when=move || light_on.get()>
                        <div class="w-[28px] h-[28px]">
                            <div class="absolute left-[9px] top-[9px] w-[10px] h-[10px] rounded-full bg-label dark:bg-label"></div>
                            <div class="absolute left-[12px] top-[1px] w-[3px] h-[6px] rounded-full bg-label dark:bg-label"></div>
                            <div class="absolute left-[12px] top-[21px] w-[3px] h-[6px] rounded-full bg-label dark:bg-label"></div>
                            <div class="absolute left-[1px] top-[12px] w-[6px] h-[3px] rounded-full bg-label dark:bg-label"></div>
                            <div class="absolute left-[21px] top-[12px] w-[6px] h-[3px] rounded-full bg-label dark:bg-label"></div>
                        </div>
                    </Show>
                    <Show when=move || dark_on.get()>
                        <div class="w-[28px] h-[28px]">
                            <div class="absolute left-[7px] top-[6px] w-[14px] h-[14px] rounded-full bg-label dark:bg-label"></div>
                            <div class="absolute left-[12px] top-[4px] w-[12px] h-[12px] rounded-full bg-window-background dark:bg-window-background"></div>
                        </div>
                    </Show>
                    <button
                        class="absolute inset-0 border-0 bg-transparent"
                        on:click=move |_| {
                            toggle_theme();
                            set_dark_on.update(|value| *value = !*value);
                            set_light_on.update(|value| *value = !*value);
                        }
                    >" "</button>
                </div>
                <button
                    class="px-3 h-[28px] rounded-control text-body bg-system-blue text-white dark:bg-system-blue"
                    on:click=move |_| { set_sheet.set(true); }
                >"+ New Note"</button>
            </div>
            <div class="h-[1px] bg-separator dark:bg-separator"></div>
            <div class="flex flex-col md:flex-row flex-1 items-stretch">
                <div class="flex flex-col w-full md:w-[220px] md:h-full vibrancy px-[10px] pt-3 gap-[2px]">
                    <span class="text-subheadline text-label-secondary dark:text-label-secondary px-2 h-[22px]">"Library"</span>
                    <div class="flex flex-row items-center h-[28px] w-full rounded-control px-[6px] gap-[6px]">
                        <Show when=move || notes.get()>
                            <div class="absolute inset-0 z-[-1] rounded-control bg-system-blue dark:bg-system-blue"></div>
                        </Show>
                        <div class="w-[16px] h-[16px]">
                            <div class="absolute left-[3px] top-[1px] w-[10px] h-[14px] rounded-[2px] bg-label dark:bg-label" class:bg-white=move || notes.get()></div>
                            <div class="absolute left-[8px] top-[1px] w-[5px] h-[4px] rounded-[1px] bg-sidebar dark:bg-sidebar" class:bg-system-blue=move || notes.get()></div>
                        </div>
                        <span class="text-body text-label dark:text-label" class:text-white=move || notes.get()>"Notes"</span>
                        <button
                            class="absolute inset-0 border-0 bg-transparent"
                            on:click=move |_| {
                                set_notes.set(true);
                                set_table.set(false);
                                set_settings.set(false);
                            }
                        >" "</button>
                    </div>
                    <div class="flex flex-row items-center h-[28px] w-full rounded-control px-[6px] gap-[6px]">
                        <Show when=move || table.get()>
                            <div class="absolute inset-0 z-[-1] rounded-control bg-system-blue dark:bg-system-blue"></div>
                        </Show>
                        <div class="w-[16px] h-[16px]">
                            <div class="absolute left-[1px] top-[1px] w-[6px] h-[6px] rounded-[1px] bg-label dark:bg-label" class:bg-white=move || table.get()></div>
                            <div class="absolute left-[9px] top-[1px] w-[6px] h-[6px] rounded-[1px] bg-label dark:bg-label" class:bg-white=move || table.get()></div>
                            <div class="absolute left-[1px] top-[9px] w-[6px] h-[6px] rounded-[1px] bg-label dark:bg-label" class:bg-white=move || table.get()></div>
                            <div class="absolute left-[9px] top-[9px] w-[6px] h-[6px] rounded-[1px] bg-label dark:bg-label" class:bg-white=move || table.get()></div>
                        </div>
                        <span class="text-body text-label dark:text-label" class:text-white=move || table.get()>"Table"</span>
                        <button
                            class="absolute inset-0 border-0 bg-transparent"
                            on:click=move |_| {
                                set_notes.set(false);
                                set_table.set(true);
                                set_settings.set(false);
                            }
                        >" "</button>
                    </div>
                    <div class="flex flex-row items-center h-[28px] w-full rounded-control px-[6px] gap-[6px]">
                        <Show when=move || settings.get()>
                            <div class="absolute inset-0 z-[-1] rounded-control bg-system-blue dark:bg-system-blue"></div>
                        </Show>
                        <div class="w-[16px] h-[16px]">
                            <div class="absolute left-[3px] top-[3px] w-[10px] h-[10px] rounded-full bg-label dark:bg-label" class:bg-white=move || settings.get()></div>
                            <div class="absolute left-[6px] top-[6px] w-[4px] h-[4px] rounded-full bg-sidebar dark:bg-sidebar" class:bg-system-blue=move || settings.get()></div>
                            <div class="absolute left-[7px] top-0 w-[2px] h-[3px] bg-label dark:bg-label" class:bg-white=move || settings.get()></div>
                            <div class="absolute left-[7px] top-[13px] w-[2px] h-[3px] bg-label dark:bg-label" class:bg-white=move || settings.get()></div>
                            <div class="absolute left-0 top-[7px] w-[3px] h-[2px] bg-label dark:bg-label" class:bg-white=move || settings.get()></div>
                            <div class="absolute left-[13px] top-[7px] w-[3px] h-[2px] bg-label dark:bg-label" class:bg-white=move || settings.get()></div>
                        </div>
                        <span class="text-body text-label dark:text-label" class:text-white=move || settings.get()>"Settings"</span>
                        <button
                            class="absolute inset-0 border-0 bg-transparent"
                            on:click=move |_| {
                                set_notes.set(false);
                                set_table.set(false);
                                set_settings.set(true);
                            }
                        >" "</button>
                    </div>
                </div>
                <div class="w-full md:w-[1px] h-[1px] md:h-full bg-separator dark:bg-separator"></div>
                <Show when=move || notes.get()>
                    <div class="flex flex-col w-full md:w-[280px] md:h-full bg-system-background-secondary dark:bg-system-background-secondary">
                        <div class="flex flex-row items-center h-[36px] px-4">
                            <span class="text-headline text-label dark:text-label">"All Notes"</span>
                        </div>
                        <div class="h-[1px] bg-separator dark:bg-separator"></div>
                        <div class="px-2 py-1">
                            <div class="flex flex-col h-[72px] rounded-control px-3 py-[6px] gap-[2px]">
                                <Show when=move || sel_kitchen.get()>
                                    <div class="absolute inset-0 z-[-1] rounded-control bg-system-blue dark:bg-system-blue"></div>
                                </Show>
                                <div class="flex flex-row items-center w-full">
                                    <span class="text-headline text-label dark:text-label" class:text-white=move || sel_kitchen.get()>"Kitchen"</span>
                                    <div class="flex-1"></div>
                                    <span class="text-caption text-label-secondary dark:text-label-secondary" class:text-white=move || sel_kitchen.get()>"10:24 AM"</span>
                                </div>
                                <span class="text-callout text-label-secondary dark:text-label-secondary w-full" class:text-white=move || sel_kitchen.get()>"Citrus, olive oil, blue linen."</span>
                                <button
                                    class="absolute inset-0 border-0 bg-transparent"
                                    on:click=move |_| {
                                        set_sel_kitchen.set(true);
                                        set_sel_other.set(false);
                                        set_chart_on.set(true);
                                        set_note_title.set("Kitchen".to_string());
                                        set_note_when.set("Edited just now".to_string());
                                        set_note_body.set("Pick up citrus, olive oil, and the blue linen. The cabinet paint is still the sample from last week.".to_string());
                                        set_note_more.set("Call the joiner before Thursday. Leave the island where it is.".to_string());
                                    }
                                >" "</button>
                            </div>
                            <div class="flex flex-col h-[72px] rounded-control px-3 py-[6px] gap-[2px]">
                                <Show when=move || sel_other.get()>
                                    <div class="absolute inset-0 z-[-1] rounded-control bg-system-blue dark:bg-system-blue"></div>
                                </Show>
                                <div class="flex flex-row items-center w-full">
                                    <span class="text-headline text-label dark:text-label" class:text-white=move || sel_other.get()>"Grocery list"</span>
                                    <div class="flex-1"></div>
                                    <span class="text-caption text-label-secondary dark:text-label-secondary" class:text-white=move || sel_other.get()>"Yesterday"</span>
                                </div>
                                <span class="text-callout text-label-secondary dark:text-label-secondary w-full" class:text-white=move || sel_other.get()>"Lemons, oil, linen, tape."</span>
                                <button
                                    class="absolute inset-0 border-0 bg-transparent"
                                    on:click=move |_| {
                                        set_sel_kitchen.set(false);
                                        set_sel_other.set(true);
                                        set_chart_on.set(false);
                                        set_note_title.set("Grocery list".to_string());
                                        set_note_when.set("Edited yesterday".to_string());
                                        set_note_body.set("Lemons, olive oil, blue linen, and a roll of painter's tape.".to_string());
                                        set_note_more.set("The market closes at four. Park on the east side.".to_string());
                                    }
                                >" "</button>
                            </div>
                        </div>
                    </div>
                </Show>
                <Show when=move || notes.get()>
                    <div class="w-full md:w-[1px] h-[1px] md:h-full bg-separator dark:bg-separator"></div>
                </Show>
                <Show when=move || notes.get()>
                    <div class="flex flex-col flex-1 md:h-full px-10 py-6 bg-system-background dark:bg-system-background">
                        <div class="flex flex-col w-full max-w-[640px] gap-4">
                            <span class="text-large-title text-label dark:text-label">{note_title}</span>
                            <span class="text-footnote text-label-secondary dark:text-label-secondary">{note_when}</span>
                            <span class="text-body text-label dark:text-label">{note_body}</span>
                            <span class="text-body text-label-secondary dark:text-label-secondary">{note_more}</span>
                            <Show when=move || chart_on.get()>
                                <div class="flex flex-col gap-2">
                                    <span class="text-subheadline text-label-secondary dark:text-label-secondary">"This week"</span>
                                    <div class="flex flex-row items-end gap-3 h-[108px]">
                                        <div class="flex flex-col items-center justify-end h-[100px] gap-1">
                                            <div class="w-[14px] h-[28px] rounded-sm bg-system-blue"></div>
                                            <span class="text-caption text-label-secondary dark:text-label-secondary">"S"</span>
                                        </div>
                                        <div class="flex flex-col items-center justify-end h-[100px] gap-1">
                                            <div class="w-[14px] h-[44px] rounded-sm bg-system-blue"></div>
                                            <span class="text-caption text-label-secondary dark:text-label-secondary">"M"</span>
                                        </div>
                                        <div class="flex flex-col items-center justify-end h-[100px] gap-1">
                                            <div class="w-[14px] h-[36px] rounded-sm bg-system-blue"></div>
                                            <span class="text-caption text-label-secondary dark:text-label-secondary">"T"</span>
                                        </div>
                                        <div class="flex flex-col items-center justify-end h-[100px] gap-1">
                                            <div class="w-[14px] h-[64px] rounded-sm bg-system-blue"></div>
                                            <span class="text-caption text-label-secondary dark:text-label-secondary">"W"</span>
                                        </div>
                                        <div class="flex flex-col items-center justify-end h-[100px] gap-1">
                                            <div class="w-[14px] h-[52px] rounded-sm bg-system-blue"></div>
                                            <span class="text-caption text-label-secondary dark:text-label-secondary">"T"</span>
                                        </div>
                                        <div class="flex flex-col items-center justify-end h-[100px] gap-1">
                                            <div class="w-[14px] h-[40px] rounded-sm bg-system-blue"></div>
                                            <span class="text-caption text-label-secondary dark:text-label-secondary">"F"</span>
                                        </div>
                                        <div class="flex flex-col items-center justify-end h-[100px] gap-1">
                                            <div class="w-[14px] h-[24px] rounded-sm bg-system-blue"></div>
                                            <span class="text-caption text-label-secondary dark:text-label-secondary">"S"</span>
                                        </div>
                                    </div>
                                </div>
                            </Show>
                        </div>
                    </div>
                </Show>
                <Show when=move || table.get()>
                    <div class="flex flex-col flex-1 md:h-full p-6 gap-3 bg-system-background dark:bg-system-background">
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
                            <For each=move || rows.get() key=|item| item.id let:item let:index>
                                <NoteRow item=item />
                            </For>
                        </div>
                    </div>
                </Show>
                <Show when=move || settings.get()>
                    <div class="flex flex-col flex-1 md:h-full p-6 gap-4 bg-system-background dark:bg-system-background">
                        <span class="text-title2 text-label dark:text-label">"Settings"</span>
                        <div class="flex flex-col md:flex-row gap-8">
                            <div class="flex flex-col gap-4">
                                <span class="text-subheadline text-label-secondary dark:text-label-secondary">"Segmented control"</span>
                                <div class="flex flex-row items-center p-[2px] rounded-control bg-system-fill dark:bg-system-fill w-fit">
                                    <div class="w-[72px] h-[22px]">
                                        <Show when=move || day.get()>
                                            <div class="w-[72px] h-[22px] rounded-[5px] bg-system-background border border-separator dark:bg-system-background dark:border-separator"></div>
                                        </Show>
                                        <button class="absolute inset-0 px-[0px] py-[0px] border-0 bg-transparent text-body text-label dark:text-label" on:click=move |_| { set_day.set(true); set_week.set(false); set_month.set(false); }>"Day"</button>
                                    </div>
                                    <div class="w-[72px] h-[22px]">
                                        <Show when=move || week.get()>
                                            <div class="w-[72px] h-[22px] rounded-[5px] bg-system-background border border-separator dark:bg-system-background dark:border-separator"></div>
                                        </Show>
                                        <button class="absolute inset-0 px-[0px] py-[0px] border-0 bg-transparent text-body text-label dark:text-label" on:click=move |_| { set_day.set(false); set_week.set(true); set_month.set(false); }>"Week"</button>
                                    </div>
                                    <div class="w-[72px] h-[22px]">
                                        <Show when=move || month.get()>
                                            <div class="w-[72px] h-[22px] rounded-[5px] bg-system-background border border-separator dark:bg-system-background dark:border-separator"></div>
                                        </Show>
                                        <button class="absolute inset-0 px-[0px] py-[0px] border-0 bg-transparent text-body text-label dark:text-label" on:click=move |_| { set_day.set(false); set_week.set(false); set_month.set(true); }>"Month"</button>
                                    </div>
                                </div>
                                <span class="text-subheadline text-label-secondary dark:text-label-secondary">"Switch"</span>
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
                                <span class="text-subheadline text-label-secondary dark:text-label-secondary">"Checkbox"</span>
                                <div class="flex flex-row items-center gap-3">
                                    <div class="w-[14px] h-[14px]">
                                        <Show when=move || check_off.get()>
                                            <div class="w-[14px] h-[14px] rounded-[2px] bg-control-background border border-separator dark:bg-control-background dark:border-separator"></div>
                                        </Show>
                                        <Show when=move || checked.get()>
                                            <div class="w-[14px] h-[14px] rounded-[2px] bg-system-blue dark:bg-system-blue">
                                                <div class="absolute left-[3px] top-[7px] w-[2px] h-[2px] bg-white"></div>
                                                <div class="absolute left-[4px] top-[8px] w-[2px] h-[2px] bg-white"></div>
                                                <div class="absolute left-[5px] top-[7px] w-[2px] h-[3px] bg-white"></div>
                                                <div class="absolute left-[6px] top-[6px] w-[2px] h-[3px] bg-white"></div>
                                                <div class="absolute left-[7px] top-[5px] w-[2px] h-[3px] bg-white"></div>
                                                <div class="absolute left-[8px] top-[4px] w-[2px] h-[3px] bg-white"></div>
                                                <div class="absolute left-[9px] top-[3px] w-[2px] h-[3px] bg-white"></div>
                                                <div class="absolute left-[10px] top-[2px] w-[2px] h-[2px] bg-white"></div>
                                            </div>
                                        </Show>
                                        <button class="absolute inset-0 border-0 bg-transparent" on:click=move |_| {
                                            if checked.get() {
                                                set_checked.set(false);
                                                set_check_off.set(true);
                                            } else {
                                                set_checked.set(true);
                                                set_check_off.set(false);
                                            }
                                        }>" "</button>
                                    </div>
                                    <span class="text-body text-label dark:text-label">"Show in menu bar"</span>
                                </div>
                                <span class="text-subheadline text-label-secondary dark:text-label-secondary">"Pop-up"</span>
                                <button class="px-3 h-[28px] rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator w-fit" on:click=move |_| { set_menu.update(|value| *value = !*value); }>
                                    {choice}
                                </button>
                                <Show when=move || menu.get()>
                                    <div class="flex flex-col w-[220px] p-1 rounded-control bg-system-background shadow-window dark:bg-system-background">
                                        <button class="px-3 h-[28px] rounded-sm text-body text-label border-0 bg-transparent dark:text-label" on:click=move |_| { set_choice.set("On My Mac".to_string()); set_menu.set(false); }>"On My Mac"</button>
                                        <button class="px-3 h-[28px] rounded-sm text-body text-label border-0 bg-transparent dark:text-label" on:click=move |_| { set_choice.set("iCloud".to_string()); set_menu.set(false); }>"iCloud"</button>
                                        <button class="px-3 h-[28px] rounded-sm text-body text-label border-0 bg-transparent dark:text-label" on:click=move |_| { set_choice.set("Recently Deleted".to_string()); set_menu.set(false); }>"Recently Deleted"</button>
                                    </div>
                                </Show>
                                <span class="text-subheadline text-label-secondary dark:text-label-secondary">"Text field"</span>
                                <div class="w-[240px] h-[28px]">
                                    <div class="absolute left-[8px] top-[11px] w-[14px] h-[8px] rounded-[2px] bg-label-tertiary dark:bg-label-tertiary"></div>
                                    <div class="absolute left-[8px] top-[8px] w-[7px] h-[4px] rounded-[2px] bg-label-tertiary dark:bg-label-tertiary"></div>
                                    <input
                                        class="w-full h-[28px] pl-[26px] rounded-control bg-control-background text-body text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator"
                                        bind:value=draft
                                        placeholder="Folder name"
                                    />
                                </div>
                            </div>
                            <div class="flex flex-col gap-4">
                                <span class="text-subheadline text-label-secondary dark:text-label-secondary">"Push buttons"</span>
                                <div class="flex flex-row items-center gap-2">
                                    <button class="px-3 h-[28px] rounded-control text-body text-system-red border-0 bg-transparent dark:text-system-red" on:click=move |_| { set_alert.set(true); }>"Delete Note"</button>
                                    <button class="px-3 h-[28px] rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_toast.set(true); }>"Bordered"</button>
                                    <button class="px-3 h-[28px] rounded-control text-body text-system-blue border-0 bg-transparent dark:text-system-blue">"Borderless"</button>
                                </div>
                                <button class="px-3 h-[28px] rounded-control text-body bg-control-background text-label-secondary border border-separator disabled dark:bg-control-background dark:text-label-secondary dark:border-separator w-fit">"Disabled"</button>
                                <span class="text-subheadline text-label-secondary dark:text-label-secondary">"Stepper"</span>
                                <div class="flex flex-row items-center gap-3">
                                    <div class="flex flex-col w-[28px] rounded-[5px] border border-separator overflow-hidden bg-control-background dark:bg-control-background dark:border-separator">
                                        <button class="w-[28px] h-[18px] px-[0px] py-[0px] border-0 bg-transparent text-body text-label dark:text-label" on:click=move |_| { set_step.set(clamp(step.get() + 1, 1, 8)); }>"+"</button>
                                        <div class="h-[1px] w-full bg-separator dark:bg-separator"></div>
                                        <button class="w-[28px] h-[18px] px-[0px] py-[0px] border-0 bg-transparent text-body text-label dark:text-label" on:click=move |_| { set_step.set(clamp(step.get() - 1, 1, 8)); }>"−"</button>
                                    </div>
                                    <span class="text-body text-label dark:text-label">{step}</span>
                                </div>
                                <span class="text-subheadline text-label-secondary dark:text-label-secondary">"Slider"</span>
                                <div class="w-[200px] h-[20px]">
                                    <div class="absolute left-0 top-[8px] z-[-1] w-full h-[4px] rounded-full bg-system-fill dark:bg-system-fill"></div>
                                    <div class="absolute left-0 top-[8px] z-[-1] h-[4px] rounded-full bg-system-blue" bind:width="level*2"></div>
                                    <div class="flex flex-row items-center h-[20px]">
                                        <div class="h-[18px]" bind:width="level*2-9"></div>
                                        <div class="w-[18px] h-[18px] rounded-full bg-zinc-300 dark:bg-zinc-500">
                                            <div class="absolute left-[2px] top-[2px] w-[14px] h-[14px] rounded-full bg-white border border-zinc-400 dark:bg-white dark:border-zinc-400"></div>
                                        </div>
                                    </div>
                                    <div class="absolute inset-0 flex flex-row">
                                        <button class="w-[25px] h-[20px] border-0 bg-transparent" on:click=move |_| { set_level.set(0); }>" "</button>
                                        <button class="w-[25px] h-[20px] border-0 bg-transparent" on:click=move |_| { set_level.set(14); }>" "</button>
                                        <button class="w-[25px] h-[20px] border-0 bg-transparent" on:click=move |_| { set_level.set(28); }>" "</button>
                                        <button class="w-[25px] h-[20px] border-0 bg-transparent" on:click=move |_| { set_level.set(43); }>" "</button>
                                        <button class="w-[25px] h-[20px] border-0 bg-transparent" on:click=move |_| { set_level.set(57); }>" "</button>
                                        <button class="w-[25px] h-[20px] border-0 bg-transparent" on:click=move |_| { set_level.set(71); }>" "</button>
                                        <button class="w-[25px] h-[20px] border-0 bg-transparent" on:click=move |_| { set_level.set(86); }>" "</button>
                                        <button class="w-[25px] h-[20px] border-0 bg-transparent" on:click=move |_| { set_level.set(100); }>" "</button>
                                    </div>
                                </div>
                                <span class="text-callout text-label-secondary dark:text-label-secondary">{level}</span>
                                <span class="text-subheadline text-label-secondary dark:text-label-secondary">"Progress"</span>
                                <div class="w-[200px] h-[4px] rounded-full bg-system-fill dark:bg-system-fill">
                                    <div class="h-[4px] rounded-full bg-system-blue" bind:width="level*2"></div>
                                </div>
                            </div>
                        </div>
                    </div>
                </Show>
                <Show when=move || table.get()>
                    <div class="w-full md:w-[1px] h-[1px] md:h-full bg-separator dark:bg-separator"></div>
                </Show>
                <Show when=move || settings.get()>
                    <div class="w-full md:w-[1px] h-[1px] md:h-full bg-separator dark:bg-separator"></div>
                </Show>
                <Show when=move || table.get()>
                    <div class="flex flex-col w-full md:w-[260px] md:h-full p-4 gap-3 bg-system-background-secondary dark:bg-system-background-secondary">
                        <span class="text-subheadline text-label-secondary dark:text-label-secondary">"Inspector"</span>
                        <span class="text-headline text-label dark:text-label">"All Notes"</span>
                        <div class="h-[1px] bg-separator dark:bg-separator"></div>
                        <span class="text-callout text-label-secondary dark:text-label-secondary">"Search"</span>
                        <span class="text-body text-label dark:text-label">{query}</span>
                        <span class="text-callout text-label-secondary dark:text-label-secondary">"Account"</span>
                        <span class="text-body text-label dark:text-label">{choice}</span>
                    </div>
                </Show>
                <Show when=move || settings.get()>
                    <div class="flex flex-col w-full md:w-[260px] md:h-full p-4 gap-3 bg-system-background-secondary dark:bg-system-background-secondary">
                        <span class="text-subheadline text-label-secondary dark:text-label-secondary">"Inspector"</span>
                        <span class="text-headline text-label dark:text-label">"Settings"</span>
                        <div class="h-[1px] bg-separator dark:bg-separator"></div>
                        <span class="text-callout text-label-secondary dark:text-label-secondary">"Search"</span>
                        <span class="text-body text-label dark:text-label">{query}</span>
                        <span class="text-callout text-label-secondary dark:text-label-secondary">"Account"</span>
                        <span class="text-body text-label dark:text-label">{choice}</span>
                        <span class="text-callout text-label-secondary dark:text-label-secondary">"Folder"</span>
                        <span class="text-body text-label dark:text-label">{draft}</span>
                    </div>
                </Show>
            </div>
            <Show when=move || toast.get()>
                <div class="absolute right-[16px] top-[64px] z-50 flex flex-row items-center gap-3 px-3 h-[36px] rounded-control bg-system-background shadow-lg dark:bg-system-background">
                    <span class="text-body text-label dark:text-label">"Moved to Recently Deleted"</span>
                    <button class="px-2 h-control rounded-control text-body text-system-blue border-0 bg-transparent dark:text-system-blue" on:click=move |_| { set_toast.set(false); }>"Dismiss"</button>
                </div>
            </Show>
            <Portal>
                <Show when=move || sheet.get()>
                    <div class="absolute inset-0">
                        <button class="absolute inset-0 z-40 border-0 bg-[#1c191780] hover:bg-[#1c191780] text-white" on:click=move |_| { set_sheet.set(false); }>" "</button>
                        <div class="absolute left-[510px] top-[160px] z-50 w-[420px] p-5 rounded-sheet bg-system-background shadow-window dark:bg-system-background">
                            <span class="text-title3 text-label dark:text-label">"New Note"</span>
                            <span class="text-callout text-label-secondary dark:text-label-secondary">"Saved on this Mac. You can find it in the library."</span>
                            <input
                                class="h-[28px] rounded-control bg-control-background text-body text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator"
                                placeholder="Title"
                            />
                            <div class="flex flex-row gap-2">
                                <button class="px-3 h-[28px] rounded-control text-body bg-system-blue text-white dark:bg-system-blue" on:click=move |_| { set_sheet.set(false); set_toast.set(true); }>"Save"</button>
                                <button class="px-3 h-[28px] rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_sheet.set(false); }>"Cancel"</button>
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
                                <button class="px-3 h-[28px] rounded-control text-body bg-system-red text-white dark:bg-system-red" on:click=move |_| { set_alert.set(false); set_toast.set(true); }>"Delete"</button>
                                <button class="px-3 h-[28px] rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_alert.set(false); }>"Cancel"</button>
                            </div>
                        </div>
                    </div>
                </Show>
            </Portal>
        </div>
    }
}
