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
    if let Expr::Call(call) = expr {
        if path_ends(&call.func, &["navigate"]) {
            let Some(arg) = call.args.first() else {
                return Err("navigate needs a path".into());
            };
            return Ok(vec![HandlerStmt::Navigate(lower_expr(arg, signals)?)]);
        }
    }
    let Expr::MethodCall(call) = expr else {
        return Err(reject_expr(expr));
    };
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
        HxExpr::Concat(parts) => {
            for part in parts {
                rewrite_name(part, from, to);
            }
        }
        _ => {}
    }
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
        "cannot lower `{text}`. Handlers may set or update a signal with arithmetic (+ - * /), a boolean not, a string, or a field write (profile.name = ...); may navigate(path); and may push, pop, reverse, swap, or remove a list by item id. This expression is outside that set"
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
