//! Makepad `script_mod!` text. The header stays a fixed height so the
//! block the watcher keys on does not move when the view grows.

use quinn_core::{Style, Tag};
use quinn_styles::{parse_classes, AlignSpec, FlowSpec, SizeSpec};

use crate::model::*;
use crate::Shell;

pub fn emit_rust(model: &Model, shell: &Shell) -> String {
    let mut body = String::new();
    body.push_str("        use mod.prelude.widgets.*\n");
    // A missing `mod.state` is an error, not nil. `ok{...}` turns that into nil.
    body.push_str("        if ok{mod.state} == nil {\n");
    body.push_str("            mod.state = {\n");
    for line in seed_lines(model) {
        body.push_str(&format!("                {line}\n"));
    }
    body.push_str("            }\n");
    body.push_str("        }\n");
    body.push_str("        let state = mod.state\n");
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
    body.push_str(&emit_show_fns(&model.nodes, &shows));
    each_for(&model.nodes, &mut |fn_name, params, row_signals, child| {
        body.push_str(&emit_row_fn(fn_name, params, row_signals, child));
    });
    if model.retain {
        body.push_str(&emit_retain(model));
    }
    let host = host_fields(&model.root_class);
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
    let root_style = parse_classes(&model.root_class);
    let root_flow = root_style.flow.unwrap_or(FlowSpec::Column);
    let root_stretch = root_style.cross_align == Some(quinn_styles::CrossAlign::Stretch);
    for node in &model.nodes {
        children.push_str(&emit_node(
            node,
            8,
            None,
            root_flow,
            root_stretch,
            &mut cursor,
        ));
    }
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
        el.click.as_ref().is_some_and(|click| act(click, op)) || el.children.iter().any(|node| walk(node, op))
    }
    fn walk(node: &ViewNode, op: ListOp) -> bool {
        match node {
            ViewNode::El(item) => el(item, op),
            ViewNode::Show { child, .. } => walk(child, op),
            ViewNode::For { child, .. } => el(child, op),
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

fn each_for(nodes: &[ViewNode], visit: &mut dyn FnMut(&str, &[String], &[Signal], &El)) {
    for node in nodes {
        match node {
            ViewNode::For {
                fn_name,
                params,
                row_signals,
                child,
                ..
            } => visit(fn_name, params, row_signals, child),
            ViewNode::El(el) => each_for(&el.children, visit),
            ViewNode::Show { child, .. } => each_for(std::slice::from_ref(child), visit),
        }
    }
}

fn row_maps(model: &Model) -> Vec<(String, RowInit)> {
    let mut out = Vec::new();
    each_for(&model.nodes, &mut |_, _, row_signals, child| {
        let count = count_signal(row_signals);
        for signal in row_signals {
            if Some(signal.read.as_str()) == count {
                continue;
            }
            let init = match &signal.init {
                Init::Bool(_) => RowInit::Bool,
                Init::Str(_) => RowInit::Str,
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
            ViewNode::Show { live_flag, child, .. } => {
                out.push(live_flag.clone());
                walk(child, out);
            }
            ViewNode::El(el) => {
                for child in &el.children {
                    walk(child, out);
                }
            }
            ViewNode::For { .. } => {}
        }
    }
    for child in &el.children {
        walk(child, &mut out);
    }
    out
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

fn seed_lines(model: &Model) -> Vec<String> {
    let mut lines = Vec::new();
    for signal in &model.signals {
        lines.push(match &signal.init {
            Init::Int(value) => format!("{}: {value}", signal.read),
            Init::Bool(value) => format!("{}: {value}", signal.read),
            Init::Str(value) => format!("{}: \"{}\"", signal.read, escape_script(value)),
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
    lines
}

fn live_flags(nodes: &[ViewNode]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(node: &ViewNode, out: &mut Vec<String>) {
        match node {
            ViewNode::Show { live_flag, child, .. } => {
                out.push(live_flag.clone());
                walk(child, out);
            }
            ViewNode::El(el) => {
                for child in &el.children {
                    walk(child, out);
                }
            }
            ViewNode::For { .. } => {}
        }
    }
    for node in nodes {
        walk(node, &mut out);
    }
    out
}

fn emit_row_fn(fn_name: &str, params: &[String], row_signals: &[Signal], child: &El) -> String {
    let mut args = String::from("id");
    for param in params {
        args.push_str(", ");
        args.push_str(param);
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
        );
    }
    let row = RowEnv {
        fn_name,
        args: &args,
        signals: row_signals,
        count: count_signal(row_signals),
    };
    let mut cursor = ShowCursor {
        sites: &sites,
        index: 0,
    };
    let widget = emit_el(child, Some(&row), 3, false, FlowSpec::Column, false, &mut cursor);
    format!("{fns}        fn {fn_name}({args}) {{\n            let key = \"k\" + id\n{widget}        }}\n")
}

struct RowEnv<'a> {
    fn_name: &'a str,
    args: &'a str,
    signals: &'a [Signal],
    count: Option<&'a str>,
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
    let mut sites = Vec::new();
    let mut nth = std::collections::HashMap::<String, usize>::new();
    for node in nodes {
        collect_show(node, &mut sites, &mut nth);
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
    }
}

/// A statement `if` with no widget on both sides unbalances Makepad's script
/// stack (the branch is popped onto the parent). Each show is a function whose
/// `if` calls two functions, one returning the branch widget and one returning
/// `0`, which a view ignores.
fn emit_show_fns(nodes: &[ViewNode], sites: &[ShowSite]) -> String {
    let mut out = String::new();
    let mut cursor = ShowCursor { sites, index: 0 };
    for node in nodes {
        emit_show_fn(node, &mut cursor, &mut out);
    }
    out
}

fn emit_show_fn(node: &ViewNode, cursor: &mut ShowCursor<'_>, out: &mut String) {
    match node {
        ViewNode::El(el) => {
            for child in &el.children {
                emit_show_fn(child, cursor, out);
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
                        emit_show_fn(nested, cursor, out);
                    }
                }
                other => emit_show_fn(other, cursor, out),
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
                ),
                other => emit_node(
                    other,
                    3,
                    None,
                    FlowSpec::Column,
                    false,
                    &mut body_cursor,
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
            out.push_str(&format!(
                "        fn {name}_on() {{\n{body}        }}\n        fn {name}_off() {{\n            0\n        }}\n        fn {name}() {{\n{resets}            if state.{signal} {{\n                {name}_on()\n            }} else {{\n                {name}_off()\n            }}\n        }}\n"
            ));
        }
        ViewNode::For { .. } => {}
    }
}

fn emit_row_show_fns(
    node: &ViewNode,
    args: &str,
    row_signals: &[Signal],
    cursor: &mut ShowCursor<'_>,
    out: &mut String,
) {
    match node {
        ViewNode::El(el) => {
            for child in &el.children {
                emit_row_show_fns(child, args, row_signals, cursor, out);
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
                        emit_row_show_fns(nested, args, row_signals, cursor, out);
                    }
                }
                other => emit_row_show_fns(other, args, row_signals, cursor, out),
            }
            let row = RowEnv {
                fn_name: "quinn_row",
                args,
                signals: row_signals,
                count: count_signal(row_signals),
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
                ),
                other => emit_node(
                    other,
                    3,
                    Some(&row),
                    FlowSpec::Column,
                    false,
                    &mut body_cursor,
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
    }
}

fn emit_node(
    node: &ViewNode,
    indent: usize,
    row: Option<&RowEnv<'_>>,
    parent: FlowSpec,
    stretch: bool,
    shows: &mut ShowCursor<'_>,
) -> String {
    match node {
        ViewNode::El(el) => {
            let parsed = parse_classes(&el.class);
            if let Some(align) = parsed.align_self {
                if align != quinn_styles::CrossAlign::Stretch {
                    return emit_self_wrap(el, align, indent, row, parent, shows);
                }
                return emit_el(el, row, indent, true, parent, true, shows);
            }
            emit_el(el, row, indent, true, parent, stretch, shows)
        }
        ViewNode::Show { .. } => {
            let pad = " ".repeat(indent * 4);
            let site = &shows.sites[shows.index];
            let name = site.name.clone();
            shows.index = site.end;
            let call = if let Some(row) = row {
                format!("{name}({})", row.args)
            } else {
                format!("{name}()")
            };
            format!("{pad}{call}\n")
        }
        ViewNode::For {
            items,
            var,
            key_field,
            fn_name,
            params,
            ..
        } => {
            let pad = " ".repeat(indent * 4);
            let mut call = format!("{fn_name}({var}.{key_field}");
            for param in params {
                call.push_str(&format!(", {var}.{param}"));
            }
            call.push(')');
            format!("{pad}for {var} in state.{items} {{\n{pad}    {call}\n{pad}}}\n")
        }
    }
}

fn emit_self_wrap(
    el: &El,
    align: quinn_styles::CrossAlign,
    indent: usize,
    row: Option<&RowEnv<'_>>,
    parent: FlowSpec,
    shows: &mut ShowCursor<'_>,
) -> String {
    let pad = " ".repeat(indent * 4);
    let inner = emit_el(el, row, indent + 1, true, parent, false, shows);
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
) -> String {
    let pad = " ".repeat(indent * 4);
    let widget = widget_name(&el.tag);
    let tag = tag_of(&el.tag);
    let mut lines = Vec::new();
    if el.tag != "div" && (el.value.is_some() || !el.parts.is_empty()) {
        lines.push(format!("text: {}", text_expr(el, row)));
    }
    if let Some(placeholder) = &el.placeholder {
        lines.push(format!(
            "empty_text: \"{}\"",
            escape_script(placeholder)
        ));
    }
    lines.extend(style_lines(el, tag, parent, stretch, row));
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
    let inner = lines
        .into_iter()
        .map(|line| format!("{pad}    {line}\n"))
        .collect::<String>();
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
    let child_stretch = el.tag == "div" && child_style.cross_align == Some(quinn_styles::CrossAlign::Stretch);
    let mut children = String::new();
    for child in &el.children {
        children.push_str(&emit_node(
            child,
            indent + 1,
            row,
            child_flow,
            child_stretch,
            shows,
        ));
    }
    let head = if named {
        format!("{pad}{} := {widget} {{\n", el.name)
    } else {
        format!("{pad}{widget} {{\n")
    };
    format!("{head}{inner}{children}{pad}}}\n")
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
                lines.push(format!(
                    "let {name} = {}",
                    emit_hx(expr, row, &locals)
                ));
                locals.insert(name.clone());
            }
            HandlerStmt::Set { signal, expr } => {
                lines.push(emit_set(signal, expr, row, &locals));
            }
            HandlerStmt::Push { items, id, label } => {
                let id = emit_hx(id, row, &locals);
                let label = emit_hx(label, row, &locals);
                lines.push(format!("quinn_row_reset({id})"));
                lines.push(format!(
                    "state.{items}.push({{id: {id}, label: {label}}})"
                ));
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
        }
    }
    lines
}

fn emit_set(signal: &str, expr: &HxExpr, row: Option<&RowEnv<'_>>, locals: &std::collections::HashSet<String>) -> String {
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

fn emit_hx(expr: &HxExpr, row: Option<&RowEnv<'_>>, locals: &std::collections::HashSet<String>) -> String {
    match expr {
        HxExpr::Int(value) => value.to_string(),
        HxExpr::Bool(value) => if *value { "true" } else { "false" }.into(),
        HxExpr::Str(value) => format!("\"{}\"", escape_script(value)),
        HxExpr::Name(name) => {
            if locals.contains(name) || name == "text" || name == "value" || name == "id" {
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
                if row.is_some() {
                    format!("\"\" + {read}")
                } else {
                    read
                }
            }
            Part::IfBool { signal, yes, no } => {
                let cond = if row.is_some_and(|row| {
                    row.signals.iter().any(|item| item.read == *signal)
                }) {
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
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

fn widget_name(tag: &str) -> &'static str {
    match tag {
        "h1" => "H1",
        "h2" => "H2",
        "button" => "Button",
        "input" => "TextInput",
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

fn host_fields(class: &str) -> String {
    let mut style = Tag::Div.defaults();
    style.merge_from(&parse_classes(class));
    style.width = Some(SizeSpec::Fill);
    style.height = Some(SizeSpec::Fill);
    let mut lines = layout_pairs(&style, true);
    lines.extend(walk_pairs(&style));
    lines.push(("show_bg".into(), "true".into()));
    lines.push(("new_batch".into(), "true".into()));
    let bg = style
        .background
        .map(|color| color.hex_script())
        .unwrap_or_else(|| "#xf4f0e6".into());
    lines.push(("draw_bg.color".into(), bg));
    lines
        .into_iter()
        .map(|(key, value)| format!("                            {key}: {value}\n"))
        .collect()
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
    for when in &el.class_when {
        let mut on = prepared_style(el, tag, parent, stretch);
        on.merge_from(&parse_classes(&when.token));
        if when.token.split_whitespace().any(|token| token == "flex-nowrap") {
            on.wrap = false;
        }
        tune_style(&mut on, parent, stretch && el.class.contains("self-stretch"));
        let on_map = prop_map(&on, tag);
        let cond = class_cond(&when.signal, row);
        for (key, value) in on_map {
            if base_map.get(&key).map(String::as_str) == Some(value.as_str()) {
                continue;
            }
            let else_value = rendered
                .get(&key)
                .cloned()
                .unwrap_or_else(|| prop_fallback(&key, &base));
            rendered.insert(key, format!("if {cond} {{ {value} }} else {{ {else_value} }}"));
        }
    }
    for when in &el.style_when {
        if let Some((key, yes, no)) = style_when_prop(&when.prop, &when.yes, &when.no) {
            let cond = class_cond(&when.signal, row);
            rendered.insert(key, format!("if {cond} {{ {yes} }} else {{ {no} }}"));
        }
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
    let slot = if main_width { style.width } else { style.height };
    let explicit = if main_width {
        style.width_explicit
    } else {
        style.height_explicit
    };
    let fixed = matches!(slot, Some(SizeSpec::Px(_)) | Some(SizeSpec::Rel { .. }));
    if fixed && explicit {
        return;
    }
    if style.flex.grow.is_none() && style.flex.basis.is_none() && !matches!(slot, Some(SizeSpec::Fill))
    {
        return;
    }
    let weight = style.flex.grow.unwrap_or(if matches!(slot, Some(SizeSpec::Fill)) {
        100.0
    } else {
        1.0
    });
    let shrink = style.flex.shrink.unwrap_or(0.0);
    let basis = style.flex.basis.unwrap_or(0.0);
    let next = if weight == 100.0 && shrink == 0.0 && basis == 0.0 {
        SizeSpec::Fill
    } else {
        SizeSpec::Flex { weight, shrink, basis }
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
    map
}

fn layout_pairs(style: &Style, container: bool) -> Vec<(String, String)> {
    let mut lines = Vec::new();
    let lay_out = container || style.flow.is_some() || style.wrap || style.distribute.is_some()
        || style.main_align.is_some()
        || style.cross_align.is_some()
        || style.gap.is_some()
        || style.gap_x.is_some()
        || style.gap_y.is_some();
    if !lay_out && !style.padding.any() {
        return lines;
    }
    if !lay_out {
        if let Some(padding) = inset_script(style.padding) {
            lines.push(("padding".into(), padding));
        }
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
    if let Some(padding) = inset_script(style.padding) {
        lines.push(("padding".into(), padding));
    }
    if let Some(align) = align_script(style) {
        lines.push(("align".into(), align));
    }
    if let Some(dist) = style.distribute {
        lines.push((
            "distribute".into(),
            format!("Distribute.{}", dist.script()),
        ));
    }
    lines
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
    if let Some(bound) = style.min_height {
        lines.push(("min_height".into(), bound_script(bound)));
    }
    if let Some(bound) = style.max_height {
        lines.push(("max_height".into(), bound_script(bound)));
    }
    if let Some(margin) = inset_script(style.margin) {
        lines.push(("margin".into(), margin));
    }
    lines
}

fn paint_pairs(style: &Style, tag: Tag) -> Vec<(String, String)> {
    let mut lines = Vec::new();
    let paints = matches!(tag, Tag::Button | Tag::Input | Tag::Div);
    if paints {
        if let Some(color) = style.background {
            lines.push(("draw_bg.color".into(), color.hex_script()));
        }
        if let Some(color) = style.hover_bg {
            lines.push(("draw_bg.color_hover".into(), color.hex_script()));
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
    }
    if let Some(size) = style.font_size {
        if tag != Tag::Div {
            lines.push(("draw_text.text_style.font_size".into(), script_num(size)));
        }
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
    let y = match flow {
        FlowSpec::Column => style.main_align.map(AlignSpec::fraction).unwrap_or(0.0),
        FlowSpec::Row => 0.0,
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
            format!("Rel{{base: {} factor: {}}}", base.script(), script_num(factor))
        }
        SizeSpec::Flex { weight, shrink, basis } => format!(
            "Fill{{weight: {} shrink: {} basis: {}}}",
            script_num(weight),
            script_num(shrink),
            script_num(basis)
        ),
    }
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

fn prop_fallback(key: &str, base: &Style) -> String {
    match key {
        "draw_bg.color" | "draw_bg.color_hover" | "draw_bg.color_down" | "draw_bg.color_disabled" => {
            base.background
                .map(|color| color.hex_script())
                .unwrap_or_else(|| "#x000000".into())
        }
        "draw_text.color" | "draw_text.color_hover" | "draw_text.color_down" | "draw_text.color_disabled" => {
            base.color
                .map(|color| color.hex_script())
                .unwrap_or_else(|| "#x000000".into())
        }
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
