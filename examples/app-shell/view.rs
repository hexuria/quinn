use quinn::prelude::*;

struct Profile {
    name: String,
    city: String,
}

#[component]
fn Card(
    #[prop(into)] title: String,
    #[prop(optional, default = "stone")] tone: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="p-2 rounded-lg bg-white shadow-md flex flex-col gap-1">
            <span class="font-semibold text-sm">{title}</span>
            <span class="text-xs">"tone " {tone}</span>
            {children()}
        </div>
    }
}

#[component]
fn Note(children: ChildrenFn) -> impl IntoView {
    view! {
        <div class="px-2 py-1 bg-amber-100 rounded">
            {children()("from-fn")}
        </div>
    }
}

#[component]
fn Shell() -> impl IntoView {
    provide_context("accent", "amber");
    let accent = use_context("accent");
    let (name, set_name) = signal(String::new());
    let (profile, set_profile) = signal(Profile {
        name: "Ada".into(),
        city: "Manila".into(),
    });

    view! {
        <div class="flex flex-col gap-2 p-3 bg-stone-100 h-full">
            <div class="flex flex-row items-center gap-2">
                <A href="/" class="px-2 py-1">"Home"</A>
                <A href="/board" class="px-2 py-1">"Board"</A>
                <A href="/form" class="px-2 py-1">"Form"</A>
                <span class="text-sm font-medium">"Accent: " {accent}</span>
            </div>
            <div class="flex flex-row items-center gap-2">
                <input class="flex-1" bind:value=name placeholder="Your name" />
                <span class="text-sm">"Hello, " {name}</span>
            </div>
            <Router>
                <Routes>
                    <Route path="/">
                        <div class="flex flex-col gap-2">
                            <>
                                <span class="text-sm">"One"</span>
                                <span class="text-sm">"Two"</span>
                            </>
                            <Card title="Today">
                                <span class="leading-snug">"Home card"</span>
                            </Card>
                            <Note><span class="text-sm">{arg}</span></Note>
                        </div>
                    </Route>
                    <Route path="/board">
                        <div class="flex flex-col gap-2">
                            <div class="grid grid-cols-3 gap-2">
                                <div class="col-span-2 h-[64px] rounded-lg bg-gradient-to-r from-amber-600 to-stone-900 shadow-lg">
                                    <span class="text-white font-bold">"Wide"</span>
                                </div>
                                <div class="h-[64px] rounded bg-amber-100 shadow">
                                    <span class="font-semibold">"Side"</span>
                                </div>
                            </div>
                            <div class="h-[28px]">
                                <div class="absolute left-4 top-[176px]">
                                    <span class="text-xs font-medium">"Pinned"</span>
                                </div>
                            </div>
                            <Card title="Board" tone="amber">
                                <span class="leading-snug">"Studio card"</span>
                            </Card>
                            <div class="h-[48px] overflow-y-auto overflow-hidden bg-white rounded">
                                <div class="flex flex-col px-2">
                                    <span class="leading-relaxed text-sm">"First line of the note."</span>
                                    <span class="leading-relaxed text-sm">"Second line, still in the box."</span>
                                    <span class="leading-relaxed text-sm">"Third line, past the clip."</span>
                                    <span class="leading-relaxed text-sm">"Fourth line, the bar can move."</span>
                                </div>
                            </div>
                        </div>
                    </Route>
                    <Route path="/form">
                        <div class="flex flex-col gap-2">
                            <span class="font-medium">"City: " {profile.city}</span>
                            <span class="text-sm">"Stored name: " {profile.name}</span>
                            <button class="px-3 py-1" on:click=move |_| {
                                set_profile.update(|p| { p.city = "Cebu".into(); });
                            }>"Move to Cebu"</button>
                        </div>
                    </Route>
                </Routes>
            </Router>
        </div>
    }
}
