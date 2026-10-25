use quinn::prelude::*;
use quinn::{MemorySurface, Runtime};

#[component]
fn Banner(title: String) -> impl IntoView {
    view! {
        <h2>{title}</h2>
    }
}

#[test]
fn view_macro_mounts_text_events_show_and_for() {
    let owner = Owner::new();
    let (count, set_count) = owner.with(|| signal(0i32));
    let (open, set_open) = owner.with(|| signal(true));
    let (items, set_items) = owner.with(|| signal(vec![1u64, 2, 3]));
    let mut rt = Runtime::new(Box::new(MemorySurface::new()));
    let root = owner.with(|| {
        let view = view! {
            <div class="flex flex-col gap-4 p-4">
                <Banner title=String::from("Markup") />
                <h1>"Count: " {move || count.get()}</h1>
                <button on:click=move |_| { set_count.update(|n| *n += 1); }>"Add"</button>
                <Show when=move || open.get()>
                    <span>"open"</span>
                </Show>
                <For each=move || items.get() key=|item| *item let:item>
                    <span>{item}</span>
                </For>
            </div>
        };
        rt.mount_view(scope_owner(owner.clone(), view))
    });
    let kids = rt.children(root);
    assert!(rt.text(kids[1]).starts_with("Count:"));
    rt.click(kids[2]);
    assert_eq!(rt.text(kids[1]), "Count: 1");
    let before = rt.children(kids[4]).len();
    assert_eq!(before, 3);
    set_items.update(|rows| {
        rows.remove(1);
    });
    rt.flush();
    assert_eq!(rt.children(kids[4]).len(), 2);
    set_open.set(false);
    rt.flush();
    assert!(rt.children(kids[3]).is_empty());
    assert_eq!(rt.children(root)[1], kids[1]);
}
