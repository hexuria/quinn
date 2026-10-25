//! Script calls from Rust. This file is not the view source, so editing
//! `main.rs` does not make Makepad try to match a `script_eval!` here.

use quinn::makepad_widgets::*;

pub fn bump(cx: &mut Cx) {
    script_eval!(cx, {
        quinn_frame()
        true
    });
}

pub fn rerender(cx: &mut Cx) {
    script_eval!(cx, {
        ui.host.render()
        true
    });
}

pub fn read_count(cx: &mut Cx) -> String {
    let value = script_eval!(cx, { mod.state.count });
    if let Some(number) = value.as_f64() {
        if number.fract() == 0.0 {
            return format!("{}", number as i64);
        }
        return format!("{number}");
    }
    let mut out = String::new();
    cx.with_vm(|vm| {
        vm.bx.heap.cast_to_string(value, &mut out);
    });
    out
}
