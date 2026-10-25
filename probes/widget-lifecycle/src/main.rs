//! Probe A. Eight widget-tree operations against Makepad's public types,
//! logged to `QUINN_LOG` (default `artifacts/probe-widget-lifecycle.log`).
//!
//! `QUINN_PROBE_EXIT=1` quits after the log is written. Leave it unset to keep
//! the window up for a click on Increment.

pub use makepad_widgets;

use makepad_widgets::makepad_script::ScriptMod;
use makepad_widgets::*;
use std::cell::RefCell;
use std::fs;
use std::rc::Rc;

app_main!(App);

script_mod! {
    use mod.prelude.widgets.*
    startup() do #(App::script_component(vm)) {
        ui: Root {
            main_window := Window {
                window.inner_size: vec2(520, 360)
                window.title: "Widget lifecycle probe"
                body +: {
                    host := RoundedView {
                        width: Fill
                        height: Fill
                        flow: Down
                        padding: 16.
                        spacing: 8.
                        show_bg: true
                        new_batch: true
                        draw_bg.color: #xf4f0e6
                    }
                }
            }
        }
    }
}

#[derive(Script, ScriptHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    probe: Probe,
}

struct Probe {
    log: Vec<String>,
    clicks: Rc<RefCell<u32>>,
    handler: Option<Box<dyn FnMut()>>,
    status: WidgetRef,
    counter: WidgetRef,
    button: WidgetRef,
    window_uid: u64,
}

impl Default for Probe {
    fn default() -> Self {
        Self {
            log: Vec::new(),
            clicks: Rc::new(RefCell::new(0)),
            handler: None,
            status: WidgetRef::empty(),
            counter: WidgetRef::empty(),
            button: WidgetRef::empty(),
            window_uid: 0,
        }
    }
}

impl Probe {
    fn note(&mut self, line: impl Into<String>) {
        let line = line.into();
        eprintln!("probe: {line}");
        self.log.push(line);
    }

    fn run(&mut self, cx: &mut Cx, ui: &WidgetRef, host: &WidgetRef) {
        let window_uid = ui.window(cx, ids!(main_window)).widget_uid().0;
        self.window_uid = window_uid;
        self.note(format!("window_uid {window_uid}"));

        let first = spawn(
            cx,
            r#"mod.widgets.Label{text:"first" width:Fill height:Fit draw_text.color:#x1c1917 draw_text.text_style.font_size:18}"#,
        );
        let first_uid = first.widget_uid().0;
        self.note(format!(
            "1 create uid={first_uid} empty={}",
            first.is_empty()
        ));
        insert(cx, host, &first);
        self.note(format!("2 insert parent_children={}", child_count(host)));

        first.set_text(cx, "updated without rebuild");
        let after_text = first.widget_uid().0;
        self.note(format!(
            "3 set_text same_uid={} text={:?}",
            after_text == first_uid,
            first.text()
        ));

        let second = spawn(
            cx,
            r#"mod.widgets.Label{text:"second" width:Fill height:Fit draw_text.color:#x1c1917 draw_text.text_style.font_size:18}"#,
        );
        let second_uid = second.widget_uid().0;
        insert(cx, host, &second);
        self.note(format!("4 second uid={second_uid}"));

        remove_child(host, first_uid);
        drop(first);
        let still_there = child_uids(host).contains(&first_uid);
        self.note(format!("5 removed first still_in_parent={still_there}"));

        let third = spawn(
            cx,
            r#"mod.widgets.Label{text:"third" width:Fill height:Fit draw_text.color:#x1c1917 draw_text.text_style.font_size:18}"#,
        );
        let third_uid = third.widget_uid().0;
        insert(cx, host, &third);
        reorder(cx, host, &[third_uid, second_uid]);
        let order = child_uids(host);
        self.note(format!(
            "6 order={order:?} second_uid_stable={}",
            order.contains(&second_uid)
        ));

        let clicks = self.clicks.clone();
        self.handler = Some(Box::new(move || {
            *clicks.borrow_mut() += 1;
        }));
        self.fire();
        let once = *self.clicks.borrow();
        self.handler = None;
        self.fire();
        let twice = *self.clicks.borrow();
        self.note(format!("7 handler once={once} after_remove={twice}"));

        let before = self.window_uid;
        cx.redraw_all();
        host.redraw(cx);
        let after = ui.window(cx, ids!(main_window)).widget_uid().0;
        self.note(format!(
            "8 redraw same_window={} before={before} after={after}",
            before == after && before != 0
        ));

        let unchanged = second_uid;
        second.set_text(cx, "second (identity held)");
        self.note(format!(
            "identity unchanged_sibling={} text_now={:?}",
            second.widget_uid().0 == unchanged,
            second.text()
        ));

        self.status = spawn(
            cx,
            r#"mod.widgets.H1{text:"Probe" width:Fill height:Fit draw_text.color:#x1c1917 draw_text.text_style.font_size:28}"#,
        );
        self.counter = spawn(
            cx,
            r#"mod.widgets.Label{text:"Clicks: 0" width:Fill height:Fit draw_text.color:#x1c1917 draw_text.text_style.font_size:20}"#,
        );
        self.button = spawn(
            cx,
            r#"mod.widgets.Button{text:"Increment" width:Fit height:Fit draw_bg.color:#x1c1917 draw_bg.color_hover:#x1c1917 draw_bg.color_down:#x1c1917 draw_text.color:#xf6f3ec draw_text.color_hover:#xf6f3ec draw_text.color_down:#xf6f3ec}"#,
        );
        clear_children(host);
        insert(cx, host, &self.status);
        insert(cx, host, &self.counter);
        insert(cx, host, &self.button);
        let ok = self.passed();
        let summary = if ok { "Probe passed" } else { "Probe failed" };
        self.status.set_text(cx, summary);
        self.note(format!("visible_status {summary}"));
        cx.redraw_all();
        self.flush();
    }

    fn fire(&mut self) {
        if let Some(handler) = self.handler.as_mut() {
            handler();
        }
    }

    fn passed(&self) -> bool {
        self.log.iter().any(|l| l.contains("same_uid=true"))
            && self.log.iter().any(|l| l.contains("still_in_parent=false"))
            && self.log.iter().any(|l| l.contains("once=1 after_remove=1"))
            && self.log.iter().any(|l| l.contains("same_window=true"))
            && self
                .log
                .iter()
                .any(|l| l.contains("unchanged_sibling=true"))
            && self.log.iter().all(|l| !l.contains("empty=true"))
    }

    fn flush(&self) {
        let path = std::env::var("QUINN_LOG")
            .unwrap_or_else(|_| "artifacts/probe-widget-lifecycle.log".into());
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut body = self.log.join("\n");
        body.push('\n');
        body.push_str(if self.passed() {
            "PROBE_OK\n"
        } else {
            "PROBE_FAIL\n"
        });
        let _ = fs::write(&path, &body);
        let _ = fs::write("/opt/cursor/artifacts/probe-widget-lifecycle.log", &body);
        let _ = fs::write("/tmp/quinn-probe.log", &body);
    }
}

impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        let host = self.ui.view(cx, ids!(host));
        self.probe.run(cx, &self.ui, &host);
        if std::env::var("QUINN_PROBE_EXIT").ok().as_deref() == Some("1") {
            if !self.probe.passed() {
                eprintln!("PROBE_FAIL");
                std::process::exit(1);
            }
            cx.quit();
        }
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        if self.probe.button.as_button().clicked(actions) {
            let next = {
                let mut n = self.probe.clicks.borrow_mut();
                *n += 1;
                *n
            };
            self.probe.counter.set_text(cx, &format!("Clicks: {next}"));
            self.probe.counter.redraw(cx);
            cx.redraw_all();
        }
    }
}

impl AppMain for App {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        crate::makepad_widgets::script_mod(vm);
        self::script_mod(vm)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}

fn spawn(cx: &mut Cx, code: &str) -> WidgetRef {
    cx.with_vm(|vm| {
        let script = ScriptMod {
            cargo_manifest_path: String::new(),
            module_path: "probe".into(),
            file: "probe.script".into(),
            line: 1,
            column: 1,
            code: format!("use mod.prelude.widgets.*\n{code};"),
            values: Vec::new(),
        };
        let value = vm.eval(script);
        WidgetRef::script_from_value(vm, value)
    })
}

fn insert(cx: &mut Cx, parent: &WidgetRef, child: &WidgetRef) {
    let uid = child.widget_uid();
    if let Some(mut view) = parent.as_view().borrow_mut() {
        view.children.push((LiveId(uid.0), child.clone()));
    }
    cx.widget_tree_insert_child(parent.widget_uid(), LiveId(uid.0), child.clone());
}

fn remove_child(parent: &WidgetRef, uid: u64) {
    if let Some(mut view) = parent.as_view().borrow_mut() {
        view.children
            .retain(|(name, widget)| name.0 != uid && widget.widget_uid().0 != uid);
    }
}

fn clear_children(parent: &WidgetRef) {
    if let Some(mut view) = parent.as_view().borrow_mut() {
        view.children.clear();
    }
}

fn reorder(cx: &mut Cx, parent: &WidgetRef, order: &[u64]) {
    if let Some(mut view) = parent.as_view().borrow_mut() {
        let existing: Vec<(LiveId, WidgetRef)> = view.children.iter().cloned().collect();
        view.children.clear();
        for uid in order {
            if let Some((_, widget)) = existing
                .iter()
                .find(|(name, widget)| name.0 == *uid || widget.widget_uid().0 == *uid)
            {
                view.children.push((LiveId(*uid), widget.clone()));
            }
        }
    }
    let _ = cx;
}

fn child_count(parent: &WidgetRef) -> usize {
    parent
        .as_view()
        .borrow()
        .map(|view| view.children.len())
        .unwrap_or(0)
}

fn child_uids(parent: &WidgetRef) -> Vec<u64> {
    parent
        .as_view()
        .borrow()
        .map(|view| {
            view.children
                .iter()
                .map(|(_, w)| w.widget_uid().0)
                .collect()
        })
        .unwrap_or_default()
}
