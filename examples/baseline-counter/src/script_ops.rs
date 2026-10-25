//! `script_eval!` bodies are ScriptMods tagged with this file.
//!
//! Makepad's `--hot` matcher counts every non-`__script_source__` body whose
//! `file` is the edited path, and requires the same number of `script_mod!`
//! blocks in that file. Keeping these evals out of `main.rs` leaves that
//! file with one `script_mod!` and one compiled site.

use makepad_widgets::*;

pub fn seed_state(vm: &mut ScriptVm) {
    script_eval!(vm, { mod.state = { counter: 0 } true });
}

pub fn increment(cx: &mut Cx) {
    script_eval!(cx, {
        mod.state.counter += 1
        ui.main_view.render()
    });
}
