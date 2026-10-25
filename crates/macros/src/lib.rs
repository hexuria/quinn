//! `view!`, `#[component]`, and `run!`.
//!
//! Parsing is [rstml](https://github.com/rs-tml/rstml) 0.13.1 (commit
//! `9fe0f4616a8224d37d64a53fa91722a08d736952`). Expansion targets `lume`,
//! not the DOM.

use proc_macro::TokenStream;
use proc_macro2::{Ident, Punct, Spacing, Span, TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens};
use rstml::node::{Node, NodeAttribute, NodeBlock, NodeElement, NodeName};
use syn::{parse_macro_input, Expr, ExprClosure, ItemFn, LitInt, LitStr};

#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    match rstml::parse2(input.into()) {
        Ok(nodes) => expand_nodes(&nodes).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);
    let block = func.block;
    func.block = syn::parse_quote! {{
        let __lume_owner = ::lume::Owner::new();
        let __lume_view = __lume_owner.with(|| #block);
        ::lume::scope_owner(__lume_owner, __lume_view)
    }};
    func.attrs.push(syn::parse_quote!(#[allow(non_snake_case)]));
    quote!(#func).into()
}

struct RunArgs {
    title: LitStr,
    _c1: syn::Token![,],
    width: LitInt,
    _c2: syn::Token![,],
    height: LitInt,
    _c3: syn::Token![,],
    root: Expr,
}

impl syn::parse::Parse for RunArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            title: input.parse()?,
            _c1: input.parse()?,
            width: input.parse()?,
            _c2: input.parse()?,
            height: input.parse()?,
            _c3: input.parse()?,
            root: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn run(input: TokenStream) -> TokenStream {
    let RunArgs {
        title,
        width,
        height,
        root,
        ..
    } = parse_macro_input!(input as RunArgs);
    let hash = Punct::new('#', Spacing::Alone);
    let mut page_color = TokenStream2::new();
    page_color.extend([
        TokenTree::Punct(Punct::new('#', Spacing::Joint)),
        TokenTree::Ident(Ident::new("xf4f0e6", Span::call_site())),
    ]);
    quote! {
        pub use ::lume::makepad_widgets;
        use ::lume::makepad_widgets::*;

        app_main!(App);

        script_mod! {
            use mod.prelude.widgets.*
            startup() do #hash (App::script_component(vm)) {
                ui: Root {
                    on_startup: || {
                        ui.host.render()
                    }
                    main_window := Window {
                        window.inner_size: vec2(#width, #height)
                        window.title: #title
                        body +: {
                            host := RoundedView {
                                width: Fill
                                height: Fill
                                flow: Down
                                padding: 16.
                                show_bg: true
                                new_batch: true
                                draw_bg.color: #page_color
                            }
                        }
                    }
                }
            }
        }

        #[derive(Script, ScriptHook)]
        pub struct App {
            #[live]
            ui: WidgetRef,
        }

        impl MatchEvent for App {
            fn handle_startup(&mut self, cx: &mut Cx) {
                ::lume::startup(cx, &self.ui, || #root);
            }

            fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
                ::lume::on_actions(cx, actions);
            }
        }

        impl AppMain for App {
            fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
                crate::makepad_widgets::script_mod(vm);
                self::script_mod(vm)
            }

            fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
                self.match_event(cx, event);
                self.ui.handle_event(cx, event, &mut Scope::empty());
            }
        }
    }
    .into()
}

fn expand_nodes(nodes: &[Node]) -> TokenStream2 {
    let mut views = Vec::new();
    for node in nodes {
        if let Some(view) = expand_child_view(node) {
            views.push(view);
        }
    }
    match views.len() {
        0 => quote!(::lume::empty()),
        1 => views.pop().unwrap(),
        _ => quote!(::lume::fragment(vec![#(#views .into_view()),*])),
    }
}

fn expand_child_view(node: &Node) -> Option<TokenStream2> {
    match node {
        Node::Element(element) => Some(expand_element(element)),
        Node::Fragment(fragment) => Some(expand_nodes(&fragment.children)),
        Node::Text(text) => {
            let value = text.value_string();
            Some(quote!(::lume::el(::lume::Tag::Span).text_fixed(#value)))
        }
        Node::Block(block) => {
            let text = text_method(block);
            Some(quote!(::lume::el(::lume::Tag::Span) #text))
        }
        Node::RawText(raw) => {
            let value = raw.to_string_best();
            if value.trim().is_empty() {
                None
            } else {
                Some(quote!(::lume::el(::lume::Tag::Span).text_fixed(#value)))
            }
        }
        Node::Comment(_) | Node::Doctype(_) | Node::Custom(_) => None,
    }
}

fn expand_element(element: &NodeElement<rstml::Infallible>) -> TokenStream2 {
    let name = element.name().to_string();
    match name.as_str() {
        "Show" | "show" => expand_show(element),
        "For" | "for" => expand_for(element),
        "div" | "span" | "h1" | "h2" | "p" | "label" | "button" | "input" => {
            expand_tag(element, &name)
        }
        _ => expand_component(element),
    }
}

fn expand_tag(element: &NodeElement<rstml::Infallible>, name: &str) -> TokenStream2 {
    let tag = Ident::new(&capitalize_tag(name), proc_macro2::Span::call_site());
    let mut class_lit = None;
    let mut class_dyn = None;
    let mut style_lit = None;
    let mut placeholder = None;
    let mut value = None;
    let mut click = None;
    let mut input = None;
    for attr in element.attributes() {
        let NodeAttribute::Attribute(attr) = attr else {
            return quote!(compile_error!("computed attribute keys are not supported"));
        };
        let key = attr.key.to_string().replace(' ', "");
        match key.as_str() {
            "class" => {
                if let Some(lit) = attr.value_literal_string() {
                    class_lit = Some(lit);
                } else if let Some(expr) = attr.value() {
                    class_dyn = Some(expr_to_string(expr));
                }
            }
            "style" => {
                if let Some(lit) = attr.value_literal_string() {
                    style_lit = Some(lit);
                } else {
                    return quote!(compile_error!("dynamic style strings are not in this milestone; bind class instead"));
                }
            }
            "placeholder" => {
                if let Some(lit) = attr.value_literal_string() {
                    placeholder = Some(lit);
                } else if attr.value().is_some() {
                    return quote!(compile_error!("placeholder must be a string literal"));
                }
            }
            "value" => value = attr.value().cloned(),
            "on:click" => click = attr.value().cloned(),
            "on:input" | "on:change" => input = attr.value().cloned(),
            _ => {
                let message = format!("unsupported attribute `{key}` on <{name}>");
                return quote!(compile_error!(#message));
            }
        }
    }

    let style = match (class_lit, style_lit) {
        (None, None) => quote!(),
        (Some(class), None) => quote!(.style(::lume::parse_classes(#class))),
        (None, Some(style)) => quote!(.style(::lume::parse_inline(#style))),
        (Some(class), Some(style)) => quote!(.style({
            let mut __style = ::lume::parse_classes(#class);
            __style.merge_from(&::lume::parse_inline(#style));
            __style
        })),
    };
    let class_dyn = class_dyn
        .map(|dyn_class| quote!(.class_dyn(#dyn_class)))
        .unwrap_or_default();
    let placeholder = placeholder
        .map(|text| quote!(.placeholder(#text)))
        .unwrap_or_default();
    let value = value
        .map(|expr| {
            let dyn_value = expr_to_string(&expr);
            quote!(.value_dyn(#dyn_value))
        })
        .unwrap_or_default();
    let click = click
        .map(|expr| {
            let call = call_handler(&expr);
            quote!(.on_click({ let __h = #expr; move || { #call } }))
        })
        .unwrap_or_default();
    let input = input
        .map(|expr| quote!(.on_input({ let __h = #expr; move |v: String| { __h(v); } })))
        .unwrap_or_default();

    let mut texts = Vec::new();
    let mut children = Vec::new();
    for child in element.children() {
        match child {
            Node::Text(text) => {
                let value = text.value_string();
                texts.push(quote!(.text_fixed(#value)));
            }
            Node::Block(block) => {
                texts.push(text_method(&block));
            }
            Node::RawText(raw) => {
                let value = raw.to_string_best();
                if !value.trim().is_empty() {
                    texts.push(quote!(.text_fixed(#value)));
                }
            }
            Node::Comment(_) | Node::Doctype(_) | Node::Custom(_) => {}
            other => {
                if let Some(view) = expand_child_view(&other) {
                    children.push(quote!(.child(#view)));
                }
            }
        }
    }

    quote! {
        ::lume::el(::lume::Tag::#tag)
            #style
            #class_dyn
            #(#texts)*
            #(#children)*
            #value
            #placeholder
            #click
            #input
    }
}

fn capitalize_tag(name: &str) -> String {
    match name {
        "div" => "Div",
        "span" => "Span",
        "h1" => "H1",
        "h2" => "H2",
        "p" => "P",
        "label" => "Label",
        "button" => "Button",
        "input" => "Input",
        other => other,
    }
    .to_string()
}

fn expand_show(element: &NodeElement<rstml::Infallible>) -> TokenStream2 {
    let mut when = None;
    for attr in element.attributes() {
        let NodeAttribute::Attribute(attr) = attr else {
            continue;
        };
        if attr.key.to_string().replace(' ', "") == "when" {
            when = attr.value().cloned();
        }
    }
    let Some(when) = when else {
        return quote!(compile_error!("<Show> needs when=..."));
    };
    let body = expand_nodes(element.children());
    quote!(::lume::show(#when, move || { #body }))
}

fn expand_for(element: &NodeElement<rstml::Infallible>) -> TokenStream2 {
    let mut each = None;
    let mut key = None;
    let mut binding = String::from("item");
    for attr in element.attributes() {
        let NodeAttribute::Attribute(attr) = attr else {
            continue;
        };
        let name = attr.key.to_string().replace(' ', "");
        if name == "each" {
            each = attr.value().cloned();
        } else if name == "key" {
            key = attr.value().cloned();
        } else if let Some(rest) = name.strip_prefix("let:") {
            binding = rest.to_string();
        }
    }
    let (Some(each), Some(key)) = (each, key) else {
        return quote!(compile_error!("<For> needs each= and key="));
    };
    let binding = Ident::new(&binding, proc_macro2::Span::call_site());
    let body = expand_nodes(element.children());
    quote!(::lume::for_each(#each, #key, move |#binding| { #body }))
}

fn expand_component(element: &NodeElement<rstml::Infallible>) -> TokenStream2 {
    if !element.children().is_empty() {
        return quote!(compile_error!(
            "child slots on custom components are not in this milestone"
        ));
    }
    let NodeName::Path(path) = element.name() else {
        return quote!(compile_error!("component name must be a path"));
    };
    let mut args = Vec::new();
    for attr in element.attributes() {
        let NodeAttribute::Attribute(attr) = attr else {
            return quote!(compile_error!("unsupported component attribute"));
        };
        let Some(expr) = attr.value() else {
            return quote!(compile_error!("component attributes need a value"));
        };
        args.push(expr.to_token_stream());
    }
    let path = &path.path;
    quote!(#path(#(#args),*))
}

fn text_method(block: &NodeBlock) -> TokenStream2 {
    match block_interp(block) {
        Interp::Fixed(value) => quote!(.text_fixed(#value)),
        Interp::Dyn(value) => quote!(.text_dyn(#value)),
    }
}

enum Interp {
    Fixed(TokenStream2),
    Dyn(TokenStream2),
}

/// A `{move || ...}` block is reactive. Anything else is interpolated once.
/// A non-Copy value cannot be moved into an `Fn` closure on every read.
fn block_interp(block: &NodeBlock) -> Interp {
    match block {
        NodeBlock::ValidBlock(block) => {
            if block.stmts.len() == 1 {
                if let syn::Stmt::Expr(expr, None) = &block.stmts[0] {
                    return expr_interp(expr);
                }
            }
            Interp::Fixed(quote!(::lume::display_text(#block)))
        }
        NodeBlock::Invalid(invalid) => {
            let body = &invalid.body;
            Interp::Fixed(quote!(::lume::display_text({ #body })))
        }
    }
}

fn expr_interp(expr: &Expr) -> Interp {
    if let Expr::Closure(closure) = expr {
        Interp::Dyn(quote!({
            let __f = #closure;
            move || ::lume::display_text(__f())
        }))
    } else {
        Interp::Fixed(quote!(::lume::display_text(#expr)))
    }
}

fn expr_to_string(expr: &Expr) -> TokenStream2 {
    if let Expr::Closure(closure) = expr {
        quote!({
            let __f = #closure;
            move || ::lume::display_text(__f())
        })
    } else {
        quote!(move || ::lume::display_text(#expr))
    }
}

fn call_handler(expr: &Expr) -> TokenStream2 {
    if let Expr::Closure(ExprClosure { inputs, .. }) = expr {
        if inputs.is_empty() {
            return quote!(__h());
        }
    }
    quote!(__h(()))
}
