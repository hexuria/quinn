use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use quinn::prelude::*;

struct LiveGuard(Arc<AtomicUsize>);

impl LiveGuard {
    fn new(counter: &Arc<AtomicUsize>) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self(counter.clone())
    }
}

impl Drop for LiveGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Clone)]
struct Item {
    id: u64,
    label: String,
}

#[component]
fn Detail(handlers: Arc<AtomicUsize>, cleanups: Arc<AtomicUsize>) -> impl IntoView {
    let guard = LiveGuard::new(&handlers);
    on_cleanup(move || {
        cleanups.fetch_add(1, Ordering::SeqCst);
    });
    view! {
        <div class="p-2">
            <button on:click=move |_| {
                let _keep = &guard;
            }>"Detail"</button>
        </div>
    }
}

#[component]
fn Row(item: Item, handlers: Arc<AtomicUsize>, cleanups: Arc<AtomicUsize>) -> impl IntoView {
    let guard = LiveGuard::new(&handlers);
    let label = item.label;
    on_cleanup(move || {
        cleanups.fetch_add(1, Ordering::SeqCst);
    });
    view! {
        <div class="flex flex-row gap-2">
            <span>{label}</span>
            <button on:click=move |_| {
                let _keep = &guard;
            }>"row"</button>
        </div>
    }
}

#[component]
fn Retain() -> impl IntoView {
    let (open, set_open) = signal(true);
    let (items, set_items) = signal(vec![Item {
        id: 1,
        label: "Alpha".into(),
    }]);
    let handlers = Arc::new(AtomicUsize::new(0));
    let cleanups = Arc::new(AtomicUsize::new(0));
    let show_handlers = handlers.clone();
    let show_cleanups = cleanups.clone();
    let row_handlers = handlers.clone();
    let row_cleanups = cleanups.clone();

    if let Ok(raw) = std::env::var("QUINN_RETAIN") {
        let cycles: u32 = if raw == "1" {
            2_000
        } else {
            raw.parse().unwrap_or(2_000)
        };
        let set_open_loop = set_open;
        let set_items_loop = set_items;
        let handlers_loop = handlers.clone();
        let cleanups_loop = cleanups.clone();
        quinn::after_mount(move || {
            let rss_before = quinn::rss_kib();
            set_open_loop.set(false);
            set_items_loop.set(vec![Item {
                id: 1,
                label: "Alpha".into(),
            }]);
            quinn::flush_now();
            let resting = quinn::session_stats();
            let handlers_before = handlers_loop.load(Ordering::SeqCst);
            let cleanups_before = cleanups_loop.load(Ordering::SeqCst);
            let mut max_nodes = resting.nodes;
            let mut max_widgets = resting.widgets;
            let mut next_id = 2u64;
            for i in 0..cycles {
                set_open_loop.set(i % 2 == 0);
                if i % 2 == 0 {
                    let id = next_id;
                    next_id += 1;
                    set_items_loop.update(move |rows| {
                        rows.push(Item {
                            id,
                            label: format!("Item {id}"),
                        });
                    });
                } else {
                    set_items_loop.update(|rows| {
                        if rows.len() > 1 {
                            rows.pop();
                        }
                    });
                }
                quinn::flush_now();
                let stats = quinn::session_stats();
                max_nodes = max_nodes.max(stats.nodes);
                max_widgets = max_widgets.max(stats.widgets);
            }
            set_open_loop.set(false);
            set_items_loop.set(vec![Item {
                id: 1,
                label: "Alpha".into(),
            }]);
            quinn::flush_now();
            let after = quinn::session_stats();
            let handlers_after = handlers_loop.load(Ordering::SeqCst);
            let cleanups_mid = cleanups_loop.load(Ordering::SeqCst);
            let rss_high = quinn::rss_kib();
            quinn::reclaim_script_heap();
            let rss_rest = quinn::rss_kib();
            // A second burst of the same length. If the script objects from
            // the first burst were actually released, resting RSS stays put.
            let mut max_nodes_2 = after.nodes;
            let mut max_widgets_2 = after.widgets;
            for i in 0..cycles {
                set_open_loop.set(i % 2 == 0);
                if i % 2 == 0 {
                    let id = next_id;
                    next_id += 1;
                    set_items_loop.update(move |rows| {
                        rows.push(Item {
                            id,
                            label: format!("Item {id}"),
                        });
                    });
                } else {
                    set_items_loop.update(|rows| {
                        if rows.len() > 1 {
                            rows.pop();
                        }
                    });
                }
                quinn::flush_now();
                let stats = quinn::session_stats();
                max_nodes_2 = max_nodes_2.max(stats.nodes);
                max_widgets_2 = max_widgets_2.max(stats.widgets);
            }
            set_open_loop.set(false);
            set_items_loop.set(vec![Item {
                id: 1,
                label: "Alpha".into(),
            }]);
            quinn::flush_now();
            let after_2 = quinn::session_stats();
            let rss_high_2 = quinn::rss_kib();
            quinn::reclaim_script_heap();
            let rss_rest_2 = quinn::rss_kib();
            let handlers_after_2 = handlers_loop.load(Ordering::SeqCst);
            let cleanups_after = cleanups_loop.load(Ordering::SeqCst);
            let body = format!(
                "retain cycles={cycles} bursts=2 rss_before_kib={rss_before} rss_high_kib={rss_high} rss_rest_kib={rss_rest} rss_high_2_kib={rss_high_2} rss_rest_2_kib={rss_rest_2} nodes_before={} nodes_after={} nodes_max={max_nodes} nodes_after_2={} nodes_max_2={max_nodes_2} widgets_before={} widgets_after={} widgets_max={max_widgets} widgets_after_2={} widgets_max_2={max_widgets_2} host_children_before={} host_children_after={} host_children_after_2={} handlers_before={handlers_before} handlers_after={handlers_after} handlers_after_2={handlers_after_2} cleanups_before={cleanups_before} cleanups_mid={cleanups_mid} cleanups_after={cleanups_after}\n",
                resting.nodes,
                after.nodes,
                after_2.nodes,
                resting.widgets,
                after.widgets,
                after_2.widgets,
                resting.host_children,
                after.host_children,
                after_2.host_children,
            );
            eprintln!("{body}");
            let path =
                std::env::var("QUINN_RETAIN_OUT").unwrap_or_else(|_| "artifacts/retain.txt".into());
            if let Some(parent) = std::path::Path::new(&path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, body);
        });
    }

    view! {
        <div class="flex flex-col gap-3 p-4">
            <h1>"Retention"</h1>
            <Show when=move || open.get()>
                <Detail handlers=show_handlers.clone() cleanups=show_cleanups.clone() />
            </Show>
            <For each=move || items.get() key=|item| item.id let:item>
                <Row item=item handlers=row_handlers.clone() cleanups=row_cleanups.clone() />
            </For>
        </div>
    }
}

quinn::run!("Retain", 480, 420, view! { <Retain /> });
