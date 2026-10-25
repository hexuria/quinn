use quinn::makepad_widgets::*;
use quinn::prelude::*;

mod chrome;

#[component]
fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);
    let set_probe = set_count;
    let read_probe = count;
    quinn::set_frame_driver(move || {
        set_probe.update(|n| *n += 1);
    });
    quinn::set_inspector(move || read_probe.get_untracked().to_string());

    view! {
        <div class="flex flex-col gap-4 p-4">
            <h1>"Counter: " {move || count.get()}</h1>
            <button on:click=move |_| {
                set_count.update(|n| *n += 1);
            }>"Increment"</button>
        </div>
    }
}

app_main!(App);

#[derive(Script, ScriptHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
}

impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        quinn::startup(cx, &self.ui, || view! { <Counter /> });
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        quinn::on_actions(cx, actions);
    }
}

impl AppMain for App {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        quinn::makepad_widgets::script_mod(vm);
        chrome::script_mod(vm)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        // Disk hot reload stores the new script with Apply::Reload, which
        // does not walk draw fields. request_style_reload asks the next tick
        // to re-apply with ScriptReapply so the page color changes and the
        // Rust-mounted counter stays. That follow-up reports Rebake here.
        if let Event::LiveEdit = event {
            if matches!(cx.live_edit_apply(), Apply::Reload) {
                eprintln!(
                    "hot_reload_followup pid={} window_stays=1 request_style_reload=1",
                    std::process::id()
                );
                cx.request_style_reload();
            }
        }
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}
