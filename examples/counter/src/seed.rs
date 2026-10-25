//! Initial `mod.state`. Kept out of `ui_script.rs` so the hot-reload matcher
//! sees one `script_mod!` and no `script_eval!` in the file it reloads.
//!
//! A reload runs inside `ScriptVm::with_reload`, so this seed does not run
//! again and `count` stays where the clicks left it.

use quinn::makepad_widgets::*;

pub fn seed(vm: &mut ScriptVm) {
    if !vm.is_reload() {
        script_eval!(vm, { mod.state = { count: 0 } true });
    }
}
