//! Opt-in automation for visual checks on a desktop that someone is using.
//!
//! `QUINN_CONTROL=/path/to/file` makes the shell poll that file four times a
//! second. Each line is one command; the file is consumed and
//! `/path/to/file.ack` gets the number of commands applied. Nothing here runs
//! unless the variable is set.
//!
//! ```text
//! size 1024 768        resize the window's inner size
//! pos 0 40             move the window
//! state <statements>   run script statements with `state` bound, then render
//! key Tab shift        deliver KeyDown + KeyUp to the widget tree
//! focus level_slider   give a named widget the key focus
//! scroll 700 400 600   deliver a wheel scroll of 600 at window point (700, 400)
//! ```
//!
//! Keys go through `WidgetRef::handle_event`, the same path a real key takes
//! after the platform layer, so Tab traversal, Escape, and slider keys are
//! exercised without the host's real keyboard.

use std::cell::Cell;

use makepad_widgets::makepad_platform::event::{ScrollEvent, ScrollPhase};
use makepad_widgets::*;

std::thread_local! {
    static TIMER: Cell<Option<Timer>> = const { Cell::new(None) };
    static APPLIED: Cell<u64> = const { Cell::new(0) };
}

fn control_path() -> Option<String> {
    std::env::var("QUINN_CONTROL").ok().filter(|path| !path.is_empty())
}

/// Call from `handle_event`, after the widget tree has seen the event.
pub fn drive_control(cx: &mut Cx, event: &Event, ui: &WidgetRef) {
    let Some(path) = control_path() else {
        return;
    };
    match event {
        Event::Startup => {
            TIMER.with(|slot| slot.set(Some(cx.start_interval(0.25))));
        }
        Event::Timer(te) => {
            let ours = TIMER.with(|slot| slot.get());
            if ours.is_some_and(|timer| timer.is_timer(te).is_some()) {
                poll(cx, &path, ui);
            }
        }
        _ => {}
    }
}

fn poll(cx: &mut Cx, path: &str, ui: &WidgetRef) {
    let taken = format!("{path}.taking");
    if std::fs::rename(path, &taken).is_err() {
        return;
    }
    let text = std::fs::read_to_string(&taken).unwrap_or_default();
    let _ = std::fs::remove_file(&taken);
    let mut count = 0u64;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        apply(cx, line, ui);
        count += 1;
    }
    cx.redraw_all();
    let total = APPLIED.with(|slot| {
        let next = slot.get() + count;
        slot.set(next);
        next
    });
    let _ = std::fs::write(format!("{path}.ack"), total.to_string());
}

fn apply(cx: &mut Cx, line: &str, ui: &WidgetRef) {
    let (head, rest) = line.split_once(' ').unwrap_or((line, ""));
    match head {
        "size" | "pos" => {
            let nums: Vec<f64> = rest
                .split_whitespace()
                .filter_map(|n| n.parse().ok())
                .collect();
            if nums.len() != 2 {
                eprintln!("quinn control: `{line}` needs two numbers");
                return;
            }
            let window = ui.window(cx, ids!(main_window));
            if head == "size" {
                window.resize(cx, dvec2(nums[0], nums[1]));
            } else {
                window.reposition(cx, dvec2(nums[0], nums[1]));
            }
        }
        "state" => {
            crate::host::eval_statements(cx, rest);
            script_eval!(cx, {
                ui.host.render()
                true
            });
        }
        "key" => {
            let mut parts = rest.split_whitespace();
            let Some(name) = parts.next() else {
                return;
            };
            let Some(key_code) = key_code(name) else {
                eprintln!("quinn control: unknown key `{name}`");
                return;
            };
            let modifiers = KeyModifiers {
                shift: parts.clone().any(|part| part == "shift"),
                ..Default::default()
            };
            let key = KeyEvent {
                key_code,
                modifiers,
                ..Default::default()
            };
            ui.handle_event(cx, &Event::KeyDown(key.clone()), &mut Scope::empty());
            crate::host::forward_key(cx, &key);
            ui.handle_event(cx, &Event::KeyUp(key), &mut Scope::empty());
        }
        "scroll" => {
            let nums: Vec<f64> = rest
                .split_whitespace()
                .filter_map(|n| n.parse().ok())
                .collect();
            let window = ui.window(cx, ids!(main_window));
            let (Some(window_id), [x, y, dy]) = (window.window_id(), nums.as_slice()) else {
                eprintln!("quinn control: `{line}` needs x y dy");
                return;
            };
            let scroll = ScrollEvent {
                window_id,
                scroll: dvec2(0.0, *dy),
                abs: dvec2(*x, *y),
                modifiers: KeyModifiers::default(),
                handled_x: Cell::new(false),
                handled_y: Cell::new(false),
                is_mouse: true,
                time: 0.0,
                phase: ScrollPhase::None,
            };
            ui.handle_event(cx, &Event::Scroll(scroll), &mut Scope::empty());
        }
        "focus" => {
            let id = LiveId::from_str(rest.trim());
            let widget = ui.widget(cx, &[id]);
            if widget.is_empty() {
                eprintln!("quinn control: no widget named `{}`", rest.trim());
            }
            widget.set_key_focus(cx);
        }
        other => eprintln!("quinn control: unknown command `{other}`"),
    }
}

fn key_code(name: &str) -> Option<KeyCode> {
    Some(match name {
        "Tab" => KeyCode::Tab,
        "Escape" => KeyCode::Escape,
        "Left" => KeyCode::ArrowLeft,
        "Right" => KeyCode::ArrowRight,
        "Up" => KeyCode::ArrowUp,
        "Down" => KeyCode::ArrowDown,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "Space" => KeyCode::Space,
        "Return" => KeyCode::ReturnKey,
        _ => return None,
    })
}
