use std::cell::RefCell;

use quinn::makepad_widgets::*;

app_main!(App);

#[derive(Script, ScriptHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
}

impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        quinn::boot_script(cx, &self.ui);
        if std::env::var("QUINN_RETAIN").is_ok() {
            cx.redraw_all();
        }
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
            "Retain",
            480,
            420,
            component,
        )
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(_) = event {
            if std::env::var("QUINN_RETAIN").is_ok() {
                step_retain(cx, &self.ui);
            }
        }
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}

/// One script step per draw. `render()` is applied by Makepad's task pump at
/// the end of the event, so the census on the next draw is that step's tree.
enum Phase {
    Reset,
    Rest,
    Burst,
    After,
    Done,
}

struct Job {
    cycles: u32,
    step: u32,
    burst: u8,
    phase: Phase,
    rss_before: u64,
    rss_high: u64,
    rss_rest: u64,
    rss_high_2: u64,
    rss_rest_2: u64,
    widgets_rest: usize,
    buttons_rest: usize,
    host_rest: usize,
    widgets_max: usize,
    buttons_max: usize,
    widgets_after: usize,
    buttons_after: usize,
    host_after: usize,
    widgets_max_2: usize,
    buttons_max_2: usize,
    widgets_after_2: usize,
    buttons_after_2: usize,
    host_after_2: usize,
}

std::thread_local! {
    static JOB: RefCell<Option<Job>> = const { RefCell::new(None) };
}

fn step_retain(cx: &mut Cx, ui: &WidgetRef) {
    let host = ui.view(cx, ids!(host));
    let finished = JOB.with(|slot| {
        let mut borrowed = slot.borrow_mut();
        let job = borrowed.get_or_insert_with(Job::start);
        match job.phase {
            Phase::Reset => {
                script_eval!(cx, {
                    mod.quinn_retain_reset()
                    true
                });
                job.phase = Phase::Rest;
                false
            }
            Phase::Rest => {
                let census = quinn::census_host(&host);
                job.rss_before = quinn::rss_kib();
                job.widgets_rest = census.widgets;
                job.buttons_rest = census.buttons;
                job.host_rest = census.host_children;
                job.widgets_max = census.widgets;
                job.buttons_max = census.buttons;
                job.phase = Phase::Burst;
                job.step = 0;
                script_eval!(cx, {
                    mod.quinn_retain_step(#(0.0))
                    true
                });
                job.step = 1;
                false
            }
            Phase::Burst => {
                let census = quinn::census_host(&host);
                let rss = quinn::rss_kib();
                if job.burst == 0 {
                    job.widgets_max = job.widgets_max.max(census.widgets);
                    job.buttons_max = job.buttons_max.max(census.buttons);
                    job.rss_high = job.rss_high.max(rss);
                } else {
                    job.widgets_max_2 = job.widgets_max_2.max(census.widgets);
                    job.buttons_max_2 = job.buttons_max_2.max(census.buttons);
                    job.rss_high_2 = job.rss_high_2.max(rss);
                }
                if job.step < job.cycles {
                    let i = job.step as f64;
                    script_eval!(cx, {
                        mod.quinn_retain_step(#(i))
                        true
                    });
                    job.step += 1;
                    false
                } else {
                    script_eval!(cx, {
                        mod.quinn_retain_reset()
                        true
                    });
                    job.phase = Phase::After;
                    false
                }
            }
            Phase::After => {
                let census = quinn::census_host(&host);
                quinn::with_cx(cx, || quinn::reclaim_script_heap());
                let rss = quinn::rss_kib();
                if job.burst == 0 {
                    job.widgets_after = census.widgets;
                    job.buttons_after = census.buttons;
                    job.host_after = census.host_children;
                    job.rss_rest = rss;
                    job.widgets_max_2 = census.widgets;
                    job.buttons_max_2 = census.buttons;
                    job.burst = 1;
                    job.step = 0;
                    job.phase = Phase::Burst;
                    script_eval!(cx, {
                        mod.quinn_retain_step(#(0.0))
                        true
                    });
                    job.step = 1;
                    false
                } else {
                    job.widgets_after_2 = census.widgets;
                    job.buttons_after_2 = census.buttons;
                    job.host_after_2 = census.host_children;
                    job.rss_rest_2 = rss;
                    job.phase = Phase::Done;
                    let body = job.report();
                    eprintln!("{body}");
                    let path = std::env::var("QUINN_RETAIN_OUT")
                        .unwrap_or_else(|_| "artifacts/retain.txt".into());
                    if let Some(parent) = std::path::Path::new(&path).parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(path, body);
                    true
                }
            }
            Phase::Done => true,
        }
    });
    if finished {
        cx.quit();
    } else {
        cx.redraw_all();
    }
}

impl Job {
    fn start() -> Self {
        let raw = std::env::var("QUINN_RETAIN").unwrap_or_else(|_| "2000".into());
        let cycles = if raw == "1" {
            2_000
        } else {
            raw.parse().unwrap_or(2_000)
        };
        Self {
            cycles,
            step: 0,
            burst: 0,
            phase: Phase::Reset,
            rss_before: 0,
            rss_high: 0,
            rss_rest: 0,
            rss_high_2: 0,
            rss_rest_2: 0,
            widgets_rest: 0,
            buttons_rest: 0,
            host_rest: 0,
            widgets_max: 0,
            buttons_max: 0,
            widgets_after: 0,
            buttons_after: 0,
            host_after: 0,
            widgets_max_2: 0,
            buttons_max_2: 0,
            widgets_after_2: 0,
            buttons_after_2: 0,
            host_after_2: 0,
        }
    }

    fn report(&self) -> String {
        format!(
            "retain cycles={} bursts=2 path=script rss_before_kib={} rss_high_kib={} rss_rest_kib={} rss_high_2_kib={} rss_rest_2_kib={} widgets_rest={} widgets_max={} widgets_after={} widgets_max_2={} widgets_after_2={} buttons_rest={} buttons_max={} buttons_after={} buttons_max_2={} buttons_after_2={} host_children_rest={} host_children_after={} host_children_after_2={}\n",
            self.cycles,
            self.rss_before,
            self.rss_high,
            self.rss_rest,
            self.rss_high_2,
            self.rss_rest_2,
            self.widgets_rest,
            self.widgets_max,
            self.widgets_after,
            self.widgets_max_2,
            self.widgets_after_2,
            self.buttons_rest,
            self.buttons_max,
            self.buttons_after,
            self.buttons_max_2,
            self.buttons_after_2,
            self.host_rest,
            self.host_after,
            self.host_after_2,
        )
    }
}
