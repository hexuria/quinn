//! Shared harness for the counter and any other `run!` app.
//!
//! `QUINN_READY` is written from Makepad's `post_draw_hook`: the frame's draw
//! lists are already recorded, and on X11 `eglSwapBuffers` is the next call in
//! that paint (`linux_x11.rs` `handle_repaint` → `draw_pass_to_window`). The
//! baseline installs the same kind of hook and the same env vars so startup
//! numbers share one marker.

use std::cell::RefCell;
use std::sync::Mutex;
use std::time::Instant;

use makepad_widgets::*;

use crate::host::{bench_labels, flush_session, session_snapshot};

struct LiveShell {
    file: String,
    code: String,
}

static FRAME_DRIVER: Mutex<Option<Box<dyn Fn() + Send>>> = Mutex::new(None);
static INSPECTOR: Mutex<Option<Box<dyn Fn() -> String + Send>>> = Mutex::new(None);
static LIVE_SHELL: Mutex<Option<LiveShell>> = Mutex::new(None);

std::thread_local! {
    static AFTER_MOUNT: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
    static PROBE: RefCell<Probe> = RefCell::new(Probe {
        ready: false,
        pending: None,
        samples: Vec::new(),
        live_started: None,
        live_gen: 0,
        live_frames: 0,
        retain_noted: false,
    });
}

struct Probe {
    ready: bool,
    pending: Option<Instant>,
    samples: Vec<u128>,
    live_started: Option<Instant>,
    live_gen: u64,
    live_frames: u32,
    retain_noted: bool,
}

pub fn set_frame_driver(driver: impl Fn() + Send + 'static) {
    *FRAME_DRIVER.lock().expect("frame driver") = Some(Box::new(driver));
}

pub fn set_inspector(inspector: impl Fn() -> String + Send + 'static) {
    *INSPECTOR.lock().expect("inspector") = Some(Box::new(inspector));
}

pub fn set_live_shell(file: &str, code: &str) {
    *LIVE_SHELL.lock().expect("live shell") = Some(LiveShell {
        file: file.to_string(),
        code: code.to_string(),
    });
}

pub fn after_mount(hook: impl FnOnce() + 'static) {
    AFTER_MOUNT.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(hook));
    });
}

pub(crate) fn take_after_mount() -> Option<Box<dyn FnOnce()>> {
    AFTER_MOUNT.with(|slot| slot.borrow_mut().take())
}

pub fn flush_now() {
    flush_session();
}

pub struct SessionStats {
    pub nodes: usize,
    pub widgets: usize,
    pub host_children: usize,
    pub texts: Vec<String>,
}

pub fn session_stats() -> SessionStats {
    let (nodes, widgets, host_children, texts) = session_snapshot();
    SessionStats {
        nodes,
        widgets,
        host_children,
        texts,
    }
}

pub fn rss_kib() -> u64 {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    for line in status.lines() {
        let Some(rest) = line.strip_prefix("VmRSS:") else {
            continue;
        };
        return rest
            .split_whitespace()
            .next()
            .and_then(|n| n.parse().ok())
            .unwrap_or(0);
    }
    0
}

pub(crate) fn install(cx: &mut Cx) {
    cx.post_draw_hook = Some(Box::new(|cx| on_frame(cx)));
}

pub(crate) fn run_widget_bench(host: &WidgetRef) {
    let Ok(raw) = std::env::var("QUINN_WIDGET_BENCH") else {
        return;
    };
    let n: usize = raw.parse().unwrap_or(200);
    let runs: usize = std::env::var("QUINN_WIDGET_RUNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7);
    let mut lines = vec![format!(
        "widget_bench labels={n} runs={runs} profile=in-process create_then_set_text"
    )];
    for run in 1..=runs {
        let (create_us, update_us) = bench_labels(host, n);
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

fn on_frame(cx: &mut Cx) {
    // `Instant::now` panics on Makepad's wasm target (`std` time is unimplemented).
    // Only the latency and live-reload harnesses need a clock.
    let latency_n = std::env::var("QUINN_LATENCY")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    let live = std::env::var("QUINN_LIVE").ok().as_deref() == Some("1");
    let exit = std::env::var("QUINN_EXIT").ok().as_deref() == Some("1");
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
                let _ = std::fs::write(&path, "ready\n");
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
        if !probe.retain_noted {
            if let Ok(path) = std::env::var("QUINN_RETAIN_OUT") {
                let stats = session_stats();
                let line = format!(
                    "after_first_frame rss_kib={} nodes={} widgets={} host_children={} texts={:?}\n",
                    rss_kib(),
                    stats.nodes,
                    stats.widgets,
                    stats.host_children,
                    stats.texts
                );
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .and_then(|mut file| {
                        use std::io::Write;
                        file.write_all(line.as_bytes())
                    });
            }
            probe.retain_noted = true;
        }
    });

    if arm_latency {
        let started = Instant::now();
        with_driver(cx);
        PROBE.with(|slot| slot.borrow_mut().pending = Some(started));
        cx.redraw_all();
    } else if arm_live {
        arm_live_reload(cx);
    } else if live_still_waiting() {
        cx.redraw_all();
    }

    // Latency runs are headless and must exit on their own. A live-reload run
    // stays up unless QUINN_EXIT=1 so a screenshot can catch the recolored frame.
    if finished_latency || (exit && (finished_live || !busy())) {
        cx.quit();
    }
}

fn busy() -> bool {
    PROBE.with(|slot| {
        let probe = slot.borrow();
        probe.pending.is_some() || probe.live_started.is_some()
    })
}

fn live_still_waiting() -> bool {
    PROBE.with(|slot| slot.borrow().live_started.is_some())
}

fn with_driver(cx: &mut Cx) {
    crate::host::with_cx(cx, || {
        let driver = FRAME_DRIVER.lock().expect("frame driver");
        if let Some(driver) = driver.as_ref() {
            driver();
        }
        flush_session();
    });
}

fn arm_live_reload(cx: &mut Cx) {
    for _ in 0..7 {
        with_driver(cx);
    }
    let shell = {
        let guard = LIVE_SHELL.lock().expect("live shell");
        guard.as_ref().map(|shell| LiveShell {
            file: shell.file.clone(),
            code: shell.code.clone(),
        })
    };
    let Some(shell) = shell else {
        let _ = std::fs::write(
            std::env::var("QUINN_LIVE_OUT").unwrap_or_else(|_| "artifacts/live-reload.txt".into()),
            "live_reload error=no shell registered\n",
        );
        cx.quit();
        return;
    };
    let patched = shell.code.replace("#xf4f0e6", "#xff3300");
    let source =
        format!("fn quinn_live_reload_synthetic() {{\nscript_mod! {{\n{patched}\n}}\n}}\n");
    cx.script_data
        .live_reload
        .queue_file_change(shell.file, source);
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

fn write_live(cx: &mut Cx, probe: &Probe, now: Instant) {
    let elapsed_us = probe
        .live_started
        .map(|t| now.duration_since(t).as_micros())
        .unwrap_or(0);
    let stats = session_stats();
    let signal = crate::host::with_cx(cx, inspect);
    let body = format!(
        "live_reload result=applied elapsed_us={elapsed_us} reload_generation={} frames_after_arm={} signal={signal:?} nodes={} widgets={} host_children={} texts={:?}\nmarker=post_draw_hook of the frame after Makepad processed the queued file change\n",
        cx.reload_generation, probe.live_frames, stats.nodes, stats.widgets, stats.host_children, stats.texts
    );
    eprintln!("{body}");
    let path =
        std::env::var("QUINN_LIVE_OUT").unwrap_or_else(|_| "artifacts/live-reload.txt".into());
    let _ = std::fs::write(path, body);
}

fn write_live_timeout(cx: &mut Cx, probe: &Probe) {
    let stats = session_stats();
    let signal = crate::host::with_cx(cx, inspect);
    let body = format!(
        "live_reload result=timeout reload_generation={} frames={} signal={:?} nodes={} widgets={} host_children={} texts={:?}\n",
        cx.reload_generation,
        probe.live_frames,
        signal,
        stats.nodes,
        stats.widgets,
        stats.host_children,
        stats.texts
    );
    eprintln!("{body}");
    let path =
        std::env::var("QUINN_LIVE_OUT").unwrap_or_else(|_| "artifacts/live-reload.txt".into());
    let _ = std::fs::write(path, body);
}

fn write_latency(samples: &[u128]) {
    let summary = summarize(samples);
    let list = samples
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let body = format!(
        "latency_us {summary}\nsamples_us={list}\nmarker=t0 just before the signal write and flush, inside the previous frame's post_draw_hook (before that frame's eglSwapBuffers); t1 is entry to the next post_draw_hook after the updated frame was recorded\n"
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
    let pick = |p: f64| -> u128 {
        let idx = ((p / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
        sorted[idx]
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

fn inspect() -> Option<String> {
    let inspector = INSPECTOR.lock().expect("inspector");
    inspector.as_ref().map(|fun| fun())
}
