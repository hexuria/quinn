//! Turns a Leptos-looking `view!` / `signal()` component into a Makepad
//! `script_mod!` the `--hot` watcher can re-read.
//!
//! The script is written to `hot/ui.rs` inside the example crate, next to
//! `src/` but not part of it. Cargo does not compile that file. Makepad's
//! watcher does, because the crate directory is the watch root.

mod emit;
mod extract;
mod handler;
mod model;
mod parse;
mod sim;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub use extract::extract_hot_script;
pub use model::{
    Act, BinOp, ClassWhen, El, HandlerStmt, HxExpr, Init, ItemSeed, Model, Part, Signal, StyleWhen,
    ViewNode,
};
pub use sim::Sim;

#[derive(Clone, Copy, Debug)]
pub struct Shell {
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
}

/// Watch `source` and rewrite `output` when the view changes.
pub fn spawn_watch(source: impl Into<PathBuf>, output: impl Into<PathBuf>, shell: Shell) {
    let source = source.into();
    let output = output.into();
    std::thread::spawn(move || {
        let mut last = mtime(&source);
        loop {
            std::thread::sleep(Duration::from_millis(80));
            let now = mtime(&source);
            if now == last {
                continue;
            }
            std::thread::sleep(Duration::from_millis(40));
            last = mtime(&source);
            match std::fs::read_to_string(&source) {
                Ok(text) => match lower_file(&text, shell) {
                    Ok(file) => write_if_changed(&output, &file),
                    Err(err) => eprintln!("quinn lower failed: {err}"),
                },
                Err(err) => eprintln!("quinn lower could not read {}: {err}", source.display()),
            }
        }
    });
}

static WATCH_ARMED: AtomicBool = AtomicBool::new(false);

/// Start [`spawn_watch`] once per process. `script_mod` runs again on reload.
pub fn watch_once(source: impl Into<PathBuf>, output: impl Into<PathBuf>, shell: Shell) {
    if WATCH_ARMED.swap(true, Ordering::SeqCst) {
        return;
    }
    spawn_watch(source, output, shell);
}

pub fn view_path(manifest_dir: &str) -> PathBuf {
    Path::new(manifest_dir).join("view.rs")
}

pub fn hot_ui_path(manifest_dir: &str) -> PathBuf {
    Path::new(manifest_dir).join("hot").join("ui.rs")
}

/// Lower `source` into `output` when the text changed.
pub fn write_lowered(source: &Path, output: &Path, shell: Shell) -> Result<(), String> {
    let text = std::fs::read_to_string(source)
        .map_err(|err| format!("read {}: {err}", source.display()))?;
    let file = lower_file(&text, shell)?;
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("mkdir {}: {err}", parent.display()))?;
    }
    write_if_changed(output, &file);
    Ok(())
}

fn mtime(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

fn write_if_changed(path: &Path, body: &str) {
    let prev = std::fs::read_to_string(path).unwrap_or_default();
    if prev == body {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(err) = std::fs::write(path, body) {
        eprintln!("quinn lower could not write {}: {err}", path.display());
    } else {
        eprintln!("quinn lowered view into {}", path.display());
    }
}

pub fn lower_file(source: &str, shell: Shell) -> Result<String, String> {
    let model = parse_model(source)?;
    Ok(emit::emit_rust(&model, &shell))
}

pub fn parse_model(source: &str) -> Result<Model, String> {
    parse::parse_model(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> Shell {
        Shell {
            title: "Counter",
            width: 480,
            height: 360,
        }
    }

    const COUNTER: &str = r#"
        #[component]
        fn Counter() -> impl IntoView {
            let (count, set_count) = signal(0);
            quinn::set_frame_driver(move || {
                set_count.update(|n| *n += 1);
            });
            view! {
                <div class="flex flex-col gap-4 p-4">
                    <h1>"Counter: " {move || count.get()}</h1>
                    <button on:click=move |_| { set_count.update(|n| *n += 1); }>"Increment"</button>
                </div>
            }
        }
    "#;

    #[test]
    fn counter_script_uses_state_render_and_on_click() {
        let file = lower_file(COUNTER, shell()).unwrap();
        assert_eq!(file.matches("script_mod!").count(), 1);
        assert!(!file.contains("script_eval"));
        assert!(file.contains("if ok{mod.state} == nil"));
        assert!(file.contains("let state = mod.state"));
        assert!(file.contains("spacing: 16."));
        assert!(file.contains("padding: 16."));
        assert!(file.contains("flow: Down"));
        assert!(file.contains("text: \"Counter: \" + state.count"));
        assert!(file.contains("on_click: || {"));
        assert!(file.contains("state.count += 1"));
        assert!(file.contains("ui.host.render()"));
        assert!(file.contains("fn quinn_frame()"));
        assert!(file.contains("mod.quinn_frame = quinn_frame"));
        assert!(file.contains("increment := Button"));
        assert!(file.contains("h1 := H1"));
        let extracted = extract_hot_script(&file).unwrap();
        assert!(extracted.code.contains("#(0)"));
        assert!(extracted.code.ends_with(';'));
        assert!(extracted.code.contains("state.count"));
    }

    #[test]
    fn gap_edit_changes_spacing_and_keeps_the_block_line() {
        let before = lower_file(COUNTER, shell()).unwrap();
        let after = lower_file(&COUNTER.replace("gap-4", "gap-8"), shell()).unwrap();
        assert!(before.contains("spacing: 16."));
        assert!(after.contains("spacing: 32."));
        let line = |file: &str| {
            file.lines()
                .position(|line| line.contains("script_mod!"))
                .unwrap()
        };
        assert_eq!(line(&before), line(&after));
        let left = extract_hot_script(&before).unwrap();
        let right = extract_hot_script(&after).unwrap();
        assert_eq!(left.line, right.line);
        assert_eq!(left.column, right.column);
    }

    #[test]
    fn click_updates_text_and_keeps_ids() {
        let model = parse_model(COUNTER).unwrap();
        let mut sim = Sim::new(model);
        let heading = sim.id("h1").unwrap();
        let button = sim.id("increment").unwrap();
        assert_eq!(sim.text("h1"), "Counter: 0");
        assert!(sim.click("increment"));
        assert_eq!(sim.text("h1"), "Counter: 1");
        assert_eq!(sim.state["count"], 1);
        assert_eq!(sim.id("h1"), Some(heading));
        assert_eq!(sim.id("increment"), Some(button));
    }

    #[test]
    fn removed_name_drops_its_handler() {
        let model = parse_model(COUNTER).unwrap();
        let mut sim = Sim::new(model);
        assert!(sim.click("increment"));
        sim.drop_named("increment");
        assert!(sim.id("increment").is_none());
        assert!(!sim.click("increment"));
        assert_eq!(sim.state["count"], 1);
        assert!(sim.id("h1").is_some());
    }

    #[test]
    fn button_text_edit_renames_the_control_and_keeps_the_count() {
        let model = parse_model(COUNTER).unwrap();
        let mut sim = Sim::new(model);
        sim.click("increment");
        let edited = COUNTER.replace("\"Increment\"", "\"Add one\"");
        let mut next = Sim::new(parse_model(&edited).unwrap());
        next.state.insert("count".into(), sim.state["count"]);
        next.render();
        assert_eq!(next.text("h1"), "Counter: 1");
        assert!(next.id("add_one").is_some());
        assert!(next.click("add_one"));
        assert_eq!(next.state["count"], 2);
    }

    const CONDITIONAL: &str = r#"
        #[component]
        fn Detail() -> impl IntoView {
            let (clicks, set_clicks) = signal(0);
            view! {
                <div class="flex flex-col gap-2 p-4 rounded bg-amber-100">
                    <h2>"Detail is mounted"</h2>
                    <button on:click=move |_| { set_clicks.update(|n| *n += 1); }>
                        "Detail clicks: " {move || clicks.get()}
                    </button>
                </div>
            }
        }
        #[component]
        fn Conditional() -> impl IntoView {
            let (open, set_open) = signal(true);
            let (ticks, set_ticks) = signal(0);
            view! {
                <div class="flex flex-col gap-4 p-4">
                    <h1>"Sibling ticks: " {move || ticks.get()}</h1>
                    <button on:click=move |_| { set_ticks.update(|n| *n += 1); }>"Tick sibling"</button>
                    <button on:click=move |_| { set_open.update(|open| *open = !*open); }>
                        {move || if open.get() { "Hide detail" } else { "Show detail" }}
                    </button>
                    <Show when=move || open.get()><Detail /></Show>
                </div>
            }
        }
    "#;

    #[test]
    fn show_unmount_resets_detail_and_keeps_the_sibling() {
        let mut sim = Sim::new(parse_model(CONDITIONAL).unwrap());
        let sibling = sim.id("h1").unwrap();
        let ticks = sim.id("tick_sibling").unwrap();
        assert!(sim.text("toggle").contains("Hide detail"));
        assert!(sim.visible("detail"));
        let detail = sim.id("detail").unwrap();
        assert!(sim.click("button"));
        assert_eq!(sim.state["detail_clicks"], 1);
        assert!(sim.text("button").contains("Detail clicks: 1"));
        assert!(sim.click("tick_sibling"));
        assert_eq!(sim.state["ticks"], 1);
        assert!(sim.click("toggle"));
        assert!(!sim.visible("detail"));
        assert!(sim.id("button").is_none());
        assert!(!sim.click("button"));
        assert_eq!(sim.state["detail_clicks"], 0);
        assert_eq!(sim.state["ticks"], 1);
        assert_eq!(sim.id("h1"), Some(sibling));
        assert_eq!(sim.id("tick_sibling"), Some(ticks));
        assert!(sim.click("toggle"));
        assert!(sim.visible("detail"));
        assert_ne!(sim.id("detail"), Some(detail));
        assert_eq!(sim.state["detail_clicks"], 0);
        assert!(sim.text("button").contains("Detail clicks: 0"));
        assert_eq!(sim.id("h1"), Some(sibling));
        let file = lower_file(CONDITIONAL, shell()).unwrap();
        assert!(file.contains("fn quinn_show_open_1()"));
        assert!(file.contains("if state.open"));
        assert!(file.contains("state.detail_clicks = if state.open"));
        assert!(file.contains("state.open = !state.open"));
        assert!(file.contains("quinn_show_open_1_off"));
    }

    const LIST: &str = r#"
        struct Item { id: u64, label: String, }
        #[component]
        fn Row(item: Item) -> impl IntoView {
            let (count, set_count) = signal(0i32);
            let label = item.label;
            view! {
                <div class="flex flex-row gap-4 items-center">
                    <span class="text-lg">{label}</span>
                    <button on:click=move |_| { set_count.update(|n| *n += 1); }>
                        {move || count.get()}
                    </button>
                </div>
            }
        }
        #[component]
        fn KeyedList() -> impl IntoView {
            let (items, set_items) = signal(vec![
                Item { id: 1, label: "Alpha".into() },
                Item { id: 2, label: "Beta".into() },
                Item { id: 3, label: "Gamma".into() },
            ]);
            let (next_id, set_next) = signal(4u64);
            view! {
                <div class="flex flex-col gap-3 p-4">
                    <h1>"Keyed rows"</h1>
                    <For each=move || items.get() key=|item| item.id let:item>
                        <Row item=item />
                    </For>
                    <div class="flex flex-row gap-2">
                        <button on:click=move |_| {
                            let id = next_id.get();
                            set_next.set(id + 1);
                            set_items.update(|rows| { rows.push(Item { id, label: format!("Item {id}") }); });
                        }>"Add"</button>
                        <button on:click=move |_| { set_items.update(|rows| { rows.pop(); }); }>"Remove last"</button>
                        <button on:click=move |_| { set_items.update(|rows| { rows.reverse(); }); }>"Reverse"</button>
                    </div>
                </div>
            }
        }
    "#;

    #[test]
    fn keyed_count_follows_beta_across_reorder_and_removal() {
        let mut sim = Sim::new(parse_model(LIST).unwrap());
        assert_eq!(sim.item_ids(), vec![1, 2, 3]);
        assert!(sim.click_row(2));
        assert_eq!(sim.row_count(2), 1);
        assert!(sim.click("reverse"));
        assert_eq!(sim.item_ids(), vec![3, 2, 1]);
        assert_eq!(sim.row_count(2), 1);
        assert_eq!(sim.item_label(2), "Beta");
        assert!(sim.click("remove_last"));
        assert_eq!(sim.item_ids(), vec![3, 2]);
        assert_eq!(sim.row_count(2), 1);
        assert!(sim.click_row(2));
        assert_eq!(sim.row_count(2), 2);
        assert!(sim.click("reverse"));
        assert!(sim.click("remove_last"));
        assert_eq!(sim.item_ids(), vec![2]);
        assert!(sim.click("remove_last"));
        assert!(sim.item_ids().is_empty());
        assert!(!sim.click_row(2));
        assert_eq!(sim.row_count(2), 2);
        assert!(sim.click("add"));
        assert_eq!(sim.item_ids(), vec![4]);
        assert_eq!(sim.item_label(4), "Item 4");
        assert_eq!(sim.row_count(4), 0);
        let file = lower_file(LIST, shell()).unwrap();
        assert!(file.contains("for item in state.items"));
        assert!(file.contains("fn quinn_row(id, label)"));
        assert!(file.contains("quinn_row_bump(id)"));
        assert!(file.contains("fn quinn_reverse"));
    }

    const FORM: &str = r#"
        #[component]
        fn Form() -> impl IntoView {
            let (name, set_name) = signal(String::new());
            view! {
                <div class="flex flex-col gap-4 p-4">
                    <h1>
                        "Hello, "
                        {move || if name.get().is_empty() { "there".to_string() } else { name.get() }}
                    </h1>
                    <input
                        placeholder="Your name"
                        value=move || name.get()
                        on:input=move |value: String| { set_name.set(value); }
                    />
                    <button on:click=move |_| { set_name.set(String::new()); }>"Clear"</button>
                </div>
            }
        }
    "#;

    #[test]
    fn form_text_survives_a_class_edit() {
        let mut sim = Sim::new(parse_model(FORM).unwrap());
        assert_eq!(sim.text("h1"), "Hello, there");
        assert!(sim.set_input("field", "Ada"));
        assert_eq!(sim.strings["name"], "Ada");
        assert_eq!(sim.text("h1"), "Hello, Ada");
        assert_eq!(sim.text("field"), "Ada");
        assert!(sim.click("clear"));
        assert_eq!(sim.text("h1"), "Hello, there");
        assert!(sim.set_input("field", "Ada"));
        let edited = FORM.replace("gap-4", "gap-8");
        let model = parse_model(&edited).unwrap();
        sim.reload(model);
        assert_eq!(sim.strings["name"], "Ada");
        assert_eq!(sim.text("h1"), "Hello, Ada");
        assert_eq!(sim.text("field"), "Ada");
        let file = lower_file(FORM, shell()).unwrap();
        assert!(file.contains("let root = ui"));
        assert!(file.contains("on_change: |text|"));
        assert!(file.contains("state.name = text"));
        assert!(file.contains("root.render()"));
        assert!(file.contains("empty_text: \"Your name\""));
        assert!(file.contains("if state.name == \"\""));
    }

    #[test]
    fn example_views_lower_to_script_state() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cases = [
            ("examples/counter/view.rs", "state.count += 1"),
            ("examples/conditional/view.rs", "quinn_show_open_1_on"),
            ("examples/keyed-list/view.rs", "fn quinn_row(id, label)"),
            ("examples/form/view.rs", "state.name = text"),
            ("examples/retain/view.rs", "fn quinn_retain_step"),
            ("examples/tailwind-gallery/view.rs", "Distribute.SpaceBetween"),
            ("examples/keyed-list/view.rs", "row_quinn_show_open_1"),
        ];
        for (rel, needle) in cases {
            let source = std::fs::read_to_string(root.join(rel))
                .unwrap_or_else(|err| panic!("read {rel}: {err}"));
            let file = lower_file(&source, shell()).unwrap_or_else(|err| panic!("{rel}: {err}"));
            assert!(
                file.contains(needle),
                "{rel} script missing {needle}:\n{file}"
            );
            let extracted = extract_hot_script(&file).unwrap();
            assert!(extracted.code.contains("#(0)"), "{rel}");
            assert_eq!(extracted.code.matches("script_mod").count(), 0);
        }
    }

    const P0: &str = r#"
        struct Item { id: u64, label: String, }
        #[component]
        fn Row(item: Item) -> impl IntoView {
            let (count, set_count) = signal(0i32);
            let (open, set_open) = signal(false);
            let label = item.label;
            view! {
                <div class="flex flex-row items-center gap-2">
                    <span>{label}</span>
                    <button on:click=move |_| { set_open.update(|v| *v = !*v); }>"Note"</button>
                    <Show when=move || open.get()>
                        <h2>"open"</h2>
                    </Show>
                    <button on:click=move |_| { set_count.update(|n| *n += 1); }>
                        {move || count.get()}
                    </button>
                </div>
            }
        }
        #[component]
        fn Gallery() -> impl IntoView {
            let (count, set_count) = signal(0);
            let (flag, set_flag) = signal(false);
            let (name, set_name) = signal(String::new());
            let (items, set_items) = signal(vec![
                Item { id: 1, label: "Alpha".into() },
                Item { id: 2, label: "Beta".into() },
            ]);
            let (next_id, set_next) = signal(3u64);
            view! {
                <div class="flex flex-col gap-2 p-2">
                    <div class="flex flex-row flex-wrap gap-x-3 gap-y-4 justify-between items-center">
                        <span class="self-end w-1/2 min-w-8 max-w-full">"half"</span>
                        <span class="flex-1 shrink-0 basis-4">"basis"</span>
                        <span class="grow">"grow"</span>
                    </div>
                    <div class="flex flex-col justify-around items-stretch">
                        <span class="self-center">"mid"</span>
                        <span class="self-start w-screen min-h-0 max-h-16">"screen"</span>
                        <span>"stretched"</span>
                    </div>
                    <div class="justify-evenly px-4 py-2 pt-1 pr-2 pb-3 pl-4 mx-1 my-2 mt-1 mr-2 mb-3 ml-4">
                        <div class="w-[13px] h-[13px] bg-[#1e293b] p-[10px] m-[6px]"><span class="text-[#fff]">"arb"</span></div>
                    </div>
                    <span class="h-screen">"tall"</span>
                    <button class="bg-stone-900 hover:bg-amber-600 active:bg-amber-700 hover:text-white" on:click=move |_| {
                        set_count.update(|n| *n += 2);
                        set_count.update(|n| *n -= 1);
                        set_count.update(|n| *n *= 3);
                        set_count.update(|n| *n /= 2);
                        set_flag.update(|v| *v = !*v);
                        set_name.set("Ada".to_string());
                    }>"Paint"</button>
                    <button class="disabled disabled:bg-slate-100 disabled:text-slate-600">"Off"</button>
                    <div class:bg-amber-100=move || flag.get() style:width=move || if flag.get() { "120px" } else { "40px" }>"swatch"</div>
                    <For each=move || items.get() key=|item| item.id let:item>
                        <Row item=item />
                    </For>
                    <button on:click=move |_| {
                        let id = next_id.get();
                        set_next.set(id + 1);
                        set_items.update(|rows| { rows.push(Item { id, label: format!("Item {id}") }); });
                    }>"Add"</button>
                    <button on:click=move |_| { set_items.update(|rows| { rows.pop(); }); }>"Pop"</button>
                    <button on:click=move |_| { set_items.update(|rows| { rows.reverse(); }); }>"Reverse"</button>
                    <button on:click=move |_| { set_items.update(|rows| { rows.swap(1, 2); }); }>"Swap"</button>
                    <button on:click=move |_| { set_items.update(|rows| { rows.remove(2); }); }>"Remove"</button>
                    <button on:click=move |_| {
                        set_items.update(|rows| { rows.push(Item { id: 2, label: "Beta".into() }); });
                    }>"Readd"</button>
                </div>
            }
        }
    "#;

    #[test]
    fn p0_utilities_and_handlers_lower_to_script() {
        let file = lower_file(P0, shell()).unwrap_or_else(|err| panic!("{err}"));
        let needles = [
            "Flow.Right{wrap: true row_align: RowAlign.Center}",
            "wrap_spacing: 16.",
            "spacing: 12.",
            "distribute: Distribute.SpaceBetween",
            "distribute: Distribute.SpaceAround",
            "distribute: Distribute.SpaceEvenly",
            "RowAlign.Bottom",
            "align: Align{x: 0.5 y: 0.}",
            "Fill{weight: 1. shrink: 0. basis: 16.}",
            "Fill{weight: 1. shrink: 0. basis: 0.}",
            "Rel{base: Base.Parent factor: 0.5}",
            "Rel{base: Base.Vw factor: 1.}",
            "Rel{base: Base.Vh factor: 1.}",
            "min_width: 32.",
            "max_width: \"100%\"",
            "min_height: 0.",
            "max_height: 64.",
            "Inset{left: 16. top: 4. right: 8. bottom: 12.}",
            "width: 13.",
            "height: 13.",
            "padding: 10.",
            "margin: 6.",
            "draw_bg.color: #x1e293b",
            "draw_text.color: #xffffff",
            "draw_bg.color_hover: #xd97706",
            "draw_bg.color_down: #xb45309",
            "draw_text.color_hover: #xffffff",
            "enabled: false",
            "draw_bg.disabled: 1.",
            "draw_bg.color_disabled:",
            "draw_text.color_disabled:",
            "if state.flag { #xfef3c7 } else { #x000000 }",
            "if state.flag { 120. } else { 40. }",
            "state.count = state.count + 2",
            "state.count = state.count - 1",
            "state.count = state.count * 3",
            "state.count = state.count / 2",
            "state.flag = !state.flag",
            "state.name = \"Ada\"",
            "quinn_row_reset(",
            "state.items.pop()",
            "fn quinn_reverse",
            "quinn_swap_key(state.items, 1, 2)",
            "quinn_remove_key(state.items, 2)",
            "fn quinn_row(id, label)",
            "fn row_quinn_show_open_1(id, label)",
            "state.r_open[key] = state.r_open[key] != true",
            "quinn_row_bump(id)",
            "r_open: {}",
            "r_show_open_live: {}",
        ];
        for needle in needles {
            assert!(file.contains(needle), "missing {needle}:\n{file}");
        }
        assert_eq!(file.matches("color_hover").count(), 2);
        assert!(!file.contains("draw_bg.color_hover: #x1c1917"));
    }

    #[test]
    fn show_inside_a_row_resets_when_the_row_is_readded() {
        let mut sim = Sim::new(parse_model(P0).unwrap());
        assert_eq!(sim.item_ids(), vec![1, 2]);
        assert!(!sim.visible("row:2:h2"));
        assert!(sim.click("row:2:toggle"));
        assert!(sim.visible("row:2:h2"));
        assert!(sim.click_row(2));
        assert_eq!(sim.row_count(2), 1);
        assert!(sim.click("row:2:toggle"));
        assert!(!sim.visible("row:2:h2"));
        assert_eq!(sim.row_count(2), 1);
        assert!(sim.click("swap"));
        assert_eq!(sim.item_ids(), vec![2, 1]);
        assert_eq!(sim.row_count(2), 1);
        assert!(sim.click("remove"));
        assert_eq!(sim.item_ids(), vec![1]);
        assert!(!sim.click_row(2));
        assert_eq!(sim.row_count(2), 1);
        assert!(sim.click("readd"));
        assert_eq!(sim.item_ids(), vec![1, 2]);
        assert_eq!(sim.row_count(2), 0);
        assert!(!sim.visible("row:2:h2"));
        assert!(sim.click("paint"));
        assert_eq!(sim.state["count"], 1);
        assert_eq!(sim.state["flag"], 1);
        assert_eq!(sim.strings["name"], "Ada");
        assert!(sim.click("pop"));
        assert_eq!(sim.item_ids(), vec![1]);
        assert!(sim.click("reverse"));
        assert_eq!(sim.item_ids(), vec![1]);
        assert!(sim.click("add"));
        assert_eq!(sim.item_label(3), "Item 3");
        assert_eq!(sim.row_count(3), 0);
    }

    #[test]
    fn handler_outside_the_closed_set_is_a_compile_error() {
        let source = r#"
            #[component]
            fn Bad() -> impl IntoView {
                let (count, set_count) = signal(0);
                let (items, set_items) = signal(vec![Item { id: 1, label: "Alpha".into() }]);
                view! {
                    <div>
                        <button on:click=move |_| { set_count.update(|n| { n.pow(2); }); }>"Bad"</button>
                        <button on:click=move |_| { set_items.update(|rows| { rows.clear(); }); }>"Clear"</button>
                    </div>
                }
            }
        "#;
        let err = parse_model(source).unwrap_err();
        assert!(
            err.contains("cannot lower"),
            "{err}"
        );
    }
}
