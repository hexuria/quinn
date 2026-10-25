use quinn::makepad_widgets::*;
use quinn::prelude::*;

mod ops;
mod seed;
mod ui_script;

/// Authoring source. The body is lowered to `ui_script.rs` and is not mounted
/// as a Rust view. Edits here hot-reload through that script when the binary
/// was started with `--hot`.
#[allow(dead_code)]
#[component]
fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);
    quinn::set_frame_driver(move || {
        set_count.update(|n| *n += 1);
    });
    quinn::set_inspector(move || count.get_untracked().to_string());

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
        quinn::set_frame_driver(|| {
            quinn::with_current_cx(|cx| ops::bump(cx));
        });
        quinn::set_inspector(|| quinn::with_current_cx(|cx| ops::read_count(cx)));
        quinn::boot_script(cx, &self.ui);
        if std::env::args().any(|arg| arg == "--hot") {
            quinn_lower::spawn_watch(
                concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"),
                concat!(env!("CARGO_MANIFEST_DIR"), "/src/ui_script.rs"),
                quinn_lower::Shell {
                    title: "Counter",
                    width: 480,
                    height: 360,
                },
            );
        }
    }
}

impl AppMain for App {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        quinn::makepad_widgets::script_mod(vm);
        seed::seed(vm);
        ui_script::script_mod(vm)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        // Apply::Reload already walked the new template. render() runs the
        // new on_render, so text and children follow the script. ScriptReapply
        // would keep the label string the widget already has.
        if let Event::LiveEdit = event {
            if matches!(cx.live_edit_apply(), Apply::Reload) {
                eprintln!(
                    "script_reload pid={} window_stays=1 render=1",
                    std::process::id()
                );
                ops::rerender(cx);
            }
        }
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}
