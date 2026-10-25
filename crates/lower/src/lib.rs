//! Turns a Leptos-looking `view!` / `signal()` component into a Makepad
//! `script_mod!` the `--hot` watcher can re-read.
//!
//! The script is written to a real `.rs` file. `quote!` cannot emit a
//! `script_mod!` whose spans round-trip through Makepad's extractor.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use proc_macro2::TokenStream;
use quinn_core::{Style, Tag};
use quinn_styles::{parse_classes, FlowSpec, SizeSpec};
use rstml::node::{Node, NodeAttribute, NodeBlock, NodeElement};
use syn::spanned::Spanned;
use syn::{Expr, Item, ItemFn, Stmt};

#[derive(Clone, Copy, Debug)]
pub struct Shell {
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Signal {
    pub read: String,
    pub write: String,
    pub init: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Lit(String),
    Sig(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeModel {
    pub name: String,
    pub tag: String,
    pub class: String,
    pub parts: Vec<Part>,
    /// Signal the click handler increments by one.
    pub click_inc: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub signals: Vec<Signal>,
    /// Signal the probe frame driver increments, when the component declares one.
    pub frame: Option<String>,
    pub root_class: String,
    pub nodes: Vec<NodeModel>,
}

/// Watch `source` and rewrite `output` when the view changes.
///
/// Makepad's watcher reloads `output` because that file holds the only
/// `script_mod!`. Run the built binary directly. `cargo run` restarts the
/// process when this write lands.
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
    if let Err(err) = std::fs::write(path, body) {
        eprintln!("quinn lower could not write {}: {err}", path.display());
    } else {
        eprintln!("quinn lowered view into {}", path.display());
    }
}

pub fn lower_file(source: &str, shell: Shell) -> Result<String, String> {
    let model = parse_model(source)?;
    Ok(emit_rust(&model, shell))
}

pub fn parse_model(source: &str) -> Result<Model, String> {
    let file = syn::parse_file(source).map_err(|err| err.to_string())?;
    let mut components = Vec::new();
    for item in file.items {
        let Item::Fn(func) = item else {
            continue;
        };
        if func
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("component"))
        {
            components.push(func);
        }
    }
    if components.len() != 1 {
        return Err(format!(
            "expected one #[component], found {}",
            components.len()
        ));
    }
    parse_component(&components[0])
}

fn parse_component(func: &ItemFn) -> Result<Model, String> {
    let mut signals = Vec::new();
    let mut frame = None;
    let mut view: Option<TokenStream> = None;
    for stmt in &func.block.stmts {
        match stmt {
            Stmt::Local(local) => {
                signals.push(parse_signal(local)?);
            }
            Stmt::Expr(expr, _) => {
                if is_call(expr, "set_frame_driver") {
                    frame = Some(parse_frame(expr, &signals)?);
                } else if is_call(expr, "set_inspector") {
                    // The probe reads `mod.state` from Rust. Nothing to emit.
                } else if let Some(tokens) = view_tokens(expr) {
                    view = Some(tokens);
                } else {
                    return Err(format!(
                        "unsupported statement in {} at {:?}",
                        func.sig.ident,
                        expr.span()
                    ));
                }
            }
            Stmt::Macro(mac) => {
                if mac
                    .mac
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "view")
                {
                    view = Some(mac.mac.tokens.clone());
                } else {
                    return Err(format!("unsupported macro in {}", func.sig.ident));
                }
            }
            Stmt::Item(_) => {
                return Err("unsupported item in component".into());
            }
        }
    }
    let Some(tokens) = view else {
        return Err(format!("{} has no view!", func.sig.ident));
    };
    let nodes = rstml::parse2(tokens).map_err(|err| err.to_string())?;
    let element = single_root(&nodes)?;
    if element.name().to_string() != "div" {
        return Err("the component root must be a <div>".into());
    }
    let root_class = class_of(element)?;
    let mut used = HashSet::new();
    let mut children = Vec::new();
    for child in element.children() {
        if is_blank(child) {
            continue;
        }
        children.push(parse_node(child, &signals, &mut used)?);
    }
    Ok(Model {
        signals,
        frame,
        root_class,
        nodes: children,
    })
}

fn parse_signal(local: &syn::Local) -> Result<Signal, String> {
    let syn::Pat::Tuple(tuple) = &local.pat else {
        return Err("signal bindings must be `let (read, set) = signal(...)`".into());
    };
    if tuple.elems.len() != 2 {
        return Err("signal bindings must be a pair".into());
    }
    let read = pat_ident(&tuple.elems[0])?;
    let write = pat_ident(&tuple.elems[1])?;
    let Some(init_expr) = &local.init else {
        return Err(format!("signal {read} has no initializer"));
    };
    let Expr::Call(call) = &*init_expr.expr else {
        return Err(format!("signal {read} must be signal(...)"));
    };
    if !is_path(&call.func, "signal") {
        return Err(format!("signal {read} must be signal(...)"));
    }
    let init = match call.args.first() {
        Some(Expr::Lit(lit)) => match &lit.lit {
            syn::Lit::Int(value) => value.base10_parse::<i64>().map_err(|err| err.to_string())?,
            _ => return Err(format!("signal {read} must start as an integer")),
        },
        _ => return Err(format!("signal {read} must start as an integer")),
    };
    Ok(Signal { read, write, init })
}

fn parse_frame(expr: &Expr, signals: &[Signal]) -> Result<String, String> {
    let Expr::Call(call) = expr else {
        return Err("set_frame_driver must be a call".into());
    };
    let Some(arg) = call.args.first() else {
        return Err("set_frame_driver needs a closure".into());
    };
    let Expr::Closure(closure) = arg else {
        return Err("set_frame_driver needs a closure".into());
    };
    let body = closure_stmts(closure)?;
    if body.len() != 1 {
        return Err("the frame driver must be a single signal update".into());
    }
    Ok(inc_signal(&body[0], signals)?.to_string())
}

fn parse_node(
    node: &Node,
    signals: &[Signal],
    used: &mut HashSet<String>,
) -> Result<NodeModel, String> {
    let Node::Element(element) = node else {
        return Err("only elements are supported in this lowerer".into());
    };
    let tag = element.name().to_string();
    if matches!(tag.as_str(), "Show" | "show" | "For" | "for") {
        return Err(format!(
            "<{tag}> is not lowered yet; conditional and keyed examples still run on the Rust graph"
        ));
    }
    let class = class_of(element).unwrap_or_default();
    let mut parts = Vec::new();
    let mut click_inc = None;
    for attr in element.attributes() {
        let NodeAttribute::Attribute(attr) = attr else {
            return Err("computed attribute keys are not supported".into());
        };
        let key = attr.key.to_string().replace(' ', "");
        if key == "class" {
            continue;
        }
        if key == "on:click" {
            let Some(value) = attr.value() else {
                return Err("on:click needs a closure".into());
            };
            click_inc = Some(parse_click(value, signals)?);
            continue;
        }
        return Err(format!("unsupported attribute `{key}` on <{tag}>"));
    }
    for child in element.children() {
        match child {
            Node::Text(text) => {
                let value = text.value_string();
                if !value.is_empty() {
                    parts.push(Part::Lit(value));
                }
            }
            Node::RawText(raw) => {
                let value = raw.to_string_best();
                if !value.trim().is_empty() {
                    parts.push(Part::Lit(value));
                }
            }
            Node::Block(block) => parts.push(parse_interp(block, signals)?),
            Node::Comment(_) | Node::Doctype(_) => {}
            other => {
                return Err(format!(
                    "nested child in <{tag}> is not lowered yet: {other:?}"
                ));
            }
        }
    }
    let name = unique_name(&tag, &parts, used);
    Ok(NodeModel {
        name,
        tag,
        class,
        parts,
        click_inc,
    })
}

fn parse_click(expr: &Expr, signals: &[Signal]) -> Result<String, String> {
    let Expr::Closure(closure) = expr else {
        return Err("on:click must be a closure".into());
    };
    let body = closure_stmts(closure)?;
    if body.len() != 1 {
        return Err("on:click must be a single signal update".into());
    }
    Ok(inc_signal(&body[0], signals)?.to_string())
}

fn parse_interp(block: &NodeBlock, signals: &[Signal]) -> Result<Part, String> {
    let NodeBlock::ValidBlock(block) = block else {
        return Err("invalid interpolation block".into());
    };
    if block.stmts.len() != 1 {
        return Err("interpolation must be one expression".into());
    }
    let Stmt::Expr(expr, None) = &block.stmts[0] else {
        return Err("interpolation must be one expression".into());
    };
    let Expr::Closure(closure) = expr else {
        return Err("interpolation must be {move || signal.get()}".into());
    };
    let Expr::MethodCall(call) = &*closure.body else {
        return Err("interpolation must call .get()".into());
    };
    if call.method != "get" {
        return Err("interpolation must call .get()".into());
    }
    let Expr::Path(path) = &*call.receiver else {
        return Err("interpolation must read a signal".into());
    };
    let name = path
        .path
        .get_ident()
        .ok_or("interpolation must read a local signal")?
        .to_string();
    if !signals.iter().any(|signal| signal.read == name) {
        return Err(format!("unknown signal {name}"));
    }
    Ok(Part::Sig(name))
}

fn inc_signal<'a>(expr: &Expr, signals: &'a [Signal]) -> Result<&'a str, String> {
    let Expr::MethodCall(call) = expr else {
        return Err("expected set_signal.update(|n| *n += 1)".into());
    };
    if call.method != "update" {
        return Err("only .update(|n| *n += 1) is lowered".into());
    }
    let Expr::Path(path) = &*call.receiver else {
        return Err("update receiver must be the signal setter".into());
    };
    let write = path
        .path
        .get_ident()
        .ok_or("update receiver must be a local")?
        .to_string();
    let signal = signals
        .iter()
        .find(|signal| signal.write == write)
        .ok_or_else(|| format!("unknown setter {write}"))?;
    let Some(Expr::Closure(closure)) = call.args.first() else {
        return Err("update needs a closure".into());
    };
    let Expr::Binary(binary) = &*closure.body else {
        return Err("update must be `*n += 1`".into());
    };
    if !matches!(binary.op, syn::BinOp::AddAssign(_)) {
        return Err("update must be `*n += 1`".into());
    }
    let Expr::Lit(lit) = &*binary.right else {
        return Err("update must add a literal".into());
    };
    let syn::Lit::Int(value) = &lit.lit else {
        return Err("update must add an integer".into());
    };
    if value.base10_parse::<i64>().map_err(|err| err.to_string())? != 1 {
        return Err("update must add 1".into());
    }
    Ok(signal.read.as_str())
}

fn closure_stmts(closure: &syn::ExprClosure) -> Result<Vec<Expr>, String> {
    match &*closure.body {
        Expr::Block(block) => {
            let mut out = Vec::new();
            for stmt in &block.block.stmts {
                match stmt {
                    Stmt::Expr(expr, _) => out.push(expr.clone()),
                    _ => return Err("closure body must be expressions".into()),
                }
            }
            Ok(out)
        }
        other => Ok(vec![other.clone()]),
    }
}

fn single_root(nodes: &[Node]) -> Result<&NodeElement<rstml::Infallible>, String> {
    let mut found = None;
    for node in nodes {
        if is_blank(node) {
            continue;
        }
        let Node::Element(element) = node else {
            return Err("the view root must be an element".into());
        };
        if found.is_some() {
            return Err("the view must have one root element".into());
        }
        found = Some(element);
    }
    found.ok_or_else(|| "the view is empty".into())
}

fn class_of(element: &NodeElement<rstml::Infallible>) -> Result<String, String> {
    for attr in element.attributes() {
        let NodeAttribute::Attribute(attr) = attr else {
            continue;
        };
        if attr.key.to_string().replace(' ', "") == "class" {
            return attr
                .value_literal_string()
                .ok_or_else(|| "class must be a string literal".into());
        }
    }
    Ok(String::new())
}

fn is_blank(node: &Node) -> bool {
    match node {
        Node::RawText(raw) => raw.to_string_best().trim().is_empty(),
        Node::Text(text) => text.value_string().trim().is_empty(),
        Node::Comment(_) => true,
        _ => false,
    }
}

fn is_call(expr: &Expr, name: &str) -> bool {
    let Expr::Call(call) = expr else {
        return false;
    };
    is_path(&call.func, name)
}

fn is_path(expr: &Expr, name: &str) -> bool {
    let Expr::Path(path) = expr else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == name)
}

fn view_tokens(expr: &Expr) -> Option<TokenStream> {
    let Expr::Macro(mac) = expr else {
        return None;
    };
    mac.mac
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "view")
        .then(|| mac.mac.tokens.clone())
}

fn pat_ident(pat: &syn::Pat) -> Result<String, String> {
    let syn::Pat::Ident(ident) = pat else {
        return Err("expected an identifier".into());
    };
    Ok(ident.ident.to_string())
}

fn unique_name(tag: &str, parts: &[Part], used: &mut HashSet<String>) -> String {
    let base = if tag == "button" {
        if let [Part::Lit(text)] = parts {
            let slug = slug(text);
            if slug.is_empty() {
                tag.to_string()
            } else {
                slug
            }
        } else {
            tag.to_string()
        }
    } else {
        tag.to_string()
    };
    let mut name = base;
    let mut n = 2;
    while !used.insert(name.clone()) {
        name = format!("{tag}_{n}");
        n += 1;
    }
    name
}

fn slug(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn emit_rust(model: &Model, shell: Shell) -> String {
    let mut body = String::new();
    body.push_str("        use mod.prelude.widgets.*\n");
    body.push_str("        let state = mod.state\n");
    if let Some(signal) = &model.frame {
        body.push_str("        fn quinn_frame() {\n");
        body.push_str(&format!("            state.{signal} += 1\n"));
        body.push_str("            ui.host.render()\n");
        body.push_str("        }\n");
    }
    let host = host_fields(&model.root_class);
    let mut children = String::new();
    for node in &model.nodes {
        children.push_str(&emit_node(node));
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
        r#"// Generated by quinn-lower from src/main.rs. Do not edit.
// This header is stable so the block token stays on the same line.
use quinn::makepad_widgets::*;

script_mod! {{
{body}}}
"#
    )
}

fn host_fields(class: &str) -> String {
    let mut style = Tag::Div.defaults();
    style.merge_from(&parse_classes(class));
    let mut lines = vec![
        "width: Fill".to_string(),
        "height: Fill".to_string(),
        flow_line(style.flow.unwrap_or(FlowSpec::Column)),
    ];
    if let Some(gap) = style.gap {
        lines.push(format!("spacing: {}", script_num(gap)));
    }
    if let Some(padding) = uniform_padding(&style) {
        lines.push(format!("padding: {}", script_num(padding)));
    }
    lines.push("show_bg: true".into());
    lines.push("new_batch: true".into());
    let bg = style
        .background
        .map(|color| color.hex_script())
        .unwrap_or_else(|| "#xf4f0e6".into());
    lines.push(format!("draw_bg.color: {bg}"));
    lines
        .into_iter()
        .map(|line| format!("                            {line}\n"))
        .collect()
}

fn emit_node(node: &NodeModel) -> String {
    let widget = match node.tag.as_str() {
        "h1" => "H1",
        "h2" => "H2",
        "button" => "Button",
        "input" => "TextInput",
        "div" => "RoundedView",
        _ => "Label",
    };
    let tag = match node.tag.as_str() {
        "h1" => Tag::H1,
        "h2" => Tag::H2,
        "button" => Tag::Button,
        "input" => Tag::Input,
        "div" => Tag::Div,
        "p" => Tag::P,
        "label" => Tag::Label,
        _ => Tag::Span,
    };
    let mut style = tag.defaults();
    if !node.class.is_empty() {
        style.merge_from(&parse_classes(&node.class));
    }
    let mut lines = Vec::new();
    if !node.parts.is_empty() {
        lines.push(format!("text: {}", text_expr(&node.parts)));
    }
    push_control(&mut lines, tag, &style);
    if let Some(signal) = &node.click_inc {
        lines.push(format!(
            "on_click: || {{\n                                    state.{signal} += 1\n                                    ui.host.render()\n                                }}"
        ));
    }
    let inner = lines
        .into_iter()
        .map(|line| format!("                                {line}\n"))
        .collect::<String>();
    format!(
        "                                {name} := {widget} {{\n{inner}                                }}\n",
        name = node.name,
    )
}

fn push_control(lines: &mut Vec<String>, tag: Tag, style: &Style) {
    match style.width {
        Some(SizeSpec::Fill) => lines.push("width: Fill".into()),
        Some(SizeSpec::Fit) => lines.push("width: Fit".into()),
        Some(SizeSpec::Px(px)) => lines.push(format!("width: {}", script_num(px))),
        None => {}
    }
    match style.height {
        Some(SizeSpec::Fill) => lines.push("height: Fill".into()),
        Some(SizeSpec::Fit) => lines.push("height: Fit".into()),
        Some(SizeSpec::Px(px)) => lines.push(format!("height: {}", script_num(px))),
        None => {}
    }
    if let Some(padding) = uniform_padding(style) {
        if !matches!(
            tag,
            Tag::Div | Tag::H1 | Tag::H2 | Tag::Span | Tag::P | Tag::Label
        ) {
            lines.push(format!("padding: {}", script_num(padding)));
        }
    }
    if matches!(tag, Tag::Button | Tag::Input | Tag::Div) {
        if let Some(color) = style.background {
            let hex = color.hex_script();
            lines.push(format!("draw_bg.color: {hex}"));
            if matches!(tag, Tag::Button) {
                lines.push(format!("draw_bg.color_hover: {hex}"));
                lines.push(format!("draw_bg.color_down: {hex}"));
                lines.push(format!("draw_bg.color_focus: {hex}"));
            }
        }
        if let Some(radius) = style.radius {
            lines.push(format!("draw_bg.border_radius: {}", script_num(radius)));
        }
    }
    if let Some(color) = style.color {
        let hex = color.hex_script();
        lines.push(format!("draw_text.color: {hex}"));
        if matches!(tag, Tag::Button) {
            lines.push(format!("draw_text.color_hover: {hex}"));
            lines.push(format!("draw_text.color_down: {hex}"));
            lines.push(format!("draw_text.color_focus: {hex}"));
        }
    }
    if let Some(size) = style.font_size {
        lines.push(format!("draw_text.text_style.font_size: {size}"));
    }
}

fn uniform_padding(style: &Style) -> Option<f64> {
    let (left, top, right, bottom) = style.padding.resolve();
    if style.padding.left.is_none() {
        return None;
    }
    if left == top && top == right && right == bottom {
        Some(left)
    } else {
        None
    }
}

fn flow_line(flow: FlowSpec) -> String {
    match flow {
        FlowSpec::Column => "flow: Down".into(),
        FlowSpec::Row => "flow: Right".into(),
    }
}

fn text_expr(parts: &[Part]) -> String {
    parts
        .iter()
        .map(|part| match part {
            Part::Lit(text) => format!("\"{}\"", escape_script(text)),
            Part::Sig(name) => format!("state.{name}"),
        })
        .collect::<Vec<_>>()
        .join(" + ")
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

/// Headless stand-in for `on_render` + named `:=` identity.
///
/// A click updates `mod.state` and renders again. A node whose name is still
/// in the template keeps its id. A name that disappeared is gone, and a click
/// on it does nothing.
#[derive(Clone, Debug)]
pub struct Sim {
    pub state: HashMap<String, i64>,
    ids: HashMap<String, u64>,
    texts: HashMap<String, String>,
    next: u64,
    model: Model,
}

impl Sim {
    pub fn new(model: Model) -> Self {
        let state = model
            .signals
            .iter()
            .map(|signal| (signal.read.clone(), signal.init))
            .collect();
        let mut sim = Self {
            state,
            ids: HashMap::new(),
            texts: HashMap::new(),
            next: 1,
            model,
        };
        sim.render();
        sim
    }

    pub fn render(&mut self) {
        let mut live = Vec::new();
        for node in &self.model.nodes {
            let text = node
                .parts
                .iter()
                .map(|part| match part {
                    Part::Lit(text) => text.clone(),
                    Part::Sig(name) => self.state.get(name).copied().unwrap_or(0).to_string(),
                })
                .collect::<String>();
            let id = if let Some(id) = self.ids.get(&node.name).copied() {
                id
            } else {
                let id = self.next;
                self.next += 1;
                self.ids.insert(node.name.clone(), id);
                id
            };
            live.push(node.name.clone());
            self.texts.insert(node.name.clone(), text);
            let _ = id;
        }
        self.ids
            .retain(|name, _| live.iter().any(|live| live == name));
        self.texts
            .retain(|name, _| live.iter().any(|live| live == name));
    }

    pub fn id(&self, name: &str) -> Option<u64> {
        self.ids.get(name).copied()
    }

    pub fn text(&self, name: &str) -> String {
        self.texts.get(name).cloned().unwrap_or_default()
    }

    pub fn click(&mut self, name: &str) -> bool {
        let Some(node) = self.model.nodes.iter().find(|node| node.name == name) else {
            return false;
        };
        if self.ids.get(name).is_none() {
            return false;
        }
        let Some(signal) = node.click_inc.clone() else {
            return false;
        };
        if let Some(value) = self.state.get_mut(&signal) {
            *value += 1;
        }
        self.render();
        true
    }

    pub fn drop_named(&mut self, name: &str) {
        self.model.nodes.retain(|node| node.name != name);
        self.render();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn shell() -> Shell {
        Shell {
            title: "Counter",
            width: 480,
            height: 360,
        }
    }

    #[test]
    fn counter_script_uses_state_render_and_on_click() {
        let file = lower_file(COUNTER, shell()).unwrap();
        assert_eq!(file.matches("script_mod!").count(), 1);
        assert!(!file.contains("script_eval"));
        assert!(file.contains("let state = mod.state"));
        assert!(file.contains("spacing: 16."));
        assert!(file.contains("padding: 16."));
        assert!(file.contains("flow: Down"));
        assert!(file.contains("text: \"Counter: \" + state.count"));
        assert!(file.contains("on_click: || {"));
        assert!(file.contains("state.count += 1"));
        assert!(file.contains("ui.host.render()"));
        assert!(file.contains("fn quinn_frame()"));
        assert!(file.contains("increment := Button"));
        assert!(file.contains("h1 := H1"));
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
}
