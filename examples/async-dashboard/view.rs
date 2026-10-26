use quinn::prelude::*;

#[component]
fn Card(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        <div class="p-4 rounded-window bg-system-background shadow-control flex flex-col gap-2 dark:bg-system-background">
            <span class="text-headline text-label dark:text-label">{title}</span>
            {children()}
        </div>
    }
}

#[component]
fn Badge() -> impl IntoView {
    let tone = use_context("tone");
    view! {
        <span class="text-caption text-label dark:text-label">{tone}</span>
    }
}

#[component]
fn Shade() -> impl IntoView {
    provide_context("tone", "shade");
    view! {
        <Badge />
    }
}

#[component]
fn Dashboard() -> impl IntoView {
    provide_context("tone", "stone");
    let tone = use_context("tone");
    let note = LocalResource::new(|| async move { fetch_note().await });
    let feed = LocalResource::new(|| async move { fetch_feed().await });
    let cities = LocalResource::new(|| async move { fetch_cities().await });
    let boom = LocalResource::new(|| async move { boom().await });
    let (city, set_city) = signal("Manila".to_string());
    let place = Resource::new(|| city.get(), |city| async move { fetch_place(city).await });
    let save = Action::new(|place: &String| {
        let p = place.clone();
        async move { save_place(p).await }
    });
    let race = Action::new(|place: &String| {
        let p = place.clone();
        async move { race_place(p).await }
    });
    let (id, _set_id) = signal("ada".to_string());
    let (page, set_page) = signal("1".to_string());
    let pair = Resource::new(
        move || (id.get(), page.get()),
        |(id, page)| async move {
            let raw = fetch_pair(id, page).await?;
            decorate(raw).await
        },
    );
    let stamp = Action::new(|place: &String| async move { save_stamp(place).await });
    let (open, set_open) = signal(false);

    view! {
        <div class="flex flex-col h-full bg-window-background dark:bg-window-background">
            <div class="flex flex-row flex-wrap items-center min-h-[52px] px-4 py-2 gap-2">
                <span class="text-headline text-label dark:text-label">"Async dashboard"</span>
                <button class="px-3 h-control-large rounded-control text-body text-system-blue border-0 bg-transparent dark:text-system-blue" on:click=move |_| { toggle_theme(); }>"Theme"</button>
                <button class="px-3 h-control-large rounded-control text-body bg-system-blue text-white dark:bg-system-blue" on:click=move |_| { set_open.set(true); }>"Open modal"</button>
                <button class="px-3 h-control-large rounded-control text-body bg-system-blue text-white dark:bg-system-blue" on:click=move |_| { save.dispatch("Cebu"); }>"Save Cebu"</button>
                <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { save.dispatch("Davao"); }>"Save Davao"</button>
                <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_city.set("Cebu"); }>"Refetch"</button>
                <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { note.refetch(); }>"Reload"</button>
                <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { race.dispatch("Cebu"); }>"Race Cebu"</button>
                <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { race.dispatch("Davao"); }>"Race Davao"</button>
                <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { set_page.set("2"); }>"Page 2"</button>
                <button class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" on:click=move |_| { stamp.dispatch(Place { city: city.get(), note: "desk" }); }>"Stamp"</button>
                <span class="text-callout text-label-secondary dark:text-label-secondary">"Tone " {tone}</span>
                <Shade />
            </div>
            <div class="h-[1px] bg-separator dark:bg-separator"></div>
            <div class="flex flex-col md:flex-row flex-1 items-stretch">
                <div class="flex flex-col w-full md:w-[200px] md:h-full vibrancy p-3 gap-2">
                    <span class="text-subheadline text-label-secondary dark:text-label-secondary">"Library"</span>
                    <span class="text-headline text-label dark:text-label">"Sidebar"</span>
                </div>
                <div class="w-full md:w-[1px] h-[1px] md:h-full bg-separator dark:bg-separator"></div>
                <div class="flex flex-col gap-2 p-4 overflow-y-auto">
                    <Suspense fallback=|| view! { <span class="text-body text-label dark:text-label">"Loading the note"</span> }>
                        <span class="text-body text-label dark:text-label">"Note: " {note}</span>
                    </Suspense>
                    <Transition fallback=|| view! { <span class="text-body text-label dark:text-label">"Refreshing"</span> }>
                        <span class="text-body text-label dark:text-label">"Place: " {place}</span>
                    </Transition>
                    <ErrorBoundary fallback=|| view! { <span class="text-body text-label dark:text-label">"Could not load the feed"</span> }>
                        <span class="text-body text-label dark:text-label">"Feed: " {feed}</span>
                    </ErrorBoundary>
                    <ErrorBoundary fallback=|| view! { <span class="text-body text-label dark:text-label">"The job panicked"</span> }>
                        <span class="text-body text-label dark:text-label">"Boom: " {boom}</span>
                    </ErrorBoundary>
                    <Show when=move || save.pending()>
                        <span class="text-body text-label dark:text-label">"Saving"</span>
                    </Show>
                    <span class="text-body text-label dark:text-label">"Action: " {save}</span>
                    <span class="text-body text-label dark:text-label">{move || save.version()}</span>
                    <Show when=move || race.pending()>
                        <span class="text-body text-label dark:text-label">"Racing"</span>
                    </Show>
                    <span class="text-body text-label dark:text-label">"Race: " {race}</span>
                    <Suspense fallback=|| view! { <span class="text-body text-label dark:text-label">"Loading cities"</span> }>
                        <For each=move || cities.get() key=|city| city.name>
                            <span class="text-body text-label dark:text-label">{city.name}</span>
                        </For>
                    </Suspense>
                    <Suspense fallback=|| view! { <span class="text-body text-label dark:text-label">"Loading the pair"</span> }>
                        <span class="text-body text-label dark:text-label">"Pair: " {pair}</span>
                    </Suspense>
                    <span class="text-body text-label dark:text-label">"Stamp: " {stamp}</span>
                    <span class="text-body text-label dark:text-label">"Cancelled: " {move || cancelled_count()}</span>
                    <Card title="Ada".into()>
                        <div class="h-[48px] rounded-control bg-system-fill dark:bg-system-fill">
                            <div class="absolute left-2 top-1 z-10">
                                <span class="text-caption text-label dark:text-label">"Pinned in the card"</span>
                            </div>
                        </div>
                    </Card>
                    <div class="flex flex-row items-center h-[40px] w-[160px] rounded-control bg-system-fill hover:bg-system-blue dark:bg-system-fill">
                        <span class="text-body text-label dark:text-label">"Hover me"</span>
                    </div>
                </div>
            </div>
            <Portal>
                <Show when=move || open.get()>
                    <div class="absolute inset-0">
                        <div class="absolute inset-0 z-40 bg-[#1c191780] hover:bg-[#1c191780]">
                            <span class="text-body text-white">"Scrim"</span>
                        </div>
                        <div class="absolute left-8 top-8 z-50 w-[320px] p-4 rounded-sheet bg-system-background shadow-window dark:bg-system-background">
                            <span class="text-title3 text-label dark:text-label">"Modal"</span>
                            <button class="px-3 h-control-large rounded-control text-body bg-system-blue text-white dark:bg-system-blue" on:click=move |_| { set_open.set(false); }>"Close"</button>
                        </div>
                    </div>
                </Show>
            </Portal>
        </div>
    }
}
