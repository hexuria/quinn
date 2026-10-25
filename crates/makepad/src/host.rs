use std::cell::Cell;
use std::collections::HashMap;

use makepad_widgets::makepad_script::ScriptMod;
use makepad_widgets::*;
use quinn_core::{IntoView, NodeSpec, Owner, Runtime, Style, Surface, Tag, UiEvent};
use quinn_styles::{AlignSpec, FlowSpec, SizeSpec};

use crate::{session_slot, Session};

std::thread_local! {
    static CX: Cell<*mut Cx> = const { Cell::new(std::ptr::null_mut()) };
}

/// Publish `cx` for surface calls made on this thread.
///
/// # Safety invariant
/// The pointer is only dereferenced while `cx` is borrowed by this call, on
/// the same thread. It is cleared before `with_cx` returns.
pub fn with_cx<R>(cx: &mut Cx, body: impl FnOnce() -> R) -> R {
    CX.with(|slot| slot.set(cx));
    let result = body();
    CX.with(|slot| slot.set(std::ptr::null_mut()));
    result
}

fn cx() -> &'static mut Cx {
    CX.with(|slot| {
        let ptr = slot.get();
        assert!(
            !ptr.is_null(),
            "Makepad surface method called outside the UI thread"
        );
        unsafe { &mut *ptr }
    })
}

struct Slot {
    widget: WidgetRef,
    parent: WidgetRef,
    tag: Tag,
}

pub struct MakepadSurface {
    host: WidgetRef,
    widgets: HashMap<u64, Slot>,
}

impl MakepadSurface {
    pub fn new(host: WidgetRef) -> Self {
        Self {
            host,
            widgets: HashMap::new(),
        }
    }

    pub fn collect(&self, actions: &Actions) -> Vec<UiEvent> {
        let mut events = Vec::new();
        for (id, slot) in &self.widgets {
            match slot.tag {
                Tag::Button => {
                    if slot.widget.as_button().clicked(actions) {
                        events.push(UiEvent::Click(*id));
                    }
                }
                Tag::Input => {
                    if let Some(text) = slot.widget.as_text_input().changed(actions) {
                        events.push(UiEvent::Input(*id, text));
                    }
                }
                _ => {}
            }
        }
        events
    }
}

impl Surface for MakepadSurface {
    fn create(&mut self, parent: Option<u64>, spec: &NodeSpec) -> u64 {
        let parent_widget = match parent {
            Some(id) => self
                .widgets
                .get(&id)
                .map(|slot| slot.widget.clone())
                .unwrap_or_else(|| self.host.clone()),
            None => self.host.clone(),
        };
        let widget = spawn(spec);
        if widget.is_empty() {
            panic!(
                "Makepad did not construct a {:?} widget from script. The prototype path is mod.widgets.*",
                spec.tag
            );
        }
        let id = widget.widget_uid().0;
        if id == 0 {
            panic!("constructed {:?} widget has uid 0", spec.tag);
        }
        attach(&parent_widget, &widget);
        apply_style(&widget, spec.tag, &spec.style);
        if !spec.text.is_empty() {
            widget.set_text(cx(), &spec.text);
        }
        if spec.tag == Tag::Input && !spec.placeholder.is_empty() {
            widget
                .as_text_input()
                .set_empty_text(cx(), spec.placeholder.clone());
        }
        widget.redraw(cx());
        self.widgets.insert(
            id,
            Slot {
                widget,
                parent: parent_widget,
                tag: spec.tag,
            },
        );
        id
    }

    fn remove(&mut self, id: u64) {
        let Some(slot) = self.widgets.remove(&id) else {
            return;
        };
        if let Some(mut view) = slot.parent.as_view().borrow_mut() {
            view.children
                .retain(|(name, widget)| name.0 != id && widget.widget_uid().0 != id);
        }
        slot.parent.redraw(cx());
        drop(slot);
    }

    fn set_text(&mut self, id: u64, text: &str) {
        let Some(slot) = self.widgets.get(&id) else {
            return;
        };
        if slot.widget.text() == text {
            return;
        }
        slot.widget.set_text(cx(), text);
        slot.widget.redraw(cx());
    }

    fn text(&self, id: u64) -> String {
        self.widgets
            .get(&id)
            .map(|slot| slot.widget.text())
            .unwrap_or_default()
    }

    fn set_style(&mut self, id: u64, style: &Style) {
        let Some(slot) = self.widgets.get(&id) else {
            return;
        };
        apply_style(&slot.widget, slot.tag, style);
        slot.widget.redraw(cx());
    }

    fn set_placeholder(&mut self, id: u64, text: &str) {
        let Some(slot) = self.widgets.get(&id) else {
            return;
        };
        slot.widget
            .as_text_input()
            .set_empty_text(cx(), text.to_string());
    }

    fn set_order(&mut self, parent: u64, children: &[u64]) {
        let Some(parent_slot) = self.widgets.get(&parent) else {
            return;
        };
        let parent_widget = parent_slot.widget.clone();
        let ordered: Vec<(LiveId, WidgetRef)> = children
            .iter()
            .filter_map(|id| {
                self.widgets
                    .get(id)
                    .map(|slot| (LiveId(*id), slot.widget.clone()))
            })
            .collect();
        if let Some(mut view) = parent_widget.as_view().borrow_mut() {
            view.children.clear();
            view.children.extend(ordered);
        }
        parent_widget.redraw(cx());
    }

    fn redraw(&mut self) {
        cx().redraw_all();
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn spawn(spec: &NodeSpec) -> WidgetRef {
    let text = script_string(&spec.text);
    let placeholder = script_string(&spec.placeholder);
    let code = match spec.tag {
        Tag::Div => {
            "mod.widgets.RoundedView{width:Fill height:Fit flow:Down show_bg:false new_batch:true}"
                .to_string()
        }
        Tag::Button => format!("mod.widgets.Button{{text:{text} width:Fit height:Fit}}"),
        Tag::Input => format!(
            "mod.widgets.TextInput{{text:{text} empty_text:{placeholder} width:Fill height:36.}}"
        ),
        Tag::H1 => format!("mod.widgets.H1{{text:{text} width:Fill height:Fit}}"),
        Tag::H2 => format!("mod.widgets.H2{{text:{text} width:Fill height:Fit}}"),
        Tag::Span | Tag::P | Tag::Label => {
            format!("mod.widgets.Label{{text:{text} width:Fit height:Fit}}")
        }
    };
    eval_widget(&code)
}

fn eval_widget(code: &str) -> WidgetRef {
    cx().with_vm(|vm| {
        let value = vm.eval(script_mod(code));
        WidgetRef::script_from_value(vm, value)
    })
}

fn script_mod(code: &str) -> ScriptMod {
    ScriptMod {
        cargo_manifest_path: String::new(),
        module_path: "quinn".into(),
        file: "quinn.script".into(),
        line: 1,
        column: 1,
        // A fresh `ScriptVm::eval` body does not inherit the app module's
        // `use mod.prelude.widgets.*`, so `Fill`, `Fit`, and `Down` are
        // missing unless this import is in the evaluated source.
        code: format!("use mod.prelude.widgets.*\n{code};"),
        values: Vec::new(),
    }
}

fn script_string(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn attach(parent: &WidgetRef, child: &WidgetRef) {
    let id = child.widget_uid();
    if let Some(mut view) = parent.as_view().borrow_mut() {
        view.children.push((LiveId(id.0), child.clone()));
    } else {
        panic!("parent widget is not a View; only containers can take children");
    }
    cx().widget_tree_insert_child(parent.widget_uid(), LiveId(id.0), child.clone());
}

fn apply_style(widget: &WidgetRef, tag: Tag, style: &Style) {
    match tag {
        Tag::Div => apply_view(widget, style),
        Tag::Button | Tag::Input => apply_control(widget, style),
        _ => apply_label(widget, style),
    }
}

fn apply_label(widget: &WidgetRef, style: &Style) {
    if let Some(mut label) = widget.as_label().borrow_mut() {
        if let Some(color) = style.color {
            let (r, g, b, a) = color.channels();
            label.set_text_color(cx(), vec4(r, g, b, a));
        }
        if let Some(size) = style.font_size {
            label.draw_text.text_style.font_size = size as f32;
        }
        apply_walk(&mut label.walk, style);
    }
    if style.color.is_some() || style.font_size.is_some() {
        patch(widget, &label_script(style));
    }
}

fn apply_view(widget: &WidgetRef, style: &Style) {
    if let Some(mut view) = widget.as_view().borrow_mut() {
        let flow = style.flow.unwrap_or(FlowSpec::Column);
        view.layout.flow = match flow {
            FlowSpec::Column => Flow::Down,
            FlowSpec::Row => Flow::right(),
        };
        if let Some(gap) = style.gap {
            view.layout.spacing = gap;
        }
        let (left, top, right, bottom) = style.padding.resolve();
        view.layout.padding = Inset {
            left,
            top,
            right,
            bottom,
        };
        let main = style.main_align.unwrap_or(AlignSpec::Start).fraction();
        let cross = style.cross_align.unwrap_or(AlignSpec::Start).fraction();
        let (x, y) = match flow {
            FlowSpec::Column => (cross, main),
            FlowSpec::Row => (main, cross),
        };
        view.layout.align = Align { x, y };
        apply_walk(&mut view.walk, style);
        let painted = style.background.is_some() || style.border_width.unwrap_or(0.0) > 0.0;
        view.show_bg = painted;
        view.new_batch = painted;
    }
    if style.background.is_some()
        || style.border_width.is_some()
        || style.border_color.is_some()
        || style.radius.is_some()
    {
        patch(widget, &chrome_script(style));
    }
}

fn apply_control(widget: &WidgetRef, style: &Style) {
    patch(widget, &control_script(style));
}

fn apply_walk(walk: &mut Walk, style: &Style) {
    if let Some(width) = style.width {
        walk.width = size_of(width);
    }
    if let Some(height) = style.height {
        walk.height = size_of(height);
    }
    if style.margin.left.is_some()
        || style.margin.top.is_some()
        || style.margin.right.is_some()
        || style.margin.bottom.is_some()
    {
        let (left, top, right, bottom) = style.margin.resolve();
        walk.margin = Inset {
            left,
            top,
            right,
            bottom,
        };
    }
}

fn size_of(size: SizeSpec) -> Size {
    match size {
        SizeSpec::Fill => Size::fill(),
        SizeSpec::Fit => Size::fit(),
        SizeSpec::Px(px) => Size::Fixed(px),
    }
}

fn patch(widget: &WidgetRef, body: &str) {
    if body.is_empty() {
        return;
    }
    let code = format!("__script_source__{{{body}}}");
    let mut widget = widget.clone();
    cx().with_vm(|vm| {
        widget.script_apply_eval(vm, script_mod(&code));
    });
}

fn label_script(style: &Style) -> String {
    let mut parts = Vec::new();
    if let Some(color) = style.color {
        let hex = color.hex_script();
        parts.push(format!("draw_text.color: {hex}"));
    }
    if let Some(size) = style.font_size {
        parts.push(format!("draw_text.text_style.font_size: {size}"));
    }
    parts.join(" ")
}

fn chrome_script(style: &Style) -> String {
    let mut parts = Vec::new();
    if let Some(color) = style.background {
        parts.push(format!("draw_bg.color: {}", color.hex_script()));
    }
    if let Some(width) = style.border_width {
        parts.push(format!("draw_bg.border_size: {width}"));
    }
    if let Some(color) = style.border_color {
        parts.push(format!("draw_bg.border_color: {}", color.hex_script()));
    }
    if let Some(radius) = style.radius {
        parts.push(format!("draw_bg.border_radius: {radius}"));
    }
    parts.join(" ")
}

fn control_script(style: &Style) -> String {
    let mut parts = Vec::new();
    if let Some(color) = style.background {
        let hex = color.hex_script();
        parts.push(format!(
            "draw_bg.color: {hex} draw_bg.color_hover: {hex} draw_bg.color_down: {hex} draw_bg.color_focus: {hex}"
        ));
    }
    if let Some(color) = style.color {
        let hex = color.hex_script();
        parts.push(format!(
            "draw_text.color: {hex} draw_text.color_hover: {hex} draw_text.color_down: {hex} draw_text.color_focus: {hex}"
        ));
    }
    if let Some(size) = style.font_size {
        parts.push(format!("draw_text.text_style.font_size: {size}"));
    }
    if let Some(radius) = style.radius {
        parts.push(format!("draw_bg.border_radius: {radius}"));
    }
    if let Some(width) = style.border_width {
        parts.push(format!("draw_bg.border_size: {width}"));
    }
    if let Some(color) = style.border_color {
        parts.push(format!("draw_bg.border_color: {}", color.hex_script()));
    }
    let (left, top, right, bottom) = style.padding.resolve();
    if style.padding.left.is_some()
        || style.padding.top.is_some()
        || style.padding.right.is_some()
        || style.padding.bottom.is_some()
    {
        // A bare object is not an `Inset`. A single number applies to every
        // side; unequal sides need the `Inset` constructor from the prelude.
        if left == top && top == right && right == bottom {
            parts.push(format!("padding: {left}"));
        } else {
            parts.push(format!(
                "padding: Inset{{left: {left} top: {top} right: {right} bottom: {bottom}}}"
            ));
        }
    }
    // `#[walk]` flattens `walk.width` to the widget property `width`.
    match style.width {
        Some(SizeSpec::Fill) => parts.push("width: Fill".into()),
        Some(SizeSpec::Fit) => parts.push("width: Fit".into()),
        Some(SizeSpec::Px(px)) => parts.push(format!("width: {px}")),
        None => {}
    }
    match style.height {
        Some(SizeSpec::Fill) => parts.push("height: Fill".into()),
        Some(SizeSpec::Fit) => parts.push("height: Fit".into()),
        Some(SizeSpec::Px(px)) => parts.push(format!("height: {px}")),
        None => {}
    }
    parts.join(" ")
}

pub fn startup<V: IntoView>(cx: &mut Cx, ui: &WidgetRef, build: impl FnOnce() -> V) {
    let host = ui.view(cx, ids!(host));
    if host.is_empty() {
        panic!("quinn host view was not created; the shell script did not register `host`");
    }
    let owner = Owner::new();
    let view = owner.with(|| build().into_view());
    let mut runtime = Runtime::new(Box::new(MakepadSurface::new((*host).clone())));
    with_cx(cx, || {
        runtime.mount_view(view);
    });
    session_slot().with(|slot| {
        *slot.borrow_mut() = Some(Session { runtime, owner });
    });
    if let Ok(path) = std::env::var("QUINN_READY") {
        let _ = std::fs::write(path, "ready\n");
    }
    if std::env::var("QUINN_EXIT").ok().as_deref() == Some("1") {
        cx.quit();
    }
}

pub fn on_actions(cx: &mut Cx, actions: &Actions) {
    with_cx(cx, || {
        let events = session_slot().with(|slot| {
            let session = slot.borrow();
            let session = session.as_ref()?;
            let surface = session
                .runtime
                .surface_any()
                .downcast_ref::<MakepadSurface>()?;
            Some(surface.collect(actions))
        });
        let Some(events) = events else {
            return;
        };
        session_slot().with(|slot| {
            if let Some(session) = slot.borrow_mut().as_mut() {
                for event in events {
                    session.runtime.dispatch(event);
                }
                session.runtime.flush();
            }
        });
    });
}
