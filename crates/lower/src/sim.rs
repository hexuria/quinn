//! Headless stand-in for `on_render`, named `:=` slots, and `mod.state`.
//!
//! A name that is still in the template keeps its id. A name that disappeared
//! is gone, and a click on it does nothing. Keyed row counts live under the
//! item id, so add / remove / reverse do not move them onto another row.
//! Hiding a `<Show>` drops that subtree and zeroes the fields it owned.

use std::collections::HashMap;

use crate::model::*;

#[derive(Clone, Debug)]
struct Slot {
    id: u64,
    text: String,
    click: Option<Act>,
    input: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Sim {
    pub state: HashMap<String, i64>,
    pub strings: HashMap<String, String>,
    pub items: Vec<ItemSeed>,
    pub counts: HashMap<i64, i64>,
    ids: HashMap<String, Slot>,
    next: u64,
    model: Model,
    next_ids: HashMap<String, i64>,
    item_signals: Vec<String>,
}

impl Sim {
    pub fn new(model: Model) -> Self {
        let mut state = HashMap::new();
        let mut strings = HashMap::new();
        let mut items = Vec::new();
        let mut item_signals = Vec::new();
        let mut next_ids = HashMap::new();
        for signal in &model.signals {
            match &signal.init {
                Init::Int(value) => {
                    state.insert(signal.read.clone(), *value);
                    if signal.read.contains("next") {
                        next_ids.insert(signal.read.clone(), *value);
                    }
                }
                Init::Bool(value) => {
                    state.insert(signal.read.clone(), if *value { 1 } else { 0 });
                }
                Init::Str(value) => {
                    strings.insert(signal.read.clone(), value.clone());
                }
                Init::Items(rows) => {
                    items = rows.clone();
                    item_signals.push(signal.read.clone());
                }
            }
        }
        let mut sim = Self {
            state,
            strings,
            items,
            counts: HashMap::new(),
            ids: HashMap::new(),
            next: 1,
            model,
            next_ids,
            item_signals,
        };
        sim.render();
        sim
    }

    pub fn render(&mut self) {
        let mut live = Vec::new();
        for node in &self.model.nodes.clone() {
            self.render_node(node, &mut live);
        }
        self.ids.retain(|name, _| live.iter().any(|live| live == name));
    }

    fn render_node(&mut self, node: &ViewNode, live: &mut Vec<String>) {
        match node {
            ViewNode::El(el) => self.render_el(el, live),
            ViewNode::Show {
                signal,
                live_flag,
                reset,
                child,
            } => {
                let open = self.state.get(signal).copied().unwrap_or(0) != 0;
                if open {
                    if self.state.get(live_flag).copied().unwrap_or(0) != 1 {
                        for name in reset {
                            if self.strings.contains_key(name) {
                                self.strings.insert(name.clone(), String::new());
                            } else {
                                self.state.insert(name.clone(), 0);
                            }
                        }
                        self.state.insert(live_flag.clone(), 1);
                    }
                    self.render_node(child, live);
                } else {
                    self.state.insert(live_flag.clone(), 0);
                    for name in reset {
                        if self.strings.contains_key(name) {
                            self.strings.insert(name.clone(), String::new());
                        } else {
                            self.state.insert(name.clone(), 0);
                        }
                    }
                }
            }
            ViewNode::For { child, .. } => {
                for (index, item) in self.items.clone().into_iter().enumerate() {
                    self.render_row(child, index, &item, live);
                }
            }
        }
    }

    fn render_row(&mut self, el: &El, index: usize, item: &ItemSeed, live: &mut Vec<String>) {
        let count = self.counts.get(&item.id).copied().unwrap_or(0);
        let name = format!("{}#{index}", el.name);
        let root_text = row_text(el, item, count);
        let root_click = el.click.clone();
        let root_input = el.input.clone();
        self.place(&name, &root_text, root_click, root_input, live);
        let children: Vec<(String, String, Option<Act>, Option<String>)> = el
            .children
            .iter()
            .filter_map(|child| {
                let ViewNode::El(child) = child else {
                    return None;
                };
                Some((
                    child.name.clone(),
                    row_text(child, item, count),
                    child.click.clone(),
                    child.input.clone(),
                ))
            })
            .collect();
        for (child_name, text, click, input) in children {
            self.place(
                &format!("{name}/{child_name}"),
                &text,
                click.clone(),
                input.clone(),
                live,
            );
            self.place(
                &format!("row:{}:{child_name}", item.id),
                &text,
                click,
                input,
                live,
            );
        }
    }

    fn render_el(&mut self, el: &El, live: &mut Vec<String>) {
        let text = self.plain_text(&el.parts, el.value.as_deref());
        self.place(&el.name, &text, el.click.clone(), el.input.clone(), live);
        for child in &el.children {
            self.render_node(child, live);
        }
    }

    fn place(
        &mut self,
        name: &str,
        text: &str,
        click: Option<Act>,
        input: Option<String>,
        live: &mut Vec<String>,
    ) {
        live.push(name.to_string());
        if let Some(slot) = self.ids.get_mut(name) {
            slot.text = text.to_string();
            slot.click = click;
            slot.input = input;
            return;
        }
        let id = self.next;
        self.next += 1;
        self.ids.insert(
            name.to_string(),
            Slot {
                id,
                text: text.to_string(),
                click,
                input,
            },
        );
    }

    fn plain_text(&self, parts: &[Part], value: Option<&str>) -> String {
        if let Some(signal) = value {
            return self.strings.get(signal).cloned().unwrap_or_default();
        }
        let mut out = String::new();
        for part in parts {
            match part {
                Part::Lit(text) => out.push_str(text),
                Part::Sig(name) => {
                    if let Some(text) = self.strings.get(name) {
                        out.push_str(text);
                    } else {
                        out.push_str(&self.state.get(name).copied().unwrap_or(0).to_string());
                    }
                }
                Part::IfBool { signal, yes, no } => {
                    if self.state.get(signal).copied().unwrap_or(0) != 0 {
                        out.push_str(yes);
                    } else {
                        out.push_str(no);
                    }
                }
                Part::IfEmpty { signal, empty } => {
                    let text = self.strings.get(signal).cloned().unwrap_or_default();
                    if text.is_empty() {
                        out.push_str(empty);
                    } else {
                        out.push_str(&text);
                    }
                }
                Part::Param(_) => {}
            }
        }
        out
    }

    pub fn id(&self, name: &str) -> Option<u64> {
        self.ids.get(name).map(|slot| slot.id)
    }

    pub fn text(&self, name: &str) -> String {
        self.ids
            .get(name)
            .map(|slot| slot.text.clone())
            .unwrap_or_default()
    }

    pub fn visible(&self, name: &str) -> bool {
        self.ids.contains_key(name)
    }

    pub fn click(&mut self, name: &str) -> bool {
        let Some(act) = self.ids.get(name).and_then(|slot| slot.click.clone()) else {
            return false;
        };
        self.apply(act, None)
    }

    pub fn click_row(&mut self, id: i64) -> bool {
        let key = self
            .ids
            .iter()
            .find(|(name, slot)| {
                name.starts_with(&format!("row:{id}:")) && matches!(slot.click, Some(Act::RowInc))
            })
            .map(|(name, _)| name.clone());
        let Some(key) = key else {
            return false;
        };
        let Some(act) = self.ids.get(&key).and_then(|slot| slot.click.clone()) else {
            return false;
        };
        self.apply(act, Some(id))
    }

    pub fn row_count(&self, id: i64) -> i64 {
        self.counts.get(&id).copied().unwrap_or(0)
    }

    pub fn item_ids(&self) -> Vec<i64> {
        self.items.iter().map(|item| item.id).collect()
    }

    pub fn item_label(&self, id: i64) -> String {
        self.items
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.label.clone())
            .unwrap_or_default()
    }

    pub fn set_input(&mut self, name: &str, value: &str) -> bool {
        let signal = self
            .ids
            .get(name)
            .and_then(|slot| slot.input.clone());
        let Some(signal) = signal else {
            return false;
        };
        self.strings.insert(signal, value.to_string());
        self.render();
        true
    }

    pub fn reload(&mut self, model: Model) {
        self.model = model;
        self.render();
    }

    pub fn drop_named(&mut self, name: &str) {
        remove_named(&mut self.model.nodes, name);
        self.render();
    }

    fn apply(&mut self, act: Act, row: Option<i64>) -> bool {
        match act {
            Act::Inc(signal) => {
                *self.state.entry(signal).or_insert(0) += 1;
            }
            Act::Toggle(signal) => {
                let value = self.state.entry(signal).or_insert(0);
                *value = if *value == 0 { 1 } else { 0 };
            }
            Act::Clear(signal) => {
                self.strings.insert(signal, String::new());
            }
            Act::Pop(signal) => {
                if self.item_signals.iter().any(|name| name == &signal) && !self.items.is_empty() {
                    self.items.pop();
                }
            }
            Act::Reverse(signal) => {
                if self.item_signals.iter().any(|name| name == &signal) {
                    self.items.reverse();
                }
            }
            Act::Push { next, items } => {
                if self.item_signals.iter().any(|name| name == &items) {
                    let id = self
                        .next_ids
                        .get(&next)
                        .copied()
                        .or_else(|| self.state.get(&next).copied())
                        .unwrap_or(1);
                    self.next_ids.insert(next.clone(), id + 1);
                    self.state.insert(next, id + 1);
                    self.items.push(ItemSeed {
                        id,
                        label: format!("Item {id}"),
                    });
                }
            }
            Act::RowInc => {
                let Some(id) = row else {
                    return false;
                };
                if !self.items.iter().any(|item| item.id == id) {
                    return false;
                }
                *self.counts.entry(id).or_insert(0) += 1;
            }
        }
        self.render();
        true
    }
}

fn row_text(el: &El, item: &ItemSeed, count: i64) -> String {
    let mut out = String::new();
    for part in &el.parts {
        match part {
            Part::Lit(text) => out.push_str(text),
            Part::Param(name) if name == "label" || name == "item" => out.push_str(&item.label),
            Part::Param(_) => out.push_str(&item.label),
            Part::Sig(_) => out.push_str(&count.to_string()),
            Part::IfBool { .. } | Part::IfEmpty { .. } => {}
        }
    }
    out
}

fn remove_named(nodes: &mut Vec<ViewNode>, name: &str) -> bool {
    let before = nodes.len();
    nodes.retain(|node| match node {
        ViewNode::El(el) => el.name != name,
        _ => true,
    });
    if nodes.len() != before {
        return true;
    }
    for node in nodes {
        match node {
            ViewNode::El(el) => {
                if remove_named(&mut el.children, name) {
                    return true;
                }
            }
            ViewNode::Show { child, .. } => {
                if let ViewNode::El(el) = child.as_mut() {
                    if el.name == name {
                        return false;
                    }
                    if remove_named(&mut el.children, name) {
                        return true;
                    }
                }
            }
            ViewNode::For { .. } => {}
        }
    }
    false
}
