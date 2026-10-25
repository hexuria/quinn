use quinn::makepad_widgets::*;

app_main!(App, font_assets: [makepad_platform::INTER_FONT_ASSET]);

#[derive(Script, ScriptHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
}

impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
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
            "Form",
            1440,
            900,
            component,
        )
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::LiveEdit = event {
            if matches!(cx.live_edit_apply(), Apply::Reload) {
                eprintln!(
                    "script_reload pid={} window_stays=1 render=1",
                    std::process::id()
                );
                script_eval!(cx, {
                    ui.host.render()
                    true
                });
            }
        }
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
        quinn::drive_shell(cx, event);
    }
}
