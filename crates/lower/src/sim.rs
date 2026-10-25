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
    /// Non-count row signals, keyed by `(row id, signal)`.
    row_vars: HashMap<(i64, String), i64>,
    row_strings: HashMap<(i64, String), String>,
    row_live: HashMap<(i64, String), i64>,
    count_signal: Option<String>,
    row_signal_names: Vec<String>,
    ctx: HashMap<String, String>,
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
                Init::Obj(fields) => flatten_obj(&signal.read, fields, &mut strings, &mut state),
            }
        }
        let mut ctx = HashMap::new();
        for (key, expr) in &model.ctx_init {
            ctx.insert(key.clone(), ctx_literal(expr));
        }
        state.insert("quinn_dark".into(), 0);
        for job in &model.jobs {
            let phase = match job.kind {
                JobKind::Action => "idle",
                JobKind::Local | JobKind::Resource => "loading",
            };
            strings.insert(format!("{}_phase", job.name), phase.into());
            strings.insert(format!("{}_value", job.name), String::new());
            strings.insert(format!("{}_error", job.name), String::new());
            state.insert(format!("{}_gen", job.name), 1);
            if let Some(source) = &job.source {
                let init = model
                    .signal(source)
                    .map(|signal| match &signal.init {
                        Init::Str(text) => text.clone(),
                        Init::Int(value) => value.to_string(),
                        Init::Bool(value) => value.to_string(),
                        _ => String::new(),
                    })
                    .unwrap_or_default();
                strings.insert(format!("{}_source", job.name), init);
            }
            if matches!(job.kind, JobKind::Action) {
                state.insert(format!("{}_pending", job.name), 0);
                strings.insert(format!("{}_arg", job.name), String::new());
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
            row_vars: HashMap::new(),
            row_strings: HashMap::new(),
            row_live: HashMap::new(),
            count_signal: None,
            row_signal_names: Vec::new(),
            ctx,
        };
        sim.refresh_row_meta();
        sim.render();
        sim
    }

    fn refresh_row_meta(&mut self) {
        let mut names = Vec::new();
        let mut count = None;
        fn walk(nodes: &[ViewNode], names: &mut Vec<String>, count: &mut Option<String>) {
            for node in nodes {
                match node {
                    ViewNode::For { row_signals, .. } => {
                        for signal in row_signals {
                            if !names.iter().any(|name| name == &signal.read) {
                                names.push(signal.read.clone());
                            }
                            if count.is_none() && matches!(signal.init, Init::Int(_)) {
                                *count = Some(signal.read.clone());
                            }
                        }
                    }
                    ViewNode::El(el) => walk(&el.children, names, count),
                    ViewNode::Show { child, .. } => walk(std::slice::from_ref(child), names, count),
                    ViewNode::Slot { nodes, .. } => walk(nodes, names, count),
                    ViewNode::Routes { routes } => {
                        for (_, nodes) in routes {
                            walk(nodes, names, count);
                        }
                    }
                    ViewNode::Suspense { fallback, child, .. }
                    | ViewNode::Boundary { fallback, child, .. } => {
                        walk(fallback, names, count);
                        walk(std::slice::from_ref(child), names, count);
                    }
                    ViewNode::Portal { nodes } => walk(nodes, names, count),
                }
            }
        }
        walk(&self.model.nodes, &mut names, &mut count);
        self.row_signal_names = names;
        self.count_signal = count;
    }

    pub fn render(&mut self) {
        self.sync_jobs();
        let mut live = Vec::new();
        for node in &self.model.nodes.clone() {
            self.render_node(node, &mut live);
        }
        self.ids
            .retain(|name, _| live.iter().any(|live| live == name));
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
            ViewNode::For {
                child, row_signals, ..
            } => {
                let signals = row_signals.clone();
                for (index, item) in self.items.clone().into_iter().enumerate() {
                    self.render_row(child, index, &item, &signals, live);
                }
            }
            ViewNode::Slot { nodes, .. } => {
                for child in nodes {
                    self.render_node(child, live);
                }
            }
            ViewNode::Routes { routes } => {
                let path = self
                    .strings
                    .get("route")
                    .cloned()
                    .unwrap_or_else(|| "/".into());
                if let Some((_, nodes)) = routes.iter().find(|(route, _)| route == &path) {
                    for child in nodes {
                        self.render_node(child, live);
                    }
                }
            }
            ViewNode::Suspense {
                jobs,
                transition,
                fallback,
                child,
            } => self.render_gate(jobs, *transition, fallback, child, live),
            ViewNode::Boundary {
                jobs,
                fallback,
                child,
            } => {
                if self.job_phase_any(jobs, "error") {
                    for node in fallback {
                        self.render_node(node, live);
                    }
                } else {
                    self.render_node(child, live);
                }
            }
            ViewNode::Portal { nodes } => {
                for node in nodes {
                    self.render_node(node, live);
                }
            }
        }
    }

    fn render_gate(
        &mut self,
        jobs: &[String],
        transition: bool,
        fallback: &[ViewNode],
        child: &ViewNode,
        live: &mut Vec<String>,
    ) {
        let loading = self.job_phase_any(jobs, "loading");
        let settled = jobs.iter().any(|name| {
            self.strings
                .get(&format!("{name}_value"))
                .is_some_and(|text| !text.is_empty())
        });
        if loading && transition && settled {
            for node in fallback {
                self.render_node(node, live);
            }
            self.render_node(child, live);
        } else if loading {
            for node in fallback {
                self.render_node(node, live);
            }
        } else {
            self.render_node(child, live);
        }
    }

    fn job_phase_any(&self, jobs: &[String], phase: &str) -> bool {
        jobs.iter().any(|name| {
            self.strings
                .get(&format!("{name}_phase"))
                .is_some_and(|have| have == phase)
        })
    }

    /// A resource whose source signal changed goes back to `"loading"` and
    /// keeps the previous value, so a `<Transition>` can show both.
    fn sync_jobs(&mut self) {
        for job in self.model.jobs.clone() {
            let Some(source) = job.source else {
                continue;
            };
            let current = self.signal_text(&source);
            let prev = self
                .strings
                .get(&format!("{}_source", job.name))
                .cloned()
                .unwrap_or_default();
            if current == prev {
                continue;
            }
            self.strings
                .insert(format!("{}_source", job.name), current);
            self.strings
                .insert(format!("{}_phase", job.name), "loading".into());
            let gen = self
                .state
                .get(&format!("{}_gen", job.name))
                .copied()
                .unwrap_or(1);
            self.state.insert(format!("{}_gen", job.name), gen + 1);
        }
    }

    fn signal_text(&self, name: &str) -> String {
        if let Some(text) = self.strings.get(name) {
            return text.clone();
        }
        if let Some(value) = self.state.get(name) {
            return value.to_string();
        }
        String::new()
    }

    pub fn set_job_phase(&mut self, name: &str, phase: &str) {
        self.strings
            .insert(format!("{name}_phase"), phase.to_string());
        self.render();
    }

    /// Finish every job the executor would complete: loading resources, and
    /// actions whose pending flag is set.
    pub fn settle(&mut self) {
        for job in self.model.jobs.clone() {
            let phase = self
                .strings
                .get(&format!("{}_phase", job.name))
                .cloned()
                .unwrap_or_default();
            let pending = self
                .state
                .get(&format!("{}_pending", job.name))
                .copied()
                .unwrap_or(0);
            let start = match job.kind {
                JobKind::Action => pending == 1,
                JobKind::Local | JobKind::Resource => phase == "loading",
            };
            if !start {
                continue;
            }
            if job.err_text.is_empty() {
                self.strings
                    .insert(format!("{}_phase", job.name), "ready".into());
                self.strings
                    .insert(format!("{}_value", job.name), job.ok_text.clone());
                self.strings
                    .insert(format!("{}_error", job.name), String::new());
            } else {
                self.strings
                    .insert(format!("{}_phase", job.name), "error".into());
                self.strings
                    .insert(format!("{}_value", job.name), String::new());
                self.strings
                    .insert(format!("{}_error", job.name), job.err_text.clone());
            }
            if matches!(job.kind, JobKind::Action) {
                self.state.insert(format!("{}_pending", job.name), 0);
            }
        }
        self.render();
    }

    fn render_row(
        &mut self,
        el: &El,
        index: usize,
        item: &ItemSeed,
        signals: &[Signal],
        live: &mut Vec<String>,
    ) {
        let name = format!("{}#{index}", el.name);
        let provides = el.provides.clone();
        let children = el.children.clone();
        let click = el.click.clone();
        let input = el.input.clone();
        self.with_provides(&provides, |sim| {
            let root_text = sim.row_text(el, item, signals);
            sim.place(&name, &root_text, click, input, live);
            for child in &children {
                sim.render_row_child(child, &name, item, signals, live);
            }
        });
    }

    fn render_row_child(
        &mut self,
        node: &ViewNode,
        prefix: &str,
        item: &ItemSeed,
        signals: &[Signal],
        live: &mut Vec<String>,
    ) {
        match node {
            ViewNode::El(el) => {
                let provides = el.provides.clone();
                let children = el.children.clone();
                let click = el.click.clone();
                let input = el.input.clone();
                let path = format!("{prefix}/{}", el.name);
                let row_key = format!("row:{}:{}", item.id, el.name);
                self.with_provides(&provides, |sim| {
                    let text = sim.row_text(el, item, signals);
                    sim.place(&path, &text, click.clone(), input.clone(), live);
                    sim.place(&row_key, &text, click, input, live);
                    for child in &children {
                        sim.render_row_child(child, &path, item, signals, live);
                    }
                });
            }
            ViewNode::Show {
                signal,
                live_flag,
                reset,
                child,
            } => {
                let open = self.row_int(item.id, signal) != 0;
                let was = self
                    .row_live
                    .get(&(item.id, live_flag.clone()))
                    .copied()
                    .unwrap_or(0);
                if open {
                    if was != 1 {
                        self.zero_row_fields(item.id, reset);
                    }
                    self.row_live.insert((item.id, live_flag.clone()), 1);
                    self.render_row_child(child, prefix, item, signals, live);
                } else {
                    self.row_live.insert((item.id, live_flag.clone()), 0);
                    self.zero_row_fields(item.id, reset);
                }
            }
            ViewNode::For { .. } => {}
            ViewNode::Slot { nodes, .. } => {
                for child in nodes {
                    self.render_row_child(child, prefix, item, signals, live);
                }
            }
            ViewNode::Routes { .. } => {}
            ViewNode::Suspense {
                jobs,
                transition,
                fallback,
                child,
            } => {
                let loading = self.job_phase_any(jobs, "loading");
                let settled = jobs.iter().any(|name| {
                    self.strings
                        .get(&format!("{name}_value"))
                        .is_some_and(|text| !text.is_empty())
                });
                if loading && *transition && settled {
                    for node in fallback {
                        self.render_row_child(node, prefix, item, signals, live);
                    }
                    self.render_row_child(child, prefix, item, signals, live);
                } else if loading {
                    for node in fallback {
                        self.render_row_child(node, prefix, item, signals, live);
                    }
                } else {
                    self.render_row_child(child, prefix, item, signals, live);
                }
            }
            ViewNode::Boundary {
                jobs,
                fallback,
                child,
            } => {
                if self.job_phase_any(jobs, "error") {
                    for node in fallback {
                        self.render_row_child(node, prefix, item, signals, live);
                    }
                } else {
                    self.render_row_child(child, prefix, item, signals, live);
                }
            }
            ViewNode::Portal { nodes } => {
                for node in nodes {
                    self.render_row_child(node, prefix, item, signals, live);
                }
            }
        }
    }

    fn row_int(&self, id: i64, signal: &str) -> i64 {
        if self.count_signal.as_deref() == Some(signal) {
            return self.counts.get(&id).copied().unwrap_or(0);
        }
        self.row_vars
            .get(&(id, signal.to_string()))
            .copied()
            .unwrap_or(0)
    }

    fn zero_row_fields(&mut self, id: i64, fields: &[String]) {
        for name in fields {
            if self.count_signal.as_deref() == Some(name.as_str()) {
                self.counts.insert(id, 0);
            } else {
                self.row_vars.insert((id, name.clone()), 0);
                self.row_strings.remove(&(id, name.clone()));
            }
        }
    }

    fn reset_row(&mut self, id: i64) {
        self.counts.insert(id, 0);
        self.row_vars.retain(|(row, _), _| *row != id);
        self.row_strings.retain(|(row, _), _| *row != id);
        self.row_live.retain(|(row, _), _| *row != id);
    }

    fn row_text(&self, el: &El, item: &ItemSeed, signals: &[Signal]) -> String {
        let mut out = String::new();
        for part in &el.parts {
            match part {
                Part::Lit(text) => out.push_str(text),
                Part::Param(name) if name == "label" || name == "item" => out.push_str(&item.label),
                Part::Param(_) => out.push_str(&item.label),
                Part::Sig(name) => {
                    if let Some(text) = self.row_strings.get(&(item.id, name.clone())) {
                        out.push_str(text);
                    } else if signals.iter().any(|signal| signal.read == *name)
                        || self.row_signal_names.iter().any(|signal| signal == name)
                    {
                        out.push_str(&self.row_int(item.id, name).to_string());
                    } else if let Some(text) = self.strings.get(name) {
                        out.push_str(text);
                    } else {
                        out.push_str(&self.state.get(name).copied().unwrap_or(0).to_string());
                    }
                }
                Part::IfBool { signal, yes, no } => {
                    let on = if signals.iter().any(|item| item.read == *signal) {
                        self.row_int(item.id, signal) != 0
                    } else {
                        self.state.get(signal).copied().unwrap_or(0) != 0
                    };
                    out.push_str(if on { yes } else { no });
                }
                Part::IfEmpty { .. } => {}
                Part::Ctx(key) => {
                    if let Some(text) = self.ctx.get(key) {
                        out.push_str(text);
                    }
                }
                Part::Job(name) => {
                    if let Some(text) = self.strings.get(&format!("{name}_value")) {
                        out.push_str(text);
                    }
                }
            }
        }
        out
    }

    fn with_provides<T>(&mut self, provides: &[(String, String)], body: impl FnOnce(&mut Self) -> T) -> T {
        if provides.is_empty() {
            return body(self);
        }
        let saved: Vec<(String, Option<String>)> = provides
            .iter()
            .map(|(key, _)| (key.clone(), self.ctx.get(key).cloned()))
            .collect();
        for (key, expr) in provides {
            self.ctx.insert(key.clone(), ctx_literal(expr));
        }
        let value = body(self);
        for (key, prev) in saved.into_iter().rev() {
            match prev {
                Some(text) => {
                    self.ctx.insert(key, text);
                }
                None => {
                    self.ctx.remove(&key);
                }
            }
        }
        value
    }

    fn render_el(&mut self, el: &El, live: &mut Vec<String>) {
        let provides = el.provides.clone();
        self.with_provides(&provides, |sim| {
            let text = sim.plain_text(&el.parts, el.value.as_deref());
            sim.place(&el.name, &text, el.click.clone(), el.input.clone(), live);
            for child in &el.children {
                sim.render_node(child, live);
            }
        });
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
                Part::Param(name) if name == "arg" => out.push_str("arg"),
                Part::Param(_) => {}
                Part::Ctx(key) => {
                    if let Some(text) = self.ctx.get(key) {
                        out.push_str(text);
                    }
                }
                Part::Job(name) => {
                    if let Some(text) = self.strings.get(&format!("{name}_value")) {
                        out.push_str(text);
                    }
                }
            }
        }
        out
    }

    pub fn id(&self, name: &str) -> Option<u64> {
        self.ids.get(name).map(|slot| slot.id)
    }

    pub fn labels(&self) -> Vec<(String, String)> {
        let mut rows: Vec<_> = self
            .ids
            .iter()
            .map(|(name, slot)| (name.clone(), slot.text.clone()))
            .collect();
        rows.sort();
        rows
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
        let row = name
            .strip_prefix("row:")
            .and_then(|rest| rest.split(':').next().and_then(|id| id.parse::<i64>().ok()));
        self.apply(act, row)
    }

    pub fn click_row(&mut self, id: i64) -> bool {
        let count = self.count_signal.clone();
        let key = self
            .ids
            .iter()
            .find(|(name, slot)| {
                name.starts_with(&format!("row:{id}:"))
                    && slot
                        .click
                        .as_ref()
                        .is_some_and(|act| bumps_row_count(act, count.as_deref()))
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
        let signal = self.ids.get(name).and_then(|slot| slot.input.clone());
        let Some(signal) = signal else {
            return false;
        };
        self.strings.insert(signal, value.to_string());
        self.render();
        true
    }

    pub fn reload(&mut self, model: Model) {
        self.model = model;
        self.refresh_row_meta();
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
                    self.reset_row(id);
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
            Act::Body(stmts) => self.exec(&stmts, None),
            Act::Row(stmts) => {
                let Some(id) = row else {
                    return false;
                };
                if !self.items.iter().any(|item| item.id == id) {
                    return false;
                }
                self.exec(&stmts, Some(id));
            }
        }
        self.render();
        true
    }

    fn exec(&mut self, stmts: &[HandlerStmt], row: Option<i64>) {
        let mut locals = HashMap::new();
        for stmt in stmts {
            match stmt {
                HandlerStmt::Let { name, expr } => {
                    let value = self.eval(expr, row, &locals);
                    locals.insert(name.clone(), value);
                }
                HandlerStmt::Set { signal, expr } => {
                    let value = self.eval(expr, row, &locals);
                    self.write_signal(signal, value, row);
                }
                HandlerStmt::Push { items, id, label } => {
                    if self.item_signals.iter().any(|name| name == items) {
                        let id = self.eval(id, row, &locals).as_int();
                        let label = self.eval(label, row, &locals).as_string();
                        self.reset_row(id);
                        self.items.push(ItemSeed { id, label });
                    }
                }
                HandlerStmt::Pop { items } => {
                    if self.item_signals.iter().any(|name| name == items) {
                        self.items.pop();
                    }
                }
                HandlerStmt::Reverse { items } => {
                    if self.item_signals.iter().any(|name| name == items) {
                        self.items.reverse();
                    }
                }
                HandlerStmt::RemoveKey { items, key } => {
                    if self.item_signals.iter().any(|name| name == items) {
                        let key = self.eval(key, row, &locals).as_int();
                        self.items.retain(|item| item.id != key);
                    }
                }
                HandlerStmt::SwapKey { items, a, b } => {
                    if self.item_signals.iter().any(|name| name == items) {
                        let a = self.eval(a, row, &locals).as_int();
                        let b = self.eval(b, row, &locals).as_int();
                        if let (Some(left), Some(right)) = (
                            self.items.iter().position(|item| item.id == a),
                            self.items.iter().position(|item| item.id == b),
                        ) {
                            self.items.swap(left, right);
                        }
                    }
                }
                HandlerStmt::Navigate(expr) => {
                    let path = self.eval(expr, row, &locals).as_string();
                    self.strings.insert("route".into(), path);
                }
                HandlerStmt::Dispatch { action, arg } => {
                    let arg = self.eval(arg, row, &locals).as_string();
                    self.strings.insert(format!("{action}_arg"), arg);
                    self.state.insert(format!("{action}_pending"), 1);
                    let gen = self
                        .state
                        .get(&format!("{action}_gen"))
                        .copied()
                        .unwrap_or(1);
                    self.state.insert(format!("{action}_gen"), gen + 1);
                }
                HandlerStmt::ToggleTheme => {
                    let dark = self.state.get("quinn_dark").copied().unwrap_or(0);
                    self.state.insert("quinn_dark".into(), if dark == 0 { 1 } else { 0 });
                }
            }
        }
    }

    fn write_signal(&mut self, name: &str, value: Val, row: Option<i64>) {
        if let Some(id) = row {
            if self.row_signal_names.iter().any(|signal| signal == name) {
                if self.count_signal.as_deref() == Some(name) {
                    self.counts.insert(id, value.as_int());
                } else if let Val::Str(text) = value {
                    self.row_strings.insert((id, name.to_string()), text);
                } else {
                    self.row_vars.insert((id, name.to_string()), value.as_int());
                }
                return;
            }
        }
        match value {
            Val::Str(text) => {
                self.strings.insert(name.to_string(), text);
            }
            other => {
                self.state.insert(name.to_string(), other.as_int());
            }
        }
    }

    fn eval(&self, expr: &HxExpr, row: Option<i64>, locals: &HashMap<String, Val>) -> Val {
        match expr {
            HxExpr::Int(value) => Val::Int(*value),
            HxExpr::Bool(value) => Val::Int(if *value { 1 } else { 0 }),
            HxExpr::Str(value) => Val::Str(value.clone()),
            HxExpr::Name(name) => {
                if let Some(value) = locals.get(name) {
                    return value.clone();
                }
                if name == "id" {
                    if let Some(id) = row {
                        return Val::Int(id);
                    }
                }
                if let Some(id) = row {
                    if self.row_signal_names.iter().any(|signal| signal == name) {
                        if let Some(text) = self.row_strings.get(&(id, name.clone())) {
                            return Val::Str(text.clone());
                        }
                        return Val::Int(self.row_int(id, name));
                    }
                }
                if let Some(text) = self.strings.get(name) {
                    return Val::Str(text.clone());
                }
                Val::Int(self.state.get(name).copied().unwrap_or(0))
            }
            HxExpr::Not(inner) => {
                let value = self.eval(inner, row, locals);
                Val::Int(if value.as_int() == 0 { 1 } else { 0 })
            }
            HxExpr::Binary(op, left, right) => {
                let left = self.eval(left, row, locals).as_int();
                let right = self.eval(right, row, locals).as_int();
                let value = match op {
                    BinOp::Add => left + right,
                    BinOp::Sub => left - right,
                    BinOp::Mul => left * right,
                    BinOp::Div => {
                        if right == 0 {
                            0
                        } else {
                            left / right
                        }
                    }
                };
                Val::Int(value)
            }
            HxExpr::Concat(parts) => Val::Str(
                parts
                    .iter()
                    .map(|part| self.eval(part, row, locals).as_string())
                    .collect(),
            ),
        }
    }
}

#[derive(Clone, Debug)]
enum Val {
    Int(i64),
    Str(String),
}

impl Val {
    fn as_int(&self) -> i64 {
        match self {
            Self::Int(value) => *value,
            Self::Str(value) => value.parse().unwrap_or(0),
        }
    }

    fn as_string(&self) -> String {
        match self {
            Self::Int(value) => value.to_string(),
            Self::Str(value) => value.clone(),
        }
    }
}

fn bumps_row_count(act: &Act, count: Option<&str>) -> bool {
    let Some(count) = count else {
        return matches!(act, Act::RowInc);
    };
    match act {
        Act::RowInc => true,
        Act::Row(stmts) => stmts.iter().any(|stmt| match stmt {
            HandlerStmt::Set { signal, expr } if signal == count => is_add_one(expr, signal),
            _ => false,
        }),
        _ => false,
    }
}

fn is_add_one(expr: &HxExpr, signal: &str) -> bool {
    matches!(
        expr,
        HxExpr::Binary(BinOp::Add, left, right)
            if matches!(left.as_ref(), HxExpr::Name(name) if name == signal)
                && matches!(right.as_ref(), HxExpr::Int(1))
    )
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
            ViewNode::Slot { nodes, .. } => {
                if remove_named(nodes, name) {
                    return true;
                }
            }
            ViewNode::Routes { routes } => {
                for (_, nodes) in routes {
                    if remove_named(nodes, name) {
                        return true;
                    }
                }
            }
            ViewNode::Suspense { fallback, child, .. }
            | ViewNode::Boundary { fallback, child, .. } => {
                if remove_named(fallback, name) {
                    return true;
                }
                if let ViewNode::El(el) = child.as_mut() {
                    if el.name == name {
                        return false;
                    }
                    if remove_named(&mut el.children, name) {
                        return true;
                    }
                }
            }
            ViewNode::Portal { nodes } => {
                if remove_named(nodes, name) {
                    return true;
                }
            }
        }
    }
    false
}

fn flatten_obj(
    prefix: &str,
    fields: &[(String, Init)],
    strings: &mut HashMap<String, String>,
    state: &mut HashMap<String, i64>,
) {
    for (name, init) in fields {
        let key = format!("{prefix}.{name}");
        match init {
            Init::Str(value) => {
                strings.insert(key, value.clone());
            }
            Init::Int(value) => {
                state.insert(key, *value);
            }
            Init::Bool(value) => {
                state.insert(key, if *value { 1 } else { 0 });
            }
            Init::Obj(inner) => flatten_obj(&key, inner, strings, state),
            Init::Items(_) => {}
        }
    }
}

fn ctx_literal(expr: &str) -> String {
    let expr = expr.trim();
    if expr.len() >= 2 && expr.starts_with('"') && expr.ends_with('"') {
        return expr[1..expr.len() - 1].to_string();
    }
    expr.to_string()
}

