use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use lm_reactive::{ImmediateEffect, Owner};

use crate::surface::{NodeSpec, Surface, Tag};
use crate::Style;

pub enum UiEvent {
    Click(u64),
    Input(u64, String),
}

enum Cmd {
    Text { id: u64, text: String },
    Style { id: u64, style: Style },
    Show { id: u64 },
    List { id: u64 },
    Placeholder { id: u64, text: String },
}

std::thread_local! {
    static CMDS: std::cell::RefCell<Vec<Cmd>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub(crate) fn push_text(id: u64, text: String) {
    CMDS.with(|cmds| cmds.borrow_mut().push(Cmd::Text { id, text }));
}

pub(crate) fn push_style(id: u64, style: Style) {
    CMDS.with(|cmds| cmds.borrow_mut().push(Cmd::Style { id, style }));
}

pub(crate) fn push_show(id: u64) {
    CMDS.with(|cmds| cmds.borrow_mut().push(Cmd::Show { id }));
}

pub(crate) fn push_list(id: u64) {
    CMDS.with(|cmds| cmds.borrow_mut().push(Cmd::List { id }));
}

pub(crate) fn push_placeholder(id: u64, text: String) {
    CMDS.with(|cmds| cmds.borrow_mut().push(Cmd::Placeholder { id, text }));
}

fn drain() -> Vec<Cmd> {
    CMDS.with(|cmds| std::mem::take(&mut *cmds.borrow_mut()))
}

pub(crate) type ClickFn = Arc<Mutex<Box<dyn FnMut() + Send>>>;
pub(crate) type InputFn = Arc<Mutex<Box<dyn FnMut(String) + Send>>>;

pub(crate) struct ShowState {
    pub flag: Arc<std::sync::atomic::AtomicBool>,
    pub view: Box<dyn Fn() -> crate::DynView + Send + Sync>,
    pub child: Option<u64>,
    pub mounted_once: bool,
}

pub(crate) trait ListDriver: Send {
    fn reconcile(&mut self, rt: &mut Runtime, parent: u64);
}

struct Node {
    parent: Option<u64>,
    children: Vec<u64>,
    owner: Owner,
    extra_owners: Vec<Owner>,
    click: Option<ClickFn>,
    input: Option<InputFn>,
    show: Option<ShowState>,
    list: Option<Box<dyn ListDriver>>,
    text: String,
    style: Style,
    tag: Tag,
}

/// Owns component nodes for one surface. Dropping a node drops its
/// [`Owner`], which disposes effects and signals created under that owner.
pub struct Runtime {
    surface: Box<dyn Surface>,
    nodes: HashMap<u64, Node>,
    root: Option<u64>,
}

impl Runtime {
    pub fn new(surface: Box<dyn Surface>) -> Self {
        let _ = drain();
        Self {
            surface,
            nodes: HashMap::new(),
            root: None,
        }
    }

    pub fn mount_view(&mut self, view: crate::DynView) -> u64 {
        let id = view.mount(self, None);
        self.root = Some(id);
        self.flush();
        id
    }

    pub fn root(&self) -> Option<u64> {
        self.root
    }

    pub fn text(&self, id: u64) -> String {
        self.surface.text(id)
    }

    pub fn children(&self, id: u64) -> Vec<u64> {
        self.nodes
            .get(&id)
            .map(|n| n.children.clone())
            .unwrap_or_default()
    }

    pub fn alive(&self, id: u64) -> bool {
        self.nodes.contains_key(&id)
    }

    pub fn tag(&self, id: u64) -> Option<Tag> {
        self.nodes.get(&id).map(|n| n.tag)
    }

    pub fn surface_log(&self) -> Option<&Vec<String>> {
        self.surface
            .as_any()
            .downcast_ref::<crate::MemorySurface>()
            .map(|s| &s.log)
    }

    pub fn memory_mut(&mut self) -> Option<&mut crate::MemorySurface> {
        self.surface
            .as_any_mut()
            .downcast_mut::<crate::MemorySurface>()
    }

    pub fn surface_any(&self) -> &dyn std::any::Any {
        self.surface.as_any()
    }

    pub fn click(&mut self, id: u64) {
        self.dispatch(UiEvent::Click(id));
        self.flush();
    }

    pub fn input(&mut self, id: u64, text: String) {
        self.dispatch(UiEvent::Input(id, text));
        self.flush();
    }

    pub fn dispatch(&mut self, event: UiEvent) {
        match event {
            UiEvent::Click(id) => {
                let listener = self.nodes.get(&id).and_then(|n| n.click.clone());
                if let Some(listener) = listener {
                    if let Ok(mut call) = listener.lock() {
                        call();
                    }
                }
            }
            UiEvent::Input(id, text) => {
                let listener = self.nodes.get(&id).and_then(|n| n.input.clone());
                if let Some(listener) = listener {
                    if let Ok(mut call) = listener.lock() {
                        call(text);
                    }
                }
            }
        }
    }

    pub fn unmount(&mut self, id: u64) {
        let children = self
            .nodes
            .get(&id)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        for child in children {
            self.unmount(child);
        }
        if let Some(node) = self.nodes.get(&id) {
            if let Some(show_child) = node.show.as_ref().and_then(|s| s.child) {
                if self.nodes.contains_key(&show_child) {
                    self.unmount(show_child);
                }
            }
        }
        let parent = self.nodes.get(&id).and_then(|n| n.parent);
        if let Some(parent) = parent {
            if let Some(parent_node) = self.nodes.get_mut(&parent) {
                parent_node.children.retain(|child| *child != id);
                if let Some(show) = parent_node.show.as_mut() {
                    if show.child == Some(id) {
                        show.child = None;
                    }
                }
            }
        }
        if self.root == Some(id) {
            self.root = None;
        }
        self.nodes.remove(&id);
        self.surface.remove(id);
    }

    pub fn flush(&mut self) {
        for _ in 0..32 {
            let cmds = drain();
            if cmds.is_empty() {
                break;
            }
            for cmd in cmds {
                self.apply(cmd);
            }
        }
        self.surface.redraw();
    }

    fn apply(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Text { id, text } => {
                if !self.nodes.contains_key(&id) {
                    return;
                }
                if self.nodes.get(&id).is_some_and(|n| n.text == text) {
                    return;
                }
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.text = text.clone();
                }
                self.surface.set_text(id, &text);
            }
            Cmd::Style { id, style } => {
                if !self.nodes.contains_key(&id) {
                    return;
                }
                if self.nodes.get(&id).is_some_and(|n| n.style == style) {
                    return;
                }
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.style = style.clone();
                }
                self.surface.set_style(id, &style);
            }
            Cmd::Placeholder { id, text } => {
                if self.nodes.contains_key(&id) {
                    self.surface.set_placeholder(id, &text);
                }
            }
            Cmd::Show { id } => self.flush_show(id),
            Cmd::List { id } => self.flush_list(id),
        }
    }

    fn flush_show(&mut self, id: u64) {
        let Some(show) = self.nodes.get(&id).and_then(|n| n.show.as_ref()) else {
            return;
        };
        let on = show
            .flag
            .load(std::sync::atomic::Ordering::SeqCst);
        let child = show.child;
        if on && child.is_none() {
            let owner = self.nodes.get(&id).map(|n| n.owner.clone());
            let view = {
                let show = self.nodes.get(&id).and_then(|n| n.show.as_ref()).unwrap();
                match &owner {
                    Some(owner) => owner.with(|| (show.view)()),
                    None => (show.view)(),
                }
            };
            let child_id = view.mount(self, Some(id));
            if let Some(show) = self.nodes.get_mut(&id).and_then(|n| n.show.as_mut()) {
                show.child = Some(child_id);
                show.mounted_once = true;
            }
        } else if !on {
            if let Some(child) = child {
                self.unmount(child);
            }
        }
    }

    fn flush_list(&mut self, id: u64) {
        let Some(mut driver) = self.nodes.get_mut(&id).and_then(|n| n.list.take()) else {
            return;
        };
        driver.reconcile(self, id);
        if let Some(node) = self.nodes.get_mut(&id) {
            node.list = Some(driver);
        }
    }

    pub(crate) fn create_node(
        &mut self,
        parent: Option<u64>,
        spec: NodeSpec,
        click: Option<ClickFn>,
        input: Option<InputFn>,
    ) -> u64 {
        let style = spec.style.clone();
        let text = spec.text.clone();
        let tag = spec.tag;
        let id = self.surface.create(parent, &spec);
        let owner = Owner::new();
        if let Some(parent) = parent {
            if let Some(parent_node) = self.nodes.get_mut(&parent) {
                parent_node.children.push(id);
            }
        }
        self.nodes.insert(
            id,
            Node {
                parent,
                children: Vec::new(),
                owner,
                extra_owners: Vec::new(),
                click,
                input,
                show: None,
                list: None,
                text,
                style,
                tag,
            },
        );
        id
    }

    pub(crate) fn keep_owner(&mut self, id: u64, owner: Owner) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.extra_owners.push(owner);
        }
    }

    pub(crate) fn node_owner(&self, id: u64) -> Option<Owner> {
        self.nodes.get(&id).map(|n| n.owner.clone())
    }

    pub(crate) fn set_show(&mut self, id: u64, show: ShowState) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.show = Some(show);
        }
    }

    pub(crate) fn set_list(&mut self, id: u64, list: Box<dyn ListDriver>) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.list = Some(list);
        }
    }

    pub(crate) fn set_children_order(&mut self, parent: u64, children: Vec<u64>) {
        if let Some(node) = self.nodes.get_mut(&parent) {
            node.children = children.clone();
        }
        self.surface.set_order(parent, &children);
    }

    pub(crate) fn watch<F>(&self, id: u64, fun: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        if let Some(owner) = self.node_owner(id) {
            owner.with(|| ImmediateEffect::new_scoped(fun));
        }
    }
}
