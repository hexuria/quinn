use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use quinn_reactive::prelude::*;
use quinn_reactive::reactive_graph::signal::WriteSignal;
use quinn_reactive::{on_cleanup, Owner};

use crate::{el, for_each, scope_owner, show, MemorySurface, Runtime, Tag};

fn runtime() -> (Owner, Runtime) {
    let owner = Owner::new();
    let rt = Runtime::new(Box::new(MemorySurface::new()));
    (owner, rt)
}

#[test]
fn signal_updates_text_without_recreating_a_sibling() {
    let (owner, mut rt) = runtime();
    let (count, set_count) = owner.with(|| signal(0i32));
    let (other, set_other) = owner.with(|| signal(1i32));
    let root = owner.with(|| {
        let view = el(Tag::Div)
            .child(
                el(Tag::H1)
                    .text_fixed("Counter: ")
                    .text_dyn(move || count.get().to_string()),
            )
            .child(el(Tag::Span).text_dyn(move || other.get().to_string()))
            .child(el(Tag::Button).text_fixed("Increment").on_click(move || {
                set_count.update(|n| *n += 1);
            }));
        rt.mount_view(scope_owner(owner.clone(), view))
    });
    let kids = rt.children(root);
    assert_eq!(kids.len(), 3);
    let heading = kids[0];
    let sibling = kids[1];
    let button = kids[2];
    assert_eq!(rt.text(heading), "Counter: 0");
    let creates_before = rt
        .surface_log()
        .unwrap()
        .iter()
        .filter(|line| line.starts_with("create "))
        .count();
    rt.click(button);
    assert_eq!(rt.text(heading), "Counter: 1");
    assert_eq!(rt.text(sibling), "1");
    assert!(rt.alive(sibling));
    assert!(rt.alive(heading));
    assert!(rt.alive(button));
    let creates_after = rt
        .surface_log()
        .unwrap()
        .iter()
        .filter(|line| line.starts_with("create "))
        .count();
    assert_eq!(creates_before, creates_after);
    set_other.set(9);
    rt.flush();
    assert_eq!(rt.text(sibling), "9");
    assert_eq!(rt.text(heading), "Counter: 1");
    assert_eq!(rt.children(root)[0], heading);
    assert_eq!(rt.children(root)[2], button);
}

#[test]
fn effect_disposed_when_conditional_unmounts_and_sibling_survives() {
    let (owner, mut rt) = runtime();
    let (open, set_open) = owner.with(|| signal(true));
    let (tick, set_tick) = owner.with(|| signal(0i32));
    let (sibling_n, set_sibling) = owner.with(|| signal(0i32));
    let runs = Arc::new(AtomicUsize::new(0));
    let cleanups = Arc::new(AtomicUsize::new(0));
    let runs2 = runs.clone();
    let cleanups2 = cleanups.clone();
    let root = owner.with(|| {
        let view = el(Tag::Div)
            .child(el(Tag::Span).text_dyn(move || sibling_n.get().to_string()))
            .child(show(
                move || open.get(),
                move || {
                    let runs2 = runs2.clone();
                    let cleanups2 = cleanups2.clone();
                    let child_owner = Owner::new();
                    child_owner.with(|| {
                        on_cleanup(move || {
                            cleanups2.fetch_add(1, Ordering::SeqCst);
                        });
                    });
                    scope_owner(
                        child_owner,
                        el(Tag::Span).text_dyn(move || {
                            runs2.fetch_add(1, Ordering::SeqCst);
                            tick.get().to_string()
                        }),
                    )
                },
            ));
        rt.mount_view(scope_owner(owner.clone(), view))
    });
    let sibling = rt.children(root)[0];
    let host = rt.children(root)[1];
    let detail = rt.children(host)[0];
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    set_tick.set(4);
    rt.flush();
    assert_eq!(runs.load(Ordering::SeqCst), 2);
    assert_eq!(rt.text(detail), "4");
    set_open.set(false);
    rt.flush();
    assert!(!rt.alive(detail));
    assert_eq!(cleanups.load(Ordering::SeqCst), 1);
    assert_eq!(rt.children(root)[0], sibling);
    let runs_after = runs.load(Ordering::SeqCst);
    set_tick.set(5);
    rt.flush();
    assert_eq!(runs.load(Ordering::SeqCst), runs_after);
    set_sibling.set(3);
    rt.flush();
    assert_eq!(rt.text(sibling), "3");
    assert!(rt.alive(sibling));
    set_open.set(true);
    rt.flush();
    let remounted = rt.children(host)[0];
    assert_ne!(remounted, detail);
    assert!(rt.alive(remounted));
    assert_eq!(rt.children(root)[0], sibling);
}

#[test]
fn keyed_list_preserves_identity_of_untouched_rows() {
    let (owner, mut rt) = runtime();
    #[derive(Clone)]
    struct Item {
        id: u64,
        name: &'static str,
    }
    let (items, set_items) = owner.with(|| {
        signal(vec![
            Item {
                id: 1,
                name: "Alpha",
            },
            Item {
                id: 2,
                name: "Beta",
            },
            Item {
                id: 3,
                name: "Gamma",
            },
        ])
    });
    let root = owner.with(|| {
        let view = for_each(
            move || items.get(),
            |item: &Item| item.id,
            |item: Item| {
                let row_owner = Owner::new();
                let (n, set_n) = row_owner.with(|| signal(0i32));
                scope_owner(
                    row_owner,
                    el(Tag::Div)
                        .child(el(Tag::Span).text_fixed(item.name))
                        .child(
                            el(Tag::Button)
                                .text_dyn(move || n.get().to_string())
                                .on_click(move || set_n.update(|v| *v += 1)),
                        ),
                )
            },
        );
        rt.mount_view(scope_owner(owner.clone(), view))
    });
    let before = rt.children(root);
    assert_eq!(before.len(), 3);
    let beta = before[1];
    rt.click(rt.children(beta)[1]);
    assert_eq!(rt.text(rt.children(beta)[1]), "1");
    set_items.update(|rows| {
        rows.swap(0, 2);
        rows.remove(1);
    });
    rt.flush();
    let after = rt.children(root);
    assert_eq!(after.len(), 2);
    assert_eq!(after[0], before[2]);
    assert_eq!(after[1], before[0]);
    assert!(!rt.alive(beta));
    assert_eq!(rt.text(rt.children(before[0])[0]), "Alpha");
    set_items.update(|rows| {
        rows.insert(
            0,
            Item {
                id: 9,
                name: "Delta",
            },
        );
    });
    rt.flush();
    let with_new = rt.children(root);
    assert_eq!(with_new.len(), 3);
    assert_eq!(with_new[1], before[2]);
    assert_eq!(with_new[2], before[0]);
    assert_ne!(with_new[0], before[0]);
    assert_ne!(with_new[0], before[2]);
}

#[test]
fn handler_does_not_run_after_unmount() {
    let (owner, mut rt) = runtime();
    let hits = Arc::new(AtomicUsize::new(0));
    let hits2 = hits.clone();
    let root = owner.with(|| {
        let view = el(Tag::Button).on_click(move || {
            hits2.fetch_add(1, Ordering::SeqCst);
        });
        rt.mount_view(scope_owner(owner.clone(), view))
    });
    rt.click(root);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    rt.unmount(root);
    rt.click(root);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert!(!rt.alive(root));
}

#[test]
fn input_updates_reactive_state_and_skips_identical_text() {
    let (owner, mut rt) = runtime();
    let (name, set_name) = owner.with(|| signal(String::new()));
    let (rev, set_rev) = owner.with(|| signal(0i32));
    let root = owner.with(|| {
        let view = el(Tag::Div)
            .child(
                el(Tag::Input)
                    .placeholder("Your name")
                    .value_dyn(move || {
                        let _ = rev.get();
                        name.get()
                    })
                    .on_input(move |value| set_name.set(value)),
            )
            .child(el(Tag::H1).text_dyn(move || format!("Hello, {}", name.get())));
        rt.mount_view(scope_owner(owner.clone(), view))
    });
    let input = rt.children(root)[0];
    let heading = rt.children(root)[1];
    rt.input(input, "Ada".into());
    assert_eq!(name.get_untracked(), "Ada");
    assert_eq!(rt.text(heading), "Hello, Ada");
    assert_eq!(rt.text(input), "Ada");
    let writes = |rt: &Runtime, id: u64| {
        rt.surface_log()
            .unwrap()
            .iter()
            .filter(|line| line.starts_with(&format!("set_text {id} ")))
            .count()
    };
    let writes_before = writes(&rt, input);
    assert_eq!(writes_before, 1);
    set_rev.set(1);
    rt.flush();
    assert_eq!(writes(&rt, input), writes_before);
    assert_eq!(rt.text(input), "Ada");
}

/// Prints real timings. Not a pass/fail budget.
#[test]
fn measured_memory_surface_update_and_create() {
    let (owner, mut rt) = runtime();
    let (count, set_count) = owner.with(|| signal(0i32));
    let root = owner.with(|| {
        let view = el(Tag::Span).text_dyn(move || count.get().to_string());
        rt.mount_view(scope_owner(owner.clone(), view))
    });
    let start = std::time::Instant::now();
    for i in 0..1_000 {
        set_count.set(i);
        rt.flush();
    }
    let elapsed = start.elapsed();
    eprintln!(
        "MEMORY_SURFACE 1000 signal flushes: {elapsed:?} ({:.1} us/update)",
        elapsed.as_secs_f64() * 1e6 / 1000.0
    );
    assert_eq!(rt.text(root), "999");

    let start = std::time::Instant::now();
    let (owner, mut rt) = runtime();
    owner.with(|| {
        let mut parent = el(Tag::Div);
        for i in 0..200 {
            parent = parent.child(el(Tag::Span).text_fixed(format!("row {i}")));
        }
        rt.mount_view(scope_owner(owner.clone(), parent));
    });
    eprintln!(
        "MEMORY_SURFACE create 1 div + 200 labels: {:?}",
        start.elapsed()
    );
}

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

/// Thousands of conditional mount/unmount and keyed add/remove cycles must
/// return to the same node, widget, handler, and cleanup footprint.
#[test]
fn retention_cycles_do_not_accumulate_nodes_or_handlers() {
    let (owner, mut rt) = runtime();
    let (open, set_open) = owner.with(|| signal(true));
    #[derive(Clone)]
    struct Item {
        id: u64,
        name: String,
    }
    let (items, set_items) = owner.with(|| {
        signal(vec![Item {
            id: 1,
            name: "Alpha".into(),
        }])
    });
    let handlers = Arc::new(AtomicUsize::new(0));
    let cleanups = Arc::new(AtomicUsize::new(0));
    let handlers_show = handlers.clone();
    let cleanups_show = cleanups.clone();
    let handlers_row = handlers.clone();
    let cleanups_row = cleanups.clone();
    let root = owner.with(|| {
        let view = el(Tag::Div)
            .child(show(
                move || open.get(),
                move || {
                    let guard = LiveGuard::new(&handlers_show);
                    let cleanups = cleanups_show.clone();
                    let child_owner = Owner::new();
                    child_owner.with(|| {
                        on_cleanup(move || {
                            cleanups.fetch_add(1, Ordering::SeqCst);
                        });
                    });
                    scope_owner(
                        child_owner,
                        el(Tag::Button).text_fixed("detail").on_click(move || {
                            let _keep = &guard;
                        }),
                    )
                },
            ))
            .child(for_each(
                move || items.get(),
                |item: &Item| item.id,
                move |item: Item| {
                    let guard = LiveGuard::new(&handlers_row);
                    let cleanups = cleanups_row.clone();
                    let row_owner = Owner::new();
                    row_owner.with(|| {
                        on_cleanup(move || {
                            cleanups.fetch_add(1, Ordering::SeqCst);
                        });
                    });
                    scope_owner(
                        row_owner,
                        el(Tag::Span).text_fixed(item.name).on_click(move || {
                            let _keep = &guard;
                        }),
                    )
                },
            ));
        rt.mount_view(scope_owner(owner.clone(), view))
    });

    let settle =
        |rt: &mut Runtime, set_open: &WriteSignal<bool>, set_items: &WriteSignal<Vec<Item>>| {
            set_open.set(false);
            set_items.set(vec![Item {
                id: 1,
                name: "Alpha".into(),
            }]);
            rt.flush();
        };
    settle(&mut rt, &set_open, &set_items);
    let nodes = rt.node_count();
    let alive = rt.memory_mut().unwrap().alive_count();
    let handlers_at_rest = handlers.load(Ordering::SeqCst);
    let cleanups_at_rest = cleanups.load(Ordering::SeqCst);
    assert!(nodes > 0, "settled tree is empty");
    assert_eq!(alive, nodes);

    let mut max_nodes = nodes;
    let mut next_id = 2u64;
    for i in 0..2_000 {
        set_open.set(i % 2 == 0);
        if i % 2 == 0 {
            let id = next_id;
            next_id += 1;
            set_items.update(move |rows| {
                rows.push(Item {
                    id,
                    name: format!("row {id}"),
                });
            });
        } else {
            set_items.update(|rows| {
                if rows.len() > 1 {
                    rows.pop();
                }
            });
        }
        rt.flush();
        max_nodes = max_nodes.max(rt.node_count());
    }
    settle(&mut rt, &set_open, &set_items);

    assert_eq!(rt.node_count(), nodes, "reactive nodes leaked");
    assert_eq!(
        rt.memory_mut().unwrap().alive_count(),
        alive,
        "surface widgets leaked"
    );
    assert_eq!(
        handlers.load(Ordering::SeqCst),
        handlers_at_rest,
        "click handlers leaked"
    );
    assert!(
        cleanups.load(Ordering::SeqCst) >= cleanups_at_rest + 1_500,
        "cleanups did not run on both show and rows, before {cleanups_at_rest} after {}",
        cleanups.load(Ordering::SeqCst)
    );
    assert!(
        max_nodes <= nodes + 40,
        "node count climbed to {max_nodes} from a resting {nodes}"
    );
    assert!(rt.alive(root));
}
