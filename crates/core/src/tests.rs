use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use lm_reactive::prelude::*;
use lm_reactive::{on_cleanup, Owner};

use crate::{
    el, for_each, scope_owner, show, MemorySurface, Runtime, Tag,
};

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
            .child(
                el(Tag::Span).text_dyn(move || other.get().to_string()),
            )
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
            Item { id: 1, name: "Alpha" },
            Item { id: 2, name: "Beta" },
            Item { id: 3, name: "Gamma" },
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
                        .child(el(Tag::Button).text_dyn(move || n.get().to_string()).on_click(
                            move || set_n.update(|v| *v += 1),
                        )),
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
