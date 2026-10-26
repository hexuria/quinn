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
    let (toast_msg, set_toast_msg) = signal("Moved to Recently Deleted".to_string());
    let (checked, set_checked) = signal(true);
    let (check_off, set_check_off) = signal(false);
    let (switched, set_switched) = signal(true);
    let (switch_off, set_switch_off) = signal(false);
    let (demo_check, set_demo_check) = signal(true);
    let (demo_uncheck, set_demo_uncheck) = signal(false);
    let (demo_on, set_demo_on) = signal(true);
    let (demo_off, set_demo_off) = signal(false);
    let (demo_nav, _set_demo_nav) = signal(true);
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
    let (density_compact, set_density_compact) = signal(false);
    let (density_std, set_density_std) = signal(true);
    let (density_touch, set_density_touch) = signal(false);
    let (unsorted, set_unsorted) = signal(true);
    let (sorted, set_sorted) = signal(false);
    let (sort_asc, set_sort_asc) = signal(false);
    let (sort_desc, set_sort_desc) = signal(false);
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
        <div class="flex flex-col h-full bg-canvas dark:bg-canvas" on:keydown=move |ev| {
            if ev.key() == "Escape" {
                if alert.get() {
                    set_alert.set(false);
                } else if sheet.get() {
                    set_sheet.set(false);
                } else if menu.get() {
                    set_menu.set(false);
                } else if nav_open.get() {
                    set_nav_open.set(false);
                } else if info_open.get() {
                    set_info_open.set(false);
                } else if toast.get() {
                    set_toast.set(false);
                }
            }
        }>
            <div class="flex flex-row items-center h-toolbar px-4 gap-2 bg-surface dark:bg-surface">
                <Button variant="ghost" on:click=move |_| { set_nav_open.update(|value| *value = !*value); }>"Library"</Button>
                <Show when=move || notes.get()>
                    <span class="text-section text-primary dark:text-primary">"Notes"</span>
                </Show>
                <Show when=move || table.get()>
                    <span class="text-section text-primary dark:text-primary">"Table"</span>
                </Show>
                <Show when=move || settings.get()>
                    <span class="text-section text-primary dark:text-primary">"Settings"</span>
                </Show>
                <Show when=move || gallery.get()>
                    <span class="text-section text-primary dark:text-primary">"Gallery"</span>
                </Show>
                <div class="flex-1"></div>
                <Search placeholder="Search" bind:value=query />
                <div class="flex-1"></div>
                <Button variant="ghost" on:click=move |_| { toggle_theme(); set_dark_on.update(|value| *value = !*value); set_light_on.update(|value| *value = !*value); }>"Theme"</Button>
                <Button on:click=move |_| { set_sheet.set(true); }>"New note"</Button>
            </div>
            <div class="h-[1px] bg-divider dark:bg-divider"></div>
            <div class="flex flex-row flex-1 items-stretch">
                <div class="flex flex-col h-full bg-subtle overflow-hidden w-[0px] bp800:w-[64px] bp1120:w-[224px] bp1120:min-w-[200px] bp1120:max-w-[280px] dark:bg-subtle">
                    <div class="flex flex-col gap-1 py-3 bp1120:px-2">
                        <span class="text-caption text-secondary px-3 w-[0px] h-[0px] bp1120:w-[160px] bp1120:h-fit dark:text-secondary">"Library"</span>
                        <NavItem icon="notes" selected=move || notes.get() on:click=move |_| { set_notes.set(true); set_table.set(false); set_settings.set(false); set_gallery.set(false); set_nav_open.set(false); }>"Notes"</NavItem>
                        <NavItem icon="table" selected=move || table.get() on:click=move |_| { set_notes.set(false); set_table.set(true); set_settings.set(false); set_gallery.set(false); set_nav_open.set(false); }>"Table"</NavItem>
                        <NavItem icon="settings" selected=move || settings.get() on:click=move |_| { set_notes.set(false); set_table.set(false); set_settings.set(true); set_gallery.set(false); set_nav_open.set(false); }>"Settings"</NavItem>
                        <NavItem icon="gallery" selected=move || gallery.get() on:click=move |_| { set_notes.set(false); set_table.set(false); set_settings.set(false); set_gallery.set(true); set_nav_open.set(false); }>"Gallery"</NavItem>
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
                                <SortHeader sorted=move || sorted.get() descending=move || sort_desc.get() on:click=move |_| {
                                    if sort_asc.get() {
                                        set_rows.update(|rows| { rows.sort_by(|a, b| b.label.cmp(&a.label)); });
                                        set_sort_asc.set(false);
                                        set_sort_desc.set(true);
                                    } else {
                                        set_rows.update(|rows| { rows.sort_by(|a, b| a.label.cmp(&b.label)); });
                                        set_sort_asc.set(true);
                                        set_sort_desc.set(false);
                                    }
                                    set_sorted.set(true);
                                    set_unsorted.set(false);
                                }>"Name"</SortHeader>
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
                                    <Segmented>
                                        <Segment class="w-[88px]" selected=move || day.get() on:click=move |_| { set_day.set(true); set_week.set(false); set_month.set(false); }>"Day"</Segment>
                                        <Segment class="w-[88px]" selected=move || week.get() on:click=move |_| { set_day.set(false); set_week.set(true); set_month.set(false); }>"Week"</Segment>
                                        <Segment class="w-[88px]" selected=move || month.get() on:click=move |_| { set_day.set(false); set_week.set(false); set_month.set(true); }>"Month"</Segment>
                                    </Segmented>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"Sync"</span>
                                    <Switch checked=move || switched.get() on:click=move |_| { set_switched.update(|value| *value = !*value); set_switch_off.update(|value| *value = !*value); }>"Sync notes"</Switch>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"Menu bar"</span>
                                    <Checkbox checked=move || checked.get() on:click=move |_| { set_checked.update(|value| *value = !*value); set_check_off.update(|value| *value = !*value); }>"Show in menu bar"</Checkbox>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"Account"</span>
                                    <Button variant="secondary" on:click=move |_| { set_menu.update(|value| *value = !*value); }>{choice}</Button>
                                    <Show when=move || menu.get()>
                                        <Menu>
                                            <MenuItem on:click=move |_| { set_choice.set("On My Mac".to_string()); set_menu.set(false); }>"On My Mac"</MenuItem>
                                            <MenuItem on:click=move |_| { set_choice.set("iCloud".to_string()); set_menu.set(false); }>"iCloud"</MenuItem>
                                            <MenuItem on:click=move |_| { set_choice.set("Recently Deleted".to_string()); set_menu.set(false); }>"Recently Deleted"</MenuItem>
                                        </Menu>
                                    </Show>
                                </div>
                                <div class="flex flex-col gap-1 w-full">
                                    <Field label="Folder" placeholder="Folder name" bind:value=draft />
                                    <span class="text-caption text-secondary dark:text-secondary">"Shown on this Mac."</span>
                                </div>
                            </div>
                            <div class="flex flex-col gap-6 w-full bp1120:w-[320px]">
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"Density"</span>
                                    <Segmented>
                                        <Segment class="w-[100px]" selected=move || density_compact.get() on:click=move |_| { set_density(0); set_density_compact.set(true); set_density_std.set(false); set_density_touch.set(false); }>"Compact"</Segment>
                                        <Segment class="w-[100px]" selected=move || density_std.get() on:click=move |_| { set_density(1); set_density_compact.set(false); set_density_std.set(true); set_density_touch.set(false); }>"Standard"</Segment>
                                        <Segment class="w-[100px]" selected=move || density_touch.get() on:click=move |_| { set_density(2); set_density_compact.set(false); set_density_std.set(false); set_density_touch.set(true); }>"Touch"</Segment>
                                    </Segmented>
                                    <span class="text-caption text-secondary w-full dark:text-secondary">"Control height 28, 36, or 48."</span>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <div class="flex flex-row items-center w-full">
                                        <span class="text-control text-primary dark:text-primary">"Level"</span>
                                        <div class="flex-1"></div>
                                        <span class="text-body text-secondary dark:text-secondary">{level}</span>
                                    </div>
                                    <Slider bind:value=level step=move || step.get() />
                                    <span class="text-caption text-secondary w-full dark:text-secondary">"Drag, or focus and use the arrow keys. Shift steps by 10."</span>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"Copies"</span>
                                    <Stepper>
                                        <Button variant="secondary" class="w-control h-control" on:click=move |_| { set_step.set(clamp(step.get() - 1, 1, 8)); }>"−"</Button>
                                        <div class="flex flex-row justify-center w-[32px]">
                                            <span class="text-body text-primary dark:text-primary">{step}</span>
                                        </div>
                                        <Button variant="secondary" class="w-control h-control" on:click=move |_| { set_step.set(clamp(step.get() + 1, 1, 8)); }>"+"</Button>
                                    </Stepper>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"Storage"</span>
                                    <div class="w-[320px] max-w-full h-[4px] rounded-full bg-divider dark:bg-divider">
                                        <div class="h-[4px] rounded-full bg-accent dark:bg-accent" bind:width="level*3.2"></div>
                                    </div>
                                    <span class="text-caption text-secondary w-full dark:text-secondary">"Follows Level, as a determinate progress bar."</span>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <span class="text-control text-primary dark:text-primary">"Actions"</span>
                                    <span class="text-caption text-secondary w-full dark:text-secondary">"Export a copy, or move this note to Recently Deleted."</span>
                                    <div class="flex flex-row flex-wrap items-center gap-2">
                                        <Button variant="secondary" on:click=move |_| { set_toast_msg.set("Exported a copy to Downloads".to_string()); set_toast.set(true); }>"Export"</Button>
                                        <Button variant="danger" on:click=move |_| { set_alert.set(true); }>"Delete note"</Button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </Show>
                <Show when=move || gallery.get()>
                    <div class="flex flex-col flex-1 h-full min-w-0 bp800:min-w-[480px] overflow-y-auto bg-canvas p-6 gap-8 dark:bg-canvas">
                        <div class="flex flex-col w-full max-w-[880px] gap-4">
                            <div class="flex flex-col gap-2">
                                <span class="text-page text-primary dark:text-primary">"Component reference"</span>
                                <span class="text-body text-secondary dark:text-secondary w-full max-w-[640px]">"Every control this build ships, with the states it can actually show. Switch theme and density here; the whole app follows."</span>
                            </div>
                            <div class="flex flex-row flex-wrap items-center gap-2">
                                <Segmented>
                                    <Segment class="w-[100px]" selected=move || density_compact.get() on:click=move |_| { set_density(0); set_density_compact.set(true); set_density_std.set(false); set_density_touch.set(false); }>"Compact"</Segment>
                                    <Segment class="w-[100px]" selected=move || density_std.get() on:click=move |_| { set_density(1); set_density_compact.set(false); set_density_std.set(true); set_density_touch.set(false); }>"Standard"</Segment>
                                    <Segment class="w-[100px]" selected=move || density_touch.get() on:click=move |_| { set_density(2); set_density_compact.set(false); set_density_std.set(false); set_density_touch.set(true); }>"Touch"</Segment>
                                </Segmented>
                                <div class="w-[8px] h-[1px]"></div>
                                <Show when=move || light_on.get()>
                                    <Button variant="ghost" on:click=move |_| { toggle_theme(); set_dark_on.set(true); set_light_on.set(false); }>"Dark theme"</Button>
                                </Show>
                                <Show when=move || dark_on.get()>
                                    <Button variant="ghost" on:click=move |_| { toggle_theme(); set_dark_on.set(false); set_light_on.set(true); }>"Light theme"</Button>
                                </Show>
                            </div>
                        </div>
                        <div class="flex flex-col w-full max-w-[880px] gap-6">
                            <span class="text-section text-primary dark:text-primary">"Foundations"</span>
                            <Specimen title="Typography" note="Seven roles. Sizes are base sizes; text wraps instead of clipping.">
                                <div class="flex flex-col w-full gap-2">
                                    <span class="text-display text-primary dark:text-primary">"Display 32"</span>
                                    <span class="text-page text-primary dark:text-primary">"Page title 24"</span>
                                    <span class="text-section text-primary dark:text-primary">"Section heading 18"</span>
                                    <span class="text-reading text-primary dark:text-primary w-full max-w-[640px]">"Reading 16. Reading text keeps its own line height and wraps inside the column."</span>
                                    <span class="text-body text-primary dark:text-primary">"Body 14"</span>
                                    <span class="text-control text-primary dark:text-primary">"Control label 14"</span>
                                    <span class="text-caption text-secondary dark:text-secondary">"Caption 12"</span>
                                </div>
                            </Specimen>
                            <Specimen title="Semantic colors" note="Tokens, not raw colors. Each swatch is the live token for the current theme.">
                                <div class="flex flex-row flex-wrap gap-4">
                                    <div class="flex flex-col gap-1 w-[72px]">
                                        <div class="w-[72px] h-[48px] rounded-control bg-canvas border border-divider dark:bg-canvas dark:border-divider"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"Canvas"</span>
                                    </div>
                                    <div class="flex flex-col gap-1 w-[72px]">
                                        <div class="w-[72px] h-[48px] rounded-control bg-surface border border-divider dark:bg-surface dark:border-divider"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"Surface"</span>
                                    </div>
                                    <div class="flex flex-col gap-1 w-[72px]">
                                        <div class="w-[72px] h-[48px] rounded-control bg-subtle border border-divider dark:bg-subtle dark:border-divider"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"Subtle"</span>
                                    </div>
                                    <div class="flex flex-col gap-1 w-[72px]">
                                        <div class="w-[72px] h-[48px] rounded-control bg-accent dark:bg-accent"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"Accent"</span>
                                    </div>
                                    <div class="flex flex-col gap-1 w-[72px]">
                                        <div class="w-[72px] h-[48px] rounded-control bg-selection dark:bg-selection"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"Selection"</span>
                                    </div>
                                    <div class="flex flex-col gap-1 w-[72px]">
                                        <div class="w-[72px] h-[48px] rounded-control bg-danger-bg border border-danger dark:bg-danger-bg dark:border-danger"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"Danger"</span>
                                    </div>
                                    <div class="flex flex-col gap-1 w-[72px]">
                                        <div class="w-[72px] h-[48px] rounded-control bg-success-bg border border-success dark:bg-success-bg dark:border-success"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"Success"</span>
                                    </div>
                                    <div class="flex flex-col gap-1 w-[72px]">
                                        <div class="w-[72px] h-[48px] rounded-control bg-warning-bg border border-warning dark:bg-warning-bg dark:border-warning"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"Warning"</span>
                                    </div>
                                </div>
                            </Specimen>
                            <Specimen title="Spacing" note="The only allowed steps. Parents own the gap between siblings.">
                                <div class="flex flex-row flex-wrap items-end gap-4">
                                    <div class="flex flex-col items-start gap-1 w-[40px]">
                                        <div class="w-[4px] h-[16px] rounded-[2px] bg-accent dark:bg-accent"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"4"</span>
                                    </div>
                                    <div class="flex flex-col items-start gap-1 w-[40px]">
                                        <div class="w-[8px] h-[16px] rounded-[2px] bg-accent dark:bg-accent"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"8"</span>
                                    </div>
                                    <div class="flex flex-col items-start gap-1 w-[40px]">
                                        <div class="w-[12px] h-[16px] rounded-[2px] bg-accent dark:bg-accent"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"12"</span>
                                    </div>
                                    <div class="flex flex-col items-start gap-1 w-[40px]">
                                        <div class="w-[16px] h-[16px] rounded-[2px] bg-accent dark:bg-accent"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"16"</span>
                                    </div>
                                    <div class="flex flex-col items-start gap-1 w-[40px]">
                                        <div class="w-[24px] h-[16px] rounded-[2px] bg-accent dark:bg-accent"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"24"</span>
                                    </div>
                                    <div class="flex flex-col items-start gap-1 w-[40px]">
                                        <div class="w-[32px] h-[16px] rounded-[2px] bg-accent dark:bg-accent"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"32"</span>
                                    </div>
                                    <div class="flex flex-col items-start gap-1 w-[40px]">
                                        <div class="w-[40px] h-[16px] rounded-[2px] bg-accent dark:bg-accent"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"40"</span>
                                    </div>
                                    <div class="flex flex-col items-start gap-1 w-[40px]">
                                        <div class="w-[48px] h-[16px] rounded-[2px] bg-accent dark:bg-accent"></div>
                                        <span class="text-caption text-secondary dark:text-secondary">"48"</span>
                                    </div>
                                </div>
                            </Specimen>
                        </div>
                        <div class="flex flex-col w-full max-w-[880px] gap-6">
                            <span class="text-section text-primary dark:text-primary">"Controls"</span>
                            <Specimen title="Buttons" note="Primary, secondary, ghost, destructive, and disabled. Hover and pressed are live. Tab to any button to see the 2-unit focus ring with its 2-unit gap.">
                                <div class="flex flex-row flex-wrap items-center gap-2">
                                    <Button>"Primary"</Button>
                                    <Button variant="secondary">"Secondary"</Button>
                                    <Button variant="ghost">"Ghost"</Button>
                                    <Button variant="danger">"Destructive"</Button>
                                    <Button disabled=true>"Disabled"</Button>
                                </div>
                                <div class="flex flex-row flex-wrap items-start gap-2">
                                    <Button class="w-[180px]">"Move the selected note to Recently Deleted"</Button>
                                    <Button variant="secondary">"Save this note to the library"</Button>
                                </div>
                                <span class="text-caption text-secondary dark:text-secondary w-full">"Loading is not supported in this build."</span>
                            </Specimen>
                            <Specimen title="Fields and search" note="Labels sit above controls. Errors are text plus a danger border, below the control.">
                                <div class="flex flex-col w-full max-w-[320px] gap-6">
                                    <Field label="Folder" placeholder="Folder name" bind:value=draft />
                                    <div class="flex flex-col w-full gap-2">
                                        <Field label="Folder name" placeholder="Required" invalid=true />
                                        <span class="text-caption text-danger w-full dark:text-danger">"A folder name is required and must stay under 40 characters."</span>
                                    </div>
                                    <Search placeholder="Search notes" bind:value=query />
                                </div>
                            </Specimen>
                            <Specimen title="Selection controls" note="18-unit checkbox, 40 by 24 switch. The whole row is the target; each one toggles.">
                                <div class="flex flex-row flex-wrap items-center gap-6">
                                    <Checkbox checked=move || demo_check.get() on:click=move |_| { set_demo_check.update(|value| *value = !*value); }>"Checked"</Checkbox>
                                    <Checkbox checked=move || demo_uncheck.get() on:click=move |_| { set_demo_uncheck.update(|value| *value = !*value); }>"Unchecked"</Checkbox>
                                    <Switch checked=move || demo_on.get() on:click=move |_| { set_demo_on.update(|value| *value = !*value); }>"On"</Switch>
                                    <Switch checked=move || demo_off.get() on:click=move |_| { set_demo_off.update(|value| *value = !*value); }>"Off"</Switch>
                                </div>
                            </Specimen>
                            <Specimen title="Slider, stepper, progress" note="The slider takes drag, arrow keys, Shift for steps of 10, Home and End. Same value as Settings.">
                                <div class="flex flex-col w-full max-w-[320px] gap-4">
                                    <div class="flex flex-row items-center w-full">
                                        <span class="text-control text-primary dark:text-primary">"Level"</span>
                                        <div class="flex-1"></div>
                                        <span class="text-body text-secondary dark:text-secondary">{level}</span>
                                    </div>
                                    <Slider bind:value=level step=move || step.get() />
                                    <Stepper>
                                        <Button variant="secondary" class="w-control h-control" on:click=move |_| { set_step.set(clamp(step.get() - 1, 1, 8)); }>"−"</Button>
                                        <div class="flex flex-row justify-center w-[32px]">
                                            <span class="text-body text-primary dark:text-primary">{step}</span>
                                        </div>
                                        <Button variant="secondary" class="w-control h-control" on:click=move |_| { set_step.set(clamp(step.get() + 1, 1, 8)); }>"+"</Button>
                                    </Stepper>
                                    <div class="w-[320px] max-w-full h-[4px] rounded-full bg-divider dark:bg-divider">
                                        <div class="h-[4px] rounded-full bg-accent dark:bg-accent" bind:width="level*3.2"></div>
                                    </div>
                                </div>
                            </Specimen>
                            <Specimen title="Navigation rows" note="Selected uses the selection fill and foreground. Long names end in an ellipsis.">
                                <div class="flex flex-col w-[240px] gap-1 p-2 rounded-control bg-subtle dark:bg-subtle">
                                    <NavItem icon="notes" selected=move || demo_nav.get()>"Selected"</NavItem>
                                    <NavItem icon="table">"Default"</NavItem>
                                    <NavItem icon="gallery">"Weekly planning notes for the cabinet review"</NavItem>
                                </div>
                            </Specimen>
                        </div>
                        <div class="flex flex-col w-full max-w-[880px] gap-6">
                            <span class="text-section text-primary dark:text-primary">"Feedback and overlays"</span>
                            <Specimen title="Toast" note="Top end corner, 16 from the edges, at most 360 wide. Escape dismisses it.">
                                <div class="flex flex-col w-full max-w-[360px]">
                                    <Toast message="Moved to Recently Deleted" />
                                </div>
                                <Button variant="secondary" on:click=move |_| { set_toast_msg.set("Toast from the gallery".to_string()); set_toast.set(true); }>"Show a toast"</Button>
                            </Specimen>
                            <Specimen title="Dialogs" note="Modal. Tab stays inside the dialog; Escape closes the top one. Clicking the scrim does not discard a draft.">
                                <div class="flex flex-row flex-wrap items-center gap-2">
                                    <Button variant="secondary" on:click=move |_| { set_sheet.set(true); }>"Open a dialog"</Button>
                                    <Button variant="danger" on:click=move |_| { set_alert.set(true); }>"Open an alert"</Button>
                                </div>
                            </Specimen>
                            <Specimen title="Empty state" note="Says what happened and what to do next.">
                                <div class="flex flex-col items-center justify-center w-full gap-2 p-6 rounded-control bg-subtle dark:bg-subtle">
                                    <Icon name="notes" class="w-[24px] text-muted dark:text-muted" />
                                    <span class="text-control text-primary dark:text-primary">"No notes yet"</span>
                                    <span class="text-body text-secondary dark:text-secondary">"Create one with New note."</span>
                                </div>
                            </Specimen>
                            <Specimen title="Long content" note="Unbroken identifiers wrap at the column edge instead of overflowing.">
                                <span class="text-body text-primary dark:text-primary w-full">"cabinet-sample-identifier-without-breaks-2026-10-26-manila-kitchen-remodel-final"</span>
                            </Specimen>
                        </div>
                        <div class="flex flex-col w-full max-w-[880px] gap-6">
                            <span class="text-section text-primary dark:text-primary">"Usage"</span>
                            <Specimen title="Leptos" note="These lines are taken from this page's view.rs and compile in this repository.">
                                <div class="flex flex-col w-full gap-1 p-4 rounded-control bg-subtle dark:bg-subtle">
                                    <span class="text-caption text-primary dark:text-primary w-full">"<Button variant=\"secondary\" on:click=move |_| { set_sheet.set(true); }>\"Open a dialog\"</Button>"</span>
                                    <span class="text-caption text-primary dark:text-primary w-full">"<Field label=\"Folder\" placeholder=\"Folder name\" bind:value=draft />"</span>
                                    <span class="text-caption text-primary dark:text-primary w-full">"<Slider bind:value=level step=move || step.get() />"</span>
                                </div>
                            </Specimen>
                        </div>
                    </div>
                </Show>
                <Show when=move || table.get()>
                    <div class="flex flex-col h-full overflow-y-auto bg-subtle p-4 gap-4 w-[0px] bp1440:w-[280px] bp1440:min-w-[260px] bp1440:max-w-[360px] dark:bg-subtle">
                        <div class="flex flex-col gap-1">
                            <span class="text-caption text-secondary dark:text-secondary">"Inspector"</span>
                            <span class="text-section text-primary dark:text-primary">"All notes"</span>
                        </div>
                        <div class="h-[1px] bg-divider dark:bg-divider"></div>
                        <InfoGroup title="Collection">
                        <InfoRow label="Notes">
                            <span class="text-body text-primary dark:text-primary">"8"</span>
                        </InfoRow>
                        <InfoRow label="Sorted by">
                            <Show when=move || unsorted.get()>
                                <span class="text-body text-primary dark:text-primary">"Added order"</span>
                            </Show>
                            <Show when=move || sort_asc.get()>
                                <span class="text-body text-primary dark:text-primary">"Name, A to Z"</span>
                            </Show>
                            <Show when=move || sort_desc.get()>
                                <span class="text-body text-primary dark:text-primary">"Name, Z to A"</span>
                            </Show>
                        </InfoRow>
                        <InfoRow label="Kind">
                            <span class="text-body text-primary dark:text-primary">"Note"</span>
                        </InfoRow>
                        </InfoGroup>
                        <div class="h-[1px] bg-divider dark:bg-divider"></div>
                        <InfoGroup title="Location">
                        <InfoRow label="Account">
                            <span class="text-body text-primary dark:text-primary">{choice}</span>
                        </InfoRow>
                        <InfoRow label="Search">
                            <span class="text-body text-primary dark:text-primary">{query}</span>
                        </InfoRow>
                        </InfoGroup>
                    </div>
                </Show>
                <Show when=move || settings.get()>
                    <div class="flex flex-col h-full overflow-y-auto bg-subtle p-4 gap-4 w-[0px] bp1440:w-[280px] bp1440:min-w-[260px] bp1440:max-w-[360px] dark:bg-subtle">
                        <div class="flex flex-col gap-1">
                            <span class="text-caption text-secondary dark:text-secondary">"Inspector"</span>
                            <span class="text-section text-primary dark:text-primary">"Settings"</span>
                        </div>
                        <div class="h-[1px] bg-divider dark:bg-divider"></div>
                        <InfoGroup title="Display">
                        <InfoRow label="View">
                            <Show when=move || day.get()>
                                <span class="text-body text-primary dark:text-primary">"Day"</span>
                            </Show><Show when=move || week.get()>
                                <span class="text-body text-primary dark:text-primary">"Week"</span>
                            </Show><Show when=move || month.get()>
                                <span class="text-body text-primary dark:text-primary">"Month"</span>
                            </Show>
                        </InfoRow>
                        <InfoRow label="Density">
                            <Show when=move || density_compact.get()>
                                <span class="text-body text-primary dark:text-primary">"Compact"</span>
                            </Show><Show when=move || density_std.get()>
                                <span class="text-body text-primary dark:text-primary">"Standard"</span>
                            </Show><Show when=move || density_touch.get()>
                                <span class="text-body text-primary dark:text-primary">"Touch"</span>
                            </Show>
                        </InfoRow>
                        <InfoRow label="Level">
                            <span class="text-body text-primary dark:text-primary">{level}</span>
                        </InfoRow>
                        <InfoRow label="Copies">
                            <span class="text-body text-primary dark:text-primary">{step}</span>
                        </InfoRow>
                        </InfoGroup>
                        <div class="h-[1px] bg-divider dark:bg-divider"></div>
                        <InfoGroup title="Sync">
                        <InfoRow label="Sync">
                            <Show when=move || switched.get()>
                                <span class="text-body text-primary dark:text-primary">"On"</span>
                            </Show><Show when=move || switch_off.get()>
                                <span class="text-body text-primary dark:text-primary">"Off"</span>
                            </Show>
                        </InfoRow>
                        <InfoRow label="Menu bar">
                            <Show when=move || checked.get()>
                                <span class="text-body text-primary dark:text-primary">"Shown"</span>
                            </Show><Show when=move || check_off.get()>
                                <span class="text-body text-primary dark:text-primary">"Hidden"</span>
                            </Show>
                        </InfoRow>
                        <InfoRow label="Account">
                            <span class="text-body text-primary dark:text-primary">{choice}</span>
                        </InfoRow>
                        </InfoGroup>
                    </div>
                </Show>
            </div>
            <Show when=move || toast.get()>
                <Overlay>
                    <Toast message=toast_msg on:click=move |_| { set_toast.set(false); } />
                </Overlay>
            </Show>
            <Show when=move || nav_open.get()>
                <Panel class="absolute left-[8px] top-[64px] z-50 gap-1 w-[224px] p-2">
                    <MenuItem on:click=move |_| { set_notes.set(true); set_table.set(false); set_settings.set(false); set_gallery.set(false); set_nav_open.set(false); }>"Notes"</MenuItem>
                    <MenuItem on:click=move |_| { set_notes.set(false); set_table.set(true); set_settings.set(false); set_gallery.set(false); set_nav_open.set(false); }>"Table"</MenuItem>
                    <MenuItem on:click=move |_| { set_notes.set(false); set_table.set(false); set_settings.set(true); set_gallery.set(false); set_nav_open.set(false); }>"Settings"</MenuItem>
                    <MenuItem on:click=move |_| { set_notes.set(false); set_table.set(false); set_settings.set(false); set_gallery.set(true); set_nav_open.set(false); }>"Gallery"</MenuItem>
                </Panel>
            </Show>
            <Portal>
                <Show when=move || sheet.get()>
                    <div class="absolute inset-0">
                        <div class="absolute inset-0 z-40 bg-scrim dark:bg-scrim"></div>
                        <div class="absolute inset-0 z-50 flex flex-col items-center justify-center p-6">
                            <Panel class="gap-4 w-[480px] max-w-full p-6">
                                <span class="text-section text-primary dark:text-primary">"New note"</span>
                                <div class="flex flex-col gap-2 w-full max-h-[240px] overflow-y-auto">
                                    <span class="text-body text-secondary w-full dark:text-secondary">"Saved on this Mac. You can find it in the library after it lands."</span>
                                    <span class="text-body text-secondary w-full dark:text-secondary">"The cabinet paint is still the sample from last week. Leave a note for the joiner and keep the island where it is."</span>
                                    <span class="text-body text-secondary w-full dark:text-secondary">"If this dialog is taller than the window, this body scrolls and the actions stay put."</span>
                                </div>
                                <input widget="QInput" class="w-full" placeholder="Title" />
                                <div class="flex flex-row gap-2">
                                    <Button on:click=move |_| { set_sheet.set(false); set_toast_msg.set("Note saved".to_string()); set_toast.set(true); }>"Save"</Button>
                                    <Button variant="secondary" on:click=move |_| { set_sheet.set(false); }>"Cancel"</Button>
                                </div>
                            </Panel>
                        </div>
                    </div>
                </Show>
                <Show when=move || alert.get()>
                    <div class="absolute inset-0">
                        <div class="absolute inset-0 z-40 bg-scrim dark:bg-scrim"></div>
                        <div class="absolute inset-0 z-50 flex flex-col items-center justify-center p-6">
                            <Panel class="gap-4 w-[400px] max-w-full p-6">
                                <span class="text-section text-primary dark:text-primary">"Delete this note?"</span>
                                <span class="text-body text-secondary w-full dark:text-secondary">"The note moves to Recently Deleted. You can recover it for 30 days."</span>
                                <div class="flex flex-row gap-2">
                                    <Button variant="danger" on:click=move |_| { set_alert.set(false); set_toast_msg.set("Moved to Recently Deleted".to_string()); set_toast.set(true); }>"Delete"</Button>
                                    <Button variant="secondary" on:click=move |_| { set_alert.set(false); }>"Cancel"</Button>
                                </div>
                            </Panel>
                        </div>
                    </div>
                </Show>
            </Portal>
        </div>
    }
}
