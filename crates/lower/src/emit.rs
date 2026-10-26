//! Makepad `script_mod!` text. The header stays a fixed height so the
//! block the watcher keys on does not move when the view grows.

use std::cell::{Cell, RefCell};

use quinn_core::{Style, Tag};
use quinn_styles::{parse_classes, parse_classes_in, shell_variants, AlignSpec, FlowSpec, SizeSpec};

use crate::model::*;
use crate::Shell;

#[derive(Clone, Copy)]
struct SlotFrame {
    flow: FlowSpec,
    gap: f64,
    cross: Option<quinn_styles::CrossAlign>,
    /// `bp1120:flex-row bp1120:gap-2` on the parent: the flow and gap the
    /// wrapper switches to at that window width.
    wide: Option<SlotWide>,
}

#[derive(Clone, Copy)]
struct SlotWide {
    min_width: f64,
    flow: FlowSpec,
    gap: f64,
}

thread_local! {
    static DEFER_PORTALS: RefCell<Vec<Vec<ViewNode>>> = const { RefCell::new(Vec::new()) };
    /// Immediate children of an `md:flex-row items-stretch` parent fill the
    /// cross axis once the window is wide enough. The static parent flow is
    /// still a column, so height would otherwise stay Fit.
    static ROW_AT: Cell<Option<f64>> = const { Cell::new(None) };
    /// `{children()}` is one returned widget, so several children share a
    /// wrapper. That wrapper has to follow the parent row, or an icon and a
    /// label stack and the label is clipped by the control height.
    static SLOT_FRAME: Cell<SlotFrame> = const {
        Cell::new(SlotFrame {
            flow: FlowSpec::Column,
            gap: 0.0,
            cross: None,
            wide: None,
        })
    };
}

fn bind_slot_frame(flow: FlowSpec, style: &Style, class: &str) -> SlotFrame {
    let gap = style.gap.unwrap_or(0.0);
    let mut wide: Option<SlotWide> = None;
    let mut merged = style.clone();
    for when in shell_variants(class) {
        let (false, Some(min_width)) = (when.dark, when.min_width) else {
            continue;
        };
        merged.merge_from(&parse_classes(&when.token));
        wide = Some(SlotWide {
            min_width,
            flow: merged.flow.unwrap_or(flow),
            gap: merged.gap.unwrap_or(gap),
        });
    }
    SLOT_FRAME.with(|cell| {
        cell.replace(SlotFrame {
            flow,
            gap,
            cross: style.cross_align,
            wide,
        })
    })
}

fn clear_portals() {
    DEFER_PORTALS.with(|slot| slot.borrow_mut().clear());
}

fn defer_portal(nodes: Vec<ViewNode>) {
    DEFER_PORTALS.with(|slot| slot.borrow_mut().push(nodes));
}

fn drain_portals() -> Vec<ViewNode> {
    DEFER_PORTALS.with(|slot| slot.borrow_mut().drain(..).flatten().collect())
}

pub fn emit_rust(model: &Model, shell: &Shell) -> String {
    clear_portals();
    reset_gates();
    let mut body = String::new();
    body.push_str("        use mod.prelude.widgets.*\n");
    body.push_str("        use mod.widgets.*\n");
    // A missing `mod.state` is an error, not nil. `ok{...}` turns that into nil.
    body.push_str("        if ok{mod.state} == nil {\n");
    body.push_str("            mod.state = {\n");
    for line in seed_lines(model, shell) {
        body.push_str(&format!("                {line}\n"));
    }
    body.push_str("            }\n");
    body.push_str("        }\n");
    body.push_str("        let state = mod.state\n");
    body.push_str(
        r#"        fn quinn_min(a, b) {
            if a < b { return a }
            return b
        }
        fn quinn_max(a, b) {
            if a > b { return a }
            return b
        }
        fn quinn_clamp(v, lo, hi) {
            return quinn_max(lo, quinn_min(hi, v))
        }
"#,
    );
    body.push_str(&emit_shell_fns(shell));
    body.push_str(&emit_keydown(&model.nodes));
    body.push_str(&emit_job_fns(model));
    body.push_str(&emit_ctx(model));
    if let Some(signal) = &model.frame {
        body.push_str("        fn quinn_frame() {\n");
        body.push_str(&format!("            state.{signal} += 1\n"));
        body.push_str("            ui.host.render()\n");
        body.push_str("        }\n");
        // `script_eval!` from Rust is a different body. It sees `mod`, not this scope.
        body.push_str("        mod.quinn_frame = quinn_frame\n");
    }
    if model.keyed {
        body.push_str(
            r#"        fn quinn_row_n(id) {
            let key = "k" + id
            let n = state.counts[key]
            if n == nil { return 0 }
            return n
        }
        fn quinn_row_bump(id) {
            let key = "k" + id
            if state.counts[key] == nil {
                state.counts[key] = 1
            } else {
                state.counts[key] = state.counts[key] + 1
            }
        }
"#,
        );
    }
    if has_list_op(&model.nodes, ListOp::Reverse) {
        body.push_str(
            r#"        fn quinn_reverse(items) {
            let n = items.len()
            let out = []
            var i = n
            loop {
                if i <= 0 { break }
                i -= 1
                out.push(items[i])
            }
            return out
        }
"#,
        );
    }
    if has_list_op(&model.nodes, ListOp::Remove) {
        body.push_str(
            r#"        fn quinn_remove_key(items, key) {
            let out = []
            for item in items {
                if item.id != key {
                    out.push(item)
                } else {
                    quinn_skip()
                }
            }
            return out
        }
        fn quinn_skip() { 0 }
"#,
        );
    }
    if has_list_op(&model.nodes, ListOp::Swap) {
        body.push_str(
            r#"        fn quinn_swap_key(items, a, b) {
            let out = []
            var i = 0
            let n = items.len()
            let left = quinn_by_key(items, a)
            let right = quinn_by_key(items, b)
            loop {
                if i >= n { break }
                let item = items[i]
                out.push(quinn_swap_pick(item, a, b, left, right))
                i += 1
            }
            return out
        }
        fn quinn_swap_pick(item, a, b, left, right) {
            if item.id == a { right } else { if item.id == b { left } else { item } }
        }
        fn quinn_by_key(items, key) {
            for item in items {
                if item.id == key { return item }
            }
            return items[0]
        }
"#,
        );
    }
    if model.keyed {
        body.push_str(&emit_row_reset(model));
    }
    let shows = collect_shows(&model.nodes);
    let mut slots = String::new();
    let mut routes = String::new();
    let pass_root = has_input(&model.nodes);
    let show_fns = emit_show_fns(&model.nodes, &shows, &mut slots, &mut routes, pass_root);
    each_for(
        &model.nodes,
        &mut |fn_name, params, row_signals, child, index| {
            body.push_str(&emit_row_fn(
                fn_name,
                params,
                row_signals,
                child,
                index,
                &mut slots,
                &mut routes,
                pass_root,
            ));
        },
    );
    if model.retain {
        body.push_str(&emit_retain(model));
    }
    // The host is the view whose `on_render` builds the tree. Makepad does not
    // repaint that view's own `draw_bg` when the script runs again, so a theme
    // toggle left the page fill on the first color (the counter card went dark
    // and the window behind it stayed light). The fill lives on `app`, a child.
    let host = "                            flow: Overlay\n                            width: Fill\n                            height: Fill\n                            show_bg: false\n"
        .to_string();
    let mut children = String::new();
    let mut cursor = ShowCursor {
        sites: &shows,
        index: 0,
    };
    // TextInput fires `on_change` with `vm.call`, which has no widget uid, so
    // `ui.host` from that closure cannot see the app host. `on_render` runs on
    // the host view; capturing it lets the input call `root.render()`.
    if has_input(&model.nodes) {
        children.push_str("                                let root = ui\n");
    }
    if model.jobs.iter().any(|job| !job.deps.is_empty()) {
        children.push_str("                                quinn_sync_jobs()\n");
    }
    let root_style = parse_classes(&model.root_class);
    let root_flow = root_style.flow.unwrap_or(FlowSpec::Column);
    let root_stretch = root_style.cross_align == Some(quinn_styles::CrossAlign::Stretch);
    // The root `<div>` is unwrapped into the app column. An `absolute` child
    // there used to be a `Fill` sibling and stole the column, which is why the
    // toast painted as a full-width bar over a blank page. Lift those children
    // onto the host overlay, next to portals.
    let mut flow_nodes = Vec::new();
    let mut abs_nodes = Vec::new();
    for node in &model.nodes {
        if child_is_absolute(node) {
            abs_nodes.push(node);
        } else {
            flow_nodes.push(node);
        }
    }
    let mut app_body = String::new();
    for node in &flow_nodes {
        app_body.push_str(&emit_node(
            node,
            9,
            None,
            root_flow,
            root_stretch,
            &mut cursor,
            &mut slots,
            &mut routes,
            pass_root,
        ));
    }
    let mut root_abs = String::new();
    for node in &abs_nodes {
        root_abs.push_str(&emit_node(
            node,
            8,
            None,
            FlowSpec::Column,
            false,
            &mut cursor,
            &mut slots,
            &mut routes,
            pass_root,
        ));
    }
    let mut portal_body = String::new();
    loop {
        let lifted = drain_portals();
        if lifted.is_empty() {
            break;
        }
        for node in &lifted {
            portal_body.push_str(&emit_node(
                node,
                8,
                None,
                FlowSpec::Column,
                false,
                &mut cursor,
                &mut slots,
                &mut routes,
                pass_root,
            ));
        }
    }
    let app_fields = host_fields(&model.root_class, false);
    children.push_str(&format!(
        "                                app := RoundedView {{\n{app_fields}{app_body}                                }}\n"
    ));
    children.push_str(&root_abs);
    if !portal_body.is_empty() {
        children.push_str(&format!(
            "                                quinn_portal := QuinnLayer {{\n{portal_body}                                }}\n"
        ));
    }
    body.push_str(&slots);
    body.push_str(&show_fns);
    body.push_str(&routes);
    let title = shell.title.replace('\\', "\\\\").replace('"', "\\\"");
    body.push_str(&format!(
        r#"        startup() do #(crate::App::script_component(vm)) {{
            ui: Root {{
                on_startup: || {{
                    ui.host.render()
                }}
                main_window := Window {{
                    window.inner_size: vec2({width}, {height})
                    window.title: "{title}"
                    body +: {{
                        host := RoundedView {{
{host}
                            on_render: || {{
{children}                            }}
                        }}
                    }}
                }}
            }}
        }}
"#,
        width = shell.width,
        height = shell.height,
    ));
    format!(
        r#"// Generated by quinn-lower. Do not edit.
// This header is stable so the block token stays on the same line.
use quinn::makepad_widgets::*;

script_mod! {{
{body}}}
"#
    )
}

fn has_input(nodes: &[ViewNode]) -> bool {
    fn el(el: &El) -> bool {
        el.input.is_some() || el.children.iter().any(walk)
    }
    fn walk(node: &ViewNode) -> bool {
        match node {
            ViewNode::El(item) => el(item),
            ViewNode::Show { child, .. } => walk(child),
            ViewNode::For { child, .. } => el(child),
            ViewNode::Slot { nodes, .. } => nodes.iter().any(walk),
            ViewNode::Routes { routes } => routes.iter().any(|(_, nodes)| nodes.iter().any(walk)),
            ViewNode::Suspense { fallback, child, .. }
            | ViewNode::Boundary { fallback, child, .. } => {
                fallback.iter().any(walk) || walk(child)
            }
            ViewNode::Portal { nodes } => nodes.iter().any(walk),
        }
    }
    nodes.iter().any(walk)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListOp {
    Reverse,
    Remove,
    Swap,
}

fn has_list_op(nodes: &[ViewNode], op: ListOp) -> bool {
    fn stmt(stmts: &[HandlerStmt], op: ListOp) -> bool {
        stmts.iter().any(|stmt| match (op, stmt) {
            (ListOp::Reverse, HandlerStmt::Reverse { .. }) => true,
            (ListOp::Remove, HandlerStmt::RemoveKey { .. }) => true,
            (ListOp::Swap, HandlerStmt::SwapKey { .. }) => true,
            _ => false,
        })
    }
    fn act(act: &Act, op: ListOp) -> bool {
        match (op, act) {
            (ListOp::Reverse, Act::Reverse(_)) => true,
            (ListOp::Reverse, Act::Body(stmts) | Act::Row(stmts)) => stmt(stmts, op),
            (_, Act::Body(stmts) | Act::Row(stmts)) => stmt(stmts, op),
            _ => false,
        }
    }
    fn el(el: &El, op: ListOp) -> bool {
        el.click.as_ref().is_some_and(|click| act(click, op))
            || el.children.iter().any(|node| walk(node, op))
    }
    fn walk(node: &ViewNode, op: ListOp) -> bool {
        match node {
            ViewNode::El(item) => el(item, op),
            ViewNode::Show { child, .. } => walk(child, op),
            ViewNode::For { child, .. } => el(child, op),
            ViewNode::Slot { nodes, .. } => nodes.iter().any(|node| walk(node, op)),
            ViewNode::Routes { routes } => routes
                .iter()
                .any(|(_, nodes)| nodes.iter().any(|node| walk(node, op))),
            ViewNode::Suspense { fallback, child, .. }
            | ViewNode::Boundary { fallback, child, .. } => {
                fallback.iter().any(|node| walk(node, op)) || walk(child, op)
            }
            ViewNode::Portal { nodes } => nodes.iter().any(|node| walk(node, op)),
        }
    }
    nodes.iter().any(|node| walk(node, op))
}

fn emit_row_reset(model: &Model) -> String {
    let mut lines = vec![
        "        fn quinn_row_reset(id) {".to_string(),
        "            let key = \"k\" + id".to_string(),
        "            state.counts[key] = 0".to_string(),
    ];
    for (name, init) in row_maps(model) {
        let value = match init {
            RowInit::Bool => "false",
            RowInit::Int => "0",
            RowInit::Str => "\"\"",
        };
        lines.push(format!("            state.{name}[key] = {value}"));
    }
    lines.push("        }".into());
    lines.join("\n") + "\n"
}

#[derive(Clone, Copy)]
enum RowInit {
    Bool,
    Int,
    Str,
}

fn each_for(
    nodes: &[ViewNode],
    visit: &mut dyn FnMut(&str, &[String], &[Signal], &El, Option<&str>),
) {
    for node in nodes {
        match node {
            ViewNode::For {
                fn_name,
                params,
                row_signals,
                child,
                index,
                ..
            } => visit(fn_name, params, row_signals, child, index.as_deref()),
            ViewNode::El(el) => each_for(&el.children, visit),
            ViewNode::Show { child, .. } => each_for(std::slice::from_ref(child), visit),
            ViewNode::Slot { nodes, .. } => each_for(nodes, visit),
            ViewNode::Routes { routes } => {
                for (_, nodes) in routes {
                    each_for(nodes, visit);
                }
            }
            ViewNode::Suspense { fallback, child, .. }
            | ViewNode::Boundary { fallback, child, .. } => {
                each_for(fallback, visit);
                each_for(std::slice::from_ref(child), visit);
            }
            ViewNode::Portal { nodes } => each_for(nodes, visit),
        }
    }
}

fn row_maps(model: &Model) -> Vec<(String, RowInit)> {
    let mut out = Vec::new();
    each_for(&model.nodes, &mut |_, _, row_signals, child, _index| {
        let count = count_signal(row_signals);
        for signal in row_signals {
            if Some(signal.read.as_str()) == count {
                continue;
            }
            let init = match &signal.init {
                Init::Bool(_) => RowInit::Bool,
                Init::Str(_) | Init::Obj(_) => RowInit::Str,
                Init::Int(_) | Init::Items(_) => RowInit::Int,
            };
            let name = format!("r_{}", signal.read);
            if !out.iter().any(|(have, _)| have == &name) {
                out.push((name, init));
            }
        }
        for flag in row_live_flags(child) {
            let name = format!("r_{flag}");
            if !out.iter().any(|(have, _)| have == &name) {
                out.push((name, RowInit::Int));
            }
        }
    });
    out
}

fn count_signal(signals: &[Signal]) -> Option<&str> {
    signals
        .iter()
        .find(|signal| matches!(signal.init, Init::Int(_)))
        .map(|signal| signal.read.as_str())
}

fn row_live_flags(el: &El) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(node: &ViewNode, out: &mut Vec<String>) {
        match node {
            ViewNode::Show {
                live_flag, child, ..
            } => {
                out.push(live_flag.clone());
                walk(child, out);
            }
            ViewNode::El(el) => {
                for child in &el.children {
                    walk(child, out);
                }
            }
            ViewNode::For { .. } => {}
            ViewNode::Slot { nodes, .. } => {
                for child in nodes {
                    walk(child, out);
                }
            }
            ViewNode::Routes { routes } => {
                for (_, nodes) in routes {
                    for child in nodes {
                        walk(child, out);
                    }
                }
            }
            ViewNode::Suspense { fallback, child, .. }
            | ViewNode::Boundary { fallback, child, .. } => {
                for node in fallback {
                    walk(node, out);
                }
                walk(child, out);
            }
            ViewNode::Portal { nodes } => {
                for node in nodes {
                    walk(node, out);
                }
            }
        }
    }
    for child in &el.children {
        walk(child, &mut out);
    }
    out
}

fn seed_obj(fields: &[(String, Init)]) -> String {
    let body = fields
        .iter()
        .map(|(name, init)| format!("{name}: {}", seed_init(init)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{{body}}}")
}

fn seed_init(init: &Init) -> String {
    match init {
        Init::Int(value) => value.to_string(),
        Init::Bool(value) => value.to_string(),
        Init::Str(value) => format!("\"{}\"", escape_script(value)),
        Init::Items(_) => "[]".into(),
        Init::Obj(fields) => seed_obj(fields),
    }
}

fn emit_retain(model: &Model) -> String {
    let open = model
        .signals
        .iter()
        .find(|signal| matches!(signal.init, Init::Bool(_)))
        .map(|signal| signal.read.as_str())
        .unwrap_or("open");
    let items = model
        .signals
        .iter()
        .find(|signal| matches!(signal.init, Init::Items(_)))
        .map(|signal| signal.read.as_str())
        .unwrap_or("items");
    let next = model
        .signals
        .iter()
        .find(|signal| signal.read.contains("next"))
        .map(|signal| signal.read.as_str())
        .unwrap_or("next_id");
    let label = model
        .signal(items)
        .and_then(|signal| match &signal.init {
            Init::Items(rows) => rows.first().map(|item| item.label.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "Alpha".into());
    let label = escape_script(&label);
    format!(
        r#"        fn quinn_retain_step(i) {{
            state.{open} = if i % 2 == 0 {{ true }} else {{ false }}
            if state.{open} {{
                let id = state.{next}
                state.{next} = id + 1
                state.{items}.push({{id: id, label: "Item " + id}})
            }} else {{
                if state.{items}.len() > 1 {{
                    state.{items}.pop()
                }}
            }}
            ui.host.render()
        }}
        fn quinn_retain_reset() {{
            state.{open} = false
            state.{items} = [{{id: 1, label: "{label}"}}]
            state.{next} = 2
            ui.host.render()
        }}
        mod.quinn_retain_step = quinn_retain_step
        mod.quinn_retain_reset = quinn_retain_reset
"#
    )
}

/// `mod.quinn_keydown(key)`, from the first `on:keydown` in the view. The
/// host calls it for Escape.
fn emit_keydown(nodes: &[ViewNode]) -> String {
    fn find(nodes: &[ViewNode]) -> Option<&Act> {
        nodes.iter().find_map(|node| match node {
            ViewNode::El(el) => el.keydown.as_ref().or_else(|| find(&el.children)),
            ViewNode::Slot { nodes, .. } => find(nodes),
            _ => None,
        })
    }
    let body = match find(nodes) {
        Some(act) => emit_act(act, None, "        "),
        None => "            0\n".into(),
    };
    format!("        fn quinn_keydown(quinn_key) {{\n{body}        }}\n        mod.quinn_keydown = quinn_keydown\n")
}

fn emit_shell_fns(shell: &Shell) -> String {
    let _ = shell;
    r#"        fn quinn_apply_width(w) {
            state.quinn_width = w
            ui.host.render()
        }
        fn quinn_skip_width() { 0 }
        fn quinn_set_width(w) {
            if state.quinn_width != w { quinn_apply_width(w) } else { quinn_skip_width() }
        }
        fn quinn_theme_on() { state.quinn_dark = 1 }
        fn quinn_theme_off() { state.quinn_dark = 0 }
        mod.quinn_set_width = quinn_set_width
"#
    .to_string()
}

fn emit_job_fns(model: &Model) -> String {
    if model.jobs.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let n = model.jobs.len();
    out.push_str(&format!("        fn quinn_job_count() {{ {n} }}\n"));
    out.push_str("        fn quinn_job_fn(i) {\n");
    out.push_str(&chain_expr(
        &model
            .jobs
            .iter()
            .map(|job| format!("\"{}\"", escape_script(&job.func)))
            .collect::<Vec<_>>(),
    ));
    out.push_str("        }\n");
    out.push_str("        fn quinn_job_key(i) {\n");
    out.push_str(&chain_expr(
        &model
            .jobs
            .iter()
            .map(|job| format!("\"{}\"", escape_script(&job.name)))
            .collect::<Vec<_>>(),
    ));
    out.push_str("        }\n");
    out.push_str("        fn quinn_arg_missing() { \"\" }\n");
    out.push_str("        fn quinn_plan_missing() { \"\" }\n");
    for job in &model.jobs {
        out.push_str(&emit_arg_fn(job));
        out.push_str(&format!(
            "        fn quinn_plan_{name}() {{ \"{}\" }}\n",
            escape_script(&job.plan),
            name = job.name
        ));
    }
    let arg_calls: Vec<String> = model
        .jobs
        .iter()
        .map(|job| format!("quinn_arg_{}()", job.name))
        .collect();
    let plan_calls: Vec<String> = model
        .jobs
        .iter()
        .map(|job| format!("quinn_plan_{}()", job.name))
        .collect();
    out.push_str("        fn quinn_job_arg(i) {\n");
    out.push_str(&chain_calls(&arg_calls, "quinn_arg_missing()"));
    out.push_str("        }\n");
    out.push_str("        fn quinn_job_plan(i) {\n");
    out.push_str(&chain_calls(&plan_calls, "quinn_plan_missing()"));
    out.push_str("        }\n");
    out.push_str("        fn quinn_job_gen(i) {\n");
    out.push_str(&chain_expr(
        &model
            .jobs
            .iter()
            .map(|job| match job.kind {
                JobKind::Action => format!(
                    "if state.{}_pending == true {{ state.{}_gen }} else {{ 0 }}",
                    job.name, job.name
                ),
                JobKind::Local | JobKind::Resource => format!(
                    "if state.{}_phase == \"loading\" {{ state.{}_gen }} else {{ 0 }}",
                    job.name, job.name
                ),
            })
            .collect::<Vec<_>>(),
    ));
    out.push_str("        }\n");
    let names: Vec<String> = model.jobs.iter().map(|job| job.name.clone()).collect();
    out.push_str("        fn quinn_dead() { 0 }\n");
    out.push_str("        fn quinn_live_yes() { 1 }\n");
    out.push_str("        fn quinn_skip_job() { 1 }\n");
    out.push_str("        fn quinn_job_live(name) {\n");
    out.push_str(&chain_named(
        &names,
        |_| "quinn_live_yes()".into(),
        "quinn_dead()",
    ));
    out.push_str("        }\n");
    out.push_str("        fn quinn_apply(name, gen, phase, value, err) {\n");
    out.push_str(
        "            if quinn_job_live(name) == 1 { quinn_apply_go(name, gen, phase, value, err) } else { quinn_skip_job() }\n",
    );
    out.push_str("        }\n");
    out.push_str("        fn quinn_apply_go(name, gen, phase, value, err) {\n");
    out.push_str(&chain_named(
        &names,
        |name| format!("quinn_write_{name}(gen, phase, value, err)"),
        "quinn_skip_job()",
    ));
    out.push_str("        }\n");
    for job in &model.jobs {
        out.push_str(&format!(
            "        fn quinn_write_{name}(gen, phase, value, err) {{\n            if state.{name}_gen == gen {{ quinn_commit_{name}(phase, value, err) }} else {{ quinn_skip_job() }}\n        }}\n",
            name = job.name
        ));
        out.push_str(&format!(
            "        fn quinn_commit_{name}(phase, value, err) {{\n            state.{name}_phase = phase\n            state.{name}_value = value\n            state.{name}_error = err\n",
            name = job.name
        ));
        if matches!(job.kind, JobKind::Action) {
            out.push_str(&format!("            state.{}_pending = false\n", job.name));
        }
        // `render()` here re-enters the script VM from inside `script_eval`
        // and the apply's return value never reaches Rust, so the delivery
        // is retried forever. The host renders once after a successful apply.
        // 2 means the generation matched and the value was written. 1, from
        // `quinn_skip_job`, means the result was stale or the binding is gone.
        out.push_str("            2\n        }\n");
    }
    let tracked: Vec<&Job> = model.jobs.iter().filter(|job| !job.deps.is_empty()).collect();
    if !tracked.is_empty() {
        out.push_str("        fn quinn_sync_jobs() {\n");
        for job in &tracked {
            let source = source_expr(&job.deps);
            out.push_str(&format!(
                "            if state.{name}_source != {source} {{ quinn_resync_{name}() }} else {{ 0 }}\n",
                name = job.name
            ));
        }
        out.push_str("        }\n");
        for job in &tracked {
            let source = source_expr(&job.deps);
            out.push_str(&format!(
                "        fn quinn_resync_{name}() {{\n            state.{name}_source = {source}\n            state.{name}_phase = \"loading\"\n            state.{name}_gen = state.{name}_gen + 1\n        }}\n",
                name = job.name
            ));
        }
    }
    out.push_str("        fn quinn_skip_cancelled() { 1 }\n");
    out.push_str("        fn quinn_write_cancelled(n) {\n            state.quinn_cancelled = n\n            2\n        }\n");
    out.push_str(
        "        fn quinn_set_cancelled(n) {\n            if state.quinn_cancelled != n { quinn_write_cancelled(n) } else { quinn_skip_cancelled() }\n        }\n",
    );
    out.push_str("        mod.quinn_job_count = quinn_job_count\n");
    out.push_str("        mod.quinn_job_fn = quinn_job_fn\n");
    out.push_str("        mod.quinn_job_key = quinn_job_key\n");
    out.push_str("        mod.quinn_job_arg = quinn_job_arg\n");
    out.push_str("        mod.quinn_job_plan = quinn_job_plan\n");
    out.push_str("        mod.quinn_job_gen = quinn_job_gen\n");
    out.push_str("        mod.quinn_job_live = quinn_job_live\n");
    out.push_str("        mod.quinn_apply = quinn_apply\n");
    out.push_str("        mod.quinn_set_cancelled = quinn_set_cancelled\n");
    out
}

fn emit_arg_fn(job: &Job) -> String {
    let body = match &job.arg {
        ArgSrc::List(items) => {
            let mut lines = String::from("let quinn_pack = []\n".to_string());
            for item in items {
                lines.push_str(&format!(
                    "            quinn_pack.push({})\n",
                    pack_item(&job.name, item)
                ));
            }
            lines.push_str("            quinn_pack.to_json()");
            lines
        }
        ArgSrc::Obj(fields) => {
            let obj = fields
                .iter()
                .map(|(key, value)| format!("{key}: {}", pack_item(&job.name, value)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("let quinn_pack = {{{obj}}}\n            quinn_pack.to_json()")
        }
        other => arg_value(&job.name, other),
    };
    format!(
        "        fn quinn_arg_{name}() {{\n            {body}\n        }}\n",
        name = job.name
    )
}

fn pack_item(job: &str, item: &PackItem) -> String {
    match item {
        PackItem::Signal(name) => format!("state.{name}"),
        PackItem::Lit(text) => format!("\"{}\"", escape_script(text)),
        PackItem::Source => format!("state.{job}_source"),
        PackItem::Dispatch => format!("state.{job}_arg"),
    }
}

fn arg_value(job: &str, arg: &ArgSrc) -> String {
    match arg {
        ArgSrc::None => "\"\"".into(),
        ArgSrc::Lit(text) => format!("\"{}\"", escape_script(text)),
        ArgSrc::Signal(name) => format!("state.{name}"),
        ArgSrc::Source => format!("state.{job}_source"),
        ArgSrc::Dispatch => format!("state.{job}_arg"),
        ArgSrc::List(_) | ArgSrc::Obj(_) => "\"\"".into(),
    }
}

fn chain_calls(calls: &[String], fallback: &str) -> String {
    if calls.is_empty() {
        return format!("            {fallback}\n");
    }
    fn nest(calls: &[String], index: usize, fallback: &str) -> String {
        if index + 1 >= calls.len() {
            return format!(
                "if i == {index} {{ {} }} else {{ {fallback} }}",
                calls[index]
            );
        }
        format!(
            "if i == {index} {{ {} }} else {{ {} }}",
            calls[index],
            nest(calls, index + 1, fallback)
        )
    }
    format!("            {}\n", nest(calls, 0, fallback))
}

fn source_expr(deps: &[String]) -> String {
    if deps.len() == 1 {
        return format!("state.{}", deps[0]);
    }
    deps.iter()
        .map(|dep| format!("(\"\" + state.{dep})"))
        .collect::<Vec<_>>()
        .join(" + \"|\" + ")
}

fn chain_named(names: &[String], body: impl Fn(&str) -> String, fallback: &str) -> String {
    let arms: Vec<String> = names
        .iter()
        .map(|name| format!("if name == \"{}\" {{ {} }}", escape_script(name), body(name)))
        .collect();
    if arms.is_empty() {
        return format!("            {fallback}\n");
    }
    fn nest(arms: &[String], index: usize, fallback: &str) -> String {
        if index + 1 >= arms.len() {
            return format!("{} else {{ {fallback} }}", arms[index]);
        }
        format!(
            "{} else {{ {} }}",
            arms[index],
            nest(arms, index + 1, fallback)
        )
    }
    format!("            {}\n", nest(&arms, 0, fallback))
}

fn chain_expr(values: &[String]) -> String {
    if values.is_empty() {
        return "            0\n".into();
    }
    fn nest(values: &[String], index: usize) -> String {
        if index + 1 >= values.len() {
            return format!("if i == {index} {{ {} }} else {{ 0 }}", values[index]);
        }
        format!(
            "if i == {index} {{ {} }} else {{ {} }}",
            values[index],
            nest(values, index + 1)
        )
    }
    format!("            {}\n", nest(values, 0))
}

fn source_seed(model: &Model, deps: &[String]) -> String {
    if deps.len() == 1 {
        return signal_literal(model, &deps[0]);
    }
    let joined = deps
        .iter()
        .map(|dep| signal_seed_text(model, dep))
        .collect::<Vec<_>>()
        .join("|");
    format!("\"{}\"", escape_script(&joined))
}

fn signal_literal(model: &Model, name: &str) -> String {
    model
        .signal(name)
        .map(|signal| match &signal.init {
            Init::Str(text) => format!("\"{}\"", escape_script(text)),
            Init::Int(value) => value.to_string(),
            Init::Bool(value) => value.to_string(),
            _ => "\"\"".into(),
        })
        .unwrap_or_else(|| "\"\"".into())
}

fn signal_seed_text(model: &Model, name: &str) -> String {
    model
        .signal(name)
        .map(|signal| match &signal.init {
            Init::Str(text) => text.clone(),
            Init::Int(value) => value.to_string(),
            Init::Bool(value) => value.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default()
}

fn seed_lines(model: &Model, shell: &Shell) -> Vec<String> {
    let mut lines = Vec::new();
    for signal in &model.signals {
        lines.push(match &signal.init {
            Init::Int(value) => format!("{}: {value}", signal.read),
            Init::Bool(value) => format!("{}: {value}", signal.read),
            Init::Str(value) => format!("{}: \"{}\"", signal.read, escape_script(value)),
            Init::Obj(fields) => format!("{}: {}", signal.read, seed_obj(fields)),
            Init::Items(items) => {
                let rows = items
                    .iter()
                    .map(|item| {
                        format!(
                            "{{id: {}, label: \"{}\"}}",
                            item.id,
                            escape_script(&item.label)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}: [{rows}]", signal.read)
            }
        });
    }
    if model.keyed {
        lines.push("counts: {}".into());
    }
    for (name, _) in row_maps(model) {
        lines.push(format!("{name}: {{}}"));
    }
    for flag in live_flags(&model.nodes) {
        lines.push(format!("{flag}: 0"));
    }
    lines.push(format!("quinn_width: {}.", shell.width));
    lines.push("quinn_dark: 0".into());
    lines.push("quinn_density: 1".into());
    if !model.jobs.is_empty() {
        lines.push("quinn_cancelled: 0".into());
    }
    for job in &model.jobs {
        let phase = match job.kind {
            JobKind::Action => "idle",
            JobKind::Local | JobKind::Resource => "loading",
        };
        lines.push(format!("{}_phase: \"{phase}\"", job.name));
        lines.push(format!("{}_value: \"\"", job.name));
        lines.push(format!("{}_error: \"\"", job.name));
        // Leptos `Action::version` is 0 until the first dispatch. Resources
        // still start at 1 so the initial load runs.
        let gen = if matches!(job.kind, JobKind::Action) {
            0
        } else {
            1
        };
        lines.push(format!("{}_gen: {gen}", job.name));
        if !job.deps.is_empty() {
            lines.push(format!(
                "{}_source: {}",
                job.name,
                source_seed(model, &job.deps)
            ));
        }
        if matches!(job.kind, JobKind::Action) {
            lines.push(format!("{}_pending: false", job.name));
            lines.push(format!("{}_arg: \"\"", job.name));
        }
    }
    lines
}

fn live_flags(nodes: &[ViewNode]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(node: &ViewNode, out: &mut Vec<String>) {
        match node {
            ViewNode::Show {
                live_flag, child, ..
            } => {
                out.push(live_flag.clone());
                walk(child, out);
            }
            ViewNode::El(el) => {
                for child in &el.children {
                    walk(child, out);
                }
            }
            ViewNode::For { .. } => {}
            ViewNode::Slot { nodes, .. } => {
                for child in nodes {
                    walk(child, out);
                }
            }
            ViewNode::Routes { routes } => {
                for (_, nodes) in routes {
                    for child in nodes {
                        walk(child, out);
                    }
                }
            }
            ViewNode::Suspense { fallback, child, .. }
            | ViewNode::Boundary { fallback, child, .. } => {
                for node in fallback {
                    walk(node, out);
                }
                walk(child, out);
            }
            ViewNode::Portal { nodes } => {
                for node in nodes {
                    walk(node, out);
                }
            }
        }
    }
    for node in nodes {
        walk(node, &mut out);
    }
    out
}

fn emit_row_fn(
    fn_name: &str,
    params: &[String],
    row_signals: &[Signal],
    child: &El,
    index: Option<&str>,
    slots: &mut String,
    routes: &mut String,
    pass_root: bool,
) -> String {
    let mut args = String::from("id");
    for param in params {
        args.push_str(", ");
        args.push_str(param);
    }
    if let Some(index) = index {
        args.push_str(", ");
        args.push_str(index);
        args.push_str(", quinn_odd");
    }
    let sites = collect_row_shows(child);
    let mut fns = String::new();
    {
        let mut cursor = ShowCursor {
            sites: &sites,
            index: 0,
        };
        emit_row_show_fns(
            &ViewNode::El(child.clone()),
            &args,
            row_signals,
            &mut cursor,
            &mut fns,
            slots,
            routes,
            pass_root,
        );
    }
    let row = RowEnv {
        fn_name,
        args: &args,
        signals: row_signals,
        count: count_signal(row_signals),
        striped: index.is_some(),
    };
    let mut cursor = ShowCursor {
        sites: &sites,
        index: 0,
    };
    let widget = emit_el(
        child,
        Some(&row),
        3,
        false,
        FlowSpec::Column,
        false,
        &mut cursor,
        slots,
        routes,
        pass_root,
    );
    format!("{fns}        fn {fn_name}({args}) {{\n            let key = \"k\" + id\n{widget}        }}\n")
}

struct RowEnv<'a> {
    fn_name: &'a str,
    args: &'a str,
    signals: &'a [Signal],
    count: Option<&'a str>,
    striped: bool,
}

fn collect_row_shows(el: &El) -> Vec<ShowSite> {
    let mut sites = Vec::new();
    let mut nth = std::collections::HashMap::<String, usize>::new();
    for child in &el.children {
        collect_show(child, &mut sites, &mut nth);
    }
    for site in &mut sites {
        site.name = format!("row_{}", site.name);
    }
    sites
}

struct ShowSite {
    name: String,
    /// Cursor index just after this show and the shows nested inside it.
    end: usize,
}

struct ShowCursor<'a> {
    sites: &'a [ShowSite],
    index: usize,
}

fn collect_shows(nodes: &[ViewNode]) -> Vec<ShowSite> {
    clear_portals();
    let mut sites = Vec::new();
    let mut nth = std::collections::HashMap::<String, usize>::new();
    for node in nodes {
        collect_show(node, &mut sites, &mut nth);
    }
    loop {
        let lifted = drain_portals();
        if lifted.is_empty() {
            break;
        }
        for node in &lifted {
            collect_show(node, &mut sites, &mut nth);
        }
    }
    sites
}

fn collect_show(
    node: &ViewNode,
    sites: &mut Vec<ShowSite>,
    nth: &mut std::collections::HashMap<String, usize>,
) {
    match node {
        ViewNode::El(el) => {
            for child in &el.children {
                collect_show(child, sites, nth);
            }
        }
        ViewNode::Show { signal, child, .. } => {
            let n = nth.entry(signal.clone()).or_insert(0);
            *n += 1;
            let name = format!("quinn_show_{signal}_{}", *n);
            let idx = sites.len();
            sites.push(ShowSite { name, end: 0 });
            collect_show(child, sites, nth);
            sites[idx].end = sites.len();
        }
        ViewNode::For { .. } => {}
        ViewNode::Slot { nodes, .. } => {
            for child in nodes {
                collect_show(child, sites, nth);
            }
        }
        ViewNode::Routes { routes } => {
            for (_, nodes) in routes {
                for child in nodes {
                    collect_show(child, sites, nth);
                }
            }
        }
        ViewNode::Suspense { fallback, child, .. } | ViewNode::Boundary { fallback, child, .. } => {
            for node in fallback {
                collect_show(node, sites, nth);
            }
            collect_show(child, sites, nth);
        }
        ViewNode::Portal { nodes } => defer_portal(nodes.clone()),
    }
}

/// A statement `if` with no widget on both sides unbalances Makepad's script
/// stack (the branch is popped onto the parent). Each show is a function whose
/// `if` calls two functions, one returning the branch widget and one returning
/// `0`, which a view ignores.
fn emit_show_fns(
    nodes: &[ViewNode],
    sites: &[ShowSite],
    slots: &mut String,
    routes: &mut String,
    pass_root: bool,
) -> String {
    let mut out = String::new();
    clear_portals();
    let mut cursor = ShowCursor { sites, index: 0 };
    for node in nodes {
        emit_show_fn(node, &mut cursor, &mut out, slots, routes, pass_root);
    }
    loop {
        let lifted = drain_portals();
        if lifted.is_empty() {
            break;
        }
        for node in &lifted {
            emit_show_fn(node, &mut cursor, &mut out, slots, routes, pass_root);
        }
    }
    out
}

fn emit_show_fn(
    node: &ViewNode,
    cursor: &mut ShowCursor<'_>,
    out: &mut String,
    slots: &mut String,
    routes: &mut String,
    pass_root: bool,
) {
    match node {
        ViewNode::El(el) => {
            for child in &el.children {
                emit_show_fn(child, cursor, out, slots, routes, pass_root);
            }
        }
        ViewNode::Show {
            signal,
            live_flag,
            reset,
            child,
        } => {
            let idx = cursor.index;
            let name = cursor.sites[idx].name.clone();
            cursor.index += 1;
            match &**child {
                ViewNode::El(el) => {
                    for nested in &el.children {
                        emit_show_fn(nested, cursor, out, slots, routes, pass_root);
                    }
                }
                other => emit_show_fn(other, cursor, out, slots, routes, pass_root),
            }
            let mut body_cursor = ShowCursor {
                sites: cursor.sites,
                index: idx + 1,
            };
            let body = match &**child {
                ViewNode::El(el) => emit_el(
                    el,
                    None,
                    3,
                    false,
                    FlowSpec::Column,
                    false,
                    &mut body_cursor,
                    slots,
                    routes,
                    pass_root,
                ),
                other => emit_node(
                    other,
                    3,
                    None,
                    FlowSpec::Column,
                    false,
                    &mut body_cursor,
                    slots,
                    routes,
                    pass_root,
                ),
            };
            let mut resets = format!("            let was = state.{live_flag}\n");
            resets.push_str(&format!(
                "            state.{live_flag} = if state.{signal} {{ 1 }} else {{ 0 }}\n"
            ));
            for field in reset {
                resets.push_str(&format!(
                    "            state.{field} = if state.{signal} {{ if was != 1 {{ 0 }} else {{ state.{field} }} }} else {{ 0 }}\n"
                ));
            }
            let on_fn = script_fn(&format!("{name}_on"), pass_root, false);
            let on_call = script_call(&format!("{name}_on"), pass_root, None);
            let show_fn = script_fn(&name, pass_root, false);
            out.push_str(&format!(
                "        {on_fn} {{\n{body}        }}\n        fn {name}_off() {{\n            0\n        }}\n        {show_fn} {{\n{resets}            if state.{signal} {{\n                {on_call}\n            }} else {{\n                {name}_off()\n            }}\n        }}\n"
            ));
        }
        ViewNode::For { .. } => {}
        ViewNode::Slot { nodes, .. } => {
            for child in nodes {
                emit_show_fn(child, cursor, out, slots, routes, pass_root);
            }
        }
        ViewNode::Routes { routes: route_list } => {
            for (_, nodes) in route_list {
                for child in nodes {
                    emit_show_fn(child, cursor, out, slots, routes, pass_root);
                }
            }
        }
        ViewNode::Suspense { fallback, child, .. } | ViewNode::Boundary { fallback, child, .. } => {
            for node in fallback {
                emit_show_fn(node, cursor, out, slots, routes, pass_root);
            }
            emit_show_fn(child, cursor, out, slots, routes, pass_root);
        }
        ViewNode::Portal { nodes } => defer_portal(nodes.clone()),
    }
}

fn emit_row_show_fns(
    node: &ViewNode,
    args: &str,
    row_signals: &[Signal],
    cursor: &mut ShowCursor<'_>,
    out: &mut String,
    slots: &mut String,
    routes: &mut String,
    pass_root: bool,
) {
    match node {
        ViewNode::El(el) => {
            for child in &el.children {
                emit_row_show_fns(
                    child,
                    args,
                    row_signals,
                    cursor,
                    out,
                    slots,
                    routes,
                    pass_root,
                );
            }
        }
        ViewNode::Show {
            signal,
            live_flag,
            reset,
            child,
        } => {
            let idx = cursor.index;
            let name = cursor.sites[idx].name.clone();
            cursor.index += 1;
            match &**child {
                ViewNode::El(el) => {
                    for nested in &el.children {
                        emit_row_show_fns(
                            nested,
                            args,
                            row_signals,
                            cursor,
                            out,
                            slots,
                            routes,
                            pass_root,
                        );
                    }
                }
                other => emit_row_show_fns(
                    other,
                    args,
                    row_signals,
                    cursor,
                    out,
                    slots,
                    routes,
                    pass_root,
                ),
            }
            let row = RowEnv {
                fn_name: "quinn_row",
                args,
                signals: row_signals,
                count: count_signal(row_signals),
                striped: args.contains("quinn_odd"),
            };
            let mut body_cursor = ShowCursor {
                sites: cursor.sites,
                index: idx + 1,
            };
            let body = match &**child {
                ViewNode::El(el) => emit_el(
                    el,
                    Some(&row),
                    3,
                    false,
                    FlowSpec::Column,
                    false,
                    &mut body_cursor,
                    slots,
                    routes,
                    pass_root,
                ),
                other => emit_node(
                    other,
                    3,
                    Some(&row),
                    FlowSpec::Column,
                    false,
                    &mut body_cursor,
                    slots,
                    routes,
                    pass_root,
                ),
            };
            let mut resets = String::from("            let key = \"k\" + id\n");
            resets.push_str(&format!(
                "            let was = if state.r_{live_flag}[key] == nil {{ 0 }} else {{ state.r_{live_flag}[key] }}\n"
            ));
            resets.push_str(&format!(
                "            let on = if state.r_{signal}[key] == nil {{ false }} else {{ state.r_{signal}[key] }}\n"
            ));
            resets.push_str(&format!(
                "            state.r_{live_flag}[key] = if on {{ 1 }} else {{ 0 }}\n"
            ));
            for field in reset {
                if count_signal(row_signals) == Some(field.as_str()) {
                    resets.push_str(&format!(
                        "            state.counts[key] = if on {{ if was != 1 {{ 0 }} else {{ state.counts[key] }} }} else {{ 0 }}\n"
                    ));
                } else {
                    resets.push_str(&format!(
                        "            state.r_{field}[key] = if on {{ if was != 1 {{ 0 }} else {{ state.r_{field}[key] }} }} else {{ 0 }}\n"
                    ));
                }
            }
            out.push_str(&format!(
                "        fn {name}_on({args}) {{\n            let key = \"k\" + id\n{body}        }}\n        fn {name}_off() {{\n            0\n        }}\n        fn {name}({args}) {{\n{resets}            if on {{\n                {name}_on({args})\n            }} else {{\n                {name}_off()\n            }}\n        }}\n"
            ));
        }
        ViewNode::For { .. } => {}
        ViewNode::Slot { nodes, .. } => {
            for child in nodes {
                emit_row_show_fns(
                    child,
                    args,
                    row_signals,
                    cursor,
                    out,
                    slots,
                    routes,
                    pass_root,
                );
            }
        }
        ViewNode::Routes { routes: route_list } => {
            for (_, nodes) in route_list {
                for child in nodes {
                    emit_row_show_fns(
                        child,
                        args,
                        row_signals,
                        cursor,
                        out,
                        slots,
                        routes,
                        pass_root,
                    );
                }
            }
        }
        ViewNode::Suspense { fallback, child, .. } | ViewNode::Boundary { fallback, child, .. } => {
            for node in fallback {
                emit_row_show_fns(
                    node, args, row_signals, cursor, out, slots, routes, pass_root,
                );
            }
            emit_row_show_fns(
                child, args, row_signals, cursor, out, slots, routes, pass_root,
            );
        }
        ViewNode::Portal { nodes } => {
            for node in nodes {
                emit_row_show_fns(
                    node, args, row_signals, cursor, out, slots, routes, pass_root,
                );
            }
        }
    }
}

fn emit_node(
    node: &ViewNode,
    indent: usize,
    row: Option<&RowEnv<'_>>,
    parent: FlowSpec,
    stretch: bool,
    shows: &mut ShowCursor<'_>,
    slots: &mut String,
    routes: &mut String,
    pass_root: bool,
) -> String {
    match node {
        ViewNode::El(el) => {
            let parsed = parse_classes(&el.class);
            if let Some(align) = parsed.align_self {
                if align != quinn_styles::CrossAlign::Stretch {
                    return emit_self_wrap(
                        el, align, indent, row, parent, shows, slots, routes, pass_root,
                    );
                }
                return emit_el(
                    el, row, indent, true, parent, true, shows, slots, routes, pass_root,
                );
            }
            emit_el(
                el, row, indent, true, parent, stretch, shows, slots, routes, pass_root,
            )
        }
        ViewNode::Show { .. } => {
            let pad = " ".repeat(indent * 4);
            let site = &shows.sites[shows.index];
            let name = site.name.clone();
            shows.index = site.end;
            let call = if let Some(row) = row {
                format!("{name}({})", row.args)
            } else {
                script_call(&name, pass_root, None)
            };
            format!("{pad}{call}\n")
        }
        ViewNode::For {
            items,
            var,
            key_field,
            fn_name,
            params,
            index,
            ..
        } => {
            let pad = " ".repeat(indent * 4);
            let mut call = format!("{fn_name}({var}.{key_field}");
            for param in params {
                call.push_str(&format!(", {var}.{param}"));
            }
            if index.is_some() {
                call.push_str(", quinn_i, quinn_odd");
            }
            call.push(')');
            if index.is_some() {
                format!(
                    "{pad}var quinn_i = 0\n{pad}var quinn_odd = 0\n{pad}for {var} in state.{items} {{\n{pad}    {call}\n{pad}    quinn_i += 1\n{pad}    if quinn_odd == 0 {{ quinn_odd = 1 }} else {{ quinn_odd = 0 }}\n{pad}}}\n"
                )
            } else {
                format!("{pad}for {var} in state.{items} {{\n{pad}    {call}\n{pad}}}\n")
            }
        }
        ViewNode::Slot {
            fn_name,
            nodes,
            arg,
        } => {
            if fn_name.is_empty() {
                let mut out = String::new();
                for child in nodes {
                    out.push_str(&emit_node(
                        child, indent, row, parent, stretch, shows, slots, routes, pass_root,
                    ));
                }
                return out;
            }
            let body = slot_body(
                nodes,
                SLOT_FRAME.with(Cell::get),
                shows,
                slots,
                routes,
                pass_root,
            );
            let call = script_call(fn_name, pass_root, arg.as_deref());
            slots.push_str(&format!(
                "        {} {{\n{body}        }}\n",
                script_fn(fn_name, pass_root, arg.is_some())
            ));
            let pad = " ".repeat(indent * 4);
            format!("{pad}{call}\n")
        }
        ViewNode::Routes { routes: pages } => {
            let pad = " ".repeat(indent * 4);
            let mut calls = String::new();
            for (path, nodes) in pages {
                let name = format!(
                    "quinn_route_{}",
                    routes.matches("if state.route ==").count() + 1
                );
                let body = slot_body(nodes, SlotFrame::column(), shows, slots, routes, pass_root);
                let path = escape_script(path);
                let on = script_call(&format!("{name}_on"), pass_root, None);
                let off = format!("{name}_off()");
                routes.push_str(&format!(
                    "        {} {{\n{body}        }}\n        fn {name}_off() {{\n            0\n        }}\n        {} {{\n            if state.route == \"{path}\" {{\n                {on}\n            }} else {{\n                {off}\n            }}\n        }}\n",
                    script_fn(&format!("{name}_on"), pass_root, false),
                    script_fn(&name, pass_root, false),
                ));
                calls.push_str(&format!("{pad}{}\n", script_call(&name, pass_root, None)));
            }
            calls
        }
        ViewNode::Suspense {
            jobs,
            transition,
            fallback,
            child,
        } => emit_suspense(
            jobs, *transition, fallback, child, indent, shows, slots, routes, pass_root,
        ),
        ViewNode::Boundary {
            jobs,
            fallback,
            child,
        } => emit_boundary(jobs, fallback, child, indent, shows, slots, routes, pass_root),
        ViewNode::Portal { nodes } => {
            defer_portal(nodes.clone());
            String::new()
        }
    }
}

fn script_fn(name: &str, pass_root: bool, arg: bool) -> String {
    let mut params = Vec::new();
    if pass_root {
        params.push("root");
    }
    if arg {
        params.push("arg");
    }
    format!("fn {name}({})", params.join(", "))
}

fn emit_suspense(
    jobs: &[String],
    transition: bool,
    fallback: &[ViewNode],
    child: &ViewNode,
    indent: usize,
    shows: &mut ShowCursor<'_>,
    slots: &mut String,
    routes: &mut String,
    pass_root: bool,
) -> String {
    let n = next_gate();
    let name = format!("quinn_gate_{n}");
    let ready = function_body(std::slice::from_ref(child), shows, slots, routes, pass_root);
    let fb = function_body(fallback, shows, slots, routes, pass_root);
    let on = script_fn(&format!("{name}_on"), pass_root, false);
    let off = script_fn(&format!("{name}_fb"), pass_root, false);
    let gate = script_fn(&name, pass_root, false);
    let call_on = script_call(&format!("{name}_on"), pass_root, None);
    let call_fb = script_call(&format!("{name}_fb"), pass_root, None);
    let loading = phase_any(jobs, "loading");
    let value = jobs
        .first()
        .map(|job| format!("state.{job}_value == \"\""))
        .unwrap_or_else(|| "true".into());
    let both = if transition {
        let both_fn = script_fn(&format!("{name}_both"), pass_root, false);
        let call_both = script_call(&format!("{name}_both"), pass_root, None);
        let wait_fn = script_fn(&format!("{name}_wait"), pass_root, false);
        let call_wait = script_call(&format!("{name}_wait"), pass_root, None);
        slots.push_str(&format!(
            "        {both_fn} {{\n            RoundedView {{\n                width: Fit\n                height: Fit\n                flow: Down\n                {call_fb}\n                {call_on}\n            }}\n        }}\n        {wait_fn} {{\n            if {value} {{\n                {call_fb}\n            }} else {{\n                {call_both}\n            }}\n        }}\n"
        ));
        format!("            {call_wait}\n")
    } else {
        format!("            {call_fb}\n")
    };
    slots.push_str(&format!(
        "        {on} {{\n{ready}        }}\n        {off} {{\n{fb}        }}\n        {gate} {{\n            if {loading} {{\n{both}            }} else {{\n                {call_on}\n            }}\n        }}\n"
    ));
    let pad = " ".repeat(indent * 4);
    format!(
        "{pad}{}\n",
        script_call(&name, pass_root, None)
    )
}

fn emit_boundary(
    jobs: &[String],
    fallback: &[ViewNode],
    child: &ViewNode,
    indent: usize,
    shows: &mut ShowCursor<'_>,
    slots: &mut String,
    routes: &mut String,
    pass_root: bool,
) -> String {
    let n = next_gate();
    let name = format!("quinn_err_{n}");
    let ready = function_body(std::slice::from_ref(child), shows, slots, routes, pass_root);
    let fb = function_body(fallback, shows, slots, routes, pass_root);
    let err = phase_any(jobs, "error");
    slots.push_str(&format!(
        "        {} {{\n{ready}        }}\n        {} {{\n{fb}        }}\n        {} {{\n            if {err} {{\n                {}\n            }} else {{\n                {}\n            }}\n        }}\n",
        script_fn(&format!("{name}_on"), pass_root, false),
        script_fn(&format!("{name}_fb"), pass_root, false),
        script_fn(&name, pass_root, false),
        script_call(&format!("{name}_fb"), pass_root, None),
        script_call(&format!("{name}_on"), pass_root, None),
    ));
    let pad = " ".repeat(indent * 4);
    format!("{pad}{}\n", script_call(&name, pass_root, None))
}

fn function_body(
    nodes: &[ViewNode],
    shows: &mut ShowCursor<'_>,
    slots: &mut String,
    routes: &mut String,
    pass_root: bool,
) -> String {
    slot_body(nodes, SlotFrame::column(), shows, slots, routes, pass_root)
}

fn phase_any(jobs: &[String], phase: &str) -> String {
    if jobs.is_empty() {
        return "false".into();
    }
    jobs.iter()
        .map(|job| format!("state.{job}_phase == \"{phase}\""))
        .collect::<Vec<_>>()
        .join(" || ")
}

thread_local! {
    static GATE: std::cell::Cell<u32> = const { std::cell::Cell::new(1) };
}

fn reset_gates() {
    GATE.with(|slot| slot.set(1));
}

fn next_gate() -> u32 {
    GATE.with(|slot| {
        let n = slot.get();
        slot.set(n + 1);
        n
    })
}

fn script_call(name: &str, pass_root: bool, arg: Option<&str>) -> String {
    let mut params = Vec::new();
    if pass_root {
        params.push("root".to_string());
    }
    if let Some(arg) = arg {
        params.push(arg.to_string());
    }
    format!("{name}({})", params.join(", "))
}

impl SlotFrame {
    const fn column() -> Self {
        Self {
            flow: FlowSpec::Column,
            gap: 0.0,
            cross: None,
            wide: None,
        }
    }
}

/// One script function returns one widget. Several children share a fit
/// wrapper that uses the parent's flow, gap, and cross alignment.
fn slot_body(
    nodes: &[ViewNode],
    frame: SlotFrame,
    shows: &mut ShowCursor<'_>,
    slots: &mut String,
    routes: &mut String,
    pass_root: bool,
) -> String {
    if nodes.is_empty() {
        return "            0\n".into();
    }
    if nodes.len() == 1 {
        // A `name := Widget` binding does not yield the widget, so the
        // function would return nil and the parent would skip it. The value
        // the function returns is an anonymous widget, the same shape as a
        // `<Show>` body. Children inside that widget stay named.
        return emit_returned(&nodes[0], shows, slots, routes, pass_root);
    }
    let mut inner = String::new();
    for node in nodes {
        inner.push_str(&emit_node(
            node,
            4,
            None,
            frame.flow,
            false,
            shows,
            slots,
            routes,
            pass_root,
        ));
    }
    let mut flow = slot_flow(frame.flow, frame.cross);
    let mut gap = script_num(frame.gap);
    if let Some(wide) = frame.wide {
        let at = script_num(wide.min_width);
        flow = format!(
            "if state.quinn_width >= {at} {{ {} }} else {{ {flow} }}",
            slot_flow(wide.flow, frame.cross)
        );
        gap = format!("if state.quinn_width >= {at} {{ {} }} else {{ {gap} }}", script_num(wide.gap));
    }
    let spacing = if frame.gap > 0.0 || frame.wide.is_some_and(|wide| wide.gap > 0.0) {
        format!("                spacing: {gap}\n")
    } else {
        String::new()
    };
    // A centered column parent centers this Fit wrapper; its own children
    // have to center inside it too, or an icon sits left of its caption.
    let align = if frame.flow == FlowSpec::Column
        && frame.cross == Some(quinn_styles::CrossAlign::Center)
    {
        "                align: Align{x: 0.5 y: 0.}\n"
    } else {
        ""
    };
    // A `w-full` child inside a `Fit` column wrapper has nothing to fill and
    // collapses to zero, which emptied every inspector row.
    let width = if frame.flow == FlowSpec::Column && frame.wide.is_none() && nodes.iter().any(fills_width) {
        "Fill"
    } else {
        "Fit"
    };
    format!("            RoundedView {{\n                width: {width}\n                height: Fit\n                flow: {flow}\n{spacing}{align}{inner}            }}\n")
}

fn fills_width(node: &ViewNode) -> bool {
    match node {
        ViewNode::El(el) => matches!(
            parse_classes(&el.class).width,
            Some(quinn_styles::SizeSpec::Fill)
        ),
        _ => false,
    }
}

fn slot_flow(flow: FlowSpec, cross: Option<quinn_styles::CrossAlign>) -> String {
    match flow {
        FlowSpec::Column => "Down".to_string(),
        FlowSpec::Row => match cross {
            Some(quinn_styles::CrossAlign::Center) => {
                "Flow.Right{wrap: false row_align: RowAlign.Center}".to_string()
            }
            Some(quinn_styles::CrossAlign::End) => {
                "Flow.Right{wrap: false row_align: RowAlign.Bottom}".to_string()
            }
            _ => "Right".to_string(),
        },
    }
}

/// The widget a slot or route function returns. Named bindings stay inside it.
fn emit_returned(
    node: &ViewNode,
    shows: &mut ShowCursor<'_>,
    slots: &mut String,
    routes: &mut String,
    pass_root: bool,
) -> String {
    match node {
        ViewNode::Slot { fn_name, nodes, .. } if fn_name.is_empty() => {
            slot_body(
                nodes,
                SLOT_FRAME.with(Cell::get),
                shows,
                slots,
                routes,
                pass_root,
            )
        }
        ViewNode::El(el) => emit_el(
            el,
            None,
            3,
            false,
            FlowSpec::Column,
            false,
            shows,
            slots,
            routes,
            pass_root,
        ),
        // A `for` emits into the widget literal around it. A slot function has
        // no literal of its own, so the rows would otherwise vanish and the
        // function would return nil. The view is what the parent mounts.
        ViewNode::For { .. } => {
            let inner = emit_node(
                node,
                4,
                None,
                FlowSpec::Column,
                false,
                shows,
                slots,
                routes,
                pass_root,
            );
            format!("            RoundedView {{\n                width: Fill\n                height: Fit\n                flow: Down\n{inner}            }}\n")
        }
        other => emit_node(
            other,
            3,
            None,
            FlowSpec::Column,
            false,
            shows,
            slots,
            routes,
            pass_root,
        ),
    }
}

fn emit_self_wrap(
    el: &El,
    align: quinn_styles::CrossAlign,
    indent: usize,
    row: Option<&RowEnv<'_>>,
    parent: FlowSpec,
    shows: &mut ShowCursor<'_>,
    slots: &mut String,
    routes: &mut String,
    pass_root: bool,
) -> String {
    let pad = " ".repeat(indent * 4);
    let inner = emit_el(
        el,
        row,
        indent + 1,
        true,
        parent,
        false,
        shows,
        slots,
        routes,
        pass_root,
    );
    let (width, height, flow, align_line) = match parent {
        FlowSpec::Row => {
            let row_align = match align {
                quinn_styles::CrossAlign::Center => "Center",
                quinn_styles::CrossAlign::End => "Bottom",
                quinn_styles::CrossAlign::Start | quinn_styles::CrossAlign::Stretch => "Top",
            };
            (
                "Fit",
                "Fill",
                format!("Flow.Right{{wrap: false row_align: RowAlign.{row_align}}}"),
                String::new(),
            )
        }
        FlowSpec::Column => {
            let x = match align {
                quinn_styles::CrossAlign::Center => "0.5",
                quinn_styles::CrossAlign::End => "1.",
                quinn_styles::CrossAlign::Start | quinn_styles::CrossAlign::Stretch => "0.",
            };
            (
                "Fill",
                "Fit",
                "Down".into(),
                format!("{pad}    align: Align{{x: {x} y: 0.}}\n"),
            )
        }
    };
    format!("{pad}RoundedView {{\n{pad}    width: {width}\n{pad}    height: {height}\n{pad}    flow: {flow}\n{align_line}{inner}{pad}}}\n")
}

fn emit_el(
    el: &El,
    row: Option<&RowEnv<'_>>,
    indent: usize,
    named: bool,
    parent: FlowSpec,
    stretch: bool,
    shows: &mut ShowCursor<'_>,
    slots: &mut String,
    routes: &mut String,
    pass_root: bool,
) -> String {
    let pad = " ".repeat(indent * 4);
    if let Some(glyph) = &el.icon {
        return emit_icon(el, glyph, row, &pad, named, parent, stretch);
    }
    if let Some(bounds) = el.range {
        return emit_range(el, bounds, row, &pad, named, parent, stretch);
    }
    let tag = tag_of(&el.tag);
    let style = prepared_style(el, tag, parent, stretch);
    let widget = widget_for(&el.tag, &style);
    let mut lines = Vec::new();
    // `bind:value` omits `text:` so a reload does not assign the field and
    // move the caret. The sibling that reads the signal still updates.
    if el.bind.is_none() && el.tag != "div" && (el.value.is_some() || !el.parts.is_empty() || el.tag == "button") {
        lines.push(format!("text: {}", text_expr(el, row)));
    }
    if let Some(placeholder) = &el.placeholder {
        lines.push(format!("empty_text: \"{}\"", escape_script(placeholder)));
    }
    lines.extend(style_lines(el, tag, parent, stretch, row));
    // Stock `Label` pads every side by `theme.mspace_1`; text boxes here are
    // the line box only, so text lines up with control and divider edges.
    if widget == "Label" && !lines.iter().any(|line| line.starts_with("padding")) {
        lines.push("padding: 0.".into());
    }
    if matches!(tag, Tag::Button | Tag::Input) {
        lines.push(format!("draw_bg.ring_color: {}", themed_fill("bg-focus")));
        lines.push(format!("draw_bg.ring_gap: {}", themed_fill("bg-surface")));
    }
    if style.grid_cols.is_some() {
        // Grid's implicit rows are 0px unless we set a size, and it rejects
        // `auto`. The tallest explicit child height is the row; otherwise 40.
        lines.push(format!(
            "implicit_row_size: {}",
            script_num(implicit_row_px(el))
        ));
    }
    if matches!(tag, Tag::Div) {
        if let Some(anim) = div_hover_animator(&style) {
            lines.push(anim);
        }
    }
    if let Some(signal) = &el.input {
        lines.push(format!(
            "on_change: |text| {{\n{pad}    state.{signal} = text\n{pad}    root.render()\n{pad}}}"
        ));
    }
    if let Some(act) = &el.click {
        let body = emit_act(act, row, &pad);
        let key = if el.input.is_some() {
            "on_change"
        } else {
            "on_click"
        };
        if el.input.is_none() {
            lines.push(format!("{key}: || {{\n{body}{pad}}}"));
        }
    }
    let child_style = {
        let mut style = tag.defaults();
        if !el.class.is_empty() {
            style.merge_from(&parse_classes(&el.class));
        }
        style
    };
    let child_flow = if el.tag == "div" {
        child_style.flow.unwrap_or(FlowSpec::Column)
    } else {
        parent
    };
    let child_stretch =
        el.tag == "div" && child_style.cross_align == Some(quinn_styles::CrossAlign::Stretch);
    let split = el.tag == "div"
        && style.grid_cols.is_none()
        && !style.absolute
        && el.children.iter().any(child_is_absolute);
    let widget_text = if style.absolute {
        emit_abs_frame(
            el,
            row,
            indent,
            named,
            parent,
            shows,
            slots,
            routes,
            pass_root,
        )
    } else if split {
        let mut flow_lines = Vec::new();
        let mut kept = Vec::new();
        for line in lines {
            if line.starts_with("flow:")
                || line.starts_with("spacing:")
                || line.starts_with("wrap_spacing:")
                || line.starts_with("align:")
                || line.starts_with("distribute:")
            {
                flow_lines.push(line);
            } else {
                kept.push(line);
            }
        }
        if !kept.iter().any(|line| line.starts_with("flow:")) {
            kept.push("flow: Overlay".into());
        }
        let inner_props = kept
            .into_iter()
            .map(|line| format!("{pad}    {line}\n"))
            .collect::<String>();
        let mut flow_nodes = Vec::new();
        let mut abs_nodes = Vec::new();
        for child in &el.children {
            if child_is_absolute(child) {
                abs_nodes.push(child);
            } else {
                flow_nodes.push(child);
            }
        }
        abs_nodes.sort_by_key(|node| node_z(node));
        let mut body = String::new();
        for child in abs_nodes.iter().copied().filter(|node| node_z(node) < 0) {
            body.push_str(&emit_abs_frame_node(
                child,
                indent + 1,
                row,
                shows,
                slots,
                routes,
                pass_root,
            ));
        }
        if !flow_nodes.is_empty() {
            let flow_pad = " ".repeat((indent + 1) * 4);
            let mut flow_body = String::new();
            let prev_row = ROW_AT.with(|cell| cell.replace(row_stretch_at(&el.class)));
            let prev_frame = bind_slot_frame(child_flow, &child_style, &el.class);
            for child in &flow_nodes {
                flow_body.push_str(&emit_node(
                    child,
                    indent + 2,
                    row,
                    child_flow,
                    child_stretch,
                    shows,
                    slots,
                    routes,
                    pass_root,
                ));
            }
            ROW_AT.with(|cell| cell.set(prev_row));
            SLOT_FRAME.with(|cell| cell.set(prev_frame));
            let flow_props = if flow_lines.is_empty() {
                format!("{flow_pad}    flow: Down\n")
            } else {
                flow_lines
                    .into_iter()
                    .map(|line| format!("{flow_pad}    {line}\n"))
                    .collect()
            };
            // A `Fit` parent shrink-wraps. A `Fill` child of that parent
            // has nothing to fill and collapses, which is how a segmented
            // control's labels disappeared. Pass `Fit` through on that axis.
            let flow_width = if matches!(style.width, Some(quinn_styles::SizeSpec::Fit)) {
                "Fit"
            } else {
                "Fill"
            };
            let flow_height = if matches!(style.height, Some(quinn_styles::SizeSpec::Fit)) {
                "Fit"
            } else {
                "Fill"
            };
            body.push_str(&format!(
                "{flow_pad}quinn_flow := RoundedView {{\n{flow_pad}    width: {flow_width}\n{flow_pad}    height: {flow_height}\n{flow_props}{flow_body}{flow_pad}}}\n"
            ));
        }
        for child in abs_nodes.iter().copied().filter(|node| node_z(node) >= 0) {
            body.push_str(&emit_abs_frame_node(
                child,
                indent + 1,
                row,
                shows,
                slots,
                routes,
                pass_root,
            ));
        }
        let head = if named {
            format!("{pad}{} := {widget} {{\n", el.name)
        } else {
            format!("{pad}{widget} {{\n")
        };
        format!("{head}{inner_props}{body}{pad}}}\n")
    } else {
        let inner = lines
            .into_iter()
            .map(|line| format!("{pad}    {line}\n"))
            .collect::<String>();
        let mut children = String::new();
        let prev_row = ROW_AT.with(|cell| cell.replace(row_stretch_at(&el.class)));
        let prev_frame = bind_slot_frame(child_flow, &child_style, &el.class);
        for child in &el.children {
            children.push_str(&emit_node(
                child,
                indent + 1,
                row,
                child_flow,
                child_stretch,
                shows,
                slots,
                routes,
                pass_root,
            ));
        }
        ROW_AT.with(|cell| cell.set(prev_row));
        SLOT_FRAME.with(|cell| cell.set(prev_frame));
        let head = if named {
            format!("{pad}{} := {widget} {{\n", el.name)
        } else {
            format!("{pad}{widget} {{\n")
        };
        format!("{head}{inner}{children}{pad}}}\n")
    };
    let widget_text = if el.popover {
        // The trigger is a control-height sibling in an 8-unit gap stack.
        format!(
            "{pad}QuinnPopup {{\n{pad}    anchor_offset: ({}) + 16\n{widget_text}{pad}}}\n",
            density_height(28.0, 36.0, 48.0)
        )
    } else {
        widget_text
    };
    wrap_provides(&el.provides, &pad, named, widget_text)
}

/// A semantic `bg-*` token as a light/dark script value.
fn themed_fill(class: &str) -> String {
    let pick = |dark| {
        parse_classes_in(class, dark)
            .background
            .map(|color| color.hex_script())
            .unwrap_or_else(|| "#00000000".into())
    };
    let (light, dark) = (pick(false), pick(true));
    if light == dark {
        light
    } else {
        format!("if state.quinn_dark == 1 {{ {dark} }} else {{ {light} }}")
    }
}

/// `<input type="range" min="0" max="100" step="1" bind:value=level>`.
/// Drag, arrows (Shift for ten steps), Home/End, and Tab come from
/// `QuinnSlider`; the value lands in the bound signal.
fn emit_range(
    el: &El,
    [min, max, step]: [f64; 3],
    row: Option<&RowEnv<'_>>,
    pad: &str,
    named: bool,
    parent: FlowSpec,
    stretch: bool,
) -> String {
    let mut lines = Vec::new();
    for line in style_lines(el, Tag::Div, parent, stretch, row) {
        if line.starts_with("width:") || line.starts_with("margin:") {
            lines.push(line);
        }
    }
    lines.push(format!("min: {}", script_num(min)));
    lines.push(format!("max: {}", script_num(max)));
    lines.push(format!("step: {}", script_num(step)));
    lines.push(format!("big_step: {}", script_num(step * 10.0)));
    if let Some(signal) = el.bind.as_ref().or(el.input.as_ref()) {
        lines.push(format!("value: state.{signal}"));
        lines.push(format!(
            "on_change: |v| {{\n{pad}    state.{signal} = v\n{pad}    ui.host.render()\n{pad}}}"
        ));
    }
    for (key, class) in [
        ("track_color", "bg-divider"),
        ("fill_color", "bg-accent"),
        ("thumb_color", "bg-surface"),
        ("thumb_border", "bg-control"),
        ("ring_color", "bg-focus"),
    ] {
        lines.push(format!("draw_bg.{key}: {}", themed_fill(class)));
    }
    let inner: String = lines.iter().map(|line| format!("{pad}    {line}\n")).collect();
    let head = if named {
        format!("{pad}{} := QuinnSlider {{\n", el.name)
    } else {
        format!("{pad}QuinnSlider {{\n")
    };
    format!("{head}{inner}{pad}}}\n")
}

/// `<Icon name="notes" class="w-4 text-secondary">`: a square SVG glyph.
/// Size comes from a fixed `w-*` (16 when absent). Ink follows the text
/// color utilities, including `dark:` and `class:` conditions.
fn emit_icon(
    el: &El,
    glyph: &str,
    row: Option<&RowEnv<'_>>,
    pad: &str,
    named: bool,
    parent: FlowSpec,
    stretch: bool,
) -> String {
    let size = match parse_classes(&el.class).width {
        Some(quinn_styles::SizeSpec::Px(px)) => px,
        _ => 16.0,
    };
    let size = script_num(size);
    let mut lines = vec![
        format!("width: {size}"),
        format!("height: {size}"),
        format!("size: {size}"),
        format!("icon_walk: Walk{{width: {size} height: {size}}}"),
        format!("draw_icon.svg: crate_resource(\"quinn_makepad:resources/icons/{glyph}.svg\")"),
    ];
    for line in style_lines(el, Tag::Span, parent, stretch, row) {
        if let Some(rest) = line.strip_prefix("draw_text.color:") {
            lines.push(format!("draw_icon.color:{rest}"));
        } else if line.starts_with("margin:") {
            lines.push(line);
        }
    }
    let inner: String = lines.iter().map(|line| format!("{pad}    {line}\n")).collect();
    let head = if named {
        format!("{pad}{} := Icon {{\n", el.name)
    } else {
        format!("{pad}Icon {{\n")
    };
    format!("{head}{inner}{pad}}}\n")
}

fn div_hover_animator(style: &Style) -> Option<String> {
    let hover = style.hover_bg?;
    let base = style.background.unwrap_or(hover).hex_script();
    let hover_color = hover.hex_script();
    let down = style.active_bg.unwrap_or(hover).hex_script();
    let off = anim_state("0.1", &base);
    let on = anim_state("0.1", &hover_color);
    // The down track's resting state has to match the base color. Using the
    // hover color here paints the hover swatch on the first frames, because
    // `default: @off` applies before any pointer event.
    let down_off = anim_state("0.1", &base);
    let down_on = anim_state("0.05", &down);
    // `View` plays `hover.on` / `hover.off` / `down.on` / `down.off` when an
    // animator is defined, and it hit-tests only then. The shader still reads
    // `self.color`, so the track swaps `draw_bg.color` instead of `color_hover`.
    Some(format!(
        "cursor: MouseCursor.Arrow animator: Animator{{hover: {{default: @off off: {off} on: {on}}} down: {{default: @off off: {down_off} on: {down_on}}}}}"
    ))
}

fn anim_state(duration: &str, color: &str) -> String {
    let mut state = String::from("AnimatorState{from: {all: Forward {duration: ");
    state.push_str(duration);
    state.push_str("}} apply: {draw_bg: {color: ");
    state.push_str(color);
    state.push_str("}}}");
    state
}

fn child_is_absolute(node: &ViewNode) -> bool {
    match node {
        ViewNode::El(el) => parse_classes(&el.class).absolute,
        ViewNode::Show { child, .. } => child_is_absolute(child),
        ViewNode::Slot { fn_name, nodes, .. } => {
            if fn_name.is_empty() {
                nodes.iter().any(child_is_absolute)
            } else {
                // A component child that is only an absolute overlay has to
                // cover its parent. A named slot is one call, so the call
                // itself has to join the absolute layer.
                !nodes.is_empty() && nodes.iter().all(child_is_absolute)
            }
        }
        _ => false,
    }
}

fn node_z(node: &ViewNode) -> i64 {
    match node {
        ViewNode::El(el) => parse_classes(&el.class).z_index.unwrap_or(0),
        ViewNode::Show { child, .. } => node_z(child),
        ViewNode::Slot { nodes, .. } => nodes.iter().map(node_z).max().unwrap_or(0),
        _ => 0,
    }
}

fn strip_position(class: &str) -> String {
    class
        .split_whitespace()
        .filter(|token| {
            !matches!(*token, "absolute" | "inset-0")
                && !token.starts_with("left-")
                && !token.starts_with("right-")
                && !token.starts_with("top-")
                && !token.starts_with("bottom-")
                && !token.starts_with("inset-")
                && !token.starts_with("z-")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn emit_abs_frame(
    el: &El,
    row: Option<&RowEnv<'_>>,
    indent: usize,
    named: bool,
    parent: FlowSpec,
    shows: &mut ShowCursor<'_>,
    slots: &mut String,
    routes: &mut String,
    pass_root: bool,
) -> String {
    let tag = tag_of(&el.tag);
    let style = prepared_style(el, tag, parent, false);
    let mut stripped = el.clone();
    stripped.class = strip_position(&el.class);
    // The second emit walks the stripped class, which no longer says
    // `absolute`, so `tune_style` would not shrink-wrap it. A toast with no
    // width class was coming out `width: Fill` — a full-width bar.
    if matches!(style.width, Some(SizeSpec::Fill)) && !has_size_class(&stripped.class, "w") {
        stripped.class.push_str(" w-full");
    } else if matches!(style.width, Some(SizeSpec::Fit)) && !has_size_class(&stripped.class, "w") {
        stripped.class.push_str(" w-fit");
    }
    if matches!(style.height, Some(SizeSpec::Fill)) && !has_size_class(&stripped.class, "h") {
        stripped.class.push_str(" h-full");
    } else if matches!(style.height, Some(SizeSpec::Fit)) && !has_size_class(&stripped.class, "h") {
        stripped.class.push_str(" h-fit");
    }
    let inner = emit_el(
        &stripped,
        row,
        indent + 1,
        named,
        parent,
        false,
        shows,
        slots,
        routes,
        pass_root,
    );
    abs_shell(&style, indent, inner)
}

fn emit_abs_frame_node(
    node: &ViewNode,
    indent: usize,
    row: Option<&RowEnv<'_>>,
    shows: &mut ShowCursor<'_>,
    slots: &mut String,
    routes: &mut String,
    pass_root: bool,
) -> String {
    match node {
        ViewNode::El(el) if parse_classes(&el.class).absolute => emit_abs_frame(
            el, row, indent, true, FlowSpec::Column, shows, slots, routes, pass_root,
        ),
        other => emit_node(
            other, indent, row, FlowSpec::Column, false, shows, slots, routes, pass_root,
        ),
    }
}

fn abs_shell(style: &Style, indent: usize, inner: String) -> String {
    let pad = " ".repeat(indent * 4);
    let left = style.abs_x.unwrap_or(0.0);
    let top = style.abs_y.unwrap_or(0.0);
    let right = style.abs_right.unwrap_or(0.0);
    let bottom = style.abs_bottom.unwrap_or(0.0);
    let x = if style.abs_right.is_some() && style.abs_x.is_none() {
        1.0
    } else {
        0.0
    };
    let y = if style.abs_bottom.is_some() && style.abs_y.is_none() {
        1.0
    } else {
        0.0
    };
    let align = if x == 0.0 && y == 0.0 {
        String::new()
    } else {
        format!(
            "{pad}    align: Align{{x: {} y: {}}}\n",
            script_num(x),
            script_num(y)
        )
    };
    let padding = if left == 0.0 && top == 0.0 && right == 0.0 && bottom == 0.0 {
        String::new()
    } else {
        format!(
            "{pad}    padding: Inset{{left: {} top: {} right: {} bottom: {}}}\n",
            script_num(left),
            script_num(top),
            script_num(right),
            script_num(bottom)
        )
    };
    // `show_bg: false` so the positioning shell does not paint over the page.
    // The shell is the full window; the child is aligned inside the padding.
    format!("{pad}RoundedView {{\n{pad}    width: Fill\n{pad}    height: Fill\n{pad}    flow: Overlay\n{pad}    show_bg: false\n{align}{padding}{inner}{pad}}}\n")
}

fn has_size_class(class: &str, axis: &str) -> bool {
    class.split_whitespace().any(|token| {
        let base = token.rsplit(':').next().unwrap_or(token);
        base == axis || base.starts_with(&format!("{axis}-")) || base.starts_with(&format!("{axis}["))
    })
}

fn wrap_provides(provides: &[(String, String)], pad: &str, named: bool, widget: String) -> String {
    if provides.is_empty() {
        return widget;
    }
    let mut open = String::new();
    let mut close = String::new();
    for (index, (key, expr)) in provides.iter().enumerate() {
        open.push_str(&format!("{pad}let quinn_prev_{index} = ok{{mod.ctx.{key}}}\n"));
        open.push_str(&format!("{pad}mod.ctx.{key} = {expr}\n"));
        close.push_str(&format!("{pad}mod.ctx.{key} = quinn_prev_{index}\n"));
    }
    if named {
        format!("{open}{widget}{close}")
    } else {
        format!("{open}{pad}let quinn_w = {{\n{widget}{pad}}}\n{close}{pad}quinn_w\n")
    }
}

fn emit_act(act: &Act, row: Option<&RowEnv<'_>>, pad: &str) -> String {
    let inner = " ".repeat(pad.len() + 4);
    let lines = match act {
        Act::Inc(signal) => vec![format!("state.{signal} += 1")],
        Act::Toggle(signal) => vec![format!("state.{signal} = !state.{signal}")],
        Act::Clear(signal) => vec![format!("state.{signal} = \"\"")],
        Act::Pop(signal) => vec![format!("state.{signal}.pop()")],
        Act::Reverse(signal) => vec![format!("state.{signal} = quinn_reverse(state.{signal})")],
        Act::Push { next, items } => vec![
            format!("let id = state.{next}"),
            format!("state.{next} = id + 1"),
            "quinn_row_reset(id)".into(),
            format!("state.{items}.push({{id: id, label: \"Item \" + id}})"),
        ],
        Act::RowInc => {
            let func = row.map(|row| row.fn_name).unwrap_or("quinn_row");
            vec![format!("{func}_bump(id)")]
        }
        Act::Body(stmts) | Act::Row(stmts) => emit_stmts(stmts, row),
    };
    let mut lines = lines;
    lines.push("ui.host.render()".into());
    lines
        .into_iter()
        .map(|line| format!("{inner}{line}\n"))
        .collect()
}

fn emit_stmts(stmts: &[HandlerStmt], row: Option<&RowEnv<'_>>) -> Vec<String> {
    let mut lines = Vec::new();
    let mut locals = std::collections::HashSet::new();
    for stmt in stmts {
        match stmt {
            HandlerStmt::Let { name, expr } => {
                lines.push(format!("let {name} = {}", emit_hx(expr, row, &locals)));
                locals.insert(name.clone());
            }
            HandlerStmt::Set { signal, expr } => {
                lines.push(emit_set(signal, expr, row, &locals));
            }
            HandlerStmt::Push { items, id, label } => {
                let id = emit_hx(id, row, &locals);
                let label = emit_hx(label, row, &locals);
                lines.push(format!("quinn_row_reset({id})"));
                lines.push(format!("state.{items}.push({{id: {id}, label: {label}}})"));
            }
            HandlerStmt::Pop { items } => lines.push(format!("state.{items}.pop()")),
            HandlerStmt::Reverse { items } => {
                lines.push(format!("state.{items} = quinn_reverse(state.{items})"))
            }
            HandlerStmt::RemoveKey { items, key } => {
                let key = emit_hx(key, row, &locals);
                lines.push(format!(
                    "state.{items} = quinn_remove_key(state.{items}, {key})"
                ));
            }
            HandlerStmt::SwapKey { items, a, b } => {
                let a = emit_hx(a, row, &locals);
                let b = emit_hx(b, row, &locals);
                lines.push(format!(
                    "state.{items} = quinn_swap_key(state.{items}, {a}, {b})"
                ));
            }
            HandlerStmt::Navigate(expr) => {
                lines.push(format!("state.route = {}", emit_hx(expr, row, &locals)));
            }
            HandlerStmt::Dispatch { action, arg } => {
                lines.push(format!("state.{action}_pending = true"));
                lines.push(format!("state.{action}_gen = state.{action}_gen + 1"));
                match arg {
                    HxExpr::List(_) | HxExpr::Obj(_) => {
                        lines.push(format!(
                            "let quinn_pack = {}",
                            emit_hx(arg, row, &locals)
                        ));
                        lines.push(format!("state.{action}_arg = quinn_pack.to_json()"));
                    }
                    other => {
                        let arg = emit_hx(other, row, &locals);
                        lines.push(format!("state.{action}_arg = {arg}"));
                    }
                }
            }
            HandlerStmt::Refetch { job } => {
                lines.push(format!("state.{job}_phase = \"loading\""));
                lines.push(format!("state.{job}_gen = state.{job}_gen + 1"));
            }
            HandlerStmt::ToggleTheme => {
                lines.push(
                    "if state.quinn_dark == 1 { quinn_theme_off() } else { quinn_theme_on() }"
                        .into(),
                );
            }
            HandlerStmt::SetDensity(n) => {
                lines.push(format!("state.quinn_density = {n}"));
            }
            HandlerStmt::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond = emit_hx(cond, row, &locals);
                lines.push(format!("if {cond} {{"));
                lines.extend(emit_stmts(then_branch, row));
                if else_branch.is_empty() {
                    lines.push("}".into());
                } else {
                    lines.push("} else {".into());
                    lines.extend(emit_stmts(else_branch, row));
                    lines.push("}".into());
                }
            }
        }
    }
    lines
}

fn emit_set(
    signal: &str,
    expr: &HxExpr,
    row: Option<&RowEnv<'_>>,
    locals: &std::collections::HashSet<String>,
) -> String {
    let toggled = matches!(
        expr,
        HxExpr::Not(inner) if matches!(inner.as_ref(), HxExpr::Name(name) if name == signal)
    );
    if toggled {
        if row.is_some_and(|row| {
            row.count != Some(signal) && row.signals.iter().any(|item| item.read == signal)
        }) {
            return format!("state.r_{signal}[key] = state.r_{signal}[key] != true");
        }
        return format!("state.{signal} = !state.{signal}");
    }
    if row.is_some_and(|row| row.count == Some(signal))
        && matches!(
            expr,
            HxExpr::Binary(BinOp::Add, left, right)
                if matches!(left.as_ref(), HxExpr::Name(name) if name == signal)
                    && matches!(right.as_ref(), HxExpr::Int(1))
        )
    {
        return "quinn_row_bump(id)".into();
    }
    let value = emit_hx(expr, row, locals);
    if let Some(row) = row {
        if row.count == Some(signal) {
            return format!("state.counts[key] = {value}");
        }
        if row.signals.iter().any(|item| item.read == signal) {
            return format!("state.r_{signal}[key] = {value}");
        }
    }
    if matches!(
        expr,
        HxExpr::Binary(BinOp::Add, left, right)
            if matches!(left.as_ref(), HxExpr::Name(name) if name == signal)
                && matches!(right.as_ref(), HxExpr::Int(1))
    ) {
        return format!("state.{signal} += 1");
    }
    format!("state.{signal} = {value}")
}

fn emit_hx(
    expr: &HxExpr,
    row: Option<&RowEnv<'_>>,
    locals: &std::collections::HashSet<String>,
) -> String {
    match expr {
        HxExpr::Int(value) => value.to_string(),
        HxExpr::Bool(value) => if *value { "true" } else { "false" }.into(),
        HxExpr::Str(value) => format!("\"{}\"", escape_script(value)),
        HxExpr::Name(name) => {
            if locals.contains(name)
                || name == "text"
                || name == "value"
                || name == "id"
                || name == "quinn_key"
            {
                if name == "value" {
                    "text".into()
                } else {
                    name.clone()
                }
            } else {
                state_read(name, row)
            }
        }
        HxExpr::Not(inner) => format!("!{}", emit_hx(inner, row, locals)),
        HxExpr::Binary(op, left, right) => {
            let op = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Eq => "==",
                BinOp::Ne => "!=",
                BinOp::Lt => "<",
                BinOp::Gt => ">",
                BinOp::Le => "<=",
                BinOp::Ge => ">=",
            };
            format!(
                "{} {} {}",
                emit_hx(left, row, locals),
                op,
                emit_hx(right, row, locals)
            )
        }
        HxExpr::Concat(parts) => parts
            .iter()
            .map(|part| emit_hx(part, row, locals))
            .collect::<Vec<_>>()
            .join(" + "),
        HxExpr::List(items) => {
            let body = items
                .iter()
                .map(|item| emit_hx(item, row, locals))
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{body}]")
        }
        HxExpr::Call { name, args } => {
            let args = args
                .iter()
                .map(|arg| emit_hx(arg, row, locals))
                .collect::<Vec<_>>()
                .join(", ");
            format!("quinn_{name}({args})")
        }
        HxExpr::If { cond, then, els } => format!(
            "if {} {{ {} }} else {{ {} }}",
            emit_hx(cond, row, locals),
            emit_hx(then, row, locals),
            emit_hx(els, row, locals)
        ),
        HxExpr::Obj(fields) => {
            let body = fields
                .iter()
                .map(|(key, value)| format!("{key}: {}", emit_hx(value, row, locals)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{body}}}")
        }
    }
}

fn state_read(name: &str, row: Option<&RowEnv<'_>>) -> String {
    if let Some(row) = row {
        if row.count == Some(name) {
            return "quinn_row_n(id)".into();
        }
        if row.signals.iter().any(|signal| signal.read == name) {
            return format!(
                "(if state.r_{name}[key] == nil {{ 0 }} else {{ state.r_{name}[key] }})"
            );
        }
    }
    format!("state.{name}")
}

fn text_expr(el: &El, row: Option<&RowEnv<'_>>) -> String {
    if let Some(signal) = &el.value {
        return state_read(signal, row);
    }
    if el.parts.is_empty() {
        return "\"\"".into();
    }
    el.parts
        .iter()
        .map(|part| match part {
            Part::Lit(text) => format!("\"{}\"", escape_script(text)),
            Part::Sig(name) => {
                let read = state_read(name, row);
                // A bare number is not a label string. A stepper or a slider
                // interpolated on its own needs `+` with an empty string.
                // Generations, the pending flag, and the cancel count are
                // numbers even beside a literal. A literal already next to
                // `state.count` is what the counter test looks for.
                let numeric = name == "quinn_cancelled"
                    || name.ends_with("_gen")
                    || name.ends_with("_pending");
                if row.is_some() || el.parts.len() == 1 || numeric {
                    format!("\"\" + {read}")
                } else {
                    read
                }
            }
            Part::IfBool { signal, yes, no } => {
                let cond =
                    if row.is_some_and(|row| row.signals.iter().any(|item| item.read == *signal)) {
                        format!("state.r_{signal}[key] == true")
                    } else {
                        format!("state.{signal}")
                    };
                format!(
                    "if {cond} {{ \"{}\" }} else {{ \"{}\" }}",
                    escape_script(yes),
                    escape_script(no)
                )
            }
            Part::IfEmpty { signal, empty } => format!(
                "if state.{signal} == \"\" {{ \"{}\" }} else {{ state.{signal} }}",
                escape_script(empty)
            ),
            Part::Param(name) => name.clone(),
            Part::Index(name) => {
                if el.parts.len() == 1 {
                    format!("\"\" + {name}")
                } else {
                    name.clone()
                }
            }
            Part::Ctx(key) => format!("mod.ctx.{key}"),
            Part::Job(name) => format!("state.{name}_value"),
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

fn widget_for(tag: &str, style: &Style) -> String {
    if tag == "div" && style.grid_cols.is_some() {
        return "Grid".into();
    }
    if tag == "div" && style.vibrancy {
        return "GaussRoundedView".into();
    }
    if tag == "div" && style.shadow.is_some() {
        // Stock RoundedView has no shadow uniforms. RoundedShadowView is the
        // same rounded fill, plus shadow_radius / shadow_offset / shadow_color.
        return "RoundedShadowView".into();
    }
    widget_name(tag).into()
}

fn widget_name(tag: &str) -> &'static str {
    match tag {
        "h1" => "H1",
        "h2" => "H2",
        "button" => "QuinnButton",
        "input" => "QuinnTextInput",
        "div" => "RoundedView",
        _ => "Label",
    }
}

fn tag_of(tag: &str) -> Tag {
    match tag {
        "h1" => Tag::H1,
        "h2" => Tag::H2,
        "button" => Tag::Button,
        "input" => Tag::Input,
        "div" => Tag::Div,
        "p" => Tag::P,
        "label" => Tag::Label,
        _ => Tag::Span,
    }
}

fn host_fields(class: &str, release_clip: bool) -> String {
    let mut style = Tag::Div.defaults();
    style.merge_from(&parse_classes(class));
    style.width = Some(SizeSpec::Fill);
    style.height = Some(SizeSpec::Fill);
    let mut lines = layout_pairs(&style, true);
    lines.extend(walk_pairs(&style));
    // `abs_pos` is a draw-space point. A clipping host hides a widget whose
    // point sits outside the host's padding box.
    if release_clip && !style.clip {
        lines.push(("clip_x".into(), "false".into()));
        lines.push(("clip_y".into(), "false".into()));
    }
    lines.push(("show_bg".into(), "true".into()));
    lines.push(("new_batch".into(), "true".into()));
    let bg = style
        .background
        .map(|color| color.hex_script())
        .unwrap_or_else(|| "#xf4f0e6".into());
    lines.push(("draw_bg.color".into(), bg));
    let mut map = std::collections::BTreeMap::new();
    for (key, value) in lines {
        map.insert(key, value);
    }
    // The root `<div class>` is the host, not a child `emit_el`. Shell
    // variants on it (`dark:bg-stone-900`) still have to reach `draw_bg`.
    for when in shell_variants(class) {
        let mut on = style.clone();
        on.merge_from(&parse_classes_in(&when.token, when.dark));
        on.width = Some(SizeSpec::Fill);
        on.height = Some(SizeSpec::Fill);
        if let Some(color) = on.background {
            let key = "draw_bg.color".to_string();
            let value = color.hex_script();
            if map.get(&key).map(String::as_str) == Some(value.as_str()) {
                continue;
            }
            let else_value = map
                .get(&key)
                .cloned()
                .unwrap_or_else(|| "#xf4f0e6".into());
            map.insert(
                key,
                format!("if {} {{ {value} }} else {{ {else_value} }}", shell_cond(&when)),
            );
        }
    }
    map.into_iter()
        .map(|(key, value)| format!("                            {key}: {value}\n"))
        .collect()
}

fn shell_cond(when: &quinn_styles::ShellWhen) -> String {
    let mut parts = Vec::new();
    if when.dark {
        parts.push("state.quinn_dark == 1".to_string());
    }
    if let Some(width) = when.min_width {
        parts.push(format!("state.quinn_width >= {}", script_num(width)));
    }
    if parts.is_empty() {
        return "true".into();
    }
    parts.join(" && ")
}

fn class_cond(signal: &str, row: Option<&RowEnv<'_>>) -> String {
    if row.is_some_and(|row| row.signals.iter().any(|item| item.read == signal)) {
        format!("state.r_{signal}[key] == true")
    } else {
        format!("state.{signal}")
    }
}

fn style_lines(
    el: &El,
    tag: Tag,
    parent: FlowSpec,
    stretch: bool,
    row: Option<&RowEnv<'_>>,
) -> Vec<String> {
    let base = prepared_style(el, tag, parent, stretch);
    let base_map = prop_map(&base, tag);
    let mut rendered = base_map.clone();
    // Tokens under one condition build on each other: `bp1120:py-0
    // bp1120:px-3` is one padding, not two that each restore the base.
    let mut under: std::collections::BTreeMap<String, Style> = std::collections::BTreeMap::new();
    for when in shell_variants(&el.class) {
        let cond = shell_cond(&when);
        let acc = under.entry(cond.clone()).or_insert_with(|| base.clone());
        acc.merge_from(&parse_classes_in(&when.token, when.dark));
        let mut on = acc.clone();
        tune_style(&mut on, parent, stretch && el.class.contains("self-stretch"));
        let on_map = prop_map(&on, tag);
        let touched = token_keys(&when.token, when.dark, tag);
        for (key, value) in on_map {
            // A later `dark:bg-transparent` equals the light `bg-transparent`
            // and still has to override an earlier `dark:bg-accent`. A key
            // this token does not set keeps whatever is rendered.
            let same_as_base = base_map.get(&key).map(String::as_str) == Some(value.as_str());
            let same_as_now = rendered.get(&key).map(String::as_str) == Some(value.as_str());
            if same_as_now || (same_as_base && !touched.contains(&key)) {
                continue;
            }
            let else_value = rendered
                .get(&key)
                .cloned()
                .unwrap_or_else(|| resting_fill(&key, &base));
            rendered.insert(
                key,
                format!("if {cond} {{ {value} }} else {{ {else_value} }}"),
            );
        }
    }
    if row.is_some_and(|row| row.striped) {
        for (odd, token) in stripe_tokens(&el.class) {
            let mut on = base.clone();
            on.merge_from(&parse_classes_in(&token, false));
            let mut dark_on = base.clone();
            dark_on.merge_from(&parse_classes_in(&token, true));
            let on_map = prop_map(&on, tag);
            let dark_map = prop_map(&dark_on, tag);
            let flag = if odd { "quinn_odd == 1" } else { "quinn_odd == 0" };
            for (key, light_value) in on_map {
                if base_map.get(&key).map(String::as_str) == Some(light_value.as_str()) {
                    if dark_map.get(&key).map(String::as_str) == Some(light_value.as_str()) {
                        continue;
                    }
                }
                let dark_value = dark_map.get(&key).cloned().unwrap_or_else(|| light_value.clone());
                let value = if light_value == dark_value {
                    light_value
                } else {
                    format!("if state.quinn_dark == 1 {{ {dark_value} }} else {{ {light_value} }}")
                };
                let else_value = rendered
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| resting_fill(&key, &base));
                rendered.insert(
                    key,
                    format!("if {flag} {{ {value} }} else {{ {else_value} }}"),
                );
            }
        }
    }
    for when in &el.class_when {
        let cond = class_cond(&when.signal, row);
        let stretch_self = stretch && el.class.contains("self-stretch");
        if when.dark {
            let mut on = prepared_style(el, tag, parent, stretch);
            on.merge_from(&parse_classes_in(&when.token, true));
            tune_style(&mut on, parent, stretch_self);
            let on_map = prop_map(&on, tag);
            let cond = format!("({cond}) && state.quinn_dark == 1");
            paint_conditional(&mut rendered, &base_map, &base, &on_map, &cond);
            continue;
        }
        let mut light = prepared_style(el, tag, parent, stretch);
        light.merge_from(&parse_classes_in(&when.token, false));
        let mut dark = prepared_style(el, tag, parent, stretch);
        dark.merge_from(&parse_classes_in(&when.token, true));
        tune_style(&mut light, parent, stretch_self);
        tune_style(&mut dark, parent, stretch_self);
        let light_map = prop_map(&light, tag);
        let dark_map = prop_map(&dark, tag);
        let mut keys: Vec<String> = light_map.keys().cloned().collect();
        for key in dark_map.keys() {
            if !keys.iter().any(|have| have == key) {
                keys.push(key.clone());
            }
        }
        for key in keys {
            let light_value = light_map
                .get(&key)
                .cloned()
                .or_else(|| base_map.get(&key).cloned());
            let dark_value = dark_map
                .get(&key)
                .cloned()
                .or_else(|| base_map.get(&key).cloned());
            let Some(light_value) = light_value else {
                continue;
            };
            let dark_value = dark_value.unwrap_or_else(|| light_value.clone());
            if base_map.get(&key).map(String::as_str) == Some(light_value.as_str())
                && light_value == dark_value
            {
                continue;
            }
            let value = if light_value == dark_value {
                light_value
            } else {
                format!("if state.quinn_dark == 1 {{ {dark_value} }} else {{ {light_value} }}")
            };
            let else_value = rendered
                .get(&key)
                .cloned()
                .unwrap_or_else(|| resting_fill(&key, &base));
            rendered.insert(
                key,
                format!("if {cond} {{ {value} }} else {{ {else_value} }}"),
            );
        }
    }
    for when in &el.style_when {
        if let Some((key, yes, no)) = style_when_prop(&when.prop, &when.yes, &when.no) {
            let cond = class_cond(&when.signal, row);
            rendered.insert(key, format!("if {cond} {{ {yes} }} else {{ {no} }}"));
        }
    }
    if let Some(min_width) = ROW_AT.with(Cell::get) {
        if !base.height_explicit {
            let current = rendered.get("height").cloned();
            let fills = current
                .as_deref()
                .is_some_and(|value| value.starts_with("Fill"));
            if !fills {
                let else_height = current.unwrap_or_else(|| "Fit".into());
                rendered.insert(
                    "height".into(),
                    format!(
                        "if state.quinn_width >= {} {{ Fill }} else {{ {else_height} }}",
                        script_num(min_width)
                    ),
                );
            }
        }
    }
    if let Some(bind) = &el.bind_width {
        let width = width_bind_script(bind);
        rendered.insert("width".into(), width);
    }
    let mut keys: Vec<String> = rendered.keys().cloned().collect();
    keys.sort_by_key(|key| prop_order(key));
    keys.into_iter()
        .map(|key| {
            let value = rendered.remove(&key).unwrap_or_default();
            format!("{key}: {value}")
        })
        .collect()
}

/// Props one utility sets on its own: the keys whose value differs from an
/// unstyled element of the same tag.
fn token_keys(token: &str, dark: bool, tag: Tag) -> std::collections::BTreeSet<String> {
    let empty = prop_map(&Style::default(), tag);
    let only = prop_map(&parse_classes_in(token, dark), tag);
    only.into_iter()
        .filter(|(key, value)| empty.get(key) != Some(value))
        .map(|(key, _)| key)
        .collect()
}

fn paint_conditional(
    rendered: &mut std::collections::BTreeMap<String, String>,
    base_map: &std::collections::BTreeMap<String, String>,
    base: &Style,
    on_map: &std::collections::BTreeMap<String, String>,
    cond: &str,
) {
    for (key, value) in on_map {
        if base_map.get(key).map(String::as_str) == Some(value.as_str()) {
            continue;
        }
        let else_value = rendered
            .get(key)
            .cloned()
            .unwrap_or_else(|| resting_fill(key, base));
        rendered.insert(
            key.clone(),
            format!("if {cond} {{ {value} }} else {{ {else_value} }}"),
        );
    }
}

fn stripe_tokens(class: &str) -> Vec<(bool, String)> {
    let mut out = Vec::new();
    for token in class.split_whitespace() {
        let mut rest = token;
        if let Some(next) = rest.strip_prefix("dark:") {
            rest = next;
        }
        if let Some(next) = rest.strip_prefix("odd:") {
            out.push((true, next.to_string()));
        } else if let Some(next) = rest.strip_prefix("even:") {
            out.push((false, next.to_string()));
        }
    }
    out
}

fn width_bind_script(bind: &WidthBind) -> String {
    let factor = script_num(bind.factor);
    let raw = if bind.factor == 1.0 {
        format!("state.{}", bind.signal)
    } else {
        format!("state.{} * {factor}", bind.signal)
    };
    if bind.bias == 0.0 {
        raw
    } else {
        format!(
            "quinn_clamp({raw} + {}, 0, 100000)",
            script_num(bind.bias)
        )
    }
}

/// `md:flex-row` (and the other breakpoints) plus `items-stretch`.
fn row_stretch_at(class: &str) -> Option<f64> {
    let parsed = parse_classes(class);
    if parsed.cross_align != Some(quinn_styles::CrossAlign::Stretch) {
        return None;
    }
    shell_variants(class).into_iter().find_map(|when| {
        if when.token == "flex-row" {
            when.min_width
        } else {
            None
        }
    })
}

fn prepared_style(el: &El, tag: Tag, parent: FlowSpec, stretch: bool) -> Style {
    let mut style = tag.defaults();
    if !el.class.is_empty() {
        style.merge_from(&parse_classes(&el.class));
    }
    if !el.style.is_empty() {
        style.merge_from(&quinn_styles::parse_inline(&el.style));
    }
    tune_style(&mut style, parent, stretch);
    style
}

fn tune_style(style: &mut Style, parent: FlowSpec, stretch: bool) {
    // `abs_pos` is one point. A stretched Fill on that walk collapses to
    // nothing, so an absolute box shrink-wraps unless the class set a size.
    if style.absolute {
        if !style.width_explicit {
            style.width = Some(SizeSpec::Fit);
            style.width_explicit = true;
        }
        if !style.height_explicit {
            style.height = Some(SizeSpec::Fit);
            style.height_explicit = true;
        }
    }
    if stretch {
        match parent {
            FlowSpec::Row if !style.height_explicit => style.height = Some(SizeSpec::Fill),
            FlowSpec::Column if !style.width_explicit => style.width = Some(SizeSpec::Fill),
            _ => {}
        }
    }
    if style.flex.is_empty() {
        return;
    }
    let main_width = matches!(parent, FlowSpec::Row);
    let slot = if main_width {
        style.width
    } else {
        style.height
    };
    let explicit = if main_width {
        style.width_explicit
    } else {
        style.height_explicit
    };
    let fixed = matches!(slot, Some(SizeSpec::Px(_)) | Some(SizeSpec::Rel { .. }));
    if fixed && explicit {
        return;
    }
    if style.flex.grow.is_none()
        && style.flex.basis.is_none()
        && !matches!(slot, Some(SizeSpec::Fill))
    {
        return;
    }
    let weight = style
        .flex
        .grow
        .unwrap_or(if matches!(slot, Some(SizeSpec::Fill)) {
            100.0
        } else {
            1.0
        });
    let shrink = style.flex.shrink.unwrap_or(0.0);
    let basis = style.flex.basis.unwrap_or(0.0);
    let next = if weight == 100.0 && shrink == 0.0 && basis == 0.0 {
        SizeSpec::Fill
    } else {
        SizeSpec::Flex {
            weight,
            shrink,
            basis,
        }
    };
    if main_width {
        style.width = Some(next);
    } else {
        style.height = Some(next);
    }
}

fn prop_map(style: &Style, tag: Tag) -> std::collections::BTreeMap<String, String> {
    let mut map = std::collections::BTreeMap::new();
    for (key, value) in layout_pairs(style, matches!(tag, Tag::Div)) {
        map.insert(key, value);
    }
    for (key, value) in walk_pairs(style) {
        map.insert(key, value);
    }
    for (key, value) in paint_pairs(style, tag) {
        map.insert(key, value);
    }
    // Makepad's Button and TextInput carry a theme margin. Inside a parent
    // sized to the control, that margin pushes the box past the parent's
    // edge and the bottom border is clipped. Siblings get their spacing
    // from the parent's gap, so a control has no margin unless a class asks.
    if matches!(tag, Tag::Button | Tag::Input) {
        map.entry("margin".into()).or_insert_with(|| "0.".into());
    }
    map
}

/// Makepad `Grid` implicit rows are a fixed pixel size and default to 0.
fn implicit_row_px(el: &El) -> f64 {
    let mut max = 0.0;
    fn walk(node: &ViewNode, max: &mut f64) {
        match node {
            ViewNode::El(child) => {
                let mut style = Style::default();
                if !child.class.is_empty() {
                    style.merge_from(&parse_classes(&child.class));
                }
                if !child.style.is_empty() {
                    style.merge_from(&quinn_styles::parse_inline(&child.style));
                }
                if let Some(SizeSpec::Px(px)) = style.height {
                    *max = (*max).max(px);
                }
            }
            ViewNode::Show { child, .. } => walk(child, max),
            ViewNode::Slot { nodes, .. } => {
                for child in nodes {
                    walk(child, max);
                }
            }
            ViewNode::Routes { routes } => {
                for (_, nodes) in routes {
                    for child in nodes {
                        walk(child, max);
                    }
                }
            }
            ViewNode::For { .. } => {}
            ViewNode::Suspense { fallback, child, .. }
            | ViewNode::Boundary { fallback, child, .. } => {
                for node in fallback {
                    walk(node, max);
                }
                walk(child, max);
            }
            ViewNode::Portal { nodes } => {
                for node in nodes {
                    walk(node, max);
                }
            }
        }
    }
    for child in &el.children {
        walk(child, &mut max);
    }
    if max > 0.0 {
        max
    } else {
        40.0
    }
}

fn layout_pairs(style: &Style, container: bool) -> Vec<(String, String)> {
    let mut lines = Vec::new();
    if let Some(cols) = style.grid_cols {
        let tracks = (0..cols).map(|_| "\"1fr\"").collect::<Vec<_>>().join(", ");
        lines.push(("columns".into(), format!("[{tracks}]")));
        if let Some(gap) = style.main_gap() {
            lines.push(("column_gap".into(), script_num(gap)));
        }
        if let Some(gap) = style.cross_gap() {
            lines.push(("row_gap".into(), script_num(gap)));
        }
        if let Some(padding) = padding_script(style) {
            lines.push(("padding".into(), padding));
        }
        push_clip(style, &mut lines);
        return lines;
    }
    let lay_out = container
        || style.flow.is_some()
        || style.wrap
        || style.distribute.is_some()
        || style.main_align.is_some()
        || style.cross_align.is_some()
        || style.gap.is_some()
        || style.gap_x.is_some()
        || style.gap_y.is_some();
    if !lay_out && !style.padding.any() {
        push_clip(style, &mut lines);
        push_scroll(style, &mut lines);
        return lines;
    }
    if !lay_out {
        if let Some(padding) = padding_script(style) {
            lines.push(("padding".into(), padding));
        }
        push_clip(style, &mut lines);
        push_scroll(style, &mut lines);
        return lines;
    }
    lines.push(("flow".into(), flow_script(style)));
    let flow = style.flow.unwrap_or(if container {
        FlowSpec::Column
    } else {
        FlowSpec::Row
    });
    let spacing = match flow {
        FlowSpec::Row => style.main_gap(),
        FlowSpec::Column => style.cross_gap(),
    };
    if let Some(gap) = spacing {
        lines.push(("spacing".into(), script_num(gap)));
    }
    if style.wrap {
        if let Some(gap) = style.cross_gap() {
            lines.push(("wrap_spacing".into(), script_num(gap)));
        }
    }
    if let Some(padding) = padding_script(style) {
        lines.push(("padding".into(), padding));
    }
    if let Some(align) = align_script(style) {
        lines.push(("align".into(), align));
    }
    if let Some(dist) = style.distribute {
        lines.push(("distribute".into(), format!("Distribute.{}", dist.script())));
    }
    push_clip(style, &mut lines);
    push_scroll(style, &mut lines);
    lines
}

fn push_clip(style: &Style, lines: &mut Vec<(String, String)>) {
    if style.clip {
        lines.push(("clip_x".into(), "true".into()));
        lines.push(("clip_y".into(), "true".into()));
    }
}

fn push_scroll(style: &Style, lines: &mut Vec<(String, String)>) {
    if style.scroll_x || style.scroll_y {
        lines.push((
            "scroll_bars".into(),
            format!(
                "ScrollBars{{show_scroll_x: {} show_scroll_y: {}}}",
                if style.scroll_x { "true" } else { "false" },
                if style.scroll_y { "true" } else { "false" }
            ),
        ));
    }
}

fn walk_pairs(style: &Style) -> Vec<(String, String)> {
    let mut lines = Vec::new();
    if let Some(size) = style.width {
        lines.push(("width".into(), size_script(size)));
    }
    if let Some(size) = style.height {
        lines.push(("height".into(), size_script(size)));
    }
    if let Some(bound) = style.min_width {
        lines.push(("min_width".into(), bound_script(bound)));
    }
    if let Some(bound) = style.max_width {
        lines.push(("max_width".into(), bound_script(bound)));
    }
    if style.min_control {
        lines.push(("min_height".into(), density_height(28.0, 36.0, 48.0)));
    } else if let Some(bound) = style.min_height {
        lines.push(("min_height".into(), bound_script(bound)));
    }
    if let Some(bound) = style.max_height {
        lines.push(("max_height".into(), bound_script(bound)));
    }
    if let Some(margin) = inset_script(style.margin) {
        lines.push(("margin".into(), margin));
    }
    if let Some(span) = style.col_span {
        lines.push(("cell".into(), format!("CellPlacement{{col_span: {span}}}")));
    }
    lines
}

fn focus_ring() -> &'static str {
    "if state.quinn_dark == 1 { #x93c5fd } else { #x1d4ed8 }"
}

fn placeholder_ink() -> String {
    "if state.quinn_dark == 1 { #x94a3b8 } else { #x5f6f85 }".into()
}

fn density_height(compact: f64, standard: f64, touch: f64) -> String {
    format!(
        "if state.quinn_density == 2 {{ {} }} else {{ if state.quinn_density == 0 {{ {} }} else {{ {} }} }}",
        script_num(touch),
        script_num(compact),
        script_num(standard)
    )
}

/// Makepad `GaussRoundedView` samples the window scene (a render-to-texture
/// pyramid in `gauss_view.rs`) and tints it. This is not desktop wallpaper
/// vibrancy: nothing behind the app window is in that texture. A sidebar
/// beside the content therefore frosts the window fill, and a sheet or toast
/// over the content frosts the content.
fn vibrancy_pairs() -> Vec<(String, String)> {
    let tint = "if state.quinn_dark == 1 { #x202022 } else { #xf6f6f6 }".to_string();
    vec![
        ("draw_bg.corner_radius".into(), "0.".into()),
        ("draw_bg.blur_level".into(), "1.5".into()),
        ("draw_bg.edge_blur_level".into(), "0.25".into()),
        ("draw_bg.lensing_effect".into(), "0.".into()),
        ("draw_bg.lensing_strength".into(), "0.".into()),
        ("draw_bg.lensing_width".into(), "0.".into()),
        ("draw_bg.tint_color".into(), tint.clone()),
        ("draw_bg.tint_alpha".into(), "0.86".into()),
        ("draw_bg.fallback_color".into(), tint),
        ("draw_bg.rim_alpha".into(), "0.".into()),
        ("draw_bg.inner_shadow_alpha".into(), "0.".into()),
        ("draw_bg.shadow_radius".into(), "0.".into()),
        ("draw_bg.shadow_offset".into(), "vec2(0., 0.)".into()),
        ("draw_bg.specular_strength".into(), "0.02".into()),
        ("draw_bg.noise_strength".into(), "0.008".into()),
        ("draw_bg.border_width".into(), "0.".into()),
        ("draw_bg.surface_alpha".into(), "0.94".into()),
    ]
}

fn separator_rgba() -> quinn_styles::Rgba {
    quinn_styles::Rgba {
        r: 60,
        g: 60,
        b: 67,
        a: 74,
    }
}

/// Push `key` only when an earlier utility did not already set it.
/// `prop_map` keeps the last write, so this must not clobber `hover:bg-*`.
fn mirror_state(lines: &mut Vec<(String, String)>, key: &str, color: Option<quinn_styles::Rgba>) {
    if lines.iter().any(|(have, _)| have == key) {
        return;
    }
    let Some(color) = color else {
        return;
    };
    lines.push((key.to_string(), color.hex_script()));
}

fn paint_pairs(style: &Style, tag: Tag) -> Vec<(String, String)> {
    let mut lines = Vec::new();
    if style.grid_cols.is_some() {
        if let Some(color) = style.background {
            lines.push(("show_bg".into(), "true".into()));
            lines.push(("draw_bg.color".into(), color.hex_script()));
        }
        return lines;
    }
    let paints = matches!(tag, Tag::Button | Tag::Input | Tag::Div);
    let gradient = style
        .gradient
        .filter(|grad| grad.from.is_some() || grad.to.is_some());
    if paints {
        if let Some(grad) = gradient.filter(|_| matches!(tag, Tag::Div)) {
            let (start, end) = if grad.flip {
                (grad.to, grad.from)
            } else {
                (grad.from, grad.to)
            };
            if let Some(color) = start.or(style.background) {
                lines.push(("draw_bg.color".into(), color.hex_script()));
            }
            if let Some(color) = end {
                lines.push(("draw_bg.color_2".into(), color.hex_script()));
            }
            let horizontal = matches!(grad.axis, quinn_styles::GradientAxis::Horizontal);
            lines.push((
                "draw_bg.gradient_fill_horizontal".into(),
                if horizontal { "1.".into() } else { "0.".into() },
            ));
        } else if let Some(color) = style.background {
            if !style.vibrancy {
                lines.push(("draw_bg.color".into(), color.hex_script()));
            }
        }
        if style.vibrancy {
            lines.extend(vibrancy_pairs());
        }
        if let Some(shadow) = style.shadow {
            if matches!(tag, Tag::Div) {
                lines.push(("draw_bg.shadow_radius".into(), script_num(shadow.radius)));
                lines.push((
                    "draw_bg.shadow_offset".into(),
                    format!("vec2(0., {})", script_num(shadow.offset_y)),
                ));
                // Four-digit `#RRGGBBAA` is the theme's translucent black.
                lines.push(("draw_bg.shadow_color".into(), "#0006".into()));
            }
        }
        if let Some(color) = style.hover_bg {
            if !matches!(tag, Tag::Div) {
                lines.push(("draw_bg.color_hover".into(), color.hex_script()));
            }
        }
        if let Some(color) = style.focus_bg {
            if matches!(tag, Tag::Button | Tag::Input) {
                lines.push(("draw_bg.color_focus".into(), color.hex_script()));
            }
        }
        if let Some(color) = style.active_bg {
            lines.push(("draw_bg.color_down".into(), color.hex_script()));
        }
        if let Some(color) = style.disabled_bg {
            lines.push(("draw_bg.color_disabled".into(), color.hex_script()));
        }
        if let Some(radius) = style.radius {
            lines.push(("draw_bg.border_radius".into(), script_num(radius)));
        }
    }
    if let Some(color) = style.color {
        if tag != Tag::Div {
            lines.push(("draw_text.color".into(), color.hex_script()));
        }
    }
    if matches!(tag, Tag::Button) {
        if let Some(color) = style.hover_fg {
            lines.push(("draw_text.color_hover".into(), color.hex_script()));
        }
        if let Some(color) = style.focus_fg {
            lines.push(("draw_text.color_focus".into(), color.hex_script()));
        }
        if let Some(color) = style.active_fg {
            lines.push(("draw_text.color_down".into(), color.hex_script()));
        }
        if let Some(color) = style.disabled_fg {
            lines.push(("draw_text.color_disabled".into(), color.hex_script()));
        }
        if style.disabled {
            lines.push(("enabled".into(), "false".into()));
            lines.push(("draw_bg.disabled".into(), "1.".into()));
            lines.push(("draw_text.disabled".into(), "1.".into()));
        }
        // The theme's color_focus / color_down are a pale outset. A click
        // focuses the button and the pressed mix can stick across render, so
        // the face went grey and the label went with the theme ink. Every
        // state uses the face and label we already chose, and the pressed
        // track is cut back to off on click. The focus ring is the border.
        mirror_state(&mut lines, "draw_bg.color_hover", style.hover_bg.or(style.background));
        mirror_state(&mut lines, "draw_bg.color_down", style.active_bg.or(style.background));
        mirror_state(&mut lines, "draw_bg.color_focus", style.focus_bg.or(style.background));
        mirror_state(
            &mut lines,
            "draw_bg.color_disabled",
            style.disabled_bg.or(style.background),
        );
        mirror_state(&mut lines, "draw_text.color_hover", style.hover_fg.or(style.color));
        mirror_state(&mut lines, "draw_text.color_down", style.active_fg.or(style.color));
        mirror_state(&mut lines, "draw_text.color_focus", style.focus_fg.or(style.color));
        mirror_state(
            &mut lines,
            "draw_text.color_disabled",
            style.disabled_fg.or(style.color),
        );
        lines.push(("reset_hover_on_click".into(), "true".into()));
        lines.push((
            "draw_bg.color_2".into(),
            "vec4(-1., -1., -1., -1.)".into(),
        ));
        lines.push((
            "draw_bg.border_color_2".into(),
            "vec4(-1., -1., -1., -1.)".into(),
        ));
        lines.push(("draw_bg.layer_hover".into(), "0.".into()));
        lines.push(("draw_bg.layer_down".into(), "0.".into()));
        let border_size = style.border_width.unwrap_or(1.0);
        lines.push(("draw_bg.border_size".into(), script_num(border_size)));
        let border = style.border_color.unwrap_or_else(separator_rgba);
        mirror_state(&mut lines, "draw_bg.border_color", Some(border));
        mirror_state(&mut lines, "draw_bg.border_color_hover", Some(border));
        mirror_state(&mut lines, "draw_bg.border_color_down", Some(border));
        // systemBlue in light, the brighter systemBlue in dark. Not the theme's pale focus fill.
        lines.push((
            "draw_bg.border_color_focus".into(),
            focus_ring().into(),
        ));
    }
    if matches!(tag, Tag::Input) {
        // The inset theme tints a focused field. Keep the well flat and put
        // systemBlue on the stroke, which is the macOS focus ring.
        mirror_state(&mut lines, "draw_bg.color_hover", style.hover_bg.or(style.background));
        mirror_state(&mut lines, "draw_bg.color_down", style.active_bg.or(style.background));
        mirror_state(&mut lines, "draw_bg.color_focus", style.focus_bg.or(style.background));
        mirror_state(&mut lines, "draw_bg.color_empty", style.background);
        mirror_state(
            &mut lines,
            "draw_bg.color_disabled",
            style.disabled_bg.or(style.background),
        );
        lines.push((
            "draw_bg.color_2".into(),
            "vec4(-1., -1., -1., -1.)".into(),
        ));
        lines.push((
            "draw_bg.border_color_2".into(),
            "vec4(-1., -1., -1., -1.)".into(),
        ));
        if style.border_width.is_none() {
            lines.push(("draw_bg.border_size".into(), "1.".into()));
        }
        let border = style.border_color.unwrap_or_else(separator_rgba);
        mirror_state(&mut lines, "draw_bg.border_color", Some(border));
        mirror_state(&mut lines, "draw_bg.border_color_hover", Some(border));
        mirror_state(&mut lines, "draw_bg.border_color_down", Some(border));
        mirror_state(&mut lines, "draw_bg.border_color_empty", Some(border));
        lines.push((
            "draw_bg.border_color_focus".into(),
            focus_ring().into(),
        ));
        // tertiaryLabel. The theme placeholder is the same grey as a light well,
        // so "Search" disappeared in the light appearance.
        let empty = placeholder_ink();
        lines.push(("draw_text.color_empty".into(), empty.clone()));
        lines.push(("draw_text.color_empty_hover".into(), empty.clone()));
        lines.push(("draw_text.color_empty_focus".into(), empty));
        mirror_state(&mut lines, "draw_text.color_hover", style.hover_fg.or(style.color));
        mirror_state(&mut lines, "draw_text.color_down", style.active_fg.or(style.color));
        mirror_state(&mut lines, "draw_text.color_focus", style.focus_fg.or(style.color));
    }
    if tag != Tag::Div {
        let size = style.font_size.unwrap_or(14.0);
        let weight = style.font_weight.unwrap_or(400.0);
        let leading = style.line_height.unwrap_or(20.0 / 14.0);
        // SF Pro cannot be redistributed. Makepad ships Inter, which is the
        // fallback face. Weight is the Inter variable axis.
        lines.push((
            "draw_text.text_style".into(),
            format!(
                "TextStyle{{font_size: {} line_spacing: {} font_family: FontFamily{{latin := FontMember{{res: crate_resource(\"makepad_widgets:resources/Inter.ttf\") asc: 0.0 desc: 0.0 weight: {}}}}}}}",
                script_num(size),
                script_num(leading),
                script_num(weight)
            ),
        ));
    }
    if let Some(width) = style.border_width {
        if matches!(tag, Tag::Input | Tag::Div) {
            lines.push(("draw_bg.border_size".into(), script_num(width)));
        }
    }
    if let Some(color) = style.border_color {
        if matches!(tag, Tag::Input | Tag::Div) {
            lines.push(("draw_bg.border_color".into(), color.hex_script()));
        }
    }
    lines
}

fn flow_script(style: &Style) -> String {
    match style.flow.unwrap_or(FlowSpec::Column) {
        FlowSpec::Column => "Down".into(),
        FlowSpec::Row => {
            let align = match style.cross_align {
                Some(quinn_styles::CrossAlign::Center) => "Center",
                Some(quinn_styles::CrossAlign::End) => "Bottom",
                Some(quinn_styles::CrossAlign::Start)
                | Some(quinn_styles::CrossAlign::Stretch)
                | None => "Top",
            };
            if !style.wrap && align == "Top" {
                "Right".into()
            } else {
                format!(
                    "Flow.Right{{wrap: {} row_align: RowAlign.{align}}}",
                    if style.wrap { "true" } else { "false" }
                )
            }
        }
    }
}

fn align_script(style: &Style) -> Option<String> {
    let flow = style.flow.unwrap_or(FlowSpec::Column);
    let x = match flow {
        FlowSpec::Row => style.main_align.map(AlignSpec::fraction).unwrap_or(0.0),
        FlowSpec::Column => match style.cross_align {
            Some(quinn_styles::CrossAlign::Center) => 0.5,
            Some(quinn_styles::CrossAlign::End) => 1.0,
            _ => 0.0,
        },
    };
    // `RowAlign` centers items within the row's line; a row taller than its
    // content (`h-control`, `inset-0`) also needs the line itself centered.
    let y = match flow {
        FlowSpec::Column => style.main_align.map(AlignSpec::fraction).unwrap_or(0.0),
        FlowSpec::Row if style.wrap => 0.0,
        FlowSpec::Row => match style.cross_align {
            Some(quinn_styles::CrossAlign::Center) => 0.5,
            Some(quinn_styles::CrossAlign::End) => 1.0,
            _ => 0.0,
        },
    };
    if x == 0.0 && y == 0.0 {
        return None;
    }
    Some(format!(
        "Align{{x: {} y: {}}}",
        script_num(x),
        script_num(y)
    ))
}

fn size_script(size: SizeSpec) -> String {
    match size {
        SizeSpec::Fill => "Fill".into(),
        SizeSpec::Fit => "Fit".into(),
        SizeSpec::Px(px) => script_num(px),
        SizeSpec::Rel { base, factor } => {
            format!(
                "Rel{{base: {} factor: {}}}",
                base.script(),
                script_num(factor)
            )
        }
        SizeSpec::Flex {
            weight,
            shrink,
            basis,
        } => format!(
            "Fill{{weight: {} shrink: {} basis: {}}}",
            script_num(weight),
            script_num(shrink),
            script_num(basis)
        ),
        SizeSpec::Control => density_height(28.0, 36.0, 48.0),
        SizeSpec::Bar => density_height(48.0, 48.0, 56.0),
    }
}

fn padding_script(style: &Style) -> Option<String> {
    if style.control_pad {
        let (_, top, _, bottom) = style.padding.resolve();
        let x = density_height(8.0, 12.0, 16.0);
        return Some(format!(
            "Inset{{left: {x} top: {} right: {x} bottom: {}}}",
            script_num(top),
            script_num(bottom)
        ));
    }
    inset_script(style.padding)
}

fn bound_script(bound: quinn_styles::Bound) -> String {
    match bound {
        quinn_styles::Bound::Px(px) => script_num(px),
        quinn_styles::Bound::Full => "\"100%\"".into(),
    }
}

fn inset_script(edges: quinn_styles::Edges) -> Option<String> {
    if !edges.any() {
        return None;
    }
    let (left, top, right, bottom) = edges.resolve();
    if left == top && top == right && right == bottom {
        Some(script_num(left))
    } else {
        Some(format!(
            "Inset{{left: {} top: {} right: {} bottom: {}}}",
            script_num(left),
            script_num(top),
            script_num(right),
            script_num(bottom)
        ))
    }
}

fn style_when_prop(prop: &str, yes: &str, no: &str) -> Option<(String, String, String)> {
    let one = |value: &str| {
        let style = quinn_styles::parse_inline(&format!("{prop}: {value}"));
        match prop {
            "width" => style.width.map(size_script),
            "height" => style.height.map(size_script),
            "color" => style.color.map(|color| color.hex_script()),
            "background" | "background-color" => style.background.map(|color| color.hex_script()),
            "font-size" => style.font_size.map(script_num),
            "padding" => inset_script(style.padding),
            "margin" => inset_script(style.margin),
            "gap" => style.gap.map(script_num),
            "radius" | "border-radius" => style.radius.map(script_num),
            _ => None,
        }
    };
    let key = match prop {
        "width" => "width",
        "height" => "height",
        "color" => "draw_text.color",
        "background" | "background-color" => "draw_bg.color",
        "font-size" => "draw_text.text_style.font_size",
        "padding" => "padding",
        "margin" => "margin",
        "gap" => "spacing",
        "radius" | "border-radius" => "draw_bg.border_radius",
        _ => return None,
    };
    Some((key.into(), one(yes)?, one(no)?))
}

/// A conditional fill with no resting color stays clear. Opaque black hid
/// unselected navigation rows, list rows, and even zebra stripes.
fn resting_fill(key: &str, base: &Style) -> String {
    if key == "draw_bg.color" && base.background.is_none() {
        "#00000000".into()
    } else {
        prop_fallback(key, base)
    }
}

fn prop_fallback(key: &str, base: &Style) -> String {
    match key {
        "draw_bg.color"
        | "draw_bg.color_hover"
        | "draw_bg.color_down"
        | "draw_bg.color_disabled"
        | "draw_bg.color_focus" => base
            .background
            .map(|color| color.hex_script())
            .unwrap_or_else(|| "#x000000".into()),
        "draw_text.color"
        | "draw_text.color_hover"
        | "draw_text.color_down"
        | "draw_text.color_disabled"
        | "draw_text.color_focus" => base
            .color
            .map(|color| color.hex_script())
            .unwrap_or_else(|| "#x000000".into()),
        // A missing max must not clamp. The generic `0` else hid the 64-wide
        // rail whenever `bp1120:max-w-*` was the only maximum.
        "max_width" => base
            .max_width
            .map(bound_script)
            .unwrap_or_else(|| "\"100%\"".into()),
        "max_height" => base
            .max_height
            .map(bound_script)
            .unwrap_or_else(|| "\"100%\"".into()),
        "width" | "height" => "Fit".into(),
        "flow" => "Down".into(),
        "distribute" => "Distribute.Start".into(),
        "align" => "Align{x: 0. y: 0.}".into(),
        _ => "0.".into(),
    }
}

fn prop_order(key: &str) -> u8 {
    match key {
        "width" => 0,
        "height" => 1,
        "min_width" => 2,
        "max_width" => 3,
        "min_height" => 4,
        "max_height" => 5,
        "margin" => 6,
        "flow" => 7,
        "spacing" => 8,
        "wrap_spacing" => 9,
        "padding" => 10,
        "align" => 11,
        "distribute" => 12,
        _ => 20,
    }
}

fn escape_script(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn script_num(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e12 {
        format!("{}.", value as i64)
    } else {
        format!("{value}")
    }
}

/// `provide_context` fills `mod.ctx` once. A later render, including a hot
/// reload, leaves a value that is already there.
fn emit_ctx(model: &Model) -> String {
    let mut provides = model.ctx_init.clone();
    collect_provides(&model.nodes, &mut provides);
    if provides.is_empty() && !model.has_ctx {
        return String::new();
    }
    let mut out =
        String::from("        if ok{mod.ctx} == nil {\n            mod.ctx = {}\n        }\n");
    let mut seen = Vec::new();
    for (key, expr) in provides {
        if seen.iter().any(|have| have == &key) {
            continue;
        }
        seen.push(key.clone());
        // A missing field errors on `.key`. `ok{}` is nil until the seed writes it.
        out.push_str(&format!(
            "        if ok{{mod.ctx.{key}}} == nil {{\n            mod.ctx.{key} = {expr}\n        }}\n"
        ));
    }
    out
}

fn collect_provides(nodes: &[ViewNode], out: &mut Vec<(String, String)>) {
    for node in nodes {
        match node {
            ViewNode::El(el) => {
                // Nested providers assign around the widget. They are not
                // copied into the root nil-guard, or the first value would win.
                collect_provides(&el.children, out);
            }
            ViewNode::Show { child, .. } => collect_provides(std::slice::from_ref(child), out),
            ViewNode::For { child, .. } => collect_provides(&child.children, out),
            ViewNode::Slot { nodes, .. } => collect_provides(nodes, out),
            ViewNode::Routes { routes } => {
                for (_, nodes) in routes {
                    collect_provides(nodes, out);
                }
            }
            ViewNode::Suspense { fallback, child, .. }
            | ViewNode::Boundary { fallback, child, .. } => {
                collect_provides(fallback, out);
                collect_provides(std::slice::from_ref(child), out);
            }
            ViewNode::Portal { nodes } => collect_provides(nodes, out),
        }
    }
}
