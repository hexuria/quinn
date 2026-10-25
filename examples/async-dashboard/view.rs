use quinn::prelude::*;

#[component]
fn Card(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        <div class="p-2 rounded-lg bg-white shadow-md flex flex-col gap-1">
            <span class="font-semibold text-sm">{title}</span>
            {children()}
        </div>
    }
}

#[component]
fn Badge() -> impl IntoView {
    let tone = use_context("tone");
    view! {
        <span class="text-xs font-medium dark:text-stone-100">{tone}</span>
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
        <div class="flex flex-col gap-2 p-4 bg-stone-100 dark:bg-stone-900 h-full">
            <div class="flex flex-row flex-wrap items-center gap-2">
                <span class="text-lg font-semibold dark:text-stone-100">"Async dashboard"</span>
                <button class="px-2 py-1 focus:bg-amber-100" on:click=move |_| { toggle_theme(); }>"Theme"</button>
                <button class="px-2 py-1" on:click=move |_| { set_open.set(true); }>"Open modal"</button>
                <button class="px-2 py-1" on:click=move |_| { save.dispatch("Cebu"); }>"Save Cebu"</button>
                <button class="px-2 py-1" on:click=move |_| { save.dispatch("Davao"); }>"Save Davao"</button>
                <button class="px-2 py-1" on:click=move |_| { set_city.set("Cebu"); }>"Refetch"</button>
                <button class="px-2 py-1" on:click=move |_| { note.refetch(); }>"Reload"</button>
                <button class="px-2 py-1" on:click=move |_| { race.dispatch("Cebu"); }>"Race Cebu"</button>
                <button class="px-2 py-1" on:click=move |_| { race.dispatch("Davao"); }>"Race Davao"</button>
                <button class="px-2 py-1" on:click=move |_| { set_page.set("2"); }>"Page 2"</button>
                <button class="px-2 py-1" on:click=move |_| { stamp.dispatch(Place { city: city.get(), note: "desk" }); }>"Stamp"</button>
                <span class="text-sm dark:text-stone-100">"Tone " {tone}</span>
                <Shade />
            </div>
            <div class="flex flex-col md:flex-row gap-3">
                <div class="h-[72px] w-full md:w-[180px] rounded bg-amber-100">
                    <span class="text-sm font-semibold">"Sidebar"</span>
                </div>
                <div class="flex flex-col gap-2">
                    <Suspense fallback=|| view! { <span class="text-sm dark:text-stone-100">"Loading the note"</span> }>
                        <span class="text-sm dark:text-stone-100">"Note: " {note}</span>
                    </Suspense>
                    <Transition fallback=|| view! { <span class="text-sm dark:text-stone-100">"Refreshing"</span> }>
                        <span class="text-sm dark:text-stone-100">"Place: " {place}</span>
                    </Transition>
                    <ErrorBoundary fallback=|| view! { <span class="text-sm dark:text-stone-100">"Could not load the feed"</span> }>
                        <span class="text-sm dark:text-stone-100">"Feed: " {feed}</span>
                    </ErrorBoundary>
                    <ErrorBoundary fallback=|| view! { <span class="text-sm dark:text-stone-100">"The job panicked"</span> }>
                        <span class="text-sm dark:text-stone-100">"Boom: " {boom}</span>
                    </ErrorBoundary>
                    <Show when=move || save.pending()>
                        <span class="text-sm font-medium dark:text-stone-100">"Saving"</span>
                    </Show>
                    <span class="text-sm dark:text-stone-100">"Action: " {save}</span>
                    <span class="text-sm dark:text-stone-100">{move || save.version()}</span>
                    <Show when=move || race.pending()>
                        <span class="text-sm font-medium dark:text-stone-100">"Racing"</span>
                    </Show>
                    <span class="text-sm dark:text-stone-100">"Race: " {race}</span>
                    <Suspense fallback=|| view! { <span class="text-sm dark:text-stone-100">"Loading cities"</span> }>
                        <For each=move || cities.get() key=|city| city.name>
                            <span class="text-sm dark:text-stone-100">{city.name}</span>
                        </For>
                    </Suspense>
                    <Suspense fallback=|| view! { <span class="text-sm dark:text-stone-100">"Loading the pair"</span> }>
                        <span class="text-sm dark:text-stone-100">"Pair: " {pair}</span>
                    </Suspense>
                    <span class="text-sm dark:text-stone-100">"Stamp: " {stamp}</span>
                    <span class="text-sm dark:text-stone-100">"Cancelled: " {move || cancelled_count()}</span>
                    <Card title="Ada".into()>
                        <div class="h-[48px] rounded bg-stone-100">
                            <div class="absolute left-2 top-1 z-10">
                                <span class="text-xs font-medium">"Pinned in the card"</span>
                            </div>
                        </div>
                    </Card>
                    <div class="h-[40px] w-[160px] rounded bg-amber-100 hover:bg-amber-600">
                        <span class="text-sm">"Hover me"</span>
                    </div>
                </div>
            </div>
            <Portal>
                <Show when=move || open.get()>
                    <div class="absolute inset-0">
                        <div class="absolute inset-0 z-40 bg-[#1c191780] hover:bg-[#1c191780]">
                            <span class="text-sm text-stone-100">"Scrim"</span>
                        </div>
                        <div class="absolute left-8 top-8 z-50 w-[280px] p-4 rounded-lg bg-white shadow-lg dark:bg-stone-900">
                            <span class="text-lg font-semibold dark:text-stone-100">"Modal"</span>
                            <button class="px-2 py-1" on:click=move |_| { set_open.set(false); }>"Close"</button>
                        </div>
                    </div>
                </Show>
            </Portal>
        </div>
    }
}
