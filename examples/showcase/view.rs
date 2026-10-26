use quinn::prelude::*;
use quinn_ui::components::*;

#[derive(Clone)]
struct Item {
    id: u64,
    label: String,
}

#[component]
fn NoteRow(item: Item) -> impl IntoView {
    let label = item.label;
    view! {
        <div class="flex flex-col odd:bg-subtle">
            <div class="flex flex-row items-center h-[40px] px-3 gap-3">
                <span class="text-body text-primary dark:text-primary">{label}</span>
                <div class="flex-1"></div>
                <span class="text-caption text-secondary dark:text-secondary">"Note"</span>
            </div>
            <div class="h-[1px] bg-divider dark:bg-divider"></div>
        </div>
    }
}

#[component]
fn Showcase() -> impl IntoView {
    let (notes, set_notes) = signal(true);
    let (table, set_table) = signal(false);
    let (settings, set_settings) = signal(false);
    let (gallery, set_gallery) = signal(false);
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
    let (nav_open, set_nav_open) = signal(false);
    let (list_open, set_list_open) = signal(false);
    let (info_open, set_info_open) = signal(false);
    let (density_std, set_density_std) = signal(true);
    let (density_touch, set_density_touch) = signal(false);
    let (rows, set_rows) = signal(vec![
        Item { id: 1, label: "Grocery list".into() },
        Item { id: 2, label: "Kitchen remodel".into() },
        Item { id: 3, label: "Saturday market".into() },
        Item { id: 4, label: "Paint colors".into() },
        Item { id: 5, label: "Joiner Thursday".into() },
        Item { id: 6, label: "Island plan".into() },
        Item { id: 7, label: "Linen order".into() },
        Item { id: 8, label: "Cabinet sample".into() },
    ]);

    view! {
        <div class="flex flex-col h-full bg-canvas dark:bg-canvas">
            <div class="flex flex-row items-center h-toolbar px-4 gap-2 bg-surface dark:bg-surface">
                <Button variant="ghost" on:click=move |_| { set_nav_open.update(|value| *value = !*value); }>"Library"</Button>
                <span class="text-section text-primary dark:text-primary">"Notes"</span>
                <div class="flex-1"></div>
                <Search placeholder="Search" bind:value=query />
                <div class="flex-1"></div>
                <Button variant="secondary" class:bg-selection=move || density_std.get() on:click=move |_| { set_density(1); set_density_std.set(true); set_density_touch.set(false); }>"Standard"</Button>
                <Button variant="secondary" class:bg-selection=move || density_touch.get() on:click=move |_| { set_density(2); set_density_std.set(false); set_density_touch.set(true); }>"Touch"</Button>
                <Button variant="ghost" on:click=move |_| { toggle_theme(); set_dark_on.update(|value| *value = !*value); set_light_on.update(|value| *value = !*value); }>"Theme"</Button>
                <Button on:click=move |_| { set_sheet.set(true); }>"New note"</Button>
            </div>
            <div class="h-[1px] bg-divider dark:bg-divider"></div>
            <div class="flex flex-row flex-1 items-stretch">
                <div class="flex flex-col h-full bg-subtle overflow-hidden w-[0px] bp800:w-[64px] bp1120:w-[224px] bp1120:min-w-[200px] bp1120:max-w-[280px] dark:bg-subtle">
                    <div class="flex flex-col gap-1 px-2 py-3">
                        <span class="text-caption text-secondary px-3 w-[0px] bp1120:w-[160px] dark:text-secondary">"Library"</span>
                        <NavRow class:bg-selection=move || notes.get() on:click=move |_| { set_notes.set(true); set_table.set(false); set_settings.set(false); set_gallery.set(false); set_nav_open.set(false); }>
                            <div class="w-[16px] h-[16px] rounded-[4px] bg-accent"></div>
                            <span class="text-body text-primary w-[0px] bp1120:w-[148px] dark:text-primary" class:text-on-selection=move || notes.get()>"Notes"</span>
                        </NavRow>
                        <NavRow class:bg-selection=move || table.get() on:click=move |_| { set_notes.set(false); set_table.set(true); set_settings.set(false); set_gallery.set(false); set_nav_open.set(false); }>
                            <div class="w-[16px] h-[16px] rounded-[4px] bg-muted"></div>
                            <span class="text-body text-primary w-[0px] bp1120:w-[148px] dark:text-primary" class:text-on-selection=move || table.get()>"Table"</span>
                        </NavRow>
                        <NavRow class:bg-selection=move || settings.get() on:click=move |_| { set_notes.set(false); set_table.set(false); set_settings.set(true); set_gallery.set(false); set_nav_open.set(false); }>
                            <div class="w-[16px] h-[16px] rounded-[4px] bg-muted"></div>
                            <span class="text-body text-primary w-[0px] bp1120:w-[148px] dark:text-primary" class:text-on-selection=move || settings.get()>"Settings"</span>
                        </NavRow>
                        <NavRow class:bg-selection=move || gallery.get() on:click=move |_| { set_notes.set(false); set_table.set(false); set_settings.set(false); set_gallery.set(true); set_nav_open.set(false); }>
                            <div class="w-[16px] h-[16px] rounded-[4px] bg-muted"></div>
                            <span class="text-body text-primary w-[0px] bp1120:w-[148px] dark:text-primary" class:text-on-selection=move || gallery.get()>"Gallery"</span>
                        </NavRow>
                    </div>
                </div>
                <Show when=move || notes.get()>
                    <div class="flex flex-col h-full bg-surface overflow-y-auto w-[0px] bp1120:w-[280px] bp1120:min-w-[240px] bp1120:max-w-[360px] dark:bg-surface">
                        <div class="flex flex-col gap-2 p-4">
                            <div class="flex flex-row items-center">
                                <span class="text-control text-primary dark:text-primary">"All notes"</span>
                                <div class="flex-1"></div>
                                <Button variant="ghost" on:click=move |_| { set_list_open.update(|value| *value = !*value); }>"List"</Button>
                            </div>
                            <div class="flex flex-col gap-1 px-3 py-2 rounded-control" class:bg-selection=move || sel_kitchen.get()>
                                <span class="text-control text-primary w-full dark:text-primary" class:text-on-selection=move || sel_kitchen.get()>"Kitchen"</span>
                                <span class="text-caption text-secondary w-full dark:text-secondary" class:text-on-selection=move || sel_kitchen.get()>"Citrus, olive oil, blue linen."</span>
                                <button class="absolute inset-0 border-0 bg-transparent" on:click=move |_| {
                                    set_sel_kitchen.set(true);
                                    set_sel_other.set(false);
                                    set_chart_on.set(true);
                                    set_note_title.set("Kitchen".to_string());
                                    set_note_when.set("Edited just now".to_string());
                                    set_note_body.set("Pick up citrus, olive oil, and the blue linen. The cabinet paint is still the sample from last week.".to_string());
                                    set_note_more.set("Call the joiner before Thursday. Leave the island where it is.".to_string());
                                }>" "</button>
                            </div>
                            <div class="flex flex-col gap-1 px-3 py-2 rounded-control" class:bg-selection=move || sel_other.get()>
                                <span class="text-control text-primary w-full dark:text-primary" class:text-on-selection=move || sel_other.get()>"Grocery list"</span>
                                <span class="text-caption text-secondary w-full dark:text-secondary" class:text-on-selection=move || sel_other.get()>"Lemons, oil, linen, tape."</span>
                                <button class="absolute inset-0 border-0 bg-transparent" on:click=move |_| {
                                    set_sel_kitchen.set(false);
                                    set_sel_other.set(true);
                                    set_chart_on.set(false);
                                    set_note_title.set("Grocery list".to_string());
                                    set_note_when.set("Edited yesterday".to_string());
                                    set_note_body.set("Lemons, olive oil, blue linen, and a roll of painter's tape.".to_string());
                                    set_note_more.set("The market closes at four. Park on the east side.".to_string());
                                }>" "</button>
                            </div>
                        </div>
                    </div>
                </Show>
                <Show when=move || notes.get()>
                    <div class="flex flex-col flex-1 h-full min-w-0 bp800:min-w-[480px] overflow-y-auto bg-canvas px-6 py-6 dark:bg-canvas">
                        <div class="flex flex-col w-full max-w-[640px] gap-6">
                            <div class="flex flex-col gap-2">
                                <span class="text-page text-primary w-full dark:text-primary">{note_title}</span>
                                <span class="text-caption text-secondary w-full dark:text-secondary">{note_when}</span>
                            </div>
                            <span class="text-reading text-primary w-full dark:text-primary">{note_body}</span>
                            <span class="text-reading text-secondary w-full dark:text-secondary">{note_more}</span>
                            <Show when=move || chart_on.get()>
                                <div class="flex flex-col gap-2">
                                    <span class="text-caption text-secondary dark:text-secondary">"This week"</span>
                                    <div class="flex flex-row items-end gap-3 h-[108px]">
                                        <div class="flex flex-col items-center justify-end h-[100px] gap-1">
                                            <div class="w-[14px] h-[28px] rounded-sm bg-accent"></div>
                                            <span class="text-caption text-secondary dark:text-secondary">"S"</span>
                                        </div>
                                        <div class="flex flex-col items-center justify-end h-[100px] gap-1">
                                            <div class="w-[14px] h-[44px] rounded-sm bg-accent"></div>
                                            <span class="text-caption text-secondary dark:text-secondary">"M"</span>
                                        </div>
                                        <div class="flex flex-col items-center justify-end h-[100px] gap-1">
                                            <div class="w-[14px] h-[36px] rounded-sm bg-accent"></div>
                                            <span class="text-caption text-secondary dark:text-secondary">"T"</span>
                                        </div>
                                        <div class="flex flex-col items-center justify-end h-[100px] gap-1">
                                            <div class="w-[14px] h-[64px] rounded-sm bg-accent"></div>
                                            <span class="text-caption text-secondary dark:text-secondary">"W"</span>
                                        </div>
                                        <div class="flex flex-col items-center justify-end h-[100px] gap-1">
                                            <div class="w-[14px] h-[52px] rounded-sm bg-accent"></div>
                                            <span class="text-caption text-secondary dark:text-secondary">"T"</span>
                                        </div>
                                        <div class="flex flex-col items-center justify-end h-[100px] gap-1">
                                            <div class="w-[14px] h-[40px] rounded-sm bg-accent"></div>
                                            <span class="text-caption text-secondary dark:text-secondary">"F"</span>
                                        </div>
                                        <div class="flex flex-col items-center justify-end h-[100px] gap-1">
                                            <div class="w-[14px] h-[24px] rounded-sm bg-accent"></div>
                                            <span class="text-caption text-secondary dark:text-secondary">"S"</span>
                                        </div>
                                    </div>
                                </div>
                            </Show>
                        </div>
                    </div>
                </Show>
                <Show when=move || table.get()>
                    <div class="flex flex-col flex-1 h-full min-w-0 bp800:min-w-[480px] overflow-y-auto bg-canvas p-6 gap-6 dark:bg-canvas">
                        <span class="text-page text-primary dark:text-primary">"All notes"</span>
                        <div class="rounded-[8px] bg-surface border border-divider overflow-y-auto dark:bg-surface dark:border-divider">
                            <div class="flex flex-row items-center h-[36px] px-3">
                                <button class="h-control px-3 rounded-control text-caption text-secondary border-0 bg-transparent dark:text-secondary" on:click=move |_| {
                                    set_rows.update(|rows| { rows.reverse(); });
                                }>"Name"</button>
                                <div class="flex-1"></div>
                                <span class="text-caption text-secondary px-3 dark:text-secondary">"Kind"</span>
                            </div>
                            <div class="h-[1px] bg-divider dark:bg-divider"></div>
                            <For each=move || rows.get() key=|item| item.id let:item let:index>
                                <NoteRow item=item />
                            </For>
                        </div>
                    </div>
                </Show>
                <Show when=move || settings.get()>
                    <div class="flex flex-col flex-1 h-full min-w-0 bp800:min-w-[480px] overflow-y-auto bg-canvas p-6 gap-8 dark:bg-canvas">
                        <span class="text-page text-primary dark:text-primary">"Settings"</span>
                        <div class="flex flex-col bp1120:flex-row gap-6">
                            <div class="flex flex-col gap-6 w-full bp1120:w-[280px]">
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"View"</span>
                                    <div class="flex flex-row items-center p-1 rounded-control bg-subtle w-fit dark:bg-subtle">
                                        <div class="w-[88px] h-control">
                                            <Show when=move || day.get()>
                                                <div class="w-[88px] h-control rounded-control bg-surface border border-divider dark:bg-surface dark:border-divider"></div>
                                            </Show>
                                            <button class="absolute inset-0 border-0 bg-transparent text-control text-primary dark:text-primary" on:click=move |_| { set_day.set(true); set_week.set(false); set_month.set(false); }>"Day"</button>
                                        </div>
                                        <div class="w-[88px] h-control">
                                            <Show when=move || week.get()>
                                                <div class="w-[88px] h-control rounded-control bg-surface border border-divider dark:bg-surface dark:border-divider"></div>
                                            </Show>
                                            <button class="absolute inset-0 border-0 bg-transparent text-control text-primary dark:text-primary" on:click=move |_| { set_day.set(false); set_week.set(true); set_month.set(false); }>"Week"</button>
                                        </div>
                                        <div class="w-[88px] h-control">
                                            <Show when=move || month.get()>
                                                <div class="w-[88px] h-control rounded-control bg-surface border border-divider dark:bg-surface dark:border-divider"></div>
                                            </Show>
                                            <button class="absolute inset-0 border-0 bg-transparent text-control text-primary dark:text-primary" on:click=move |_| { set_day.set(false); set_week.set(false); set_month.set(true); }>"Month"</button>
                                        </div>
                                    </div>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"Sync"</span>
                                    <Show when=move || switched.get()>
                                        <SwitchOn label="Sync notes" on:click=move |_| { set_switched.set(false); set_switch_off.set(true); } />
                                    </Show>
                                    <Show when=move || switch_off.get()>
                                        <SwitchOff label="Sync notes" on:click=move |_| { set_switched.set(true); set_switch_off.set(false); } />
                                    </Show>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"Menu bar"</span>
                                    <Show when=move || checked.get()>
                                        <CheckRow label="Show in menu bar" on:click=move |_| { set_checked.set(false); set_check_off.set(true); }>
                                            <CheckOn />
                                        </CheckRow>
                                    </Show>
                                    <Show when=move || check_off.get()>
                                        <CheckRow label="Show in menu bar" on:click=move |_| { set_checked.set(true); set_check_off.set(false); }>
                                            <CheckOff />
                                        </CheckRow>
                                    </Show>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"Account"</span>
                                    <Button variant="secondary" on:click=move |_| { set_menu.update(|value| *value = !*value); }>{choice}</Button>
                                    <Show when=move || menu.get()>
                                        <div class="flex flex-col w-[220px] p-1 rounded-control bg-surface border border-divider shadow-lg dark:bg-surface dark:border-divider">
                                            <button class="h-[32px] px-3 rounded-control text-body text-primary border-0 bg-transparent dark:text-primary" on:click=move |_| { set_choice.set("On My Mac".to_string()); set_menu.set(false); }>"On My Mac"</button>
                                            <button class="h-[32px] px-3 rounded-control text-body text-primary border-0 bg-transparent dark:text-primary" on:click=move |_| { set_choice.set("iCloud".to_string()); set_menu.set(false); }>"iCloud"</button>
                                            <button class="h-[32px] px-3 rounded-control text-body text-primary border-0 bg-transparent dark:text-primary" on:click=move |_| { set_choice.set("Recently Deleted".to_string()); set_menu.set(false); }>"Recently Deleted"</button>
                                        </div>
                                    </Show>
                                </div>
                                <div class="flex flex-col gap-1 w-full">
                                    <Field label="Folder" placeholder="Folder name" bind:value=draft />
                                    <span class="text-caption text-secondary dark:text-secondary">"Shown on this Mac."</span>
                                </div>
                            </div>
                            <div class="flex flex-col gap-6 w-full bp1120:w-[280px]">
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"Actions"</span>
                                    <div class="flex flex-row flex-wrap items-center gap-2">
                                        <Button variant="danger" on:click=move |_| { set_alert.set(true); }>"Delete note"</Button>
                                        <Button variant="secondary" on:click=move |_| { set_toast.set(true); }>"Bordered"</Button>
                                        <Button variant="ghost">"Borderless"</Button>
                                    </div>
                                    <Button disabled=true>"Disabled"</Button>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"Copies"</span>
                                    <Stepper>
                                        <Button variant="secondary" class="w-control h-control" on:click=move |_| { set_step.set(clamp(step.get() - 1, 1, 8)); }>"−"</Button>
                                        <span class="text-body text-primary w-[32px] dark:text-primary">{step}</span>
                                        <Button variant="secondary" class="w-control h-control" on:click=move |_| { set_step.set(clamp(step.get() + 1, 1, 8)); }>"+"</Button>
                                    </Stepper>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"Level"</span>
                                    <Slider signal="level">
                                        <div class="absolute inset-0 flex flex-row">
                                            <button class="w-[30px] h-control border-0 bg-transparent" on:click=move |_| { set_level.set(0); }>" "</button>
                                            <button class="w-[30px] h-control border-0 bg-transparent" on:click=move |_| { set_level.set(14); }>" "</button>
                                            <button class="w-[30px] h-control border-0 bg-transparent" on:click=move |_| { set_level.set(28); }>" "</button>
                                            <button class="w-[30px] h-control border-0 bg-transparent" on:click=move |_| { set_level.set(43); }>" "</button>
                                            <button class="w-[30px] h-control border-0 bg-transparent" on:click=move |_| { set_level.set(57); }>" "</button>
                                            <button class="w-[30px] h-control border-0 bg-transparent" on:click=move |_| { set_level.set(71); }>" "</button>
                                            <button class="w-[30px] h-control border-0 bg-transparent" on:click=move |_| { set_level.set(86); }>" "</button>
                                            <button class="w-[30px] h-control border-0 bg-transparent" on:click=move |_| { set_level.set(100); }>" "</button>
                                        </div>
                                    </Slider>
                                    <Stepper>
                                        <Button variant="secondary" class="w-control h-control" on:click=move |_| { set_level.set(clamp(level.get() - 5, 0, 100)); }>"−"</Button>
                                        <span class="text-body text-primary w-[40px] dark:text-primary">{level}</span>
                                        <Button variant="secondary" class="w-control h-control" on:click=move |_| { set_level.set(clamp(level.get() + 5, 0, 100)); }>"+"</Button>
                                    </Stepper>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"Progress"</span>
                                    <div class="w-[240px] max-w-full h-[4px] rounded-full bg-divider dark:bg-divider">
                                        <div class="h-[4px] rounded-full bg-accent" bind:width="level*2.24"></div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </Show>
                <Show when=move || gallery.get()>
                    <div class="flex flex-col flex-1 h-full min-w-0 bp800:min-w-[480px] overflow-y-auto bg-canvas p-6 gap-8 dark:bg-canvas">
                        <div class="flex flex-col gap-2">
                            <span class="text-page text-primary dark:text-primary">"Component reference"</span>
                            <span class="text-body text-secondary w-full max-w-[640px] dark:text-secondary">"Foundations, controls, and the states this build can actually show. A static swatch is not a working hover."</span>
                        </div>
                        <div class="flex flex-col gap-4">
                            <span class="text-section text-primary dark:text-primary">"Foundations"</span>
                            <span class="text-display text-primary dark:text-primary">"Display"</span>
                            <span class="text-page text-primary dark:text-primary">"Page title"</span>
                            <span class="text-section text-primary dark:text-primary">"Section heading"</span>
                            <span class="text-reading text-primary w-full max-w-[640px] dark:text-primary">"Reading text stays on its own line height and wraps inside the column."</span>
                            <span class="text-body text-primary dark:text-primary">"Body text"</span>
                            <span class="text-control text-primary dark:text-primary">"Control label"</span>
                            <span class="text-caption text-secondary dark:text-secondary">"Caption and metadata"</span>
                            <div class="flex flex-row gap-2">
                                <div class="w-[48px] h-[48px] rounded-control bg-canvas border border-divider"></div>
                                <div class="w-[48px] h-[48px] rounded-control bg-surface border border-divider"></div>
                                <div class="w-[48px] h-[48px] rounded-control bg-subtle border border-divider"></div>
                                <div class="w-[48px] h-[48px] rounded-control bg-accent"></div>
                                <div class="w-[48px] h-[48px] rounded-control bg-selection border border-divider"></div>
                                <div class="w-[48px] h-[48px] rounded-control bg-danger-bg border border-danger"></div>
                                <div class="w-[48px] h-[48px] rounded-control bg-success-bg border border-success"></div>
                                <div class="w-[48px] h-[48px] rounded-control bg-warning-bg border border-warning"></div>
                            </div>
                            <div class="flex flex-row items-end gap-2">
                                <div class="w-[8px] h-[4px] bg-accent"></div>
                                <div class="w-[8px] h-[8px] bg-accent"></div>
                                <div class="w-[8px] h-[12px] bg-accent"></div>
                                <div class="w-[8px] h-[16px] bg-accent"></div>
                                <div class="w-[8px] h-[24px] bg-accent"></div>
                                <div class="w-[8px] h-[32px] bg-accent"></div>
                            </div>
                        </div>
                        <div class="flex flex-col gap-4">
                            <span class="text-section text-primary dark:text-primary">"Controls"</span>
                            <div class="flex flex-row flex-wrap items-center gap-2">
                                <Button>"Primary"</Button>
                                <Button variant="secondary">"Secondary"</Button>
                                <Button variant="ghost">"Ghost"</Button>
                                <Button variant="danger">"Destructive"</Button>
                                <Button disabled=true>"Disabled"</Button>
                            </div>
                            <div class="w-full max-w-[280px]">
                                <Field label="Folder" placeholder="Folder name" bind:value=draft />
                            </div>
                            <Search placeholder="Search notes" bind:value=query />
                            <div class="w-[224px] border border-divider rounded-control">
                                <NavRow class="bg-selection">
                                    <div class="w-[16px] h-[16px] rounded-[4px] bg-accent"></div>
                                    <span class="text-body text-on-selection w-[160px]">"Weekly planning notes for the kitchen cabinet sample review."</span>
                                </NavRow>
                            </div>
                            <CheckRow label="Show in menu bar">
                                <CheckOn />
                            </CheckRow>
                            <SwitchOff label="Sync is off" />
                            <div class="flex flex-col gap-2 w-full max-w-[360px] p-4 rounded-lg bg-surface border border-divider dark:bg-surface dark:border-divider">
                                <span class="text-control text-primary dark:text-primary">"Moved to Recently Deleted"</span>
                                <span class="text-caption text-secondary w-full dark:text-secondary">"The live toast uses Overlay, pinned 16 units from the corner, width 360."</span>
                            </div>
                        </div>
                        <div class="flex flex-col gap-4">
                            <span class="text-section text-primary dark:text-primary">"State specimens"</span>
                            <span class="text-caption text-secondary w-full dark:text-secondary">"Disabled is a solid face. Hover and pressed are live on the buttons above. Focus is the accent stroke after a click. This build has no keyboard focus ring to photograph."</span>
                            <div class="flex flex-col gap-1 w-full max-w-[280px]">
                                <span class="text-control text-primary dark:text-primary">"Folder"</span>
                                <input class="w-full h-control px-3 rounded-control bg-surface text-body text-primary border border-danger dark:bg-surface dark:text-primary dark:border-danger" placeholder="Folder name" />
                                <span class="text-caption text-danger w-full">"The folder name is required, must stay under 40 characters, and cannot include a slash or a leading space."</span>
                            </div>
                            <Button class="w-[180px]">"Move the selected note to Recently Deleted"</Button>
                            <Button variant="secondary">"Save this note to the library"</Button>
                        </div>
                        <div class="flex flex-col gap-4">
                            <span class="text-section text-primary dark:text-primary">"Content and layout stress"</span>
                            <span class="text-caption text-secondary w-full dark:text-secondary">"cabinet-sample-identifier-without-breaks-2026-10-26-manila"</span>
                            <div class="flex flex-col gap-2 p-4 rounded-control bg-surface border border-divider w-full max-w-[280px] dark:bg-surface dark:border-divider">
                                <span class="text-control text-primary dark:text-primary">"Empty list"</span>
                                <span class="text-body text-secondary w-full dark:text-secondary">"No notes yet. Create one from New note."</span>
                            </div>
                            <div class="w-full h-[240px] bg-subtle rounded-control dark:bg-subtle">
                                <div class="absolute left-[8px] top-[8px] px-3 py-2 rounded-control bg-surface border border-divider dark:bg-surface dark:border-divider">
                                    <span class="text-caption text-primary dark:text-primary">"Top start"</span>
                                </div>
                                <div class="absolute right-[8px] top-[8px] px-3 py-2 rounded-control bg-surface border border-divider dark:bg-surface dark:border-divider">
                                    <span class="text-caption text-primary dark:text-primary">"Top end"</span>
                                </div>
                                <div class="absolute left-[8px] bottom-[8px] px-3 py-2 rounded-control bg-surface border border-divider dark:bg-surface dark:border-divider">
                                    <span class="text-caption text-primary dark:text-primary">"Bottom start"</span>
                                </div>
                                <div class="absolute right-[8px] bottom-[8px] px-3 py-2 rounded-control bg-surface border border-divider dark:bg-surface dark:border-divider">
                                    <span class="text-caption text-primary dark:text-primary">"Bottom end"</span>
                                </div>
                            </div>
                            <div class="flex flex-row gap-2">
                                <Button variant="secondary" on:click=move |_| { set_density(1); set_density_std.set(true); set_density_touch.set(false); }>"Standard density"</Button>
                                <Button variant="secondary" on:click=move |_| { set_density(2); set_density_std.set(false); set_density_touch.set(true); }>"Touch density"</Button>
                                <Button variant="ghost" on:click=move |_| { toggle_theme(); set_dark_on.update(|value| *value = !*value); set_light_on.update(|value| *value = !*value); }>"Theme"</Button>
                            </div>
                        </div>
                        <div class="flex flex-col gap-2 w-full max-w-[640px]">
                            <span class="text-section text-primary dark:text-primary">"Usage"</span>
                            <span class="text-caption text-secondary w-full dark:text-secondary">"Button variant secondary, on click, label Save. Field label Folder, placeholder Folder name, bind value draft."</span>
                        </div>
                    </div>
                </Show>
                <Show when=move || table.get()>
                    <div class="flex flex-col h-full overflow-y-auto bg-subtle p-4 gap-4 w-[0px] bp1440:w-[280px] bp1440:min-w-[260px] bp1440:max-w-[360px] dark:bg-subtle">
                        <span class="text-caption text-secondary dark:text-secondary">"Inspector"</span>
                        <span class="text-section text-primary dark:text-primary">"All notes"</span>
                        <div class="h-[1px] bg-divider dark:bg-divider"></div>
                        <span class="text-caption text-secondary dark:text-secondary">"Search"</span>
                        <span class="text-body text-primary w-full dark:text-primary">{query}</span>
                        <span class="text-caption text-secondary dark:text-secondary">"Account"</span>
                        <span class="text-body text-primary w-full dark:text-primary">{choice}</span>
                        <Button variant="ghost" on:click=move |_| { set_info_open.update(|value| *value = !*value); }>"Info"</Button>
                    </div>
                </Show>
                <Show when=move || settings.get()>
                    <div class="flex flex-col h-full overflow-y-auto bg-subtle p-4 gap-4 w-[0px] bp1440:w-[280px] bp1440:min-w-[260px] bp1440:max-w-[360px] dark:bg-subtle">
                        <span class="text-caption text-secondary dark:text-secondary">"Inspector"</span>
                        <span class="text-section text-primary dark:text-primary">"Settings"</span>
                        <div class="h-[1px] bg-divider dark:bg-divider"></div>
                        <span class="text-caption text-secondary dark:text-secondary">"Search"</span>
                        <span class="text-body text-primary w-full dark:text-primary">{query}</span>
                        <span class="text-caption text-secondary dark:text-secondary">"Account"</span>
                        <span class="text-body text-primary w-full dark:text-primary">{choice}</span>
                        <span class="text-caption text-secondary dark:text-secondary">"Folder"</span>
                        <span class="text-body text-primary w-full dark:text-primary">{draft}</span>
                    </div>
                </Show>
            </div>
            <Show when=move || toast.get()>
                <Overlay>
                    <span class="text-body text-primary dark:text-primary">"Moved to Recently Deleted"</span>
                    <Button variant="ghost" on:click=move |_| { set_toast.set(false); }>"Dismiss"</Button>
                </Overlay>
            </Show>
            <Show when=move || nav_open.get()>
                <div class="absolute left-[8px] top-[64px] z-50 flex flex-col gap-1 w-[224px] p-2 rounded-lg bg-surface border border-divider shadow-lg dark:bg-surface dark:border-divider">
                    <Button variant="ghost" on:click=move |_| { set_notes.set(true); set_table.set(false); set_settings.set(false); set_gallery.set(false); set_nav_open.set(false); }>"Notes"</Button>
                    <Button variant="ghost" on:click=move |_| { set_notes.set(false); set_table.set(true); set_settings.set(false); set_gallery.set(false); set_nav_open.set(false); }>"Table"</Button>
                    <Button variant="ghost" on:click=move |_| { set_notes.set(false); set_table.set(false); set_settings.set(true); set_gallery.set(false); set_nav_open.set(false); }>"Settings"</Button>
                    <Button variant="ghost" on:click=move |_| { set_notes.set(false); set_table.set(false); set_settings.set(false); set_gallery.set(true); set_nav_open.set(false); }>"Gallery"</Button>
                </div>
            </Show>
            <Portal>
                <Show when=move || sheet.get()>
                    <div class="absolute inset-0">
                        <button class="absolute inset-0 z-40 border-0 bg-scrim text-primary dark:bg-scrim" on:click=move |_| { set_sheet.set(false); }>" "</button>
                        <div class="absolute left-[24px] top-[48px] z-50 flex flex-col gap-4 w-[480px] max-w-full p-6 rounded-lg bg-surface border border-divider shadow-lg dark:bg-surface dark:border-divider">
                            <span class="text-section text-primary dark:text-primary">"New note"</span>
                            <div class="flex flex-col gap-2 w-full max-h-[240px] overflow-y-auto">
                                <span class="text-body text-secondary w-full dark:text-secondary">"Saved on this Mac. You can find it in the library after it lands."</span>
                                <span class="text-body text-secondary w-full dark:text-secondary">"The cabinet paint is still the sample from last week. Leave a note for the joiner and keep the island where it is."</span>
                                <span class="text-body text-secondary w-full dark:text-secondary">"If this dialog is taller than the window, this body scrolls and the actions stay put."</span>
                            </div>
                            <input class="w-full h-control px-3 rounded-control bg-surface text-body text-primary border border-control dark:bg-surface dark:text-primary dark:border-control" placeholder="Title" />
                            <div class="flex flex-row gap-2">
                                <Button on:click=move |_| { set_sheet.set(false); set_toast.set(true); }>"Save"</Button>
                                <Button variant="secondary" on:click=move |_| { set_sheet.set(false); }>"Cancel"</Button>
                            </div>
                        </div>
                    </div>
                </Show>
                <Show when=move || alert.get()>
                    <div class="absolute inset-0">
                        <div class="absolute inset-0 z-40 bg-scrim dark:bg-scrim"></div>
                        <div class="absolute left-[24px] top-[80px] z-50 flex flex-col gap-4 w-[400px] max-w-full p-6 rounded-lg bg-surface border border-divider shadow-lg dark:bg-surface dark:border-divider">
                            <span class="text-section text-primary dark:text-primary">"Delete this note?"</span>
                            <span class="text-body text-secondary w-full dark:text-secondary">"The note moves to Recently Deleted. You can recover it for 30 days."</span>
                            <div class="flex flex-row gap-2">
                                <Button variant="danger" on:click=move |_| { set_alert.set(false); set_toast.set(true); }>"Delete"</Button>
                                <Button variant="secondary" on:click=move |_| { set_alert.set(false); }>"Cancel"</Button>
                            </div>
                        </div>
                    </div>
                </Show>
            </Portal>
        </div>
    }
}
