use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use lm_styles::{parse_classes, Style};

use crate::runtime::{
    push_list, push_placeholder, push_show, push_style, push_text, ClickFn, InputFn, ListDriver,
    Runtime, ShowState,
};
use crate::surface::{NodeSpec, Tag};
use crate::Owner;

pub fn display_text(value: impl std::fmt::Display) -> String {
    value.to_string()
}

enum TextPart {
    Fixed(String),
    Dyn(Box<dyn Fn() -> String + Send + Sync>),
}

/// A mounted view. The mount closure runs once.
pub struct DynView {
    mount: Box<dyn FnOnce(&mut Runtime, Option<u64>) -> u64 + Send>,
}

impl DynView {
    pub fn mount(self, rt: &mut Runtime, parent: Option<u64>) -> u64 {
        (self.mount)(rt, parent)
    }

    fn from_fn(mount: impl FnOnce(&mut Runtime, Option<u64>) -> u64 + Send + 'static) -> Self {
        Self {
            mount: Box::new(mount),
        }
    }
}

pub trait IntoView {
    fn into_view(self) -> DynView;
}

impl IntoView for DynView {
    fn into_view(self) -> DynView {
        self
    }
}

pub struct El {
    tag: Tag,
    style: Style,
    class_dyn: Option<Box<dyn Fn() -> String + Send + Sync>>,
    texts: Vec<TextPart>,
    value_dyn: Option<Box<dyn Fn() -> String + Send + Sync>>,
    placeholder: Option<String>,
    children: Vec<DynView>,
    click: Option<ClickFn>,
    input: Option<InputFn>,
}

pub fn el(tag: Tag) -> El {
    El {
        tag,
        style: tag.defaults(),
        class_dyn: None,
        texts: Vec::new(),
        value_dyn: None,
        placeholder: None,
        children: Vec::new(),
        click: None,
        input: None,
    }
}

impl El {
    pub fn style(mut self, over: Style) -> Self {
        self.style.merge_from(&over);
        self
    }

    pub fn class_dyn(mut self, class_name: impl Fn() -> String + Send + Sync + 'static) -> Self {
        self.class_dyn = Some(Box::new(class_name));
        self
    }

    pub fn text_fixed(mut self, text: impl Into<String>) -> Self {
        self.texts.push(TextPart::Fixed(text.into()));
        self
    }

    pub fn text_dyn(mut self, text: impl Fn() -> String + Send + Sync + 'static) -> Self {
        self.texts.push(TextPart::Dyn(Box::new(text)));
        self
    }

    pub fn value_dyn(mut self, value: impl Fn() -> String + Send + Sync + 'static) -> Self {
        self.value_dyn = Some(Box::new(value));
        self
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    pub fn child(mut self, view: impl IntoView) -> Self {
        self.children.push(view.into_view());
        self
    }

    pub fn on_click(mut self, handler: impl FnMut() + Send + 'static) -> Self {
        self.click = Some(Arc::new(Mutex::new(Box::new(handler))));
        self
    }

    pub fn on_input(mut self, handler: impl FnMut(String) + Send + 'static) -> Self {
        self.input = Some(Arc::new(Mutex::new(Box::new(handler))));
        self
    }
}

impl IntoView for El {
    fn into_view(self) -> DynView {
        DynView::from_fn(move |rt, parent| mount_el(self, rt, parent))
    }
}

fn mount_el(el: El, rt: &mut Runtime, parent: Option<u64>) -> u64 {
    let El {
        tag,
        style,
        class_dyn,
        texts,
        value_dyn,
        placeholder,
        children,
        click,
        input,
    } = el;
    let initial = join_fixed(&texts);
    let spec = NodeSpec {
        tag,
        style: style.clone(),
        text: initial,
        placeholder: placeholder.clone().unwrap_or_default(),
    };
    let id = rt.create_node(parent, spec, click, input);
    if texts.iter().any(|part| matches!(part, TextPart::Dyn(_))) || value_dyn.is_some() {
        let base_texts = texts;
        let value_dyn = value_dyn;
        rt.watch(id, move || {
            let text = if let Some(value) = &value_dyn {
                value()
            } else {
                join_parts(&base_texts)
            };
            push_text(id, text);
        });
    }
    if class_dyn.is_some() {
        let base = style.clone();
        rt.watch(id, move || {
            let mut next = base.clone();
            if let Some(class_name) = &class_dyn {
                next.merge_from(&parse_classes(&class_name()));
            }
            push_style(id, next);
        });
    }
    if let Some(placeholder) = placeholder {
        push_placeholder(id, placeholder);
    }
    for child in children {
        let _ = child.mount(rt, Some(id));
    }
    id
}

fn join_fixed(parts: &[TextPart]) -> String {
    let mut out = String::new();
    for part in parts {
        if let TextPart::Fixed(text) = part {
            out.push_str(text);
        }
    }
    out
}

fn join_parts(parts: &[TextPart]) -> String {
    let mut out = String::new();
    for part in parts {
        match part {
            TextPart::Fixed(text) => out.push_str(text),
            TextPart::Dyn(fun) => out.push_str(&fun()),
        }
    }
    out
}

pub fn scope_owner(owner: Owner, view: impl IntoView) -> DynView {
    let view = view.into_view();
    DynView::from_fn(move |rt, parent| {
        let id = owner.with(|| view.mount(rt, parent));
        rt.keep_owner(id, owner);
        id
    })
}

pub fn empty() -> DynView {
    DynView::from_fn(|_, _| 0)
}

pub fn fragment(views: Vec<DynView>) -> DynView {
    el(Tag::Div).children_from(views).into_view()
}

impl El {
    fn children_from(mut self, views: Vec<DynView>) -> Self {
        self.children.extend(views);
        self
    }
}

pub struct Show {
    when: Box<dyn Fn() -> bool + Send + Sync>,
    view: Box<dyn Fn() -> DynView + Send + Sync>,
}

pub fn show<W, F, IV>(when: W, view: F) -> Show
where
    W: Fn() -> bool + Send + Sync + 'static,
    F: Fn() -> IV + Send + Sync + 'static,
    IV: IntoView,
{
    Show {
        when: Box::new(when),
        view: Box::new(move || view().into_view()),
    }
}

impl IntoView for Show {
    fn into_view(self) -> DynView {
        DynView::from_fn(move |rt, parent| {
            let host = el(Tag::Div)
                .style(host_style())
                .into_view()
                .mount(rt, parent);
            let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let flag_effect = flag.clone();
            let when = self.when;
            rt.watch(host, move || {
                let on = when();
                flag_effect.store(on, std::sync::atomic::Ordering::SeqCst);
                push_show(host);
            });
            rt.set_show(
                host,
                ShowState {
                    flag,
                    view: self.view,
                    child: None,
                    mounted_once: false,
                },
            );
            host
        })
    }
}

fn host_style() -> Style {
    let mut style = Tag::Div.defaults();
    style.flow = Some(lm_styles::FlowSpec::Column);
    style.width = Some(lm_styles::SizeSpec::Fill);
    style.height = Some(lm_styles::SizeSpec::Fit);
    style.padding = lm_styles::Edges::default();
    style.margin = lm_styles::Edges::default();
    style
}

struct ListState<T, K> {
    pending: Arc<Mutex<Option<Vec<T>>>>,
    key: Box<dyn Fn(&T) -> K + Send + Sync>,
    view: Box<dyn Fn(T) -> DynView + Send + Sync>,
    rows: HashMap<K, u64>,
}

impl<T, K> ListDriver for ListState<T, K>
where
    T: Clone + Send + 'static,
    K: Clone + Eq + Hash + Send + 'static,
{
    fn reconcile(&mut self, rt: &mut Runtime, parent: u64) {
        let Some(items) = self.pending.lock().ok().and_then(|mut slot| slot.take()) else {
            return;
        };
        let mut next_rows = HashMap::new();
        let mut order = Vec::with_capacity(items.len());
        for item in items {
            let key = (self.key)(&item);
            if let Some(id) = self.rows.remove(&key) {
                order.push(id);
                next_rows.insert(key, id);
            } else {
                let owner = rt.node_owner(parent);
                let id = match owner {
                    Some(owner) => {
                        let view = owner.with(|| (self.view)(item));
                        view.mount(rt, Some(parent))
                    }
                    None => (self.view)(item).mount(rt, Some(parent)),
                };
                order.push(id);
                next_rows.insert(key, id);
            }
        }
        let stale: Vec<u64> = self.rows.drain().map(|(_, id)| id).collect();
        for id in stale {
            rt.unmount(id);
        }
        self.rows = next_rows;
        rt.set_children_order(parent, order);
    }
}

pub struct For<T, K, E, KF, V> {
    each: E,
    key: KF,
    view: V,
    _marker: std::marker::PhantomData<(T, K)>,
}

pub fn for_each<T, K, E, KF, V, IV>(each: E, key: KF, view: V) -> For<T, K, E, KF, V>
where
    T: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: Fn() -> Vec<T> + Send + Sync + 'static,
    KF: Fn(&T) -> K + Send + Sync + 'static,
    V: Fn(T) -> IV + Send + Sync + 'static,
    IV: IntoView,
{
    For {
        each,
        key,
        view,
        _marker: std::marker::PhantomData,
    }
}

impl<T, K, E, KF, V, IV> IntoView for For<T, K, E, KF, V>
where
    T: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: Fn() -> Vec<T> + Send + Sync + 'static,
    KF: Fn(&T) -> K + Send + Sync + 'static,
    V: Fn(T) -> IV + Send + Sync + 'static,
    IV: IntoView,
{
    fn into_view(self) -> DynView {
        DynView::from_fn(move |rt, parent| {
            let host = el(Tag::Div)
                .style(host_style())
                .into_view()
                .mount(rt, parent);
            let pending = Arc::new(Mutex::new(None));
            let pending_effect = pending.clone();
            let each = self.each;
            rt.watch(host, move || {
                let items = each();
                if let Ok(mut slot) = pending_effect.lock() {
                    *slot = Some(items);
                }
                push_list(host);
            });
            let view = self.view;
            rt.set_list(
                host,
                Box::new(ListState {
                    pending,
                    key: Box::new(self.key),
                    view: Box::new(move |item| view(item).into_view()),
                    rows: HashMap::new(),
                }),
            );
            host
        })
    }
}
