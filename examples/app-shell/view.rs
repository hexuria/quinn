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
        <div class="p-4 rounded-window bg-system-background shadow-control flex flex-col gap-1 dark:bg-system-background">
            <span class="text-headline text-label dark:text-label">{title}</span>
            <span class="text-caption text-label-secondary dark:text-label-secondary">"tone " {tone}</span>
            {children()}
        </div>
    }
}

#[component]
fn Note(children: ChildrenFn) -> impl IntoView {
    view! {
        <div class="px-3 py-2 bg-system-fill rounded-control dark:bg-system-fill">
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
        <div class="flex flex-col h-full bg-window-background dark:bg-window-background">
            <div class="flex flex-row items-center h-[52px] px-4 gap-2">
                <A href="/" class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator">"Home"</A>
                <A href="/board" class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator">"Board"</A>
                <A href="/form" class="px-3 h-control-large rounded-control text-body bg-control-background text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator">"Form"</A>
                <div class="flex-1"></div>
                <span class="text-callout text-label-secondary dark:text-label-secondary">"Accent: " {accent}</span>
            </div>
            <div class="h-[1px] bg-separator dark:bg-separator"></div>
            <div class="flex flex-row items-center gap-3 px-4 py-3">
                <input class="flex-1 h-control-large rounded-control bg-control-background text-body text-label border border-separator dark:bg-control-background dark:text-label dark:border-separator" bind:value=name placeholder="Your name" />
                <span class="text-body text-label dark:text-label">"Hello, " {name}</span>
            </div>
            <div class="flex flex-col px-4 pb-4 gap-3">
                <Router>
                    <Routes>
                        <Route path="/">
                            <div class="flex flex-col gap-3">
                                <>
                                    <span class="text-body text-label dark:text-label">"One"</span>
                                    <span class="text-body text-label dark:text-label">"Two"</span>
                                </>
                                <Card title="Today">
                                    <span class="text-body text-label dark:text-label leading-snug">"Home card"</span>
                                </Card>
                                <Note><span class="text-body text-label dark:text-label">{arg}</span></Note>
                            </div>
                        </Route>
                        <Route path="/board">
                            <div class="flex flex-col gap-3">
                                <div class="grid grid-cols-3 gap-2">
                                    <div class="col-span-2 h-[88px] rounded-window bg-gradient-to-r from-system-blue to-system-indigo shadow-window">
                                        <span class="text-title3 text-white font-bold">"Wide"</span>
                                    </div>
                                    <div class="h-[88px] rounded-window bg-system-fill shadow-control dark:bg-system-fill">
                                        <span class="text-headline text-label dark:text-label">"Side"</span>
                                    </div>
                                </div>
                                <div class="h-[32px]">
                                    <div class="absolute left-2 top-1">
                                        <span class="text-caption text-label-secondary dark:text-label-secondary font-medium">"Pinned"</span>
                                    </div>
                                </div>
                                <Card title="Board" tone="amber">
                                    <span class="text-body text-label dark:text-label leading-snug">"Studio card"</span>
                                </Card>
                                <div class="h-[72px] overflow-y-auto overflow-hidden bg-system-background rounded-window dark:bg-system-background">
                                    <div class="flex flex-col px-3 py-2">
                                        <span class="text-body text-label dark:text-label leading-relaxed">"First line of the note."</span>
                                        <span class="text-body text-label dark:text-label leading-relaxed">"Second line, still in the box."</span>
                                        <span class="text-body text-label dark:text-label leading-relaxed">"Third line, past the clip."</span>
                                        <span class="text-body text-label dark:text-label leading-relaxed">"Fourth line, the bar can move."</span>
                                    </div>
                                </div>
                            </div>
                        </Route>
                        <Route path="/form">
                            <div class="flex flex-col gap-2 p-4 rounded-window bg-system-background shadow-control dark:bg-system-background">
                                <span class="text-headline text-label dark:text-label">"City: " {profile.city}</span>
                                <span class="text-body text-label-secondary dark:text-label-secondary">"Stored name: " {profile.name}</span>
                                <button class="px-3 h-control-large rounded-control text-body bg-system-blue text-white dark:bg-system-blue" on:click=move |_| {
                                    set_profile.update(|p| { p.city = "Cebu".into(); });
                                }>"Move to Cebu"</button>
                            </div>
                        </Route>
                    </Routes>
                </Router>
            </div>
        </div>
    }
}
