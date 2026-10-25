//! Window chrome as a real `script_mod!` block.
//!
//! `quinn::run!` evaluates its shell from a string. Makepad's `--hot` watcher
//! only reloads `script_mod!` blocks it can read back out of a Rust file, and
//! the string is not one of those blocks. This file is the block. It is the
//! only `script_mod!` here, which is what the matcher requires.

use quinn::makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets.*
    startup() do #(crate::App::script_component(vm)) {
        ui: Root {
            main_window := Window {
                window.inner_size: vec2(480, 360)
                window.title: "Counter"
                body +: {
                    host := RoundedView {
                        width: Fill
                        height: Fill
                        flow: Down
                        padding: 16.
                        show_bg: true
                        new_batch: true
                        draw_bg.color: #xf4f0e6
                    }
                }
            }
        }
    }
}
