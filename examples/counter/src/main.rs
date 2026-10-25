use quinn::makepad_widgets::*;

mod ops;

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
    }
}

impl AppMain for App {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        quinn::makepad_widgets::script_mod(vm);
        let component = Self::script_component(vm).script_to_value(vm);
        quinn::load_hot_view(
            vm,
            env!("CARGO_MANIFEST_DIR"),
            module_path!(),
            "Counter",
            480,
            360,
            component,
        )
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        // Apply::Reload already walked the new template. render() runs the
        // new on_render, so text and children follow the script.
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
