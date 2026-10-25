pub use makepad_widgets;

use std::cell::RefCell;
use std::time::Instant;

use makepad_widgets::*;

mod script_ops;

app_main!(App);

script_mod! {
    use mod.prelude.widgets.*
    let state = mod.state
    startup() do #(App::script_component(vm)){
        ui: Root{
            on_startup:||{
                ui.main_view.render()
            }
            main_window := Window{
                window.inner_size: vec2(420, 280)
                window.title: "Makepad baseline counter"
                body +: {
                    main_view := View{
                        width: Fill
                        height: Fill
                        flow: Down
                        spacing: 12
                        padding: 16
                        align: Center
                        on_render: ||{
                            // Light ink on Makepad's dark window. The earlier
                            // #x1c1917 label sat in the title band because the
                            // button was a sibling after a Fill view.
                            counter_label := Label{
                                text: "Count: " + state.counter
                                draw_text.color: #xf6f3ec
                                draw_text.text_style.font_size: 24
                            }
                            increment_button := Button{
                                text: "Increment"
                                draw_bg.color: #x1c1917
                                draw_bg.color_hover: #x292524
                                draw_bg.color_down: #x44403c
                                draw_text.color: #xf6f3ec
                                draw_text.color_hover: #xf6f3ec
                                draw_text.color_down: #xf6f3ec
                            }
                        }
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
}

std::thread_local! {
    static UI: RefCell<Option<WidgetRef>> = const { RefCell::new(None) };
    static PROBE: RefCell<Probe> = RefCell::new(Probe {
        ready: false,
        pending: None,
        samples: Vec::new(),
        live_started: None,
        live_gen: 0,
        live_frames: 0,
    });
}

struct Probe {
    ready: bool,
    pending: Option<Instant>,
    samples: Vec<u128>,
    live_started: Option<Instant>,
    live_gen: u64,
    live_frames: u32,
}

impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        UI.with(|slot| *slot.borrow_mut() = Some(self.ui.clone()));
        run_widget_bench(cx, &self.ui);
        cx.post_draw_hook = Some(Box::new(|cx| on_frame(cx)));
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        if self.ui.button(cx, ids!(increment_button)).clicked(actions) {
            script_ops::increment(cx);
        }
    }
}

impl AppMain for App {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        crate::makepad_widgets::script_mod(vm);
        if !vm.is_reload() {
            script_ops::seed_state(vm);
        }
        self::script_mod(vm)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        // A disk edit arrives as Apply::Reload. That stores the new
        // script_mod override but does not walk draw fields of the existing
        // tree, so the window keeps the old color. The next tick's
        // request_style_reload re-applies with ScriptReapply. The follow-up
        // reports Rebake here, which stops the loop.
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

fn on_frame(cx: &mut Cx) {
    let latency_n = std::env::var("QUINN_LATENCY")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    let live = std::env::var("QUINN_LIVE").ok().as_deref() == Some("1");
    let exit = std::env::var("QUINN_EXIT").ok().as_deref() == Some("1");
    // `Instant::now` panics on Makepad's wasm target. Ready/exit runs do not need a clock.
    let now = (latency_n > 0 || live).then(Instant::now);
    let mut finished_latency = false;
    let mut finished_live = false;
    let mut arm_latency = false;
    let mut arm_live = false;

    PROBE.with(|slot| {
        let probe = &mut *slot.borrow_mut();
        if let Some(started) = probe.pending.take() {
            if let Some(now) = now {
                probe.samples.push(now.duration_since(started).as_micros());
            }
            if latency_n > 0 && probe.samples.len() as u32 >= latency_n {
                write_latency(&probe.samples);
                finished_latency = true;
            }
        }
        if !probe.ready {
            if let Ok(path) = std::env::var("QUINN_READY") {
                let _ = std::fs::write(path, "ready\n");
            }
            probe.ready = true;
            if latency_n > 0 {
                arm_latency = true;
            } else if live {
                arm_live = true;
            }
        } else if latency_n > 0 && (probe.samples.len() as u32) < latency_n {
            arm_latency = true;
        }
        if probe.live_started.is_some() {
            probe.live_frames += 1;
            if cx.reload_generation != probe.live_gen {
                if let Some(now) = now {
                    write_live(cx, probe, now);
                }
                probe.live_started = None;
                finished_live = true;
            } else if probe.live_frames > 180 {
                write_live_timeout(cx, probe);
                probe.live_started = None;
                finished_live = true;
            }
        }
    });

    if arm_latency {
        let started = Instant::now();
        step_counter(cx);
        PROBE.with(|slot| slot.borrow_mut().pending = Some(started));
        cx.redraw_all();
    } else if arm_live {
        arm_live_reload(cx);
    } else if PROBE.with(|slot| slot.borrow().live_started.is_some()) {
        cx.redraw_all();
    }

    let busy = PROBE.with(|slot| {
        let probe = slot.borrow();
        probe.pending.is_some() || probe.live_started.is_some()
    });
    if finished_latency || (exit && (finished_live || !busy)) {
        cx.quit();
    }
}

fn step_counter(cx: &mut Cx) {
    script_ops::increment(cx);
}

fn arm_live_reload(cx: &mut Cx) {
    for _ in 0..7 {
        script_ops::increment(cx);
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs");
    // `script_eval!` registers extra ScriptMods for this file, and the
    // extractor requires one `script_mod!` per compiled site. Replay those
    // bodies in line order and recolor the label in the UI block.
    let source = cx.with_vm(|vm| {
        let bodies = vm.bx.code.bodies.borrow();
        let mut sites = Vec::new();
        for body in bodies.iter() {
            let makepad_widgets::makepad_script::ScriptSource::Mod(script_mod) = &body.source
            else {
                continue;
            };
            if script_mod
                .code
                .trim_start()
                .starts_with("__script_source__")
            {
                continue;
            }
            if !path_matches(&script_mod.file, path) {
                continue;
            }
            sites.push((script_mod.line, script_mod.column, script_mod.code.clone()));
        }
        sites.sort_by_key(|(line, column, _)| (*line, *column));
        sites.dedup_by_key(|(line, column, _)| (*line, *column));
        let mut source = String::from("fn baseline_hot_reload() {\n");
        for (_, _, code) in sites {
            let code = code
                .replacen("\"Count: \"", "\"Total: \"", 1)
                .replacen("#xf6f3ec", "#xffff00", 1);
            source.push_str("script_mod! {\n");
            source.push_str(&code);
            source.push_str("\n}\n");
        }
        source.push_str("}\n");
        source
    });
    cx.script_data
        .live_reload
        .queue_file_change(path.to_string(), source);
    // FileChange alone re-applies with `Apply::Reload`, which left the existing
    // on_render label untouched. A style reload asks that same edit to use
    // `Apply::ScriptReapply` so draw properties are walked again.
    cx.request_style_reload();
    let gen = cx.reload_generation;
    PROBE.with(|slot| {
        let probe = &mut *slot.borrow_mut();
        probe.live_gen = gen;
        probe.live_started = Some(Instant::now());
        probe.live_frames = 0;
    });
    cx.redraw_all();
}

fn path_matches(compiled: &str, changed: &str) -> bool {
    if compiled.is_empty() {
        return false;
    }
    changed == compiled || changed.ends_with(&format!("/{compiled}")) || compiled.ends_with(changed)
}

fn label_text(cx: &Cx) -> String {
    UI.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|ui| ui.label(cx, ids!(counter_label)).text())
            .unwrap_or_default()
    })
}

fn write_live(cx: &Cx, probe: &Probe, now: Instant) {
    let elapsed_us = probe
        .live_started
        .map(|started| now.duration_since(started).as_micros())
        .unwrap_or(0);
    let body = format!(
        "live_reload result=applied elapsed_us={elapsed_us} reload_generation={} frames_after_arm={} label={:?}\nmarker=post_draw_hook of the frame after the queued script_mod change\n",
        cx.reload_generation,
        probe.live_frames,
        label_text(cx)
    );
    eprintln!("{body}");
    let path = std::env::var("QUINN_LIVE_OUT")
        .unwrap_or_else(|_| "artifacts/baseline-live-reload.txt".into());
    let _ = std::fs::write(path, body);
}

fn write_live_timeout(cx: &Cx, probe: &Probe) {
    let body = format!(
        "live_reload result=timeout reload_generation={} frames={} label={:?}\n",
        cx.reload_generation,
        probe.live_frames,
        label_text(cx)
    );
    eprintln!("{body}");
    let path = std::env::var("QUINN_LIVE_OUT")
        .unwrap_or_else(|_| "artifacts/baseline-live-reload.txt".into());
    let _ = std::fs::write(path, body);
}

fn write_latency(samples: &[u128]) {
    let summary = summarize(samples);
    let list = samples
        .iter()
        .map(|sample| sample.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let body = format!(
        "latency_us {summary}\nsamples_us={list}\nmarker=t0 just before script_eval increments mod.state.counter, inside the previous frame's post_draw_hook; t1 is entry to the next post_draw_hook after the updated label was recorded\n"
    );
    eprintln!("{body}");
    if let Ok(path) = std::env::var("QUINN_LATENCY_OUT") {
        let _ = std::fs::write(path, body);
    }
}

fn summarize(samples: &[u128]) -> String {
    if samples.is_empty() {
        return "n=0".into();
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let pick = |pct: f64| -> u128 {
        let index = ((pct / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
        sorted[index]
    };
    format!(
        "n={} min={} p50={} p95={} max={}",
        sorted.len(),
        sorted[0],
        pick(50.0),
        pick(95.0),
        sorted[sorted.len() - 1]
    )
}

fn run_widget_bench(cx: &mut Cx, ui: &WidgetRef) {
    let Ok(raw) = std::env::var("QUINN_WIDGET_BENCH") else {
        return;
    };
    let n: usize = raw.parse().unwrap_or(200);
    let runs: usize = std::env::var("QUINN_WIDGET_RUNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(7);
    let host = ui.view(cx, ids!(main_view));
    let mut lines = vec![format!(
        "widget_bench labels={n} runs={runs} profile=in-process create_then_set_text"
    )];
    for run in 1..=runs {
        let (create_us, update_us) = bench_labels(cx, &host, n);
        lines.push(format!(
            "run={run} create_us={create_us} update_us={update_us}"
        ));
    }
    let body = lines.join("\n") + "\n";
    eprintln!("{body}");
    if let Ok(path) = std::env::var("QUINN_WIDGET_OUT") {
        let _ = std::fs::write(path, body);
    }
}

fn bench_labels(cx: &mut Cx, host: &WidgetRef, n: usize) -> (u128, u128) {
    let base = host
        .as_view()
        .borrow()
        .map(|view| view.children.len())
        .unwrap_or(0);
    let started = Instant::now();
    let mut created = Vec::with_capacity(n);
    for i in 0..n {
        let widget = cx.with_vm(|vm| {
            let code = format!(
                "use mod.prelude.widgets.*\nmod.widgets.Label{{text:\"row {i}\" width:Fit height:Fit}};"
            );
            let value = vm.eval(ScriptMod {
                cargo_manifest_path: String::new(),
                module_path: "baseline".into(),
                file: "baseline.script".into(),
                line: 1,
                column: 1,
                code,
                values: Vec::new(),
            });
            WidgetRef::script_from_value(vm, value)
        });
        if widget.is_empty() {
            continue;
        }
        let id = widget.widget_uid();
        if let Some(mut view) = host.as_view().borrow_mut() {
            view.children.push((LiveId(id.0), widget.clone()));
        }
        cx.widget_tree_insert_child(host.widget_uid(), LiveId(id.0), widget.clone());
        created.push(widget);
    }
    let create_us = started.elapsed().as_micros();
    let started = Instant::now();
    for (i, widget) in created.iter().enumerate() {
        widget.set_text(cx, &format!("upd {i}"));
    }
    let update_us = started.elapsed().as_micros();
    if let Some(mut view) = host.as_view().borrow_mut() {
        view.children.truncate(base);
    }
    drop(created);
    host.redraw(cx);
    (create_us, update_us)
}
