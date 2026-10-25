//! Read `#[component]` / `view!` source. The functions are not executed.

use std::collections::{HashMap, HashSet};

use proc_macro2::TokenStream;
use rstml::node::{Node, NodeAttribute, NodeBlock, NodeElement};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, Item, ItemFn, Stmt};

use crate::model::*;

#[derive(Clone)]
struct PropSpec {
    name: String,
    optional: bool,
    default: Option<PropVal>,
    into: bool,
    kind: PropKind,
}

#[derive(Clone, Debug)]
enum PropKind {
    Text,
    Int,
    Bool,
    Children,
    /// `ChildrenFn`. The slot function takes `arg`.
    ChildrenFn,
    /// A row struct (`item: Item`). Fields arrive as `<For>` params.
    Item,
}

#[derive(Clone, Debug)]
enum PropVal {
    Lit(String),
    Int(i64),
    Bool(bool),
    Sig(String),
    Empty,
}

#[derive(Clone)]
struct Provide {
    key: String,
    expr: String,
}

struct Comp {
    params: Vec<PropSpec>,
    signals: Vec<Signal>,
    aliases: Vec<(String, String, String)>,
    contexts: Vec<(String, String)>,
    provides: Vec<Provide>,
    jobs: Vec<Job>,
    view: TokenStream,
    frame: Option<String>,
    retain: bool,
}

struct Build {
    comps: HashMap<String, Comp>,
    used: HashSet<String>,
    fns: u32,
    slots: u32,
    /// Signals of components lowered while inside a `<For>` row.
    pending_row_signals: Vec<Signal>,
    prop_env: HashMap<String, PropVal>,
    ctx_env: HashMap<String, String>,
    provides_here: Vec<(String, String)>,
    /// Call-site children of a `ChildrenFn` may read `{arg}`.
    in_children_fn: bool,
    /// Resource and action names in scope for `{note}` and `note.get()`.
    job_names: Vec<String>,
    /// `<For>` loop variable, so `{city.name}` becomes a row field.
    row_var: Option<String>,
}

pub fn parse_model(source: &str) -> Result<Model, String> {
    let file = syn::parse_file(source).map_err(|err| err.to_string())?;
    let mut comps = HashMap::new();
    let mut order = Vec::new();
    for item in file.items {
        let Item::Fn(func) = item else {
            continue;
        };
        if !func
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("component"))
        {
            continue;
        }
        let name = func.sig.ident.to_string();
        order.push(name.clone());
        comps.insert(name, parse_comp(&func)?);
    }
    let Some(root_name) = order.last() else {
        return Err("expected a #[component]".into());
    };
    let mut build = Build {
        comps,
        used: HashSet::new(),
        fns: 0,
        slots: 0,
        pending_row_signals: Vec::new(),
        prop_env: HashMap::new(),
        ctx_env: HashMap::new(),
        provides_here: Vec::new(),
        in_children_fn: false,
        job_names: Vec::new(),
        row_var: None,
    };
    let (signals, frame, retain, has_ctx, root_provides, root_ctx, view) = {
        let root = build
            .comps
            .get(root_name)
            .ok_or_else(|| format!("missing {root_name}"))?;
        let has_ctx = build
            .comps
            .values()
            .any(|comp| !comp.provides.is_empty() || !comp.contexts.is_empty());
        (
            root.signals.clone(),
            root.frame.clone(),
            root.retain,
            has_ctx,
            root.provides
                .iter()
                .map(|provide| (provide.key.clone(), provide.expr.clone()))
                .collect::<Vec<_>>(),
            root.contexts.clone(),
            root.view.clone(),
        )
    };
    let ctx_init = root_provides;
    let jobs = {
        let root = build
            .comps
            .get(root_name)
            .ok_or_else(|| format!("missing {root_name}"))?;
        root.jobs.clone()
    };
    build.job_names = jobs.iter().map(|job| job.name.clone()).collect();
    build.ctx_env = root_ctx.into_iter().collect();
    let nodes = rstml::parse2(view).map_err(|err| err.to_string())?;
    let element = single_root(&nodes)?;
    if element.name().to_string() != "div" {
        return Err("the component root must be a <div>".into());
    }
    let root_class = class_of(element)?;
    let mut children = Vec::new();
    let mut keyed = false;
    for child in element.children() {
        if is_blank(child) {
            continue;
        }
        let node = build.lower_node(child, &signals, false)?;
        if contains_for(&node) {
            keyed = true;
        }
        children.push(node);
    }
    let mut signals = signals;
    collect_resets(&children, &mut signals);
    if contains_routes(&children) && signals.iter().all(|signal| signal.read != "route") {
        let path = first_route(&children).unwrap_or_else(|| "/".into());
        signals.push(Signal {
            read: "route".into(),
            write: String::new(),
            init: Init::Str(path),
        });
    }
    Ok(Model {
        signals,
        frame,
        root_class,
        nodes: children,
        keyed,
        retain,
        has_ctx,
        ctx_init,
        jobs,
    })
}

fn contains_routes(nodes: &[ViewNode]) -> bool {
    nodes.iter().any(node_has_routes)
}

fn node_has_routes(node: &ViewNode) -> bool {
    match node {
        ViewNode::Routes { .. } => true,
        ViewNode::El(el) => el.children.iter().any(node_has_routes),
        ViewNode::Show { child, .. } => node_has_routes(child),
        ViewNode::Slot { nodes, .. } => nodes.iter().any(node_has_routes),
        ViewNode::For { .. } => false,
        ViewNode::Suspense { fallback, child, .. } => {
            fallback.iter().any(node_has_routes) || node_has_routes(child)
        }
        ViewNode::Boundary { fallback, child, .. } => {
            fallback.iter().any(node_has_routes) || node_has_routes(child)
        }
        ViewNode::Portal { nodes } => nodes.iter().any(node_has_routes),
    }
}

fn first_route(nodes: &[ViewNode]) -> Option<String> {
    for node in nodes {
        if let Some(path) = first_route_node(node) {
            return Some(path);
        }
    }
    None
}

fn first_route_node(node: &ViewNode) -> Option<String> {
    match node {
        ViewNode::Routes { routes } => routes.first().map(|(path, _)| path.clone()),
        ViewNode::El(el) => el.children.iter().find_map(first_route_node),
        ViewNode::Show { child, .. } => first_route_node(child),
        ViewNode::Slot { nodes, .. } => nodes.iter().find_map(first_route_node),
        ViewNode::For { .. } => None,
        ViewNode::Suspense { fallback, child, .. } => fallback
            .iter()
            .find_map(first_route_node)
            .or_else(|| first_route_node(child)),
        ViewNode::Boundary { fallback, child, .. } => fallback
            .iter()
            .find_map(first_route_node)
            .or_else(|| first_route_node(child)),
        ViewNode::Portal { nodes } => nodes.iter().find_map(first_route_node),
    }
}

fn contains_for(node: &ViewNode) -> bool {
    match node {
        ViewNode::For { .. } => true,
        ViewNode::El(el) => el.children.iter().any(contains_for),
        ViewNode::Show { child, .. } => contains_for(child),
        ViewNode::Slot { nodes, .. } => nodes.iter().any(contains_for),
        ViewNode::Routes { routes } => routes
            .iter()
            .any(|(_, nodes)| nodes.iter().any(contains_for)),
        ViewNode::Suspense { fallback, child, .. } => {
            fallback.iter().any(contains_for) || contains_for(child)
        }
        ViewNode::Boundary { fallback, child, .. } => {
            fallback.iter().any(contains_for) || contains_for(child)
        }
        ViewNode::Portal { nodes } => nodes.iter().any(contains_for),
    }
}

fn collect_resets(nodes: &[ViewNode], signals: &mut Vec<Signal>) {
    for node in nodes {
        match node {
            ViewNode::El(el) => collect_resets(&el.children, signals),
            ViewNode::Show { reset, child, .. } => {
                for name in reset {
                    if signals.iter().any(|signal| signal.read == *name) {
                        continue;
                    }
                    signals.push(Signal {
                        read: name.clone(),
                        write: String::new(),
                        init: Init::Int(0),
                    });
                }
                collect_resets(std::slice::from_ref(child), signals);
            }
            ViewNode::For { .. } => {}
            ViewNode::Slot { nodes, .. } => collect_resets(nodes, signals),
            ViewNode::Routes { routes } => {
                for (_, nodes) in routes {
                    collect_resets(nodes, signals);
                }
            }
            ViewNode::Suspense { fallback, child, .. } | ViewNode::Boundary { fallback, child, .. } => {
                collect_resets(fallback, signals);
                collect_resets(std::slice::from_ref(child), signals);
            }
            ViewNode::Portal { nodes } => collect_resets(nodes, signals),
        }
    }
}

fn parse_comp(func: &ItemFn) -> Result<Comp, String> {
    let mut params = Vec::new();
    for arg in func.sig.inputs.iter() {
        let syn::FnArg::Typed(pat) = arg else {
            continue;
        };
        params.push(parse_prop(pat)?);
    }
    let mut signals = Vec::new();
    let mut aliases = Vec::new();
    let mut contexts = Vec::new();
    let mut provides = Vec::new();
    let mut jobs = Vec::new();
    let mut frame = None;
    let mut retain = false;
    let mut view = None;
    for stmt in &func.block.stmts {
        match stmt {
            Stmt::Local(local) => {
                if let Some(signal) = try_signal(local)? {
                    signals.push(signal);
                } else if let Some(job) = try_job(local)? {
                    jobs.push(job);
                } else if let Some(ctx) = try_use_context(local)? {
                    contexts.push(ctx);
                } else if let Some(alias) = try_alias(local)? {
                    aliases.push(alias);
                } else {
                    return Err(format!("unsupported binding in {}", func.sig.ident));
                }
            }
            Stmt::Expr(expr, _) => {
                if let Some(provide) = try_provide(expr)? {
                    provides.push(provide);
                } else if is_call(expr, "set_frame_driver") {
                    frame = Some(parse_frame(expr, &signals)?);
                } else if is_call(expr, "set_inspector") || is_call(expr, "retain_probe") {
                    if is_call(expr, "retain_probe") {
                        retain = true;
                    }
                } else if let Some(tokens) = view_tokens(expr) {
                    view = Some(tokens);
                } else {
                    return Err(format!(
                        "unsupported statement in {} ({})",
                        func.sig.ident,
                        snippet(expr)
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
            Stmt::Item(_) => return Err("unsupported item in component".into()),
        }
    }
    let Some(view) = view else {
        return Err(format!("{} has no view!", func.sig.ident));
    };
    Ok(Comp {
        params,
        signals,
        aliases,
        contexts,
        provides,
        jobs,
        view,
        frame,
        retain,
    })
}

fn parse_prop(pat: &syn::PatType) -> Result<PropSpec, String> {
    let name = pat_ident(&pat.pat)?;
    let mut optional = false;
    let mut into = false;
    let mut default = None;
    for attr in &pat.attrs {
        if !attr.path().is_ident("prop") {
            continue;
        }
        let parser = Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
        let nested = attr
            .parse_args_with(parser)
            .map_err(|err| err.to_string())?;
        for meta in nested {
            match meta {
                syn::Meta::Path(path) if path.is_ident("optional") => optional = true,
                syn::Meta::Path(path) if path.is_ident("into") => into = true,
                syn::Meta::NameValue(nv) if nv.path.is_ident("default") => {
                    default = Some(prop_default(&nv.value)?);
                }
                other => {
                    return Err(format!(
                        "unsupported #[prop] on {name}: {}",
                        snippet_meta(&other)
                    ));
                }
            }
        }
    }
    let kind = prop_kind(&pat.ty);
    if matches!(kind, PropKind::Children | PropKind::ChildrenFn) {
        optional = true;
    }
    Ok(PropSpec {
        name,
        optional,
        default,
        into,
        kind,
    })
}

fn prop_kind(ty: &syn::Type) -> PropKind {
    let name = type_name(ty);
    match name.as_str() {
        "Children" => PropKind::Children,
        "ChildrenFn" => PropKind::ChildrenFn,
        "String" | "str" => PropKind::Text,
        "i32" | "i64" | "u32" | "u64" | "usize" => PropKind::Int,
        "bool" => PropKind::Bool,
        "Item" => PropKind::Item,
        other if other.starts_with("Option") => PropKind::Text,
        _ => PropKind::Item,
    }
}

fn type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default(),
        syn::Type::Reference(refer) => type_name(&refer.elem),
        _ => String::new(),
    }
}

fn prop_default(expr: &Expr) -> Result<PropVal, String> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Str(value) => Ok(PropVal::Lit(value.value())),
            syn::Lit::Int(value) => Ok(PropVal::Int(
                value.base10_parse::<i64>().map_err(|err| err.to_string())?,
            )),
            syn::Lit::Bool(value) => Ok(PropVal::Bool(value.value)),
            _ => Err("#[prop(default)] must be a string, integer, or bool".into()),
        },
        other => Err(format!(
            "#[prop(default)] must be a literal, got {}",
            snippet(other)
        )),
    }
}

fn snippet_meta(meta: &syn::Meta) -> String {
    match meta {
        syn::Meta::Path(path) => path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_else(|| "prop".into()),
        syn::Meta::List(list) => list
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_else(|| "prop".into()),
        syn::Meta::NameValue(nv) => nv
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_else(|| "prop".into()),
    }
}

fn try_provide(expr: &Expr) -> Result<Option<Provide>, String> {
    let Expr::Call(call) = expr else {
        return Ok(None);
    };
    if !is_call(expr, "provide_context") {
        return Ok(None);
    }
    if call.args.is_empty() {
        return Err("provide_context needs a key or a struct".into());
    }
    if call.args.len() == 1 {
        let Expr::Struct(item) = &call.args[0] else {
            return Err(
                "provide_context takes (\"key\", value) or provide_context(Type { .. })".into(),
            );
        };
        let key = item
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .ok_or("provide_context struct needs a name")?;
        let mut fields = Vec::new();
        for field in &item.fields {
            let syn::Member::Named(name) = &field.member else {
                continue;
            };
            fields.push(format!("{}: {}", name, ctx_expr(&field.expr)?));
        }
        return Ok(Some(Provide {
            key,
            expr: format!("{{{}}}", fields.join(", ")),
        }));
    }
    let key = lit_string(&call.args[0]).ok_or("provide_context key must be a string")?;
    let expr = ctx_expr(&call.args[1])?;
    Ok(Some(Provide { key, expr }))
}

fn ctx_expr(expr: &Expr) -> Result<String, String> {
    if let Some(text) = lit_string(expr) {
        return Ok(format!(
            "\"{}\"",
            text.replace('\\', "\\\\").replace('"', "\\\"")
        ));
    }
    if let Expr::Lit(lit) = expr {
        if let syn::Lit::Int(value) = &lit.lit {
            return Ok(value.base10_digits().to_string());
        }
        if let syn::Lit::Bool(value) = &lit.lit {
            return Ok(if value.value { "true" } else { "false" }.into());
        }
    }
    if let Expr::Path(path) = expr {
        if let Some(ident) = path.path.get_ident() {
            return Ok(format!("state.{ident}"));
        }
    }
    if let Expr::MethodCall(call) = expr {
        if call.method == "get" {
            if let Some(name) = path_ident_opt(&call.receiver) {
                return Ok(format!("state.{name}"));
            }
        }
    }
    Err("provide_context value must be a literal or a signal".into())
}

fn path_ident_opt(expr: &Expr) -> Option<String> {
    let Expr::Path(path) = expr else {
        return None;
    };
    path.path.get_ident().map(|ident| ident.to_string())
}

fn try_use_context(local: &syn::Local) -> Result<Option<(String, String)>, String> {
    let syn::Pat::Ident(ident) = &local.pat else {
        return Ok(None);
    };
    let Some(init) = &local.init else {
        return Ok(None);
    };
    let Expr::Call(call) = &*init.expr else {
        return Ok(None);
    };
    if !is_path(&call.func, "use_context") {
        return Ok(None);
    }
    let key = if let Some(arg) = call.args.first() {
        lit_string(arg).ok_or("use_context key must be a string")?
    } else if let syn::Expr::Path(_) = &*call.func {
        turbofish_key(&call.func).ok_or("use_context needs a key or a type")?
    } else {
        return Err("use_context needs a key or a type".into());
    };
    Ok(Some((ident.ident.to_string(), key)))
}

fn turbofish_key(func: &Expr) -> Option<String> {
    let Expr::Path(path) = func else {
        return None;
    };
    let segment = path.path.segments.last()?;
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let syn::GenericArgument::Type(ty) = args.args.first()? else {
        return None;
    };
    let name = type_name(ty);
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

impl Build {
    fn lower_node(
        &mut self,
        node: &Node,
        signals: &[Signal],
        in_row: bool,
    ) -> Result<ViewNode, String> {
        match node {
            Node::Fragment(fragment) => {
                let mut nodes = Vec::new();
                for child in fragment.children() {
                    if is_blank(child) {
                        continue;
                    }
                    nodes.push(self.lower_node(child, signals, in_row)?);
                }
                return Ok(ViewNode::Slot {
                    fn_name: String::new(),
                    nodes,
                    arg: None,
                });
            }
            Node::Element(element) => {
                let tag = element.name().to_string();
                if tag == "Show" || tag == "show" {
                    return self.lower_show(element, signals, in_row);
                }
                if tag == "For" || tag == "for" {
                    return self.lower_for(element, signals);
                }
                if tag == "Router" || tag == "router" {
                    return self.lower_router(element, signals, in_row);
                }
                if tag == "Routes" || tag == "routes" {
                    return self.lower_routes(element, signals, in_row);
                }
                if tag == "A" || tag == "a" {
                    return self.lower_anchor(element, signals, in_row);
                }
                if tag == "Suspense" || tag == "suspense" {
                    return self.lower_suspense(element, signals, in_row, false);
                }
                if tag == "Transition" || tag == "transition" {
                    return self.lower_suspense(element, signals, in_row, true);
                }
                if tag == "ErrorBoundary" || tag == "error_boundary" {
                    return self.lower_boundary(element, signals, in_row);
                }
                if tag == "Portal" || tag == "portal" {
                    return self.lower_portal(element, signals, in_row);
                }
                if self.comps.contains_key(&tag) {
                    return self.lower_component(&tag, element, signals, in_row);
                }
                self.lower_el(element, signals, in_row)
            }
            _ => Err("only elements are supported".into()),
        }
    }

    fn lower_router(
        &mut self,
        element: &NodeElement<rstml::Infallible>,
        signals: &[Signal],
        in_row: bool,
    ) -> Result<ViewNode, String> {
        let mut nodes = Vec::new();
        for child in element.children() {
            if is_blank(child) {
                continue;
            }
            nodes.push(self.lower_node(child, signals, in_row)?);
        }
        if nodes.len() == 1 {
            return Ok(nodes.pop().unwrap());
        }
        Ok(ViewNode::Slot {
            fn_name: String::new(),
            nodes,
            arg: None,
        })
    }

    fn lower_routes(
        &mut self,
        element: &NodeElement<rstml::Infallible>,
        signals: &[Signal],
        in_row: bool,
    ) -> Result<ViewNode, String> {
        let mut routes = Vec::new();
        for child in element.children() {
            let Node::Element(route) = child else {
                if is_blank(child) {
                    continue;
                }
                return Err("<Routes> may only contain <Route>".into());
            };
            if route.name().to_string() != "Route" && route.name().to_string() != "route" {
                return Err("<Routes> may only contain <Route>".into());
            }
            let mut path = None;
            for attr in route.attributes() {
                let NodeAttribute::Attribute(attr) = attr else {
                    continue;
                };
                if attr_key(attr) == "path" {
                    let Some(value) = attr.value() else {
                        return Err("<Route path> needs a value".into());
                    };
                    path = Some(lit_string(value).ok_or("<Route path> must be a string")?);
                }
            }
            let Some(path) = path else {
                return Err("<Route> needs path".into());
            };
            let mut body = Vec::new();
            for child in route.children() {
                if is_blank(child) {
                    continue;
                }
                body.push(self.lower_node(child, signals, in_row)?);
            }
            routes.push((path, body));
        }
        if routes.is_empty() {
            return Err("<Routes> needs a <Route>".into());
        }
        Ok(ViewNode::Routes { routes })
    }

    fn lower_anchor(
        &mut self,
        element: &NodeElement<rstml::Infallible>,
        signals: &[Signal],
        in_row: bool,
    ) -> Result<ViewNode, String> {
        let mut href = None;
        for attr in element.attributes() {
            let NodeAttribute::Attribute(attr) = attr else {
                continue;
            };
            if attr_key(attr) == "href" {
                let Some(value) = attr.value() else {
                    return Err("<A href> needs a value".into());
                };
                href = Some(lit_string(value).ok_or("<A href> must be a string")?);
            }
        }
        let Some(href) = href else {
            return Err("<A> needs href".into());
        };
        let mut el = self.lower_el(element, signals, in_row)?;
        if let ViewNode::El(el) = &mut el {
            el.tag = "button".to_string();
            el.click = Some(Act::Body(vec![HandlerStmt::Navigate(HxExpr::Str(href))]));
        }
        Ok(el)
    }

    fn lower_show(
        &mut self,
        element: &NodeElement<rstml::Infallible>,
        signals: &[Signal],
        in_row: bool,
    ) -> Result<ViewNode, String> {
        let mut when = None;
        for attr in element.attributes() {
            let NodeAttribute::Attribute(attr) = attr else {
                continue;
            };
            let key = attr_key(attr);
            if key == "when" {
                let Some(value) = attr.value() else {
                    return Err("Show when needs a value".into());
                };
                when = Some(bool_signal(value, signals)?);
            }
        }
        let Some(signal) = when else {
            return Err("<Show> needs when".into());
        };
        let mut body = Vec::new();
        for child in element.children() {
            if is_blank(child) {
                continue;
            }
            body.push(self.lower_node(child, signals, in_row)?);
        }
        if body.len() != 1 {
            return Err("<Show> needs one child".into());
        }
        let child = body.pop().unwrap();
        // The condition is written by the button that closes the show. Resetting
        // it on the opening frame clears the flag before the on-branch runs.
        let reset = show_resets(&child, &signal);
        Ok(ViewNode::Show {
            live_flag: format!("show_{signal}_live"),
            signal,
            reset,
            child: Box::new(child),
        })
    }

    fn lower_suspense(
        &mut self,
        element: &NodeElement<rstml::Infallible>,
        signals: &[Signal],
        in_row: bool,
        transition: bool,
    ) -> Result<ViewNode, String> {
        let fallback = self.fallback_of(element, signals, in_row)?;
        let child = self.one_body(element, signals, in_row, "Suspense")?;
        let jobs = jobs_in(&child, &self.job_names);
        Ok(ViewNode::Suspense {
            jobs,
            transition,
            fallback,
            child: Box::new(child),
        })
    }

    fn lower_boundary(
        &mut self,
        element: &NodeElement<rstml::Infallible>,
        signals: &[Signal],
        in_row: bool,
    ) -> Result<ViewNode, String> {
        let fallback = self.fallback_of(element, signals, in_row)?;
        let child = self.one_body(element, signals, in_row, "ErrorBoundary")?;
        let jobs = jobs_in(&child, &self.job_names);
        Ok(ViewNode::Boundary {
            jobs,
            fallback,
            child: Box::new(child),
        })
    }

    fn lower_portal(
        &mut self,
        element: &NodeElement<rstml::Infallible>,
        signals: &[Signal],
        in_row: bool,
    ) -> Result<ViewNode, String> {
        let mut nodes = Vec::new();
        for child in element.children() {
            if is_blank(child) {
                continue;
            }
            nodes.push(self.lower_node(child, signals, in_row)?);
        }
        if nodes.is_empty() {
            return Err("<Portal> needs a child".into());
        }
        Ok(ViewNode::Portal { nodes })
    }

    fn fallback_of(
        &mut self,
        element: &NodeElement<rstml::Infallible>,
        signals: &[Signal],
        in_row: bool,
    ) -> Result<Vec<ViewNode>, String> {
        for attr in element.attributes() {
            let NodeAttribute::Attribute(attr) = attr else {
                continue;
            };
            if attr_key(attr) != "fallback" {
                continue;
            }
            let Some(value) = attr.value() else {
                return Err("fallback needs || view! { ... }".into());
            };
            return self.lower_fallback(value, signals, in_row);
        }
        Err("needs fallback=|| view! { ... }".into())
    }

    fn lower_fallback(
        &mut self,
        expr: &Expr,
        signals: &[Signal],
        in_row: bool,
    ) -> Result<Vec<ViewNode>, String> {
        let body = match expr {
            Expr::Closure(closure) => &*closure.body,
            other => other,
        };
        let Some(tokens) = view_tokens(body) else {
            return Err("fallback must be || view! { ... }".into());
        };
        let nodes = rstml::parse2(tokens).map_err(|err| err.to_string())?;
        let mut out = Vec::new();
        for node in &nodes {
            if is_blank(node) {
                continue;
            }
            out.push(self.lower_node(node, signals, in_row)?);
        }
        if out.is_empty() {
            return Err("fallback is empty".into());
        }
        Ok(out)
    }

    fn one_body(
        &mut self,
        element: &NodeElement<rstml::Infallible>,
        signals: &[Signal],
        in_row: bool,
        tag: &str,
    ) -> Result<ViewNode, String> {
        let mut body = Vec::new();
        for child in element.children() {
            if is_blank(child) {
                continue;
            }
            body.push(self.lower_node(child, signals, in_row)?);
        }
        if body.is_empty() {
            return Err(format!("<{tag}> needs a child"));
        }
        if body.len() == 1 {
            return Ok(body.pop().unwrap());
        }
        Ok(ViewNode::Slot {
            fn_name: String::new(),
            nodes: body,
            arg: None,
        })
    }

    fn lower_for(
        &mut self,
        element: &NodeElement<rstml::Infallible>,
        signals: &[Signal],
    ) -> Result<ViewNode, String> {
        let mut items = None;
        let mut var = "item".to_string();
        let mut key_field = "id".to_string();
        for attr in element.attributes() {
            let NodeAttribute::Attribute(attr) = attr else {
                continue;
            };
            let key = attr_key(attr);
            if key == "each" {
                let Some(value) = attr.value() else {
                    return Err("<For each> needs a value".into());
                };
                items = Some(each_source(value, signals, &self.job_names)?);
            } else if key == "key" {
                if let Some(value) = attr.value() {
                    if let Some((name, field)) = key_field_of(value) {
                        var = name;
                        key_field = field;
                    }
                }
            }
        }
        let Some(items) = items else {
            return Err("<For> needs each".into());
        };
        let mut body = Vec::new();
        for child in element.children() {
            if is_blank(child) {
                continue;
            }
            body.push(child);
        }
        if body.len() != 1 {
            return Err("<For> needs one child".into());
        }
        self.pending_row_signals.clear();
        let saved_row = self.row_var.clone();
        self.row_var = Some(var.clone());
        let child = self.lower_node(body[0], signals, true)?;
        self.row_var = saved_row;
        let ViewNode::El(el) = child else {
            return Err("<For> child must be one element".into());
        };
        self.fns += 1;
        let fn_name = if self.fns == 1 {
            "quinn_row".to_string()
        } else {
            format!("quinn_row{}", self.fns)
        };
        let params = row_params(&el);
        let row_signals = std::mem::take(&mut self.pending_row_signals);
        Ok(ViewNode::For {
            items,
            var,
            key_field,
            fn_name,
            params,
            row_signals,
            child: el,
        })
    }

    fn lower_component(
        &mut self,
        name: &str,
        element: &NodeElement<rstml::Infallible>,
        parent_signals: &[Signal],
        in_row: bool,
    ) -> Result<ViewNode, String> {
        let comp = self
            .comps
            .get(name)
            .ok_or_else(|| format!("unknown component <{name}>"))?
            .clone_refs();
        let takes_children = comp
            .params
            .iter()
            .any(|prop| matches!(prop.kind, PropKind::Children | PropKind::ChildrenFn));
        let children_fn = comp
            .params
            .iter()
            .any(|prop| matches!(prop.kind, PropKind::ChildrenFn));
        let saved_children_fn = self.in_children_fn;
        self.in_children_fn = children_fn;
        let mut call_children = Vec::new();
        for child in element.children() {
            if is_blank(child) {
                continue;
            }
            if !takes_children {
                return Err(format!("<{name}> does not take child elements"));
            }
            call_children.push(self.lower_node(child, parent_signals, in_row)?);
        }
        self.in_children_fn = saved_children_fn;
        let given = prop_values(&comp, element, parent_signals)?;
        let saved_props = std::mem::replace(&mut self.prop_env, given);
        let saved_ctx =
            std::mem::replace(&mut self.ctx_env, comp.contexts.iter().cloned().collect());
        let saved_provides = std::mem::replace(
            &mut self.provides_here,
            comp.provides
                .iter()
                .map(|provide| (provide.key.clone(), provide.expr.clone()))
                .collect(),
        );
        let tokens = comp.view.clone();
        let parsed = rstml::parse2(tokens).map_err(|err| err.to_string())?;
        let mut node = self.lower_view_root(&parsed, &comp.signals, in_row)?;
        if let ViewNode::El(el) = &mut node {
            el.provides.append(&mut self.provides_here);
        }
        self.prop_env = saved_props;
        self.ctx_env = saved_ctx;
        self.provides_here = saved_provides;
        let prefix = slug(name);
        if !in_row {
            rename_signals(&mut node, &comp.signals, &prefix);
        }
        if in_row {
            for signal in &comp.signals {
                if !self
                    .pending_row_signals
                    .iter()
                    .any(|have| have.read == signal.read)
                {
                    self.pending_row_signals.push(signal.clone());
                }
            }
            inline_children(&mut node, call_children);
        } else {
            self.seal_children(&mut node, call_children);
        }
        if let ViewNode::El(el) = &mut node {
            if el.name.starts_with("div") || el.name == "div" {
                el.name = self.fresh(&prefix);
            }
        }
        let _ = comp.aliases;
        Ok(node)
    }

    fn lower_view_root(
        &mut self,
        nodes: &[Node],
        signals: &[Signal],
        in_row: bool,
    ) -> Result<ViewNode, String> {
        let mut found = Vec::new();
        for node in nodes {
            if is_blank(node) {
                continue;
            }
            found.push(node);
        }
        if found.len() == 1 {
            if let Node::Fragment(fragment) = found[0] {
                let mut body = Vec::new();
                for child in fragment.children() {
                    if is_blank(child) {
                        continue;
                    }
                    body.push(self.lower_node(child, signals, in_row)?);
                }
                return Ok(ViewNode::Slot {
                    fn_name: String::new(),
                    nodes: body,
                    arg: None,
                });
            }
        }
        if found.len() != 1 {
            return Err("the view must have one root element".into());
        }
        self.lower_node(found[0], signals, in_row)
    }

    fn seal_children(&mut self, node: &mut ViewNode, children: Vec<ViewNode>) {
        match node {
            ViewNode::El(el) => {
                let mut next = Vec::new();
                for child in std::mem::take(&mut el.children) {
                    if let ViewNode::Slot { fn_name, arg, .. } = &child {
                        if fn_name == "__children__" {
                            self.slots += 1;
                            next.push(ViewNode::Slot {
                                fn_name: format!("quinn_slot_{}", self.slots),
                                nodes: children.clone(),
                                arg: arg.clone(),
                            });
                            continue;
                        }
                    }
                    let mut child = child;
                    self.seal_children(&mut child, children.clone());
                    next.push(child);
                }
                el.children = next;
            }
            ViewNode::Show { child, .. } => self.seal_children(child, children),
            ViewNode::For { .. } => {}
            ViewNode::Slot { nodes, .. } => {
                for child in nodes {
                    self.seal_children(child, children.clone());
                }
            }
            ViewNode::Routes { routes } => {
                for (_, nodes) in routes {
                    for child in nodes {
                        self.seal_children(child, children.clone());
                    }
                }
            }
            ViewNode::Suspense { fallback, child, .. } | ViewNode::Boundary { fallback, child, .. } => {
                for node in fallback {
                    self.seal_children(node, children.clone());
                }
                self.seal_children(child, children);
            }
            ViewNode::Portal { nodes } => {
                for child in nodes {
                    self.seal_children(child, children.clone());
                }
            }
        }
    }

    fn lower_el(
        &mut self,
        element: &NodeElement<rstml::Infallible>,
        signals: &[Signal],
        in_row: bool,
    ) -> Result<ViewNode, String> {
        let tag = element.name().to_string();
        let class = class_of(element).unwrap_or_default();
        let mut class_when = Vec::new();
        let mut style_when = Vec::new();
        let mut inline_style = String::new();
        let mut parts = Vec::new();
        let mut placeholder = None;
        let mut value = None;
        let mut click = None;
        let mut input = None;
        let mut bind = None;
        for attr in element.attributes() {
            let NodeAttribute::Attribute(attr) = attr else {
                return Err("computed attribute keys are not supported".into());
            };
            let key = attr_key(attr);
            if key == "class" || key.starts_with("let:") {
                continue;
            }
            if let Some(token) = key.strip_prefix("class:") {
                let Some(expr) = attr.value() else {
                    return Err(format!("class:{token} needs a bool signal"));
                };
                class_when.push(ClassWhen {
                    token: token.to_string(),
                    signal: bool_signal(expr, signals)?,
                });
                continue;
            }
            if let Some(prop) = key.strip_prefix("style:") {
                let Some(expr) = attr.value() else {
                    return Err(format!("style:{prop} needs a value"));
                };
                let (signal, yes, no) = style_branches(expr, signals)?;
                style_when.push(StyleWhen {
                    prop: prop.to_string(),
                    signal,
                    yes,
                    no,
                });
                continue;
            }
            if key == "bind:value" {
                let Some(expr) = attr.value() else {
                    return Err("bind:value needs a signal".into());
                };
                let signal = bind_signal(expr, signals)?;
                bind = Some(signal.clone());
                input = Some(signal);
                continue;
            }
            if (tag == "A" || tag == "a") && key == "href" {
                continue;
            }
            let Some(expr) = attr.value() else {
                return Err(format!("<{tag} {key}> needs a value"));
            };
            match key.as_str() {
                "on:click" => click = Some(crate::handler::parse_handler(expr, signals, in_row)?),
                "on:input" | "on:change" => {
                    input = Some(crate::handler::parse_input_signal(expr, signals)?);
                }
                "placeholder" => {
                    placeholder = Some(lit_string(expr).unwrap_or_else(|| expr_snippet(expr)));
                }
                "value" => value = Some(signal_get(expr, signals)?),
                "style" => {
                    inline_style = lit_string(expr).ok_or("style must be a string literal")?;
                }
                "item" => {}
                other => return Err(format!("unsupported attribute `{other}` on <{tag}>")),
            }
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
                Node::Block(block) => {
                    if children_call(block).is_some() {
                        continue;
                    }
                    parts.push(parse_interp(
                        block,
                        signals,
                        in_row,
                        &self.prop_env,
                        &self.ctx_env,
                        self.in_children_fn,
                        &self.job_names,
                        self.row_var.as_deref(),
                    )?);
                }
                Node::Comment(_) | Node::Doctype(_) => {}
                Node::Element(_) | Node::Fragment(_) => {}
                other => {
                    return Err(format!("unsupported child in <{tag}>: {other:?}"));
                }
            }
        }
        let mut children = Vec::new();
        for child in element.children() {
            if let Node::Block(block) = child {
                if let Some(arg) = children_call(block) {
                    children.push(ViewNode::Slot {
                        fn_name: "__children__".into(),
                        nodes: Vec::new(),
                        arg,
                    });
                    continue;
                }
            }
            if let Node::Fragment(_) = child {
                let node = self.lower_node(child, signals, in_row)?;
                if let ViewNode::Slot {
                    fn_name,
                    nodes,
                    arg,
                } = node
                {
                    if fn_name.is_empty() {
                        children.extend(nodes);
                        continue;
                    }
                    children.push(ViewNode::Slot {
                        fn_name,
                        nodes,
                        arg,
                    });
                    continue;
                }
                children.push(node);
                continue;
            }
            if let Node::Element(_) = child {
                let node = self.lower_node(child, signals, in_row)?;
                if let ViewNode::Slot {
                    fn_name,
                    nodes,
                    arg,
                } = node
                {
                    if fn_name.is_empty() {
                        children.extend(nodes);
                        continue;
                    }
                    children.push(ViewNode::Slot {
                        fn_name,
                        nodes,
                        arg,
                    });
                    continue;
                }
                children.push(node);
            }
        }
        let name = self.fresh(&widget_base(&tag, &parts, click.as_ref()));
        Ok(ViewNode::El(El {
            name,
            tag,
            class,
            style: inline_style,
            class_when,
            style_when,
            parts,
            placeholder,
            value,
            click,
            input,
            bind,
            provides: Vec::new(),
            children,
        }))
    }

    fn fresh(&mut self, base: &str) -> String {
        let mut name = if base.is_empty() {
            "node".to_string()
        } else {
            base.to_string()
        };
        let mut n = 2;
        while !self.used.insert(name.clone()) {
            name = format!("{base}_{n}");
            n += 1;
        }
        name
    }
}

impl Comp {
    fn clone_refs(&self) -> Comp {
        Comp {
            params: self.params.clone(),
            signals: self.signals.clone(),
            aliases: self.aliases.clone(),
            contexts: self.contexts.clone(),
            provides: self.provides.clone(),
            jobs: self.jobs.clone(),
            view: self.view.clone(),
            frame: self.frame.clone(),
            retain: self.retain,
        }
    }
}

fn prop_values(
    comp: &Comp,
    element: &NodeElement<rstml::Infallible>,
    signals: &[Signal],
) -> Result<HashMap<String, PropVal>, String> {
    let mut given = HashMap::new();
    for attr in element.attributes() {
        let NodeAttribute::Attribute(attr) = attr else {
            continue;
        };
        let key = attr_key(attr);
        if key == "class" || key.starts_with("let:") {
            continue;
        }
        let Some(spec) = comp.params.iter().find(|prop| prop.name == key) else {
            if key == "item" {
                continue;
            }
            return Err(format!("<{key}> is not a prop of this component"));
        };
        let Some(expr) = attr.value() else {
            return Err(format!("{key} needs a value"));
        };
        given.insert(spec.name.clone(), prop_val(spec, expr, signals)?);
    }
    for spec in &comp.params {
        if matches!(
            spec.kind,
            PropKind::Children | PropKind::ChildrenFn | PropKind::Item
        ) {
            continue;
        }
        if given.contains_key(&spec.name) {
            continue;
        }
        if let Some(default) = &spec.default {
            given.insert(spec.name.clone(), default.clone());
        } else if spec.optional {
            given.insert(spec.name.clone(), PropVal::Empty);
        } else {
            return Err(format!("missing prop {}", spec.name));
        }
    }
    Ok(given)
}

fn prop_val(spec: &PropSpec, expr: &Expr, signals: &[Signal]) -> Result<PropVal, String> {
    if let Some(text) = lit_string(expr) {
        return Ok(PropVal::Lit(text));
    }
    // `#[prop(into)]` unwraps `"Ada".into()` / `.to_string()` to the literal.
    // Script has no Rust `Into` impl. A path that is not a signal is not one.
    if spec.into {
        if let Ok(text) = stringish(expr) {
            return Ok(PropVal::Lit(text));
        }
    }
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Int(value) => Ok(PropVal::Int(
                value.base10_parse::<i64>().map_err(|err| err.to_string())?,
            )),
            syn::Lit::Bool(value) => Ok(PropVal::Bool(value.value)),
            _ => Err(format!("unsupported prop {}", spec.name)),
        },
        Expr::Closure(_) | Expr::MethodCall(_) => {
            let name = signal_get(expr, signals)?;
            Ok(PropVal::Sig(name))
        }
        Expr::Path(path) => {
            let name = path
                .path
                .get_ident()
                .map(|ident| ident.to_string())
                .ok_or_else(|| format!("unsupported prop {}", spec.name))?;
            if signals.iter().any(|signal| signal.read == name) {
                Ok(PropVal::Sig(name))
            } else if spec.into {
                Err(format!(
                    "prop {} is not a signal. Pass a string, \"...\".into(), or a signal",
                    spec.name
                ))
            } else if matches!(spec.kind, PropKind::Text | PropKind::Item) {
                Ok(PropVal::Sig(name))
            } else {
                Err(format!("unsupported prop {}", spec.name))
            }
        }
        other => Err(format!(
            "unsupported prop {} ({})",
            spec.name,
            snippet(other)
        )),
    }
}

fn prop_part(props: &HashMap<String, PropVal>, name: &str) -> Option<Part> {
    let value = props.get(name)?;
    Some(match value {
        PropVal::Lit(text) => Part::Lit(text.clone()),
        PropVal::Int(value) => Part::Lit(value.to_string()),
        PropVal::Bool(value) => Part::Lit(if *value { "true" } else { "false" }.into()),
        PropVal::Sig(signal) => Part::Sig(signal.clone()),
        PropVal::Empty => Part::Lit(String::new()),
    })
}

fn inline_children(node: &mut ViewNode, children: Vec<ViewNode>) {
    match node {
        ViewNode::El(el) => {
            let mut next = Vec::new();
            for child in std::mem::take(&mut el.children) {
                if let ViewNode::Slot { fn_name, .. } = &child {
                    if fn_name == "__children__" {
                        next.extend(children.clone());
                        continue;
                    }
                }
                let mut child = child;
                inline_children(&mut child, children.clone());
                next.push(child);
            }
            el.children = next;
        }
        ViewNode::Show { child, .. } => inline_children(child, children),
        ViewNode::Slot { nodes, .. } => {
            for child in nodes {
                inline_children(child, children.clone());
            }
        }
        ViewNode::Routes { routes } => {
            for (_, nodes) in routes {
                for child in nodes {
                    inline_children(child, children.clone());
                }
            }
        }
        ViewNode::For { .. } => {}
        ViewNode::Suspense { fallback, child, .. } | ViewNode::Boundary { fallback, child, .. } => {
            for node in fallback {
                inline_children(node, children.clone());
            }
            inline_children(child, children);
        }
        ViewNode::Portal { nodes } => {
            for node in nodes {
                inline_children(node, children.clone());
            }
        }
    }
}

/// `Some(None)` is `{children()}`. `Some(Some(script))` is `{children()(expr)}`.
fn children_call(block: &NodeBlock) -> Option<Option<String>> {
    let NodeBlock::ValidBlock(block) = block else {
        return None;
    };
    if block.stmts.len() != 1 {
        return None;
    }
    let Stmt::Expr(expr, None) = &block.stmts[0] else {
        return None;
    };
    let Expr::Call(call) = expr else {
        return None;
    };
    if path_ident_opt(&call.func).as_deref() == Some("children") {
        return Some(None);
    }
    let Expr::Call(inner) = &*call.func else {
        return None;
    };
    if path_ident_opt(&inner.func).as_deref() != Some("children") {
        return None;
    }
    let arg = call.args.first()?;
    Some(Some(script_arg(arg)))
}

fn script_arg(expr: &Expr) -> String {
    if let Some(text) = lit_string(expr) {
        return format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""));
    }
    if let Expr::Lit(lit) = expr {
        if let syn::Lit::Int(value) = &lit.lit {
            return value.base10_digits().to_string();
        }
        if let syn::Lit::Bool(value) = &lit.lit {
            return if value.value { "true" } else { "false" }.into();
        }
    }
    if let Some(name) = path_ident_opt(expr) {
        return format!("state.{name}");
    }
    "arg".into()
}

fn bind_signal(expr: &Expr, signals: &[Signal]) -> Result<String, String> {
    if let Some(path) = signal_field_path(expr) {
        let base = path.split('.').next().unwrap_or(path.as_str());
        if signals.iter().any(|signal| signal.read == base) {
            return Ok(path);
        }
        return Err(format!("unknown signal {base}"));
    }
    if let Expr::Path(path) = expr {
        if let Some(name) = path.path.get_ident() {
            if signals.iter().any(|signal| signal.read == name.to_string()) {
                return Ok(name.to_string());
            }
            return Err(format!("unknown signal {name}"));
        }
    }
    signal_get(expr, signals)
}

fn signal_field_path(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Closure(closure) => match &*closure.body {
            Expr::Field(field) => field_chain(field),
            _ => None,
        },
        Expr::Field(field) => field_chain(field),
        _ => None,
    }
}

fn field_chain(field: &syn::ExprField) -> Option<String> {
    let member = match &field.member {
        syn::Member::Named(name) => name.to_string(),
        syn::Member::Unnamed(_) => return None,
    };
    let base = match &*field.base {
        Expr::MethodCall(call) if call.method == "get" => path_ident_opt(&call.receiver)?,
        Expr::Field(inner) => field_chain(inner)?,
        Expr::Path(path) => path.path.get_ident()?.to_string(),
        _ => return None,
    };
    Some(format!("{base}.{member}"))
}

fn rename_signals(node: &mut ViewNode, orig: &[Signal], prefix: &str) {
    match node {
        ViewNode::El(el) => {
            for part in &mut el.parts {
                rename_part(part, orig, prefix);
            }
            if let Some(value) = &mut el.value {
                *value = prefixed(value, orig, prefix);
            }
            if let Some(input) = &mut el.input {
                *input = prefixed(input, orig, prefix);
            }
            if let Some(bind) = &mut el.bind {
                *bind = prefixed(bind, orig, prefix);
            }
            if let Some(click) = &mut el.click {
                rename_act(click, orig, prefix);
            }
            for when in &mut el.class_when {
                when.signal = prefixed(&when.signal, orig, prefix);
            }
            for when in &mut el.style_when {
                when.signal = prefixed(&when.signal, orig, prefix);
            }
            for child in &mut el.children {
                rename_signals(child, orig, prefix);
            }
        }
        ViewNode::Show {
            signal,
            reset,
            child,
            ..
        } => {
            *signal = prefixed(signal, orig, prefix);
            for name in reset.iter_mut() {
                *name = prefixed(name, orig, prefix);
            }
            rename_signals(child, orig, prefix);
        }
        ViewNode::For { items, child, .. } => {
            *items = prefixed(items, orig, prefix);
            rename_el_signals(child, orig, prefix);
        }
        ViewNode::Slot { nodes, .. } => {
            for child in nodes {
                rename_signals(child, orig, prefix);
            }
        }
        ViewNode::Routes { routes } => {
            for (_, nodes) in routes {
                for child in nodes {
                    rename_signals(child, orig, prefix);
                }
            }
        }
        ViewNode::Suspense { fallback, child, .. } | ViewNode::Boundary { fallback, child, .. } => {
            for node in fallback {
                rename_signals(node, orig, prefix);
            }
            rename_signals(child, orig, prefix);
        }
        ViewNode::Portal { nodes } => {
            for child in nodes {
                rename_signals(child, orig, prefix);
            }
        }
    }
}

fn rename_el_signals(el: &mut El, orig: &[Signal], prefix: &str) {
    for part in &mut el.parts {
        rename_part(part, orig, prefix);
    }
    if let Some(click) = &mut el.click {
        rename_act(click, orig, prefix);
    }
    for when in &mut el.class_when {
        when.signal = prefixed(&when.signal, orig, prefix);
    }
    for when in &mut el.style_when {
        when.signal = prefixed(&when.signal, orig, prefix);
    }
    for child in &mut el.children {
        rename_signals(child, orig, prefix);
    }
}

fn rename_part(part: &mut Part, orig: &[Signal], prefix: &str) {
    match part {
        Part::Sig(name)
        | Part::IfEmpty { signal: name, .. }
        | Part::IfBool { signal: name, .. } => {
            *name = prefixed(name, orig, prefix);
        }
        Part::Lit(_) | Part::Param(_) | Part::Ctx(_) | Part::Job(_) => {}
    }
}

fn rename_act(act: &mut Act, orig: &[Signal], prefix: &str) {
    match act {
        Act::Inc(name)
        | Act::Toggle(name)
        | Act::Clear(name)
        | Act::Pop(name)
        | Act::Reverse(name) => {
            *name = prefixed(name, orig, prefix);
        }
        Act::Push { next, items } => {
            *next = prefixed(next, orig, prefix);
            *items = prefixed(items, orig, prefix);
        }
        Act::RowInc => {}
        Act::Body(stmts) | Act::Row(stmts) => {
            for stmt in stmts {
                match stmt {
                    HandlerStmt::Set { signal, expr } => {
                        *signal = prefixed(signal, orig, prefix);
                        rename_expr(expr, orig, prefix);
                    }
                    HandlerStmt::Let { expr, .. } => rename_expr(expr, orig, prefix),
                    HandlerStmt::Push { items, id, label } => {
                        *items = prefixed(items, orig, prefix);
                        rename_expr(id, orig, prefix);
                        rename_expr(label, orig, prefix);
                    }
                    HandlerStmt::Pop { items } | HandlerStmt::Reverse { items } => {
                        *items = prefixed(items, orig, prefix);
                    }
                    HandlerStmt::RemoveKey { items, key } => {
                        *items = prefixed(items, orig, prefix);
                        rename_expr(key, orig, prefix);
                    }
                    HandlerStmt::SwapKey { items, a, b } => {
                        *items = prefixed(items, orig, prefix);
                        rename_expr(a, orig, prefix);
                        rename_expr(b, orig, prefix);
                    }
                    HandlerStmt::Navigate(expr) => rename_expr(expr, orig, prefix),
                    HandlerStmt::Dispatch { arg, .. } => rename_expr(arg, orig, prefix),
                    HandlerStmt::Refetch { .. } | HandlerStmt::ToggleTheme => {}
                }
            }
        }
    }
}

fn rename_expr(expr: &mut HxExpr, orig: &[Signal], prefix: &str) {
    match expr {
        HxExpr::Name(name) => *name = prefixed(name, orig, prefix),
        HxExpr::Not(inner) => rename_expr(inner, orig, prefix),
        HxExpr::Binary(_, left, right) => {
            rename_expr(left, orig, prefix);
            rename_expr(right, orig, prefix);
        }
        HxExpr::Concat(parts) | HxExpr::List(parts) => {
            for part in parts {
                rename_expr(part, orig, prefix);
            }
        }
        HxExpr::Obj(fields) => {
            for (_, value) in fields {
                rename_expr(value, orig, prefix);
            }
        }
        HxExpr::Int(_) | HxExpr::Bool(_) | HxExpr::Str(_) => {}
    }
}

fn prefixed(name: &str, orig: &[Signal], prefix: &str) -> String {
    if orig.iter().any(|signal| signal.read == name) {
        format!("{prefix}_{name}")
    } else {
        name.to_string()
    }
}

fn show_resets(node: &ViewNode, condition: &str) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(node: &ViewNode, condition: &str, out: &mut Vec<String>) {
        match node {
            ViewNode::El(el) => {
                for name in act_reset_names(el.click.as_ref()) {
                    if name != condition && !out.contains(&name) {
                        out.push(name);
                    }
                }
                for part in &el.parts {
                    if let Part::Sig(name) = part {
                        if name != condition && !out.contains(name) {
                            out.push(name.clone());
                        }
                    }
                }
                for child in &el.children {
                    walk(child, condition, out);
                }
            }
            ViewNode::Show { child, .. } => walk(child, condition, out),
            ViewNode::For { .. } => {}
            ViewNode::Slot { nodes, .. } => {
                for child in nodes {
                    walk(child, condition, out);
                }
            }
            ViewNode::Routes { routes } => {
                for (_, nodes) in routes {
                    for child in nodes {
                        walk(child, condition, out);
                    }
                }
            }
            ViewNode::Suspense { fallback, child, .. } | ViewNode::Boundary { fallback, child, .. } => {
                for node in fallback {
                    walk(node, condition, out);
                }
                walk(child, condition, out);
            }
            ViewNode::Portal { nodes } => {
                for child in nodes {
                    walk(child, condition, out);
                }
            }
        }
    }
    walk(node, condition, &mut out);
    out
}

fn row_params(el: &El) -> Vec<String> {
    let mut out = Vec::new();
    fn walk_el(el: &El, out: &mut Vec<String>) {
        for part in &el.parts {
            if let Part::Param(name) = part {
                if !out.contains(&name) {
                    out.push(name.clone());
                }
            }
        }
        for child in &el.children {
            walk_node(child, out);
        }
    }
    fn walk_node(node: &ViewNode, out: &mut Vec<String>) {
        match node {
            ViewNode::El(el) => walk_el(el, out),
            ViewNode::Show { child, .. } => walk_node(child, out),
            ViewNode::For { .. } => {}
            ViewNode::Slot { nodes, .. } => {
                for child in nodes {
                    walk_node(child, out);
                }
            }
            ViewNode::Routes { routes } => {
                for (_, nodes) in routes {
                    for child in nodes {
                        walk_node(child, out);
                    }
                }
            }
            ViewNode::Suspense { fallback, child, .. } | ViewNode::Boundary { fallback, child, .. } => {
                for node in fallback {
                    walk_node(node, out);
                }
                walk_node(child, out);
            }
            ViewNode::Portal { nodes } => {
                for child in nodes {
                    walk_node(child, out);
                }
            }
        }
    }
    walk_el(el, &mut out);
    out
}

fn try_job(local: &syn::Local) -> Result<Option<Job>, String> {
    let syn::Pat::Ident(ident) = &local.pat else {
        return Ok(None);
    };
    let Some(init) = &local.init else {
        return Ok(None);
    };
    let Expr::Call(call) = &*init.expr else {
        return Ok(None);
    };
    let name = ident.ident.to_string();
    if path_ends(&call.func, &["LocalResource", "new"]) {
        let Some(body) = call.args.first() else {
            return Err(format!("{name} needs a closure that calls an async fn"));
        };
        let mut deps = Vec::new();
        let (func, arg, plan) = parse_job_body(body, JobKind::Local, &mut deps)?;
        return Ok(Some(Job {
            name,
            kind: JobKind::Local,
            func,
            arg,
            plan,
            deps,
        }));
    }
    if path_ends(&call.func, &["Resource", "new"]) {
        if call.args.len() < 2 {
            return Err(format!("{name} needs a source and a fetcher"));
        }
        let mut deps = source_deps(&call.args[0])?;
        let (func, arg, plan) = parse_job_body(&call.args[1], JobKind::Resource, &mut deps)?;
        return Ok(Some(Job {
            name,
            kind: JobKind::Resource,
            func,
            arg,
            plan,
            deps,
        }));
    }
    if path_ends(&call.func, &["Action", "new"]) {
        let Some(body) = call.args.first() else {
            return Err(format!("{name} needs a closure that calls an async fn"));
        };
        let mut deps = Vec::new();
        let (func, arg, plan) = parse_job_body(body, JobKind::Action, &mut deps)?;
        return Ok(Some(Job {
            name,
            kind: JobKind::Action,
            func,
            arg,
            plan,
            deps,
        }));
    }
    Ok(None)
}

fn source_deps(expr: &Expr) -> Result<Vec<String>, String> {
    let mut deps = Vec::new();
    walk_gets(expr, &mut deps);
    if deps.is_empty() {
        return Err("resource source must read a signal with .get()".into());
    }
    Ok(deps)
}

fn walk_gets(expr: &Expr, deps: &mut Vec<String>) {
    match expr {
        Expr::Closure(closure) => walk_gets(&closure.body, deps),
        Expr::MethodCall(call) if call.method == "get" => {
            if let Ok(name) = path_ident(&call.receiver) {
                if !deps.iter().any(|have| have == &name) {
                    deps.push(name);
                }
            }
        }
        Expr::Block(block) => {
            for stmt in &block.block.stmts {
                if let Stmt::Expr(expr, _) = stmt {
                    walk_gets(expr, deps);
                }
            }
        }
        Expr::Async(block) => {
            for stmt in &block.block.stmts {
                if let Stmt::Expr(expr, _) = stmt {
                    walk_gets(expr, deps);
                }
            }
        }
        Expr::Tuple(tuple) => {
            for elem in &tuple.elems {
                walk_gets(elem, deps);
            }
        }
        Expr::Paren(paren) => walk_gets(&paren.expr, deps),
        Expr::Try(inner) => walk_gets(&inner.expr, deps),
        Expr::Await(inner) => walk_gets(&inner.base, deps),
        _ => {}
    }
}

fn parse_job_body(
    expr: &Expr,
    kind: JobKind,
    deps: &mut Vec<String>,
) -> Result<(String, ArgSrc, String), String> {
    let (expr, params) = unwrap_job_closure(expr);
    let mut aliases = Vec::new();
    let mut bindings = Vec::new();
    let steps = collect_steps(expr, kind, &params, &mut aliases, &mut bindings, deps)?;
    if steps.is_empty() {
        return Err("a job closure has no function call".into());
    }
    lower_steps(&steps, kind, &params, deps)
}

fn unwrap_job_closure(expr: &Expr) -> (&Expr, Vec<String>) {
    if let Expr::Closure(closure) = expr {
        let params = closure.inputs.iter().flat_map(bind_names).collect();
        return (&closure.body, params);
    }
    (expr, Vec::new())
}

fn bind_names(pat: &syn::Pat) -> Vec<String> {
    match pat {
        syn::Pat::Ident(ident) => vec![ident.ident.to_string()],
        syn::Pat::Type(typed) => bind_names(&typed.pat),
        syn::Pat::Reference(refer) => bind_names(&refer.pat),
        syn::Pat::Tuple(tuple) => tuple.elems.iter().flat_map(bind_names).collect(),
        _ => Vec::new(),
    }
}

struct Step {
    func: String,
    inputs: Vec<JobInput>,
}

enum JobInput {
    Lit(String),
    Signal(String),
    Param(String),
    Step(usize),
    Obj(Vec<(String, JobInput)>),
}

fn collect_steps(
    expr: &Expr,
    kind: JobKind,
    params: &[String],
    aliases: &mut Vec<(String, String)>,
    bindings: &mut Vec<(String, usize)>,
    deps: &mut Vec<String>,
) -> Result<Vec<Step>, String> {
    match expr {
        Expr::Async(block) => {
            collect_block(&block.block, kind, params, aliases, bindings, deps)
        }
        Expr::Block(block) => collect_block(&block.block, kind, params, aliases, bindings, deps),
        Expr::Paren(paren) => {
            collect_steps(&paren.expr, kind, params, aliases, bindings, deps)
        }
        other => {
            let step = parse_step(other, kind, params, aliases, bindings, deps)?;
            Ok(vec![step])
        }
    }
}

fn collect_block(
    block: &syn::Block,
    kind: JobKind,
    params: &[String],
    aliases: &mut Vec<(String, String)>,
    bindings: &mut Vec<(String, usize)>,
    deps: &mut Vec<String>,
) -> Result<Vec<Step>, String> {
    let mut steps = Vec::new();
    for stmt in &block.stmts {
        match stmt {
            Stmt::Local(local) => {
                if await_let(local) {
                    let syn::Pat::Ident(ident) = &local.pat else {
                        return Err("unsupported job shape: let pattern. A job let is a name."
                            .into());
                    };
                    let init = &local.init.as_ref().unwrap().expr;
                    let step = parse_step(init, kind, params, aliases, bindings, deps)?;
                    let index = steps.len();
                    steps.push(step);
                    bindings.push((ident.ident.to_string(), index));
                } else {
                    record_job_alias(local, params, aliases)?;
                }
            }
            Stmt::Expr(expr, _) => match expr {
                Expr::Async(_) | Expr::Block(_) | Expr::Paren(_) => {
                    steps.extend(collect_steps(
                        expr, kind, params, aliases, bindings, deps,
                    )?);
                }
                other => steps.push(parse_step(
                    other, kind, params, aliases, bindings, deps,
                )?),
            },
            _ => {
                return Err(
                    "unsupported job shape: statement. A job is calls and `let name = call().await?`."
                        .into(),
                )
            }
        }
    }
    if steps.is_empty() {
        return Err("a job closure has no function call".into());
    }
    Ok(steps)
}

fn await_let(local: &syn::Local) -> bool {
    let Some(init) = &local.init else {
        return false;
    };
    contains_await(&init.expr) || matches!(peel_call(&init.expr), Expr::Call(_))
}

fn contains_await(expr: &Expr) -> bool {
    match expr {
        Expr::Await(_) => true,
        Expr::Try(inner) => contains_await(&inner.expr),
        Expr::Paren(inner) => contains_await(&inner.expr),
        Expr::Reference(inner) => contains_await(&inner.expr),
        _ => false,
    }
}

fn parse_step(
    expr: &Expr,
    kind: JobKind,
    params: &[String],
    aliases: &[(String, String)],
    bindings: &[(String, usize)],
    deps: &mut Vec<String>,
) -> Result<Step, String> {
    let peeled = peel_call(expr);
    match peeled {
        Expr::Call(call) => {
            let func = call_ident(&call.func)?;
            let mut inputs = Vec::new();
            for arg in &call.args {
                inputs.push(classify_input(
                    arg, kind, params, aliases, bindings, deps,
                )?);
            }
            Ok(Step { func, inputs })
        }
        Expr::MethodCall(call) => {
            if call.method == "get"
                || call.method == "clone"
                || call.method == "to_string"
                || call.method == "to_owned"
            {
                return Err(unsupported("method", &call.method.to_string()));
            }
            let func = call.method.to_string();
            let mut inputs = vec![classify_input(
                &call.receiver,
                kind,
                params,
                aliases,
                bindings,
                deps,
            )?];
            for arg in &call.args {
                inputs.push(classify_input(
                    arg, kind, params, aliases, bindings, deps,
                )?);
            }
            Ok(Step { func, inputs })
        }
        Expr::If(_) => Err(unsupported("if", "")),
        Expr::Match(_) => Err(unsupported("match", "")),
        Expr::While(_) => Err(unsupported("while", "")),
        Expr::Loop(_) => Err(unsupported("loop", "")),
        Expr::ForLoop(_) => Err(unsupported("for", "")),
        Expr::Binary(_) => Err(unsupported("arithmetic", "")),
        Expr::Closure(_) => Err(unsupported("nested closure", "")),
        other => Err(unsupported(&shape_name(other), &snippet(other))),
    }
}

fn peel_call(expr: &Expr) -> &Expr {
    match expr {
        Expr::Try(inner) => peel_call(&inner.expr),
        Expr::Await(inner) => peel_call(&inner.base),
        Expr::Reference(inner) => peel_call(&inner.expr),
        Expr::Paren(inner) => peel_call(&inner.expr),
        other => other,
    }
}

fn unsupported(shape: &str, detail: &str) -> String {
    let extra = if detail.is_empty() {
        String::new()
    } else {
        format!(" ({detail})")
    };
    format!(
        "unsupported job shape: {shape}{extra}. A job may call registered functions, method calls, and `let name = call(...).await?` chains. The async body stays in Rust."
    )
}

fn shape_name(expr: &Expr) -> String {
    match expr {
        Expr::If(_) => "if".into(),
        Expr::Match(_) => "match".into(),
        Expr::Binary(_) => "arithmetic".into(),
        Expr::Field(_) => "field".into(),
        Expr::Index(_) => "index".into(),
        Expr::Return(_) => "return".into(),
        Expr::Assign(_) => "assign".into(),
        other => snippet(other),
    }
}

fn classify_input(
    expr: &Expr,
    kind: JobKind,
    params: &[String],
    aliases: &[(String, String)],
    bindings: &[(String, usize)],
    deps: &mut Vec<String>,
) -> Result<JobInput, String> {
    let expr = match expr {
        Expr::Reference(refer) => &*refer.expr,
        Expr::Paren(paren) => &*paren.expr,
        other => other,
    };
    if let Some(text) = lit_string(expr) {
        return Ok(JobInput::Lit(text));
    }
    if let Expr::Lit(lit) = expr {
        if let syn::Lit::Int(value) = &lit.lit {
            return Ok(JobInput::Lit(value.base10_digits().to_string()));
        }
        if let syn::Lit::Bool(value) = &lit.lit {
            return Ok(JobInput::Lit(value.value.to_string()));
        }
    }
    if let Expr::Struct(item) = expr {
        if item.rest.is_some() {
            return Err(unsupported("struct update", ""));
        }
        let mut fields = Vec::new();
        for field in &item.fields {
            let syn::Member::Named(name) = &field.member else {
                return Err(unsupported("tuple struct", ""));
            };
            fields.push((
                name.to_string(),
                classify_input(&field.expr, kind, params, aliases, bindings, deps)?,
            ));
        }
        return Ok(JobInput::Obj(fields));
    }
    if let Expr::MethodCall(call) = expr {
        if call.method == "get" {
            let name = path_ident(&call.receiver)?;
            if kind != JobKind::Action && !deps.iter().any(|dep| dep == &name) {
                deps.push(name.clone());
            }
            return Ok(JobInput::Signal(name));
        }
        if call.method == "clone" || call.method == "to_string" || call.method == "to_owned" {
            return classify_input(&call.receiver, kind, params, aliases, bindings, deps);
        }
    }
    if let Ok(name) = path_ident(expr) {
        if let Some((_, index)) = bindings.iter().rev().find(|(bind, _)| bind == &name) {
            return Ok(JobInput::Step(*index));
        }
        let resolved = if let Some((_, source)) = aliases.iter().rev().find(|(alias, _)| alias == &name)
        {
            source.clone()
        } else {
            name.clone()
        };
        if let Some((_, index)) = bindings.iter().rev().find(|(bind, _)| bind == &resolved) {
            return Ok(JobInput::Step(*index));
        }
        if params.iter().any(|param| param == &resolved || param == &name)
            || aliases.iter().any(|(alias, _)| alias == &name)
        {
            return Ok(JobInput::Param(resolved));
        }
    }
    Err(format!(
        "unsupported job shape: argument {}. A job argument is signal.get(), a string, a struct, the closure argument, or a previous let.",
        snippet(expr)
    ))
}

fn lower_steps(
    steps: &[Step],
    kind: JobKind,
    params: &[String],
    deps: &[String],
) -> Result<(String, ArgSrc, String), String> {
    if steps.len() == 1 {
        let arg = single_arg(&steps[0], kind, params, deps)?;
        return Ok((steps[0].func.clone(), arg, String::new()));
    }
    let mut pack = Vec::new();
    let mut pieces = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        let mut specs = Vec::new();
        flatten_specs(&step.inputs, index, kind, params, deps, &mut pack, &mut specs)?;
        if specs.is_empty() {
            pieces.push(step.func.clone());
        } else {
            pieces.push(format!("{}@{}", step.func, specs.join(",")));
        }
    }
    let func = format!(
        "chain:{}",
        steps
            .iter()
            .map(|step| step.func.as_str())
            .collect::<Vec<_>>()
            .join("+")
    );
    let arg = if pack.is_empty() {
        ArgSrc::None
    } else {
        ArgSrc::List(pack)
    };
    Ok((func, arg, pieces.join(";")))
}

fn flatten_specs(
    inputs: &[JobInput],
    step_index: usize,
    kind: JobKind,
    params: &[String],
    deps: &[String],
    pack: &mut Vec<PackItem>,
    specs: &mut Vec<String>,
) -> Result<(), String> {
    for input in inputs {
        match input {
            JobInput::Step(index) => {
                if *index + 1 == step_index {
                    specs.push("prev".into());
                } else {
                    specs.push(format!("s{index}"));
                }
            }
            JobInput::Obj(_) => {
                return Err(unsupported(
                    "struct literal inside a chain",
                    "pass the struct as the only argument of one function, or dispatch it",
                ));
            }
            other => {
                let item = input_item(other, kind, params, deps)?;
                let slot = if let Some(found) = pack.iter().position(|have| have == &item) {
                    found
                } else {
                    pack.push(item);
                    pack.len() - 1
                };
                specs.push(slot.to_string());
            }
        }
    }
    Ok(())
}

fn single_arg(
    step: &Step,
    kind: JobKind,
    params: &[String],
    deps: &[String],
) -> Result<ArgSrc, String> {
    match step.inputs.as_slice() {
        [] => Ok(ArgSrc::None),
        [JobInput::Lit(text)] => Ok(ArgSrc::Lit(text.clone())),
        [JobInput::Signal(name)] => Ok(ArgSrc::Signal(name.clone())),
        [JobInput::Param(name)] => Ok(param_arg(name, params, deps, kind)?),
        [JobInput::Obj(fields)] => {
            let mut out = Vec::new();
            for (key, value) in fields {
                out.push((key.clone(), input_item(value, kind, params, deps)?));
            }
            Ok(ArgSrc::Obj(out))
        }
        [JobInput::Step(_)] => Err(unsupported("step", "a single call has no previous step")),
        many => {
            let mut items = Vec::new();
            for input in many {
                items.push(input_item(input, kind, params, deps)?);
            }
            Ok(ArgSrc::List(items))
        }
    }
}

fn param_arg(
    name: &str,
    params: &[String],
    _deps: &[String],
    kind: JobKind,
) -> Result<ArgSrc, String> {
    if params.len() == 1 {
        return match kind {
            JobKind::Action => Ok(ArgSrc::Dispatch),
            JobKind::Resource => Ok(ArgSrc::Source),
            JobKind::Local => Err(
                "a LocalResource argument is signal.get(), not the closure parameter".into(),
            ),
        };
    }
    Err(format!(
        "internal: parameter {name} should have been packed with the other arguments"
    ))
}

fn input_item(
    input: &JobInput,
    kind: JobKind,
    params: &[String],
    deps: &[String],
) -> Result<PackItem, String> {
    match input {
        JobInput::Lit(text) => Ok(PackItem::Lit(text.clone())),
        JobInput::Signal(name) => Ok(PackItem::Signal(name.clone())),
        JobInput::Param(name) => param_item(name, params, deps, kind),
        JobInput::Obj(_) => Err(unsupported("struct literal", "use one struct argument")),
        JobInput::Step(_) => Err(unsupported("step", "a packed argument is not a previous step")),
    }
}

fn param_item(
    name: &str,
    params: &[String],
    deps: &[String],
    kind: JobKind,
) -> Result<PackItem, String> {
    if deps.iter().any(|dep| dep == name) {
        return Ok(PackItem::Signal(name.to_string()));
    }
    if let Some(index) = params.iter().position(|param| param == name) {
        if params.len() == 1 {
            return match kind {
                JobKind::Action => Ok(PackItem::Dispatch),
                JobKind::Resource => Ok(PackItem::Source),
                JobKind::Local => Err(
                    "a LocalResource argument is signal.get(), not the closure parameter".into(),
                ),
            };
        }
        if let Some(dep) = deps.get(index) {
            return Ok(PackItem::Signal(dep.clone()));
        }
    }
    Err(format!(
        "unsupported job shape: argument {name}. It is not a signal read or a closure parameter."
    ))
}

fn record_job_alias(
    local: &syn::Local,
    params: &[String],
    aliases: &mut Vec<(String, String)>,
) -> Result<(), String> {
    let syn::Pat::Ident(ident) = &local.pat else {
        return Err("unsupported job shape: let pattern. A job let is a name.".into());
    };
    let Some(init) = &local.init else {
        return Err(format!("let {} needs a value", ident.ident));
    };
    let source = alias_source(&init.expr)?;
    let resolved = resolve_job_name(&source, params, aliases)?;
    aliases.push((ident.ident.to_string(), resolved));
    Ok(())
}

fn alias_source(expr: &Expr) -> Result<String, String> {
    match expr {
        Expr::Path(path) => path
            .path
            .get_ident()
            .map(|ident| ident.to_string())
            .ok_or_else(|| "a job let must copy the closure argument".into()),
        Expr::Reference(refer) => alias_source(&refer.expr),
        Expr::MethodCall(call)
            if call.method == "clone" || call.method == "to_string" || call.method == "to_owned" =>
        {
            path_ident(&call.receiver)
        }
        _ => Err(
            "unsupported job shape: let. A job let copies the closure argument or is `let name = call().await?`."
                .into(),
        ),
    }
}

fn resolve_job_name(
    name: &str,
    params: &[String],
    aliases: &[(String, String)],
) -> Result<String, String> {
    if let Some((_, source)) = aliases.iter().rev().find(|(alias, _)| alias == name) {
        return Ok(source.clone());
    }
    if params.iter().any(|param| param == name) {
        return Ok(name.to_string());
    }
    Err(format!("job let {name} is not the closure argument"))
}

fn call_ident(expr: &Expr) -> Result<String, String> {
    let Expr::Path(path) = expr else {
        return Err("a job calls a function by name".into());
    };
    path.path
        .get_ident()
        .map(|ident| ident.to_string())
        .ok_or_else(|| "a job calls a function by its registered name".into())
}

fn each_source(expr: &Expr, signals: &[Signal], jobs: &[String]) -> Result<String, String> {
    let expr = match expr {
        Expr::Closure(closure) => &*closure.body,
        other => other,
    };
    let Expr::MethodCall(call) = expr else {
        return Err("<For each> needs signal.get() or resource.get()".into());
    };
    if call.method != "get" {
        return Err("<For each> needs .get()".into());
    }
    let name = path_ident(&call.receiver)?;
    if jobs.iter().any(|job| job == &name) {
        return Ok(format!("{name}_value"));
    }
    if signals.iter().any(|signal| signal.read == name) {
        return Ok(name);
    }
    Err(format!("unknown each source {name}"))
}

fn jobs_in(node: &ViewNode, known: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    fn walk(node: &ViewNode, known: &[String], found: &mut Vec<String>) {
        let consider = |part: &Part, found: &mut Vec<String>, known: &[String]| {
            if let Part::Job(name) = part {
                if known.iter().any(|job| job == name) && !found.iter().any(|have| have == name) {
                    found.push(name.clone());
                }
            }
        };
        match node {
            ViewNode::El(el) => {
                for part in &el.parts {
                    consider(part, found, known);
                }
                for child in &el.children {
                    walk(child, known, found);
                }
            }
            ViewNode::Show { child, .. } => walk(child, known, found),
            ViewNode::For { items, child, .. } => {
                if let Some(name) = items.strip_suffix("_value") {
                    if known.iter().any(|job| job == name)
                        && !found.iter().any(|have| have == name)
                    {
                        found.push(name.to_string());
                    }
                }
                for part in &child.parts {
                    consider(part, found, known);
                }
                for node in &child.children {
                    walk(node, known, found);
                }
            }
            ViewNode::Slot { nodes, .. } => {
                for child in nodes {
                    walk(child, known, found);
                }
            }
            ViewNode::Routes { routes } => {
                for (_, nodes) in routes {
                    for child in nodes {
                        walk(child, known, found);
                    }
                }
            }
            ViewNode::Suspense { fallback, child, .. } | ViewNode::Boundary { fallback, child, .. } => {
                for node in fallback {
                    walk(node, known, found);
                }
                walk(child, known, found);
            }
            ViewNode::Portal { nodes } => {
                for child in nodes {
                    walk(child, known, found);
                }
            }
        }
    }
    walk(node, known, &mut found);
    found
}

fn try_signal(local: &syn::Local) -> Result<Option<Signal>, String> {
    let syn::Pat::Tuple(tuple) = &local.pat else {
        return Ok(None);
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
    let Some(arg) = call.args.first() else {
        return Err(format!("signal {read} needs an initial value"));
    };
    let init = parse_init(arg)?;
    Ok(Some(Signal { read, write, init }))
}

fn try_alias(local: &syn::Local) -> Result<Option<(String, String, String)>, String> {
    let syn::Pat::Ident(ident) = &local.pat else {
        return Ok(None);
    };
    let Some(init) = &local.init else {
        return Ok(None);
    };
    let Expr::Field(field) = &*init.expr else {
        return Err(format!("unsupported binding {}", ident.ident));
    };
    let Expr::Path(path) = &*field.base else {
        return Err("alias must be param.field".into());
    };
    let base = path
        .path
        .get_ident()
        .ok_or("alias must be param.field")?
        .to_string();
    let member = match &field.member {
        syn::Member::Named(name) => name.to_string(),
        syn::Member::Unnamed(_) => return Err("tuple fields are not lowered".into()),
    };
    Ok(Some((ident.ident.to_string(), base, member)))
}

fn parse_init(expr: &Expr) -> Result<Init, String> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Int(value) => Ok(Init::Int(
                value.base10_parse::<i64>().map_err(|err| err.to_string())?,
            )),
            syn::Lit::Bool(value) => Ok(Init::Bool(value.value)),
            syn::Lit::Str(value) => Ok(Init::Str(value.value())),
            _ => Err("unsupported signal literal".into()),
        },
        Expr::Call(call) if path_ends(&call.func, &["String", "new"]) && call.args.is_empty() => {
            Ok(Init::Str(String::new()))
        }
        Expr::MethodCall(call) if call.method == "into" || call.method == "to_string" => {
            if let Some(text) = lit_string(&call.receiver) {
                Ok(Init::Str(text))
            } else {
                Err("signal string must be a literal".into())
            }
        }
        Expr::Struct(item) => {
            let mut fields = Vec::new();
            for field in &item.fields {
                let syn::Member::Named(name) = &field.member else {
                    return Err("tuple fields are not lowered".into());
                };
                fields.push((name.to_string(), parse_init(&field.expr)?));
            }
            Ok(Init::Obj(fields))
        }
        Expr::Macro(mac)
            if mac
                .mac
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "vec") =>
        {
            let parsed = Punctuated::<Expr, syn::Token![,]>::parse_terminated
                .parse2(mac.mac.tokens.clone())
                .map_err(|err| err.to_string())?;
            let mut items = Vec::new();
            for expr in parsed {
                items.push(parse_item(&expr)?);
            }
            Ok(Init::Items(items))
        }
        other => Err(format!("unsupported signal init {}", snippet(other))),
    }
}

fn parse_item(expr: &Expr) -> Result<ItemSeed, String> {
    let Expr::Struct(item) = expr else {
        return Err("list items must be struct literals".into());
    };
    let mut id = None;
    let mut label = None;
    for field in &item.fields {
        let syn::Member::Named(name) = &field.member else {
            continue;
        };
        match name.to_string().as_str() {
            "id" => {
                let Expr::Lit(lit) = &field.expr else {
                    return Err("item id must be an integer".into());
                };
                let syn::Lit::Int(value) = &lit.lit else {
                    return Err("item id must be an integer".into());
                };
                id = Some(value.base10_parse::<i64>().map_err(|err| err.to_string())?);
            }
            "label" => {
                label = Some(stringish(&field.expr)?);
            }
            _ => {}
        }
    }
    Ok(ItemSeed {
        id: id.ok_or("item needs id")?,
        label: label.ok_or("item needs label")?,
    })
}

fn parse_interp(
    block: &NodeBlock,
    signals: &[Signal],
    in_row: bool,
    props: &HashMap<String, PropVal>,
    ctx: &HashMap<String, String>,
    in_children_fn: bool,
    jobs: &[String],
    row_var: Option<&str>,
) -> Result<Part, String> {
    let NodeBlock::ValidBlock(block) = block else {
        return Err("invalid interpolation block".into());
    };
    if block.stmts.len() != 1 {
        return Err("interpolation must be one expression".into());
    }
    let Stmt::Expr(expr, None) = &block.stmts[0] else {
        return Err("interpolation must be one expression".into());
    };
    if let Expr::Path(path) = expr {
        let name = path
            .path
            .get_ident()
            .ok_or("interpolation path must be a local")?
            .to_string();
        if let Some(part) = prop_part(props, &name) {
            return Ok(part);
        }
        if let Some(key) = ctx.get(&name) {
            return Ok(Part::Ctx(key.clone()));
        }
        if in_children_fn && name == "arg" {
            return Ok(Part::Param("arg".into()));
        }
        if signals.iter().any(|signal| signal.read == name) {
            return Ok(Part::Sig(name));
        }
        if jobs.iter().any(|job| job == &name) {
            return Ok(Part::Job(name));
        }
        if in_row && signals.iter().all(|signal| signal.read != name) {
            return Ok(Part::Param(name));
        }
    }
    if let Some(path) = signal_field_path(expr) {
        if let Some((base, rest)) = path.split_once('.') {
            if in_row && row_var == Some(base) {
                return Ok(Part::Param(rest.to_string()));
            }
            if signals.iter().any(|signal| signal.read == base) || props.contains_key(base) {
                return Ok(Part::Sig(path));
            }
            if let Some(key) = ctx.get(base) {
                return Ok(Part::Ctx(format!("{key}.{rest}")));
            }
        }
    }
    let peeled = match expr {
        Expr::Closure(closure) => &*closure.body,
        other => other,
    };
    if is_cancelled_count(expr) || is_cancelled_count(peeled) {
        return Ok(Part::Sig("quinn_cancelled".into()));
    }
    if let Expr::MethodCall(call) = peeled {
        if call.method == "get" || call.method == "value" {
            let name = path_ident(&call.receiver)?;
            if jobs.iter().any(|job| job == &name) {
                return Ok(Part::Job(name));
            }
            if call.method == "get" && signals.iter().any(|signal| signal.read == name) {
                return Ok(Part::Sig(name));
            }
            return Err(format!("unknown signal {name}"));
        }
        if call.method == "version" {
            let name = path_ident(&call.receiver)?;
            if jobs.iter().any(|job| job == &name) {
                return Ok(Part::Sig(format!("{name}_gen")));
            }
            return Err(format!("unknown job {name}"));
        }
        if call.method == "pending" {
            let name = path_ident(&call.receiver)?;
            return Ok(Part::Sig(format!("{name}_pending")));
        }
    }
    if let Expr::If(expr) = peeled {
        return parse_if_text(expr, signals);
    }
    Err(format!("unsupported interpolation {}", snippet(peeled)))
}

fn is_cancelled_count(expr: &Expr) -> bool {
    let Expr::Call(call) = expr else {
        return false;
    };
    let Expr::Path(path) = &*call.func else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "cancelled_count")
}

fn parse_if_text(expr: &syn::ExprIf, signals: &[Signal]) -> Result<Part, String> {
    if expr.else_branch.is_none() {
        return Err("text if needs an else".into());
    }
    let (_, else_expr) = expr.else_branch.as_ref().unwrap();
    if is_empty_check(&expr.cond) {
        let name = empty_signal(&expr.cond)?;
        if !signals.iter().any(|signal| signal.read == name) {
            return Err(format!("unknown signal {name}"));
        }
        let empty = stringish(&block_expr(&expr.then_branch)?)?;
        let _else = else_expr;
        return Ok(Part::IfEmpty {
            signal: name,
            empty,
        });
    }
    let name = signal_get(&expr.cond, signals)?;
    let yes = stringish(&block_expr(&expr.then_branch)?)?;
    let no = match &**else_expr {
        Expr::Block(block) => stringish(&block_expr(&block.block)?)?,
        other => stringish(other)?,
    };
    Ok(Part::IfBool {
        signal: name,
        yes,
        no,
    })
}

fn is_empty_check(expr: &Expr) -> bool {
    let Expr::MethodCall(call) = expr else {
        return false;
    };
    call.method == "is_empty"
}

fn empty_signal(expr: &Expr) -> Result<String, String> {
    let Expr::MethodCall(call) = expr else {
        return Err("expected is_empty".into());
    };
    let Expr::MethodCall(inner) = &*call.receiver else {
        return Err("is_empty receiver must be signal.get()".into());
    };
    if inner.method != "get" {
        return Err("is_empty receiver must be signal.get()".into());
    }
    path_ident(&inner.receiver)
}

fn block_expr(block: &syn::Block) -> Result<Expr, String> {
    if block.stmts.len() != 1 {
        return Err("branch must be one expression".into());
    }
    let Stmt::Expr(expr, _) = &block.stmts[0] else {
        return Err("branch must be one expression".into());
    };
    Ok(expr.clone())
}

fn signal_get(expr: &Expr, signals: &[Signal]) -> Result<String, String> {
    let expr = match expr {
        Expr::Closure(closure) => &*closure.body,
        other => other,
    };
    let Expr::MethodCall(call) = expr else {
        return Err(format!("expected signal.get() in {}", snippet(expr)));
    };
    if call.method != "get" {
        return Err("expected .get()".into());
    }
    let name = path_ident(&call.receiver)?;
    if !signals.iter().any(|signal| signal.read == name) {
        return Err(format!("unknown signal {name}"));
    }
    Ok(name)
}

fn key_field_of(expr: &Expr) -> Option<(String, String)> {
    let Expr::Closure(closure) = expr else {
        return None;
    };
    let Expr::Field(field) = &*closure.body else {
        return None;
    };
    let Expr::Path(path) = &*field.base else {
        return None;
    };
    let var = path.path.get_ident()?.to_string();
    let syn::Member::Named(member) = &field.member else {
        return None;
    };
    Some((var, member.to_string()))
}

fn stringish(expr: &Expr) -> Result<String, String> {
    if let Some(text) = lit_string(expr) {
        return Ok(text);
    }
    if let Expr::MethodCall(call) = expr {
        if call.method == "into" || call.method == "to_string" {
            if let Some(text) = lit_string(&call.receiver) {
                return Ok(text);
            }
        }
    }
    Err("item label must be a string".into())
}

fn parse_frame(expr: &Expr, signals: &[Signal]) -> Result<String, String> {
    let Expr::Call(call) = expr else {
        return Err("set_frame_driver must be a call".into());
    };
    let Some(arg) = call.args.first() else {
        return Err("set_frame_driver needs a closure".into());
    };
    if !matches!(arg, Expr::Closure(_)) {
        return Err("set_frame_driver needs a closure".into());
    }
    let act = crate::handler::parse_handler(arg, signals, false)?;
    let Act::Body(stmts) = act else {
        return Err("the frame driver must increment a signal".into());
    };
    if stmts.len() == 1 {
        if let HandlerStmt::Set {
            signal,
            expr: HxExpr::Binary(BinOp::Add, _, right),
        } = &stmts[0]
        {
            if matches!(right.as_ref(), HxExpr::Int(1)) {
                return Ok(signal.clone());
            }
        }
    }
    Err("the frame driver must increment a signal".into())
}

fn bool_signal(expr: &Expr, signals: &[Signal]) -> Result<String, String> {
    let expr = match expr {
        Expr::Closure(closure) => &*closure.body,
        other => other,
    };
    if let Expr::MethodCall(call) = expr {
        if call.method == "pending" {
            let name = path_ident(&call.receiver)?;
            return Ok(format!("{name}_pending"));
        }
    }
    if let Ok(name) = signal_get(expr, signals) {
        return Ok(name);
    }
    if let Expr::Path(path) = expr {
        if let Some(ident) = path.path.get_ident() {
            let name = ident.to_string();
            if signals.iter().any(|signal| signal.read == name) {
                return Ok(name);
            }
        }
    }
    Err("class: value must be a bool signal, written as move || flag.get()".into())
}

fn style_branches(expr: &Expr, signals: &[Signal]) -> Result<(String, String, String), String> {
    let expr = match expr {
        Expr::Closure(closure) => &*closure.body,
        other => other,
    };
    let Expr::If(iff) = expr else {
        return Err(
            "style:prop must be `if signal.get() { \"value\" } else { \"value\" }`, not a runtime string"
                .into(),
        );
    };
    if iff.else_branch.is_none() {
        return Err("style:prop needs an else branch with a literal".into());
    }
    let signal = signal_get(&iff.cond, signals)?;
    let literal = |expr: &Expr| {
        stringish(expr).map_err(|_| {
            "style:prop branches must be string literals, not a runtime string".to_string()
        })
    };
    let yes = literal(&block_expr(&iff.then_branch)?)?;
    let (_, else_expr) = iff.else_branch.as_ref().unwrap();
    let no = match &**else_expr {
        Expr::Block(block) => literal(&block_expr(&block.block)?)?,
        other => literal(other)?,
    };
    Ok((signal, yes, no))
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
        if attr_key(attr) == "class" {
            return attr
                .value_literal_string()
                .ok_or_else(|| "class must be a string literal".into());
        }
    }
    Ok(String::new())
}

fn attr_key(attr: &rstml::node::KeyedAttribute) -> String {
    attr.key.to_string().replace(' ', "")
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
    path_ends(expr, &[name])
}

fn path_ends(expr: &Expr, tail: &[&str]) -> bool {
    let Expr::Path(path) = expr else {
        return false;
    };
    let segs: Vec<String> = path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    segs.len() >= tail.len()
        && segs[segs.len() - tail.len()..] == tail.iter().map(|s| s.to_string()).collect::<Vec<_>>()
}

fn path_ident(expr: &Expr) -> Result<String, String> {
    let Expr::Path(path) = expr else {
        return Err("expected an identifier".into());
    };
    path.path
        .get_ident()
        .map(|ident| ident.to_string())
        .ok_or_else(|| "expected an identifier".into())
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

fn lit_string(expr: &Expr) -> Option<String> {
    let Expr::Lit(lit) = expr else {
        return None;
    };
    let syn::Lit::Str(value) = &lit.lit else {
        return None;
    };
    Some(value.value())
}

fn act_reset_names(act: Option<&Act>) -> Vec<String> {
    let Some(act) = act else {
        return Vec::new();
    };
    match act {
        Act::Inc(name) | Act::Toggle(name) | Act::Clear(name) => vec![name.clone()],
        Act::Body(stmts) | Act::Row(stmts) => stmts
            .iter()
            .filter_map(|stmt| match stmt {
                HandlerStmt::Set { signal, .. } => Some(signal.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn widget_base(tag: &str, parts: &[Part], click: Option<&Act>) -> String {
    if act_is_toggle(click) {
        return "toggle".into();
    }
    if act_is_clear(click) {
        return "clear".into();
    }
    if tag == "button" {
        if let [Part::Lit(text)] = parts {
            let slug = slug(text);
            if !slug.is_empty() {
                return slug;
            }
        }
    }
    if tag == "input" {
        return "field".into();
    }
    tag.to_string()
}

fn act_is_toggle(act: Option<&Act>) -> bool {
    match act {
        Some(Act::Toggle(_)) => true,
        Some(Act::Body(stmts) | Act::Row(stmts)) => matches!(
            stmts.as_slice(),
            [HandlerStmt::Set {
                expr: HxExpr::Not(_),
                ..
            }]
        ),
        _ => false,
    }
}

fn act_is_clear(act: Option<&Act>) -> bool {
    match act {
        Some(Act::Clear(_)) => true,
        Some(Act::Body(stmts) | Act::Row(stmts)) => matches!(
            stmts.as_slice(),
            [HandlerStmt::Set {
                expr: HxExpr::Str(text),
                ..
            }] if text.is_empty()
        ),
        _ => false,
    }
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

fn snippet(expr: &Expr) -> String {
    let text = quote_expr(expr);
    let mut short: String = text.chars().take(80).collect();
    if text.chars().count() > 80 {
        short.push_str("...");
    }
    short
}

fn expr_snippet(expr: &Expr) -> String {
    snippet(expr)
}

fn quote_expr(expr: &Expr) -> String {
    // syn's Display is enough for an error. Avoid pulling quote.
    format!("{expr:?}")
        .chars()
        .filter(|ch| *ch != '\n')
        .take(120)
        .collect()
}

fn _span_hint(expr: &Expr) -> String {
    let _ = expr.span();
    String::new()
}
