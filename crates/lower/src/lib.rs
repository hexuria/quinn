//! Turns a Leptos-looking `view!` / `signal()` component into a Makepad
//! `script_mod!` the `--hot` watcher can re-read.
//!
//! The script is written to `hot/ui.rs` inside the example crate, next to
//! `src/` but not part of it. Cargo does not compile that file. Makepad's
//! watcher does, because the crate directory is the watch root.

mod emit;
mod extract;
mod handler;
mod jobrt;
mod model;
mod parse;
mod sim;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub use extract::extract_hot_script;
pub use jobrt::{
    accept_gen, cancelled_count, guard, job, lookup, panic_message, run_registered, track, JobArg,
    JobRun, JobValue,
};
pub use model::{
    Act, ArgSrc, BinOp, ClassWhen, El, HandlerStmt, HxExpr, Init, ItemSeed, Job, JobKind, Model,
    PackItem, Part, Signal, StyleWhen, ViewNode,
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
    spawn_watch_with(source, output, shell, String::new());
}

/// Like [`spawn_watch`], but component calls resolve against `library` source.
pub fn spawn_watch_with(
    source: impl Into<PathBuf>,
    output: impl Into<PathBuf>,
    shell: Shell,
    library: String,
) {
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
                Ok(text) => match lower_file_with(&text, shell, &library) {
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
    watch_once_with(source, output, shell, String::new());
}

/// Start [`spawn_watch_with`] once per process.
pub fn watch_once_with(
    source: impl Into<PathBuf>,
    output: impl Into<PathBuf>,
    shell: Shell,
    library: String,
) {
    if WATCH_ARMED.swap(true, Ordering::SeqCst) {
        return;
    }
    spawn_watch_with(source, output, shell, library);
}

pub fn view_path(manifest_dir: &str) -> PathBuf {
    Path::new(manifest_dir).join("view.rs")
}

pub fn hot_ui_path(manifest_dir: &str) -> PathBuf {
    Path::new(manifest_dir).join("hot").join("ui.rs")
}

/// Lower `source` into `output` when the text changed.
pub fn write_lowered(source: &Path, output: &Path, shell: Shell) -> Result<(), String> {
    write_lowered_with(source, output, shell, "")
}

/// Lower `source` with `library` component source merged in front of it.
pub fn write_lowered_with(
    source: &Path,
    output: &Path,
    shell: Shell,
    library: &str,
) -> Result<(), String> {
    let text = std::fs::read_to_string(source)
        .map_err(|err| format!("read {}: {err}", source.display()))?;
    let file = lower_file_with(&text, shell, library)?;
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
    lower_file_with(source, shell, "")
}

pub fn lower_file_with(source: &str, shell: Shell, library: &str) -> Result<String, String> {
    let model = parse::parse_model_with(source, library)?;
    let mut unknown = Vec::new();
    collect_unknown(&model.root_class, &mut unknown);
    for node in &model.nodes {
        walk_unknown(node, &mut unknown);
    }
    if !unknown.is_empty() {
        return Err(format!("unknown class: {}", unknown.join(", ")));
    }
    Ok(emit::emit_rust(&model, &shell))
}

fn collect_unknown(class: &str, unknown: &mut Vec<String>) {
    for token in quinn_styles::unknown_classes(class) {
        if !unknown.contains(&token) {
            unknown.push(token);
        }
    }
}

fn walk_unknown(node: &ViewNode, unknown: &mut Vec<String>) {
    match node {
        ViewNode::El(el) => walk_el(el, unknown),
        ViewNode::Show { child, .. } => walk_unknown(child, unknown),
        ViewNode::For { child, .. } => walk_el(child, unknown),
        ViewNode::Slot { nodes, .. } => {
            for node in nodes {
                walk_unknown(node, unknown);
            }
        }
        ViewNode::Routes { routes } => {
            for (_, nodes) in routes {
                for node in nodes {
                    walk_unknown(node, unknown);
                }
            }
        }
        ViewNode::Suspense {
            fallback, child, ..
        } => {
            for node in fallback {
                walk_unknown(node, unknown);
            }
            walk_unknown(child, unknown);
        }
        ViewNode::Boundary {
            fallback, child, ..
        } => {
            for node in fallback {
                walk_unknown(node, unknown);
            }
            walk_unknown(child, unknown);
        }
        ViewNode::Portal { nodes } => {
            for node in nodes {
                walk_unknown(node, unknown);
            }
        }
    }
}

fn walk_el(el: &El, unknown: &mut Vec<String>) {
    collect_unknown(&el.class, unknown);
    for when in &el.class_when {
        collect_unknown(&when.token, unknown);
    }
    for child in &el.children {
        walk_unknown(child, unknown);
    }
}

pub fn parse_model(source: &str) -> Result<Model, String> {
    parse::parse_model(source)
}

pub fn parse_model_with(source: &str, library: &str) -> Result<Model, String> {
    parse::parse_model_with(source, library)
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
        assert!(file.contains("increment := QuinnButton"));
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

    #[test]
    fn show_keeps_a_parent_string_after_the_menu_closes() {
        const POPUP: &str = r#"
            #[component]
            fn Popup() -> impl IntoView {
                let (open, set_open) = signal(false);
                let (choice, set_choice) = signal("On My Mac".to_string());
                view! {
                    <div>
                        <button on:click=move |_| { set_open.set(true); }>{choice}</button>
                        <Show when=move || open.get()>
                            <button on:click=move |_| { set_choice.set("iCloud".to_string()); set_open.set(false); }>"iCloud"</button>
                        </Show>
                    </div>
                }
            }
        "#;
        let mut sim = Sim::new(parse_model(POPUP).unwrap());
        assert_eq!(sim.text("button"), "On My Mac");
        assert!(sim.click("button"));
        assert!(sim.visible("icloud"));
        assert!(sim.click("icloud"));
        assert!(!sim.visible("icloud"));
        assert_eq!(sim.text("button"), "iCloud");
        let file = lower_file(POPUP, shell()).unwrap();
        assert!(!file.contains("state.choice = if state.open"));
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
            ("examples/keyed-list/view.rs", "fn quinn_row(id, label, index, quinn_odd)"),
            ("examples/form/view.rs", "state.name = text"),
            ("examples/retain/view.rs", "fn quinn_retain_step"),
            (
                "examples/tailwind-gallery/view.rs",
                "Distribute.SpaceBetween",
            ),
            (
                "examples/tailwind-gallery/view.rs",
                "columns: [\"1fr\", \"1fr\", \"1fr\"]",
            ),
            ("examples/keyed-list/view.rs", "row_quinn_show_open_1"),
            ("examples/app-shell/view.rs", "fn quinn_route_1("),
            ("examples/app-shell/view.rs", "mod.ctx.accent"),
            ("examples/app-shell/view.rs", "RoundedShadowView"),
            ("examples/app-shell/view.rs", "padding: Inset{left: 8. top: 4. right: 0. bottom: 0.}"),
            ("examples/async-dashboard/view.rs", "fn quinn_gate_"),
            ("examples/async-dashboard/view.rs", "fn quinn_err_"),
            ("examples/async-dashboard/view.rs", "mod.quinn_apply = quinn_apply"),
            ("examples/async-dashboard/view.rs", "state.quinn_dark == 1"),
            ("examples/tailwind-gallery/view.rs", "state.quinn_width >= 768."),
            ("examples/showcase/view.rs", "state.quinn_dark == 1"),
            // rounded-sheet (12) is a radius of 6 to Makepad's `Sdf2d.box`.
            ("examples/showcase/view.rs", "draw_bg.border_radius: 6."),
            ("examples/showcase/view.rs", "QButtonSecondary {"),
            ("examples/showcase/view.rs", "draw_bg.selected: if state.density_std"),
            ("examples/showcase/view.rs", "step: state.step\n"),
            ("examples/showcase/view.rs", "quinn_portal := QuinnLayer"),
            ("examples/showcase/view.rs", "QInput {"),
            ("examples/showcase/view.rs", "QInputSubtle {"),
            ("examples/showcase/view.rs", "draw_bg.invalid: 1."),
            ("examples/showcase/view.rs", "QSegment {"),
            ("examples/showcase/view.rs", "QSwitch {"),
            ("examples/showcase/view.rs", "QCheckbox {"),
            ("examples/showcase/view.rs", "QNavItem {"),
            ("examples/showcase/view.rs", "label_walk: if state.quinn_width >= 1120. { Walk{width: Fill height: Fit} }"),
            ("examples/showcase/view.rs", "draw_icon.svg: crate_resource(\"quinn_makepad:resources/icons/notes.svg\")"),
        ];
        for (rel, needle) in cases {
            let source = std::fs::read_to_string(root.join(rel))
                .unwrap_or_else(|err| panic!("read {rel}: {err}"));
            let library = if source.contains("quinn_ui::components") {
                let path = root.join("crates/ui/components.rs");
                std::fs::read_to_string(&path)
                    .unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
            } else {
                String::new()
            };
            let file = lower_file_with(&source, shell(), &library)
                .unwrap_or_else(|err| panic!("{rel}: {err}"));
            assert!(
                file.contains(needle),
                "{rel} script missing {needle}:\n{file}"
            );
            let extracted = extract_hot_script(&file).unwrap();
            assert!(extracted.code.contains("#(0)"), "{rel}");
            assert_eq!(extracted.code.matches("script_mod").count(), 0);
        }
    }

    #[test]
    fn library_button_forwards_click_variant_and_field() {
        let library = r#"
            #[component]
            fn Button(
                #[prop(default = "primary")] variant: String,
                #[prop(optional, default = false)] disabled: bool,
                children: Children,
            ) -> impl IntoView {
                view! {
                    <button class="h-control min-h-control px-control bg-accent text-on-accent border border-accent rounded-control">{children()}</button>
                }
            }
            #[component]
            fn Field(
                #[prop(into)] label: String,
                #[prop(optional, into, default = "")] placeholder: String,
            ) -> impl IntoView {
                view! {
                    <div class="flex flex-col gap-2 w-full">
                        <span class="text-control text-primary">{label}</span>
                        <input class="w-full h-control px-3 rounded-control bg-surface text-body text-primary border border-control" />
                    </div>
                }
            }
        "#;
        let source = r#"
            use quinn_ui::components::*;
            #[component]
            fn App() -> impl IntoView {
                let (n, set_n) = signal(0i64);
                let (name, set_name) = signal(String::new());
                view! {
                    <div class="flex flex-col gap-6 bg-canvas">
                        <Button variant="secondary" on:click=move |_| { set_n.update(|v| *v += 1); }>"Save"</Button>
                        <Button disabled=true>"Waiting"</Button>
                        <Field label="Folder" placeholder="Folder name" bind:value=name />
                    </div>
                }
            }
        "#;
        let file = lower_file_with(source, shell(), library).unwrap();
        assert!(file.contains("state.n += 1"), "{file}");
        assert!(file.contains("on_click"), "{file}");
        assert!(file.contains("quinn_density"), "{file}");
        assert!(file.contains("empty_text: \"Folder name\""), "{file}");
        assert!(file.contains("state.name = text"), "{file}");
        assert!(file.contains("#xffffff"), "{file}");
        assert!(file.contains("enabled: false"), "{file}");
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
            "if state.flag { #xfef3c7 } else { #00000000 }",
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
        assert!(file.contains("draw_bg.color_hover: #xd97706"));
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
        assert!(err.contains("cannot lower"), "{err}");
    }

    const P1: &str = r#"
        struct Profile { name: String, city: String, }
        #[component]
        fn Card(
            #[prop(into)] title: String,
            #[prop(optional, default = "stone")] tone: String,
            children: Children,
        ) -> impl IntoView {
            view! {
                <div class="p-3 rounded-lg bg-white shadow-md">
                    <span class="font-semibold">{title}</span>
                    <span class="text-xs">{tone}</span>
                    {children()}
                </div>
            }
        }
        #[component]
        fn Note(children: ChildrenFn) -> impl IntoView {
            view! {
                <div>{children()("from-fn")}</div>
            }
        }
        #[component]
        fn Shell() -> impl IntoView {
            provide_context("accent", "amber");
            let accent = use_context("accent");
            let (name, set_name) = signal(String::new());
            let (profile, set_profile) = signal(Profile { name: "Ada".into(), city: "Manila".into() });
            view! {
                <div class="flex flex-col gap-2 p-3">
                    <span>"Accent: " {accent}</span>
                    <A href="/">"Home"</A>
                    <A href="/board">"Board"</A>
                    <button on:click=move |_| { navigate("/form"); }>"Go form"</button>
                    <Router>
                        <Routes>
                            <Route path="/">
                                <div>
                                    <>
                                        <span>"One"</span>
                                        <span>"Two"</span>
                                    </>
                                    <Card title="Today">
                                        <span class="leading-snug">"Hello card"</span>
                                    </Card>
                                    <Note><span>{arg}</span></Note>
                                </div>
                            </Route>
                            <Route path="/board">
                                <div class="grid grid-cols-3 gap-2">
                                    <div class="col-span-2 h-[40px] rounded bg-gradient-to-r from-amber-600 to-stone-900 shadow-lg">
                                        <span class="text-white font-bold">"Wide"</span>
                                    </div>
                                    <div class="absolute left-2 top-1 overflow-hidden">
                                        <span class="font-medium">"Pin"</span>
                                    </div>
                                    <div class="h-[32px] overflow-y-auto overflow-x-auto">
                                        <span class="leading-relaxed">"Scroll"</span>
                                    </div>
                                </div>
                            </Route>
                            <Route path="/form">
                                <div>
                                    <input bind:value=name placeholder="Name" />
                                    <span>"Hello, " {name}</span>
                                    <span>{profile.city}</span>
                                    <button on:click=move |_| { set_profile.update(|p| { p.city = "Cebu".into(); }); }>"Move"</button>
                                </div>
                            </Route>
                        </Routes>
                    </Router>
                </div>
            }
        }
    "#;

    #[test]
    fn p1_components_context_router_grid_and_bind_lower() {
        let file = lower_file(P1, shell()).unwrap_or_else(|err| panic!("{err}"));
        let needles = [
            "fn quinn_slot_",
            "\"from-fn\"",
            "if ok{mod.ctx} == nil",
            "if ok{mod.ctx.accent} == nil",
            "mod.ctx.accent = \"amber\"",
            "mod.ctx.accent",
            "state.route = \"/\"",
            "state.route = \"/board\"",
            "state.route = \"/form\"",
            "fn quinn_route_1_on(root) {\n            RoundedView {",
            "fn quinn_route_1(",
            "fn quinn_route_2(",
            "fn quinn_route_3(",
            "if state.route == \"/\"",
            "Grid {",
            "columns: [\"1fr\", \"1fr\", \"1fr\"]",
            "implicit_row_size: 40.",
            "column_gap: 8.",
            "row_gap: 8.",
            "CellPlacement{col_span: 2}",
            "padding: Inset{left: 8. top: 4. right: 0. bottom: 0.}",
            "flow: Overlay",
            "width: Fit",
            "clip_x: true",
            "clip_y: true",
            "ScrollBars{show_scroll_x: true show_scroll_y: true}",
            "Inter.ttf",
            "weight: 700.",
            "weight: 600.",
            "weight: 500.",
            "line_spacing: 1.375",
            "line_spacing: 1.625",
            "draw_bg.color_2:",
            "gradient_fill_horizontal: 1.",
            "RoundedShadowView",
            "shadow_radius: 16.",
            "shadow_radius: 8.",
            "profile: {name: \"Ada\", city: \"Manila\"}",
            "state.profile.city = \"Cebu\"",
            "text: \"Hello, \" + state.name",
            "text: \"One\"",
            "text: \"Two\"",
            "text: \"stone\"",
            "text: \"Today\"",
            "text: arg",
        ];
        for needle in needles {
            assert!(file.contains(needle), "missing {needle}:\n{file}");
        }
        let field = file.split("field := QuinnTextInput").nth(1).expect("input");
        let field = field.split("on_change").next().unwrap();
        // A render resets every field the script leaves out, so a bound
        // input is controlled: it is given its own text back.
        assert!(
            field.lines().any(|line| line.trim() == "text: state.name"),
            "bind:value must hand the input its text:\n{field}"
        );
        assert!(file.contains("state.name = text"));
        assert!(file.contains("root.render()"));
        let mut sim = Sim::new(parse_model(P1).unwrap());
        assert_eq!(sim.text("span"), "Accent: amber");
        assert_eq!(sim.strings.get("route").map(String::as_str), Some("/"));
        assert!(sim.click("A_2"));
        assert_eq!(sim.strings["route"], "/board");
        assert!(sim.click("go_form"));
        assert_eq!(sim.strings["route"], "/form");
        assert!(sim.set_input("field", "Quinn"));
        assert_eq!(sim.strings["name"], "Quinn");
        assert_eq!(sim.text("span_11"), "Hello, Quinn");
        assert_eq!(sim.text("field"), "", "the input text is not rewritten");
        assert!(sim.click("move"));
        assert_eq!(sim.strings["profile.city"], "Cebu");
    }

    fn has_label(sim: &Sim, needle: &str) -> bool {
        sim.labels().iter().any(|(_, text)| text.contains(needle))
    }

    #[test]
    fn p2_async_portal_shell_and_nested_context() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let source = std::fs::read_to_string(root.join("examples/async-dashboard/view.rs")).unwrap();
        let file = lower_file(&source, shell()).unwrap_or_else(|err| panic!("{err}"));
        let needles = [
            "note_phase: \"loading\"",
            "feed_phase: \"loading\"",
            "place_phase: \"loading\"",
            "save_phase: \"idle\"",
            "save_pending: false",
            "quinn_width: 480.",
            "quinn_dark: 0",
            "fn quinn_job_count()",
            "fn quinn_job_fn(i)",
            "fn quinn_apply(name, gen, phase, value, err)",
            "mod.quinn_apply = quinn_apply",
            "\"fetch_note\"",
            "\"fetch_place\"",
            "\"save_place\"",
            "\"race_place\"",
            "\"fetch_cities\"",
            "for city in state.cities_value",
            "RoundedView {\n                width: Fill\n                height: Fit\n                flow: Down\n                clip_x: false\n                clip_y: false\n                for city in state.cities_value",
            "state.note_phase = \"loading\"",
            "state.note_gen = state.note_gen + 1",
            "fn quinn_sync_jobs()",
            "fn quinn_theme_on()",
            "if state.quinn_dark == 1 { quinn_theme_off() } else { quinn_theme_on() }",
            "if state.quinn_dark == 1 { #x282828 } else",
            "state.quinn_width >= 768.",
            "state.note_phase == \"loading\"",
            "state.place_value == \"\"",
            "state.feed_phase == \"error\"",
            "state.save_pending = true",
            "state.save_arg = \"Cebu\"",
            "state.save_arg = \"Davao\"",
            "state.race_arg = \"Cebu\"",
            "state.race_arg = \"Davao\"",
            "text: \"Ada\"",
            "mod.ctx.tone = \"stone\"",
            "mod.ctx.tone = \"shade\"",
            "padding: Inset{left: 8. top: 4. right: 0. bottom: 0.}",
            "flow: Overlay",
            "Animator{hover:",
            "draw_bg.color_focus:",
            "draw_bg.color: #1c191780",
            "app := RoundedView",
            "text: \"Modal\"",
            "text: \"Loading the note\"",
            "text: \"Refreshing\"",
            "text: \"Could not load the feed\"",
        ];
        let missing: Vec<_> = needles
            .into_iter()
            .filter(|needle| !file.contains(needle))
            .collect();
        assert!(missing.is_empty(), "missing {missing:?}");
        assert!(!file.contains("abs_pos:"), "{file}");
        let scrim = file.find("text: \"Scrim\"").expect("scrim");
        let modal = file.find("text: \"Modal\"").expect("modal");
        assert!(scrim < modal, "higher z paints later");
        let pinned = file.find("Pinned in the card").expect("pin");
        assert!(
            file[..pinned].contains("padding: Inset{left: 8. top: 4. right: 0. bottom: 0.}"),
            "the pin is padded inside its parent overlay"
        );

        let mut sim = Sim::new(parse_model(&source).unwrap());
        assert!(has_label(&sim, "Loading the note"));
        assert!(has_label(&sim, "Loading cities"));
        assert!(!has_label(&sim, "Ada's note"));
        assert!(has_label(&sim, "Refreshing"));
        assert!(!has_label(&sim, "Manila desk"));
        assert!(!has_label(&sim, "Could not load the feed"));
        assert!(!has_label(&sim, "The job panicked"));
        assert!(has_label(&sim, "Tone stone"));
        assert!(has_label(&sim, "shade"), "{:?}", sim.labels());
        assert!(!has_label(&sim, "Modal"));
        let note_gen = sim.gen("note");
        let place_gen = sim.gen("place");
        let feed_gen = sim.gen("feed");
        let cities_gen = sim.gen("cities");
        let boom_gen = sim.gen("boom");
        assert!(sim.complete("note", note_gen, crate::JobValue::from("Ada's note")));
        assert!(!sim.complete("note", note_gen - 1, crate::JobValue::from("stale note")));
        assert!(sim.fail("feed", feed_gen, "feed dropped"));
        assert!(sim.complete("place", place_gen, crate::JobValue::from("Manila desk")));
        assert!(sim.complete(
            "cities",
            cities_gen,
            crate::JobValue::list(vec![
                crate::JobValue::obj(vec![
                    ("id", crate::JobValue::from(1_i64)),
                    ("name", crate::JobValue::from("Manila")),
                ]),
                crate::JobValue::obj(vec![
                    ("id", crate::JobValue::from(2_i64)),
                    ("name", crate::JobValue::from("Cebu")),
                ]),
                crate::JobValue::obj(vec![
                    ("id", crate::JobValue::from(3_i64)),
                    ("name", crate::JobValue::from("Davao")),
                ]),
            ])
        ));
        assert!(sim.fail("boom", boom_gen, "panic: boom"));
        assert!(has_label(&sim, "Ada's note"), "{:?}", sim.labels());
        assert!(!has_label(&sim, "Loading the note"));
        assert!(has_label(&sim, "Manila desk"));
        assert!(!has_label(&sim, "Refreshing"));
        assert!(has_label(&sim, "Could not load the feed"));
        assert!(has_label(&sim, "The job panicked"));
        assert!(
            sim.labels().iter().any(|(_, text)| text == "Manila"),
            "{:?}",
            sim.labels()
        );
        assert!(sim.labels().iter().any(|(_, text)| text == "Cebu"));
        assert!(sim.labels().iter().any(|(_, text)| text == "Davao"));
        assert!(!has_label(&sim, "Loading cities"));
        assert!(sim.click("save_cebu"));
        assert_eq!(sim.state["save_pending"], 1);
        assert_eq!(sim.strings["save_arg"], "Cebu");
        assert!(has_label(&sim, "Saving"));
        let save_cebu = sim.gen("save");
        assert!(sim.complete("save", save_cebu, crate::JobValue::from("Saved Cebu")));
        assert_eq!(sim.state["save_pending"], 0);
        assert!(has_label(&sim, "Saved Cebu"));
        assert!(!has_label(&sim, "Saving"));
        assert!(sim.click("save_davao"));
        assert_eq!(sim.strings["save_arg"], "Davao");
        let save_davao = sim.gen("save");
        assert!(save_davao > save_cebu);
        assert!(!sim.complete(
            "save",
            save_cebu,
            crate::JobValue::from("Saved Cebu")
        ));
        assert_eq!(sim.state["save_pending"], 1);
        assert!(sim.complete(
            "save",
            save_davao,
            crate::JobValue::from("Saved Davao")
        ));
        assert!(has_label(&sim, "Saved Davao"));
        assert_eq!(sim.state["save_pending"], 0);
        assert!(sim.click("race_cebu"));
        let race_old = sim.gen("race");
        assert_eq!(sim.strings["race_arg"], "Cebu");
        assert!(sim.click("race_davao"));
        let race_new = sim.gen("race");
        assert!(race_new > race_old);
        assert_eq!(sim.strings["race_arg"], "Davao");
        assert!(!sim.complete("race", race_old, crate::JobValue::from("Cebu won")));
        assert!(!has_label(&sim, "Cebu won"));
        assert_eq!(sim.state["race_pending"], 1);
        assert!(sim.complete("race", race_new, crate::JobValue::from("Davao won")));
        assert!(has_label(&sim, "Davao won"));
        assert!(!has_label(&sim, "Cebu won"));
        assert_eq!(sim.state["race_pending"], 0);
        assert!(sim.click("theme"));
        assert_eq!(sim.state["quinn_dark"], 1);
        assert!(sim.click("theme"));
        assert_eq!(sim.state["quinn_dark"], 0);
        assert!(sim.click("open_modal"));
        assert!(has_label(&sim, "Modal"));
        sim.render();
        assert!(
            has_label(&sim, "Modal"),
            "opening the show must not clear its own condition"
        );
        assert_eq!(sim.state["open"], 1);
        assert!(sim.click("close"));
        assert!(!has_label(&sim, "Modal"));
        assert!(sim.click("refetch"));
        assert_eq!(sim.strings["place_phase"], "loading");
        assert_eq!(sim.strings["place_value"], "Manila desk");
        let place_next = sim.gen("place");
        assert!(place_next > place_gen);
        assert!(has_label(&sim, "Refreshing"));
        assert!(has_label(&sim, "Manila desk"));
        assert!(!sim.complete(
            "place",
            place_gen,
            crate::JobValue::from("Manila desk")
        ));
        assert_eq!(sim.strings["place_phase"], "loading");
        assert!(sim.complete(
            "place",
            place_next,
            crate::JobValue::from("Cebu desk")
        ));
        assert_eq!(sim.strings["place_phase"], "ready");
        assert!(has_label(&sim, "Cebu desk"));
        assert!(!has_label(&sim, "Refreshing"));
        let before_reload = sim.gen("note");
        assert!(sim.click("reload"));
        assert_eq!(sim.strings["note_phase"], "loading");
        assert_eq!(sim.strings["note_value"], "Ada's note");
        assert!(sim.gen("note") > before_reload);

        let bad = r#"
            #[component]
            fn Card(#[prop(into)] title: String) -> impl IntoView {
                view! { <div><span>{title}</span></div> }
            }
            #[component]
            fn App() -> impl IntoView {
                view! { <div><Card title=missing /></div> }
            }
        "#;
        let err = parse_model(bad).unwrap_err();
        assert!(err.contains("not a signal"), "{err}");
    }

    #[test]
    fn p3b_chain_struct_cancel_and_rejected_shapes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let source = std::fs::read_to_string(root.join("examples/async-dashboard/view.rs")).unwrap();
        let file = lower_file(&source, shell()).unwrap_or_else(|err| panic!("{err}"));
        let needles = [
            "quinn_cancelled: 0",
            "fn quinn_set_cancelled(n)",
            "mod.quinn_job_plan = quinn_job_plan",
            "mod.quinn_set_cancelled = quinn_set_cancelled",
            "\"chain:fetch_pair+decorate\"",
            "fetch_pair@0,1;decorate@prev",
            "quinn_pack.push(state.id)",
            "quinn_pack.push(state.page)",
            "state.stamp_arg = quinn_pack.to_json()",
            "\"\" + state.quinn_cancelled",
            "text: \"Loading the pair\"",
            "text: \"Cancelled: \"",
        ];
        let missing: Vec<_> = needles
            .into_iter()
            .filter(|needle| !file.contains(needle))
            .collect();
        assert!(missing.is_empty(), "missing {missing:?}\n{file}");

        let model = parse_model(&source).unwrap();
        let pair = model.jobs.iter().find(|job| job.name == "pair").unwrap();
        assert_eq!(pair.func, "chain:fetch_pair+decorate");
        assert_eq!(pair.plan, "fetch_pair@0,1;decorate@prev");
        assert_eq!(pair.deps, vec!["id".to_string(), "page".to_string()]);
        let place = model.jobs.iter().find(|job| job.name == "place").unwrap();
        assert!(place.plan.is_empty(), "a one-arg resource stays a single call");
        assert_eq!(place.func, "fetch_place");

        let mut sim = Sim::new(model);
        assert!(has_label(&sim, "Loading the pair"));
        assert!(has_label(&sim, "Cancelled: 0"));
        assert!(sim.click("stamp"));
        assert_eq!(
            sim.strings["stamp_arg"],
            r#"{"city":"Manila","note":"desk"}"#
        );
        let stamp_gen = sim.gen("stamp");
        assert!(sim.complete(
            "stamp",
            stamp_gen,
            crate::JobValue::from("Stamped Manila (desk)")
        ));
        assert!(has_label(&sim, "Stamped Manila (desk)"));
        let pair_gen = sim.gen("pair");
        assert!(sim.click("page_2"));
        assert!(sim.gen("pair") > pair_gen);
        assert_eq!(sim.strings["pair_phase"], "loading");
        assert!(has_label(&sim, "Loading the pair"));

        let branched = r#"
            #[component]
            fn App() -> impl IntoView {
                let note = LocalResource::new(|| async move {
                    if ready { fetch_note().await } else { fetch_feed().await }
                });
                view! { <span>{note}</span> }
            }
        "#;
        let err = parse_model(branched).unwrap_err();
        assert!(
            err.contains("unsupported job shape: if"),
            "{err}"
        );

        let method = r#"
            #[component]
            fn App() -> impl IntoView {
                let (city, set_city) = signal("Cebu".to_string());
                let keep = Action::new(|city: &String| async move { city.keep(city).await });
                view! {
                    <div>
                        <button on:click=move |_| { keep.dispatch(city.get()); }>"Keep"</button>
                        <span>{keep}</span>
                    </div>
                }
            }
        "#;
        let model = parse_model(method).unwrap();
        let keep = model.jobs.iter().find(|job| job.name == "keep").unwrap();
        assert_eq!(keep.func, "keep");
        assert!(keep.plan.is_empty());
    }

    #[test]
    fn action_version_starts_at_zero_and_unknown_classes_fail() {
        let source = r#"
            #[component]
            fn App() -> impl IntoView {
                let save = Action::new(|name: &String| async move { save_place(name).await });
                let note = LocalResource::new(|| async move { fetch_note().await });
                view! {
                    <div>
                        <span>{move || save.version()}</span>
                        <span>{note}</span>
                        <button on:click=move |_| { save.dispatch("Cebu"); }>"Save"</button>
                    </div>
                }
            }
        "#;
        let file = lower_file(source, shell()).unwrap_or_else(|err| panic!("{err}"));
        assert!(file.contains("save_gen: 0"), "{file}");
        assert!(file.contains("note_gen: 1"), "{file}");
        assert!(!file.contains("save_gen: 1"), "{file}");
        let sim = Sim::new(parse_model(source).unwrap());
        assert_eq!(sim.gen("save"), 0);
        assert_eq!(sim.gen("note"), 1);

        let bad = r#"
            #[component]
            fn App() -> impl IntoView {
                view! { <div class="bg-not-a-color p-4"><span>"Hi"</span></div> }
            }
        "#;
        let err = lower_file(bad, shell()).unwrap_err();
        assert!(err.contains("unknown class:"), "{err}");
        assert!(err.contains("bg-not-a-color"), "{err}");
    }
    /// Makepad fits an SVG to its geometry, not its viewBox. A full-viewBox
    /// rect in every icon makes the two the same, so a 24-unit icon scales
    /// as it does in a browser and its strokes stay inside the box.
    #[test]
    fn every_icon_spans_its_view_box() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../makepad/resources/icons");
        for name in crate::parse::ICONS {
            let path = dir.join(format!("{name}.svg"));
            let svg = std::fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
            assert!(svg.contains("viewBox=\"0 0 24 24\""), "{name}");
            assert!(
                svg.contains("<rect width=\"24\" height=\"24\" fill=\"none\" stroke=\"none\"/>"),
                "{name}.svg needs the full-viewBox rect"
            );
        }
    }

    /// Makepad keeps a view's old children when a render hands it none, so
    /// an off `<Show>` has to leave a hidden widget in its slot, and the on
    /// branch has to turn that slot back on.
    #[test]
    fn show_off_branch_hides_its_slot() {
        let src = r#"
            #[component]
            fn Counter() -> impl IntoView {
                let (open, set_open) = signal(false);
                view! {
                    <div class="flex flex-col">
                        <div class="w-[100px] h-[36px]">
                            <Show when=move || open.get()>
                                <div class="w-[100px] h-[36px] bg-white"></div>
                            </Show>
                        </div>
                        <button on:click=move |_| { set_open.set(true); }>"Open"</button>
                    </div>
                }
            }
        "#;
        let file = lower_file(src, shell()).unwrap();
        let off = file.find("fn quinn_show_open_1_off()").expect("off fn");
        assert!(file[off..].starts_with("fn quinn_show_open_1_off() {\n            RoundedView {\n                visible: false\n"), "{file}");
        let on = file.find("fn quinn_show_open_1_on(").expect("on fn");
        assert!(file[on..off].contains("visible: true"), "{file}");
    }

    /// CSS keeps children inside a border, and the top painted element takes
    /// the pointer.
    #[test]
    fn borders_inset_children_and_overlays_take_the_pointer() {
        let src = r#"
            #[component]
            fn Counter() -> impl IntoView {
                view! {
                    <div class="flex flex-col">
                        <div class="flex flex-col p-2 border border-gray-300">
                            <span>"Row"</span>
                        </div>
                        <div class="absolute inset-0 z-40 bg-black"></div>
                    </div>
                }
            }
        "#;
        let file = lower_file(src, shell()).unwrap();
        assert!(file.contains("padding: 9."), "{file}");
        assert!(file.contains("cursor: MouseCursor.Arrow"), "{file}");
    }

    /// A breakpoint that returns a row to the start has to say so: the
    /// default alignment is otherwise left out and the base one stays.
    #[test]
    fn breakpoint_can_reset_alignment_to_start() {
        let src = r#"
            #[component]
            fn Counter() -> impl IntoView {
                view! {
                    <div class="flex flex-col">
                        <div class="flex flex-col items-center justify-center bp1120:flex-row bp1120:justify-start bp1120:items-start">
                            <span>"Row"</span>
                        </div>
                    </div>
                }
            }
        "#;
        let file = lower_file(src, shell()).unwrap();
        assert!(
            file.contains("align: if state.quinn_width >= 1120. { Align{x: 0. y: 0.} }"),
            "{file}"
        );
    }

}
