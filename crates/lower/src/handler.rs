//! Closed handler bodies. Anything outside this set is a compile error, not
//! a silent drop.

use syn::{Expr, Stmt};

use crate::model::*;

pub fn parse_handler(expr: &Expr, signals: &[Signal], in_row: bool) -> Result<Act, String> {
    let Expr::Closure(closure) = expr else {
        return Err("event handlers must be closures".into());
    };
    let stmts = body_stmts(closure)?;
    if stmts.is_empty() {
        return Err("event handler is empty".into());
    }
    let mut out = Vec::new();
    for stmt in stmts {
        out.extend(lower_stmt(stmt, signals)?);
    }
    if in_row {
        Ok(Act::Row(out))
    } else {
        Ok(Act::Body(out))
    }
}

pub fn parse_input_signal(expr: &Expr, signals: &[Signal]) -> Result<String, String> {
    let act = parse_handler(expr, signals, false)?;
    let Act::Body(stmts) = act else {
        return Err("on:input must set a signal".into());
    };
    if stmts.len() == 1 {
        if let HandlerStmt::Set { signal, expr } = &stmts[0] {
            if matches!(expr, HxExpr::Name(_)) {
                return Ok(signal.clone());
            }
        }
    }
    Err(
        "on:input must be one signal set of the closure argument, such as set_name.set(value)"
            .into(),
    )
}

enum BodyStmt {
    Let(syn::Local),
    Expr(Expr),
}

fn body_stmts(closure: &syn::ExprClosure) -> Result<Vec<BodyStmt>, String> {
    match &*closure.body {
        Expr::Block(block) => {
            let mut out = Vec::new();
            for stmt in &block.block.stmts {
                match stmt {
                    Stmt::Expr(expr, _) => out.push(BodyStmt::Expr(expr.clone())),
                    Stmt::Local(local) => out.push(BodyStmt::Let(local.clone())),
                    _ => {
                        return Err(reject(
                            "a handler statement",
                            "let bindings and signal updates",
                        ))
                    }
                }
            }
            Ok(out)
        }
        other => Ok(vec![BodyStmt::Expr(other.clone())]),
    }
}

fn lower_stmt(stmt: BodyStmt, signals: &[Signal]) -> Result<Vec<HandlerStmt>, String> {
    match stmt {
        BodyStmt::Let(local) => {
            let syn::Pat::Ident(ident) = &local.pat else {
                return Err(reject("that let pattern", "let name = ..."));
            };
            let Some(init) = &local.init else {
                return Err(format!("let {} needs a value", ident.ident));
            };
            Ok(vec![HandlerStmt::Let {
                name: ident.ident.to_string(),
                expr: lower_expr(&init.expr, signals)?,
            }])
        }
        BodyStmt::Expr(expr) => lower_expr_stmt(&expr, signals),
    }
}

fn lower_expr_stmt(expr: &Expr, signals: &[Signal]) -> Result<Vec<HandlerStmt>, String> {
    if let Expr::If(iff) = expr {
        return lower_if(iff, signals);
    }
    if let Expr::Call(call) = expr {
        if path_ends(&call.func, &["navigate"]) {
            let Some(arg) = call.args.first() else {
                return Err("navigate needs a path".into());
            };
            return Ok(vec![HandlerStmt::Navigate(lower_expr(arg, signals)?)]);
        }
        if path_ends(&call.func, &["toggle_theme"]) {
            return Ok(vec![HandlerStmt::ToggleTheme]);
        }
        if path_ends(&call.func, &["set_density"]) {
            let Some(arg) = call.args.first() else {
                return Err("set_density needs 0, 1, or 2".into());
            };
            let Expr::Lit(lit) = arg else {
                return Err("set_density needs an integer".into());
            };
            let syn::Lit::Int(value) = &lit.lit else {
                return Err("set_density needs an integer".into());
            };
            let n = value
                .base10_parse::<i64>()
                .map_err(|err| err.to_string())?;
            return Ok(vec![HandlerStmt::SetDensity(n)]);
        }
    }
    let Expr::MethodCall(call) = expr else {
        return Err(reject_expr(expr));
    };
    if call.method == "refetch" {
        let job = path_ident(&call.receiver)?;
        return Ok(vec![HandlerStmt::Refetch { job }]);
    }
    if call.method == "dispatch" {
        let action = path_ident(&call.receiver)?;
        let arg = match call.args.first() {
            Some(arg) => lower_expr(arg, signals)?,
            None => HxExpr::Str(String::new()),
        };
        return Ok(vec![HandlerStmt::Dispatch { action, arg }]);
    }
    if call.method == "set" {
        let signal = setter(&call.receiver, signals)?;
        let Some(arg) = call.args.first() else {
            return Err(format!("{signal}.set needs a value"));
        };
        return Ok(vec![HandlerStmt::Set {
            signal,
            expr: lower_expr(arg, signals)?,
        }]);
    }
    if call.method == "update" {
        let signal = setter(&call.receiver, signals)?;
        let Some(Expr::Closure(closure)) = call.args.first() else {
            return Err(format!("{signal}.update needs a closure"));
        };
        let kind = signal_kind(&signal, signals);
        return lower_update(&signal, kind, closure, signals);
    }
    Err(reject_expr(expr))
}

fn lower_update(
    signal: &str,
    kind: InitKind,
    closure: &syn::ExprClosure,
    signals: &[Signal],
) -> Result<Vec<HandlerStmt>, String> {
    let stmts = body_stmts(closure)?;
    if stmts.is_empty() {
        return Err(format!("{signal}.update is empty"));
    }
    let mut out = Vec::new();
    for stmt in stmts {
        let BodyStmt::Expr(expr) = stmt else {
            return Err(reject(
                "a let inside update",
                "updates of the signal, or list push/pop/remove/swap/reverse",
            ));
        };
        match kind {
            InitKind::Items => out.push(lower_list_op(signal, &expr)?),
            _ => {
                let param = closure_param(closure);
                let mut stmt = lower_scalar_update(signal, &expr, signals, param.as_deref())?;
                if let Some(param) = &param {
                    if let HandlerStmt::Set { expr, .. } = &mut stmt {
                        rewrite_name(expr, param, signal);
                    }
                }
                out.push(stmt);
            }
        }
    }
    Ok(out)
}

fn closure_param(closure: &syn::ExprClosure) -> Option<String> {
    let pat = closure.inputs.first()?;
    let syn::Pat::Ident(ident) = pat else {
        return None;
    };
    if ident.ident == "_" {
        return None;
    }
    Some(ident.ident.to_string())
}

fn rewrite_name(expr: &mut HxExpr, from: &str, to: &str) {
    match expr {
        HxExpr::Name(name) if name == from => *name = to.to_string(),
        HxExpr::Not(inner) => rewrite_name(inner, from, to),
        HxExpr::Binary(_, left, right) => {
            rewrite_name(left, from, to);
            rewrite_name(right, from, to);
        }
        HxExpr::Concat(parts) | HxExpr::List(parts) => {
            for part in parts {
                rewrite_name(part, from, to);
            }
        }
        HxExpr::Obj(fields) => {
            for (_, value) in fields {
                rewrite_name(value, from, to);
            }
        }
        HxExpr::Call { args, .. } => {
            for arg in args {
                rewrite_name(arg, from, to);
            }
        }
        HxExpr::If { cond, then, els } => {
            rewrite_name(cond, from, to);
            rewrite_name(then, from, to);
            rewrite_name(els, from, to);
        }
        _ => {}
    }
}

fn lower_if(iff: &syn::ExprIf, signals: &[Signal]) -> Result<Vec<HandlerStmt>, String> {
    let cond = lower_expr(&iff.cond, signals)?;
    let then_branch = lower_block_stmts(&iff.then_branch, signals)?;
    let else_branch = match iff.else_branch.as_ref().map(|(_, expr)| expr.as_ref()) {
        None => Vec::new(),
        Some(Expr::Block(block)) => lower_block_stmts(&block.block, signals)?,
        Some(Expr::If(nested)) => lower_if(nested, signals)?,
        Some(_) => {
            return Err(reject(
                "that else branch",
                "else { ... } or else if, with signal updates inside",
            ))
        }
    };
    Ok(vec![HandlerStmt::If {
        cond,
        then_branch,
        else_branch,
    }])
}

fn block_value(block: &syn::Block) -> Result<Expr, String> {
    if block.stmts.len() == 1 {
        if let syn::Stmt::Expr(expr, None) = &block.stmts[0] {
            return Ok(expr.clone());
        }
    }
    Err("if used as a value needs one expression in each branch".into())
}

fn lower_block_stmts(block: &syn::Block, signals: &[Signal]) -> Result<Vec<HandlerStmt>, String> {
    let mut out = Vec::new();
    for stmt in &block.stmts {
        let body = match stmt {
            syn::Stmt::Expr(expr, _) => BodyStmt::Expr(expr.clone()),
            syn::Stmt::Local(local) => BodyStmt::Let(local.clone()),
            _ => {
                return Err(reject(
                    "a handler statement",
                    "let bindings, signal updates, and if / else",
                ))
            }
        };
        out.extend(lower_stmt(body, signals)?);
    }
    Ok(out)
}

fn lower_scalar_update(
    signal: &str,
    expr: &Expr,
    signals: &[Signal],
    param: Option<&str>,
) -> Result<HandlerStmt, String> {
    match expr {
        Expr::Binary(binary) => {
            let op = match binary.op {
                syn::BinOp::AddAssign(_) => BinOp::Add,
                syn::BinOp::SubAssign(_) => BinOp::Sub,
                syn::BinOp::MulAssign(_) => BinOp::Mul,
                syn::BinOp::DivAssign(_) => BinOp::Div,
                _ => return Err(reject_expr(expr)),
            };
            let right = lower_expr(&binary.right, signals)?;
            Ok(HandlerStmt::Set {
                signal: signal.to_string(),
                expr: HxExpr::Binary(
                    op,
                    Box::new(HxExpr::Name(signal.to_string())),
                    Box::new(right),
                ),
            })
        }
        Expr::Assign(assign) => {
            let expr = lower_expr(&assign.right, signals)?;
            let path = param
                .and_then(|param| field_signal(&assign.left, param, signal))
                .unwrap_or_else(|| signal.to_string());
            Ok(HandlerStmt::Set { signal: path, expr })
        }
        other => Err(reject_expr(other)),
    }
}

fn field_signal(expr: &Expr, param: &str, signal: &str) -> Option<String> {
    match expr {
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
            field_signal(&unary.expr, param, signal)
        }
        Expr::Field(field) => {
            let member = match &field.member {
                syn::Member::Named(name) => name.to_string(),
                syn::Member::Unnamed(_) => return None,
            };
            let base = field_signal(&field.base, param, signal)?;
            Some(format!("{base}.{member}"))
        }
        Expr::Path(path) => {
            let name = path.path.get_ident()?.to_string();
            if name == param {
                Some(signal.to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn lower_list_op(signal: &str, expr: &Expr) -> Result<HandlerStmt, String> {
    let Expr::MethodCall(call) = expr else {
        return Err(reject_expr(expr));
    };
    let method = call.method.to_string();
    match method.as_str() {
        "push" => {
            let Some(arg) = call.args.first() else {
                return Err("push needs an item".into());
            };
            let (id, label) = item_fields(arg)?;
            Ok(HandlerStmt::Push {
                items: signal.to_string(),
                id,
                label,
            })
        }
        "pop" => Ok(HandlerStmt::Pop {
            items: signal.to_string(),
        }),
        "reverse" => Ok(HandlerStmt::Reverse {
            items: signal.to_string(),
        }),
        "swap" | "swap_by_key" | "swap_key" => {
            let (a, b) = two_args(&call.args, "swap")?;
            Ok(HandlerStmt::SwapKey {
                items: signal.to_string(),
                a,
                b,
            })
        }
        "remove" | "remove_by_key" | "remove_key" => {
            let Some(arg) = call.args.first() else {
                return Err("remove needs the item id".into());
            };
            Ok(HandlerStmt::RemoveKey {
                items: signal.to_string(),
                key: lower_expr(arg, &[])?,
            })
        }
        "retain" => {
            let Some(Expr::Closure(closure)) = call.args.first() else {
                return Err(reject("retain", "rows.retain(|row| row.id != <id>)"));
            };
            let key = retain_key(closure)?;
            Ok(HandlerStmt::RemoveKey {
                items: signal.to_string(),
                key,
            })
        }
        other => Err(reject(
            &format!("list.{other}()"),
            "push, pop, reverse, swap(id, id), remove(id), or retain(|row| row.id != id)",
        )),
    }
}

fn two_args(
    args: &syn::punctuated::Punctuated<Expr, syn::Token![,]>,
    what: &str,
) -> Result<(HxExpr, HxExpr), String> {
    let mut iter = args.iter();
    let Some(a) = iter.next() else {
        return Err(format!("{what} needs two item ids"));
    };
    let Some(b) = iter.next() else {
        return Err(format!("{what} needs two item ids"));
    };
    Ok((lower_expr(a, &[])?, lower_expr(b, &[])?))
}

fn retain_key(closure: &syn::ExprClosure) -> Result<HxExpr, String> {
    let expr = match &*closure.body {
        Expr::Block(block) => {
            if block.block.stmts.len() != 1 {
                return Err(reject("that retain body", "row.id != <id>"));
            }
            let Stmt::Expr(expr, _) = &block.block.stmts[0] else {
                return Err(reject("that retain body", "row.id != <id>"));
            };
            expr.clone()
        }
        other => other.clone(),
    };
    let Expr::Binary(binary) = expr else {
        return Err(reject("that retain test", "|row| row.id != <id>"));
    };
    if !matches!(binary.op, syn::BinOp::Ne(_)) {
        return Err(reject("that retain test", "|row| row.id != <id>"));
    }
    let id_field = match &*binary.left {
        Expr::Field(field) => field_is_id(field),
        _ => false,
    };
    if !id_field {
        return Err(reject("that retain test", "|row| row.id != <id>"));
    }
    lower_expr(&binary.right, &[])
}

fn field_is_id(field: &syn::ExprField) -> bool {
    matches!(&field.member, syn::Member::Named(name) if name == "id")
}

fn item_fields(expr: &Expr) -> Result<(HxExpr, HxExpr), String> {
    let Expr::Struct(item) = expr else {
        return Err(reject_expr(expr));
    };
    let mut id = None;
    let mut label = None;
    for field in &item.fields {
        let syn::Member::Named(name) = &field.member else {
            continue;
        };
        match name.to_string().as_str() {
            "id" => id = Some(lower_expr(&field.expr, &[])?),
            "label" => label = Some(lower_expr(&field.expr, &[])?),
            _ => {}
        }
    }
    Ok((
        id.ok_or("pushed item needs id")?,
        label.ok_or("pushed item needs label")?,
    ))
}

fn lower_expr(expr: &Expr, signals: &[Signal]) -> Result<HxExpr, String> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Int(value) => Ok(HxExpr::Int(
                value.base10_parse::<i64>().map_err(|err| err.to_string())?,
            )),
            syn::Lit::Bool(value) => Ok(HxExpr::Bool(value.value)),
            syn::Lit::Str(value) => Ok(HxExpr::Str(value.value())),
            _ => Err(reject_expr(expr)),
        },
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Not(_)) => {
            Ok(HxExpr::Not(Box::new(lower_expr(&unary.expr, signals)?)))
        }
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
            lower_expr(&unary.expr, signals)
        }
        Expr::Binary(binary) => {
            let op = match binary.op {
                syn::BinOp::Add(_) => BinOp::Add,
                syn::BinOp::Sub(_) => BinOp::Sub,
                syn::BinOp::Mul(_) => BinOp::Mul,
                syn::BinOp::Div(_) => BinOp::Div,
                syn::BinOp::Eq(_) => BinOp::Eq,
                syn::BinOp::Ne(_) => BinOp::Ne,
                syn::BinOp::Lt(_) => BinOp::Lt,
                syn::BinOp::Gt(_) => BinOp::Gt,
                syn::BinOp::Le(_) => BinOp::Le,
                syn::BinOp::Ge(_) => BinOp::Ge,
                _ => return Err(reject_expr(expr)),
            };
            Ok(HxExpr::Binary(
                op,
                Box::new(lower_expr(&binary.left, signals)?),
                Box::new(lower_expr(&binary.right, signals)?),
            ))
        }
        Expr::Path(path) => {
            let name = path
                .path
                .get_ident()
                .ok_or_else(|| reject_expr(expr))?
                .to_string();
            Ok(HxExpr::Name(name))
        }
        Expr::MethodCall(call) if call.method == "get" && call.args.is_empty() => {
            let name = path_ident(&call.receiver)?;
            if !signals.is_empty()
                && signals
                    .iter()
                    .all(|signal| signal.read != name && signal.write != name)
            {
                return Err(format!("unknown signal {name}"));
            }
            let read = signals
                .iter()
                .find(|signal| signal.read == name || signal.write == name)
                .map(|signal| signal.read.clone())
                .unwrap_or(name);
            Ok(HxExpr::Name(read))
        }
        Expr::MethodCall(call) if call.method == "to_string" || call.method == "into" => {
            lower_expr(&call.receiver, signals)
        }
        Expr::Call(call) if path_ends(&call.func, &["String", "new"]) && call.args.is_empty() => {
            Ok(HxExpr::Str(String::new()))
        }
        Expr::Call(call)
            if path_ends(&call.func, &["min"])
                || path_ends(&call.func, &["max"])
                || path_ends(&call.func, &["clamp"]) =>
        {
            let name = path_ident(&call.func)?;
            let mut args = Vec::new();
            for arg in &call.args {
                args.push(lower_expr(arg, signals)?);
            }
            let need = if name == "clamp" { 3 } else { 2 };
            if args.len() != need {
                return Err(format!("{name} takes {need} arguments"));
            }
            Ok(HxExpr::Call { name, args })
        }
        Expr::If(iff) => {
            let cond = Box::new(lower_expr(&iff.cond, signals)?);
            let then = Box::new(lower_expr(&block_value(&iff.then_branch)?, signals)?);
            let Some((_, else_expr)) = &iff.else_branch else {
                return Err("if used as a value needs an else branch".into());
            };
            let els = Box::new(match else_expr.as_ref() {
                Expr::Block(block) => lower_expr(&block_value(&block.block)?, signals)?,
                other => lower_expr(other, signals)?,
            });
            Ok(HxExpr::If { cond, then, els })
        }
        Expr::Macro(mac)
            if mac
                .mac
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "format") =>
        {
            lower_format(&mac.mac.tokens)
        }
        Expr::Reference(refer) => lower_expr(&refer.expr, signals),
        Expr::Paren(paren) => lower_expr(&paren.expr, signals),
        Expr::Tuple(tuple) => {
            let mut items = Vec::new();
            for elem in &tuple.elems {
                items.push(lower_expr(elem, signals)?);
            }
            Ok(HxExpr::List(items))
        }
        Expr::Struct(item) => {
            if item.rest.is_some() {
                return Err(reject(
                    "that struct",
                    "Place { city: city.get(), note: \"desk\" } with no ..rest",
                ));
            }
            let mut fields = Vec::new();
            for field in &item.fields {
                let syn::Member::Named(name) = &field.member else {
                    return Err(reject("that struct", "named fields"));
                };
                fields.push((name.to_string(), lower_expr(&field.expr, signals)?));
            }
            Ok(HxExpr::Obj(fields))
        }
        other => Err(reject_expr(other)),
    }
}

fn lower_format(tokens: &proc_macro2::TokenStream) -> Result<HxExpr, String> {
    let lit = syn::parse2::<syn::LitStr>(tokens.clone()).map_err(|_| {
        reject(
            "that format!",
            "format!(\"text {name}\") with identifiers in braces",
        )
    })?;
    let text = lit.value();
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                buf.push('{');
                continue;
            }
            if !buf.is_empty() {
                parts.push(HxExpr::Str(std::mem::take(&mut buf)));
            }
            let mut name = String::new();
            for ch in chars.by_ref() {
                if ch == '}' {
                    break;
                }
                name.push(ch);
            }
            let name = name.trim();
            if name.is_empty()
                || !name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                return Err(reject(
                    "that format!",
                    "format!(\"text {name}\") with a plain identifier",
                ));
            }
            parts.push(HxExpr::Name(name.to_string()));
        } else {
            buf.push(ch);
        }
    }
    if !buf.is_empty() {
        parts.push(HxExpr::Str(buf));
    }
    if parts.is_empty() {
        return Ok(HxExpr::Str(String::new()));
    }
    if parts.len() == 1 {
        return Ok(parts.pop().unwrap());
    }
    Ok(HxExpr::Concat(parts))
}

enum InitKind {
    Items,
    Other,
}

fn signal_kind(name: &str, signals: &[Signal]) -> InitKind {
    match signals
        .iter()
        .find(|signal| signal.read == name)
        .map(|signal| &signal.init)
    {
        Some(crate::model::Init::Items(_)) => InitKind::Items,
        _ => InitKind::Other,
    }
}

fn setter(expr: &Expr, signals: &[Signal]) -> Result<String, String> {
    let write = path_ident(expr)?;
    signals
        .iter()
        .find(|signal| signal.write == write || signal.read == write)
        .map(|signal| signal.read.clone())
        .ok_or_else(|| format!("unknown setter {write}"))
}

fn path_ident(expr: &Expr) -> Result<String, String> {
    let Expr::Path(path) = expr else {
        return Err(reject_expr(expr));
    };
    path.path
        .get_ident()
        .map(|ident| ident.to_string())
        .ok_or_else(|| reject_expr(expr))
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
        && segs[segs.len() - tail.len()..]
            == tail.iter().map(|s| (*s).to_string()).collect::<Vec<_>>()
}

fn reject(found: &str, expected: &str) -> String {
    format!("cannot lower {found}. Handlers may {expected}")
}

fn reject_expr(expr: &Expr) -> String {
    let text = short_expr(expr);
    format!(
        "cannot lower `{text}`. Handlers may set or update a signal with arithmetic (+ - * /), comparisons, min, max, and clamp, a boolean not, if / else, a string, or a field write (profile.name = ...); may navigate(path); and may push, pop, reverse, swap, or remove a list by item id. This expression is outside that set"
    )
}

fn short_expr(expr: &Expr) -> String {
    let text = format!("{expr:?}");
    let text: String = text.chars().filter(|ch| *ch != '\n').collect();
    let mut short: String = text.chars().take(90).collect();
    if text.chars().count() > 90 {
        short.push_str("...");
    }
    short
}
