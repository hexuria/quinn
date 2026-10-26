use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use makepad_widgets::makepad_platform::SignalToUI;
use makepad_widgets::makepad_script::ScriptMod;
use makepad_widgets::*;
use quinn_core::{IntoView, NodeSpec, Owner, Runtime, Style, Surface, Tag, UiEvent};
use quinn_lower::JobValue;
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

impl MakepadSurface {
    pub(crate) fn widget_count(&self) -> usize {
        self.widgets.len()
    }

    pub(crate) fn host_child_count(&self) -> usize {
        self.host
            .as_view()
            .borrow()
            .map(|view| view.children.len())
            .unwrap_or(0)
    }

    pub(crate) fn widget_texts(&self) -> Vec<String> {
        let mut texts: Vec<String> = self
            .widgets
            .values()
            .map(|slot| slot.widget.text())
            .filter(|text| !text.is_empty())
            .collect();
        texts.sort();
        texts
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

pub(crate) fn flush_session() {
    session_slot().with(|slot| {
        if let Some(session) = slot.borrow_mut().as_mut() {
            session.runtime.flush();
        }
    });
}

pub(crate) fn session_snapshot() -> (usize, usize, usize, Vec<String>) {
    session_slot().with(|slot| {
        let borrowed = slot.borrow();
        let Some(session) = borrowed.as_ref() else {
            return (0, 0, 0, Vec::new());
        };
        let nodes = session.runtime.node_count();
        let Some(surface) = session
            .runtime
            .surface_any()
            .downcast_ref::<MakepadSurface>()
        else {
            return (nodes, 0, 0, Vec::new());
        };
        (
            nodes,
            surface.widget_count(),
            surface.host_child_count(),
            surface.widget_texts(),
        )
    })
}

/// Create and update `n` Makepad labels on `host`, then detach them.
///
/// Returns `(create_us, update_us)`. This is the renderer cost, not the
/// signal flush. The baseline repeats the same script and `set_text` calls.
pub(crate) fn bench_labels(host: &WidgetRef, n: usize) -> (u128, u128) {
    let base = host
        .as_view()
        .borrow()
        .map(|view| view.children.len())
        .unwrap_or(0);
    let started = std::time::Instant::now();
    let mut created = Vec::with_capacity(n);
    for i in 0..n {
        let widget = eval_widget(&format!(
            "mod.widgets.Label{{text:\"row {i}\" width:Fit height:Fit}}"
        ));
        if widget.is_empty() {
            continue;
        }
        attach(host, &widget);
        created.push(widget);
    }
    let create_us = started.elapsed().as_micros();
    let started = std::time::Instant::now();
    for (i, widget) in created.iter().enumerate() {
        widget.set_text(cx(), &format!("upd {i}"));
    }
    let update_us = started.elapsed().as_micros();
    if let Some(mut view) = host.as_view().borrow_mut() {
        view.children.truncate(base);
    }
    drop(created);
    host.redraw(cx());
    (create_us, update_us)
}

/// Run script statements with `state` bound to `mod.state`.
pub(crate) fn eval_statements(cx: &mut Cx, statements: &str) {
    let code = format!("let state = mod.state\n{statements}\ntrue");
    cx.with_vm(|vm| {
        vm.eval(script_mod(&code));
    });
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
            label.draw_text.text_style.font_size = quinn_styles::font_size_pts(size) as f32;
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
            FlowSpec::Row => {
                let row_align = match style.cross_align {
                    Some(quinn_styles::CrossAlign::Center) => turtle::RowAlign::Center,
                    Some(quinn_styles::CrossAlign::End) => turtle::RowAlign::Bottom,
                    _ => turtle::RowAlign::Top,
                };
                Flow::Right {
                    wrap: style.wrap,
                    row_align,
                }
            }
        };
        let spacing = match flow {
            FlowSpec::Row => style.main_gap(),
            FlowSpec::Column => style.cross_gap(),
        };
        if let Some(gap) = spacing {
            view.layout.spacing = gap;
        }
        if style.wrap {
            if let Some(gap) = style.cross_gap() {
                view.layout.wrap_spacing = gap;
            }
        }
        let (left, top, right, bottom) = style.padding.resolve();
        view.layout.padding = Inset {
            left,
            top,
            right,
            bottom,
        };
        if let Some(dist) = style.distribute {
            view.layout.distribute = match dist {
                quinn_styles::DistributeSpec::Between => Distribute::SpaceBetween,
                quinn_styles::DistributeSpec::Around => Distribute::SpaceAround,
                quinn_styles::DistributeSpec::Evenly => Distribute::SpaceEvenly,
            };
        }
        let main = style.main_align.unwrap_or(AlignSpec::Start).fraction();
        let cross = match style.cross_align {
            Some(quinn_styles::CrossAlign::Center) => 0.5,
            Some(quinn_styles::CrossAlign::End) => 1.0,
            _ => 0.0,
        };
        let (x, y) = match flow {
            FlowSpec::Column => (cross, main),
            FlowSpec::Row => (main, 0.0),
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
    if let Some(bound) = style.min_width {
        walk.min_width = Some(bound_of(bound));
    }
    if let Some(bound) = style.max_width {
        walk.max_width = Some(bound_of(bound));
    }
    if let Some(bound) = style.min_height {
        walk.min_height = Some(bound_of(bound));
    }
    if let Some(bound) = style.max_height {
        walk.max_height = Some(bound_of(bound));
    }
}

fn bound_of(bound: quinn_styles::Bound) -> FitBound {
    match bound {
        quinn_styles::Bound::Px(px) => FitBound::Abs(px),
        quinn_styles::Bound::Full => FitBound::Rel {
            base: Base::Full,
            factor: 1.0,
        },
    }
}

fn size_of(size: SizeSpec) -> Size {
    match size {
        SizeSpec::Fill => Size::fill(),
        SizeSpec::Fit => Size::fit(),
        SizeSpec::Px(px) => Size::Fixed(px),
        SizeSpec::Rel { base, factor } => Size::Rel {
            base: match base {
                quinn_styles::RelBase::Parent => Base::Parent,
                quinn_styles::RelBase::Vw => Base::Vw,
                quinn_styles::RelBase::Vh => Base::Vh,
            },
            factor,
        },
        SizeSpec::Control => Size::Fixed(36.0),
        SizeSpec::Bar => Size::Fixed(48.0),
        SizeSpec::Flex { weight, shrink, basis } => Size::Fill {
            weight,
            basis: FitBound::Abs(basis),
            shrink,
            min: None,
            max: None,
        },
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
        parts.push(format!(
            "draw_text.text_style.font_size: {}",
            quinn_styles::font_size_pts(size)
        ));
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
        parts.push(format!(
            "draw_bg.border_radius: {}",
            quinn_styles::sdf_radius(radius)
        ));
    }
    parts.join(" ")
}

fn control_script(style: &Style) -> String {
    let mut parts = Vec::new();
    if let Some(color) = style.background {
        let hex = color.hex_script();
        parts.push(format!("draw_bg.color: {hex}"));
        let hover = style.hover_bg.unwrap_or(color).hex_script();
        let down = style.active_bg.unwrap_or(color).hex_script();
        let focus = style.focus_bg.unwrap_or(color).hex_script();
        let disabled = style.disabled_bg.unwrap_or(color).hex_script();
        parts.push(format!("draw_bg.color_hover: {hover}"));
        parts.push(format!("draw_bg.color_down: {down}"));
        parts.push(format!("draw_bg.color_focus: {focus}"));
        parts.push(format!("draw_bg.color_disabled: {disabled}"));
    }
    if let Some(color) = style.color {
        let hex = color.hex_script();
        parts.push(format!("draw_text.color: {hex}"));
        let hover = style.hover_fg.unwrap_or(color).hex_script();
        let down = style.active_fg.unwrap_or(color).hex_script();
        let focus = style.focus_fg.unwrap_or(color).hex_script();
        let disabled = style.disabled_fg.unwrap_or(color).hex_script();
        parts.push(format!("draw_text.color_hover: {hover}"));
        parts.push(format!("draw_text.color_down: {down}"));
        parts.push(format!("draw_text.color_focus: {focus}"));
        parts.push(format!("draw_text.color_disabled: {disabled}"));
    }
    // Flat face. The theme outset gradient and pressed mix are what turned a
    // clicked button pale grey with an unreadable label.
    parts.push("draw_bg.color_2: vec4(-1., -1., -1., -1.)".into());
    parts.push("draw_bg.border_color_2: vec4(-1., -1., -1., -1.)".into());
    parts.push("draw_bg.border_color_focus: #x1d4ed8".into());
    if !style.margin.any() {
        parts.push("margin: 0.".into());
    }
    if let Some(size) = style.font_size {
        parts.push(format!(
            "draw_text.text_style.font_size: {}",
            quinn_styles::font_size_pts(size)
        ));
    }
    if let Some(radius) = style.radius {
        parts.push(format!(
            "draw_bg.border_radius: {}",
            quinn_styles::sdf_radius(radius)
        ));
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
        Some(size) => parts.push(format!("width: {}", size_script_host(size))),
        None => {}
    }
    match style.height {
        Some(size) => parts.push(format!("height: {}", size_script_host(size))),
        None => {}
    }
    parts.join(" ")
}

fn size_script_host(size: SizeSpec) -> String {
    match size {
        SizeSpec::Fill => "Fill".into(),
        SizeSpec::Fit => "Fit".into(),
        SizeSpec::Px(px) => px.to_string(),
        SizeSpec::Rel { base, factor } => {
            let base = match base {
                quinn_styles::RelBase::Parent => "Base.Parent",
                quinn_styles::RelBase::Vw => "Base.Vw",
                quinn_styles::RelBase::Vh => "Base.Vh",
            };
            format!("Rel{{base: {base} factor: {factor}}}")
        }
        SizeSpec::Flex { weight, shrink, basis } => {
            format!("Fill{{weight: {weight} shrink: {shrink} basis: {basis}}}")
        }
        SizeSpec::Control => "36".into(),
        SizeSpec::Bar => "48".into(),
    }
}

/// Mark-sweep the Makepad script heap and return spare capacity to the
/// allocator. Routine `ScriptVm::gc` (the paint loop) refills free lists and
/// deliberately keeps the high-water allocation. Widget mounts go through
/// `ScriptVm::eval`, which allocates a scope object each time; this is the
/// call that releases those objects once the widgets are gone.
pub fn reclaim_script_heap() {
    cx().with_vm(|vm| vm.gc_and_compact());
    #[cfg(target_os = "linux")]
    unsafe {
        malloc_trim(0);
    }
}

#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn malloc_trim(pad: usize) -> i32;
}

/// Read the `Cx` published by [`with_cx`] for the current UI-thread call.
pub fn with_current_cx<R>(body: impl FnOnce(&mut Cx) -> R) -> R {
    body(cx())
}

/// Widgets under a script host, counted by walking `View::children`.
///
/// The script path does not mount a Rust [`quinn_core::Runtime`], so
/// [`crate::session_stats`] stays at zero there. This census is the count
/// the retain probe prints.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WidgetCensus {
    pub widgets: usize,
    pub buttons: usize,
    pub host_children: usize,
}

pub fn census_host(host: &WidgetRef) -> WidgetCensus {
    let mut census = WidgetCensus::default();
    let children = view_children(host);
    census.host_children = children.len();
    for child in children {
        walk_census(&child, &mut census);
    }
    census
}

fn view_children(widget: &WidgetRef) -> Vec<WidgetRef> {
    widget
        .as_view()
        .borrow()
        .map(|view| view.children.iter().map(|(_, child)| child.clone()).collect())
        .unwrap_or_default()
}

fn walk_census(widget: &WidgetRef, census: &mut WidgetCensus) {
    census.widgets += 1;
    if widget.borrow::<Button>().is_some() {
        census.buttons += 1;
    }
    for child in view_children(widget) {
        walk_census(&child, census);
    }
}

/// Eval `hot/ui.rs` as the app's `script_mod`.
///
/// The file path, line, and column are the key Makepad's `--hot` watcher
/// uses. The code string is the same normalization the watcher applies, so a
/// later edit of this file overrides this body instead of being rejected as a
/// different block.
pub fn eval_hot_file(
    vm: &mut ScriptVm,
    path: &std::path::Path,
    manifest_dir: &str,
    module_path: &str,
    component: ScriptValue,
) -> ScriptValue {
    let source = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("read hot script {}: {err}", path.display()));
    let extracted = quinn_lower::extract_hot_script(&source)
        .unwrap_or_else(|err| panic!("extract hot script {}: {err}", path.display()));
    let file = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/");
    // Also registers this crate's manifest, which is how
    // `crate_resource("quinn_makepad:resources/icons/...")` resolves.
    crate::widgets::script_mod(vm);
    crate::list::script_mod(vm);
    vm.eval(ScriptMod {
        cargo_manifest_path: manifest_dir.trim_start_matches(r"\\?\").to_string(),
        module_path: module_path.to_string(),
        file,
        line: extracted.line,
        column: extracted.column,
        code: extracted.code,
        values: vec![component],
    })
}

/// Lower `view.rs` to `hot/ui.rs` on the first eval, watch `view.rs` when the
/// process was started with `--hot`, and eval the generated script.
///
/// `view.rs` and `hot/ui.rs` sit next to `src/`, not inside it. Cargo's
/// fingerprint does not include them, so `cargo run -- --hot` stays up while
/// the view is edited. Makepad watches `hot/ui.rs` because that path is the
/// `ScriptMod` file.
pub fn load_hot_view(
    vm: &mut ScriptVm,
    manifest_dir: &str,
    module_path: &str,
    title: &'static str,
    width: u32,
    height: u32,
    component: ScriptValue,
) -> ScriptValue {
    let view = quinn_lower::view_path(manifest_dir);
    let hot = quinn_lower::hot_ui_path(manifest_dir);
    if !vm.is_reload() {
        let shell = quinn_lower::Shell {
            title,
            width,
            height,
        };
        if std::env::var("QUINN_SKIP_LOWER").is_err() {
            quinn_lower::write_lowered(&view, &hot, shell)
                .unwrap_or_else(|err| panic!("lower {title} view: {err}"));
        }
        if std::env::args().any(|arg| arg == "--hot") {
            quinn_lower::watch_once(view, hot.clone(), shell);
        }
    }
    eval_hot_file(vm, &hot, manifest_dir, module_path, component)
}

/// [`load_hot_view`] plus a component-library source string. Hot reload keeps
/// that string, so a `view.rs` edit still resolves `<Button>`.
pub fn load_hot_view_with(
    vm: &mut ScriptVm,
    manifest_dir: &str,
    module_path: &str,
    title: &'static str,
    width: u32,
    height: u32,
    component: ScriptValue,
    library: &str,
) -> ScriptValue {
    let view = quinn_lower::view_path(manifest_dir);
    let hot = quinn_lower::hot_ui_path(manifest_dir);
    if !vm.is_reload() {
        let shell = quinn_lower::Shell {
            title,
            width,
            height,
        };
        if std::env::var("QUINN_SKIP_LOWER").is_err() {
            quinn_lower::write_lowered_with(&view, &hot, shell, library)
                .unwrap_or_else(|err| panic!("lower {title} view: {err}"));
        }
        if std::env::args().any(|arg| arg == "--hot") {
            quinn_lower::watch_once_with(view, hot.clone(), shell, library.to_string());
        }
    }
    eval_hot_file(vm, &hot, manifest_dir, module_path, component)
}

struct Delivery {
    name: String,
    gen: u64,
    result: Result<JobValue, String>,
}

fn inbox() -> &'static Mutex<Vec<Delivery>> {
    static INBOX: OnceLock<Mutex<Vec<Delivery>>> = OnceLock::new();
    INBOX.get_or_init(|| Mutex::new(Vec::new()))
}

fn job_runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_time()
            .thread_name("quinn-job")
            .build()
            .expect("quinn tokio runtime")
    })
}

std::thread_local! {
    /// `(binding, generation)` already spawned. Hot reload keeps this set, so
    /// an in-flight future is not started again when `mod.state` survives.
    static STARTED: RefCell<HashSet<(String, u64)>> = RefCell::new(HashSet::new());
}

/// Drive shell signals and client jobs.
///
/// Call this at the end of `handle_event`, after `boot_script` and
/// `ui.handle_event`, so a click's `dispatch` is visible on the same event.
/// `WindowGeomChange` writes `state.quinn_width`. A job whose script gen is
/// non-zero (`"loading"`, or an action with `pending`) is spawned on the tokio
/// pool. The worker calls `SignalToUI::set_ui_signal`. On `Event::Signal` the
/// result is written with `mod.quinn_apply` only when that generation is still
/// current and the binding is still in the reloaded tree. Mouse moves are ignored.
///
/// Markup and classes hot-reload. The async fn body is compiled Rust and needs
/// a rebuild, the same split Makepad has between script and Rust.
/// The view's root `on:keydown` handler sees Escape as `ev.key() == "Escape"`.
/// Other keys stay with the focused widget.
pub(crate) fn forward_key(cx: &mut Cx, ke: &KeyEvent) {
    if ke.key_code != KeyCode::Escape || ke.is_repeat {
        return;
    }
    script_eval!(cx, {
        if ok{mod.quinn_keydown} != nil { mod.quinn_keydown("Escape") } else { 0 }
        true
    });
}

pub fn drive_shell(cx: &mut Cx, event: &Event) {
    if std::env::var("QUINN_JOB_LOG").is_ok() {
        eprintln!("drive_shell");
    }
    if let Event::WindowGeomChange(ev) = event {
        let w = ev.new_geom.inner_size.x;
        script_eval!(cx, {
            if ok{mod.quinn_set_width} != nil { mod.quinn_set_width(#(w)) } else { 0 }
            true
        });
    }
    if let Event::KeyDown(ke) = event {
        forward_key(cx, ke);
    }
    let poll = matches!(
        event,
        Event::Startup
            | Event::Signal
            | Event::WindowGeomChange(_)
            | Event::Timer(_)
            | Event::LiveEdit
            | Event::MouseUp(_)
            | Event::KeyDown(_)
            | Event::TextInput(_)
    );
    if poll {
        drain_jobs(cx);
        reap_unmounted(cx);
        poll_jobs(cx);
        if sync_cancelled(cx) {
            script_eval!(cx, {
                ui.host.render()
                true
            });
        }
    }
}

struct Flight {
    gen: u64,
    abort: tokio::task::AbortHandle,
}

fn flights() -> &'static Mutex<HashMap<String, Flight>> {
    static FLIGHTS: OnceLock<Mutex<HashMap<String, Flight>>> = OnceLock::new();
    FLIGHTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cancel_binding(name: &str) {
    let flight = flights()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .remove(name);
    let Some(flight) = flight else {
        return;
    };
    if std::env::var("QUINN_JOB_LOG").is_ok() {
        eprintln!("quinn job {name} gen {} cancelled", flight.gen);
    }
    flight.abort.abort();
    SignalToUI::set_ui_signal();
}

fn clear_flight(name: &str, gen: u64) {
    let mut flights = flights().lock().unwrap_or_else(|err| err.into_inner());
    if flights.get(name).is_some_and(|flight| flight.gen == gen) {
        flights.remove(name);
    }
}

/// Abort a flight whose binding left the tree. A missing `quinn_job_live`
/// means the script is mid-reload, so the flight stays.
fn reap_unmounted(cx: &mut Cx) {
    let names: Vec<String> = flights()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .keys()
        .cloned()
        .collect();
    for name in names {
        let live_name = name.clone();
        let live = script_eval!(cx, {
            fn quinn_hold() { 2 }
            if ok{mod.quinn_job_live} == nil { quinn_hold() } else { mod.quinn_job_live(#(live_name)) }
        });
        if script_number(live).round() as i64 == 0 {
            if std::env::var("QUINN_JOB_LOG").is_ok() {
                eprintln!("quinn job {name} unmounted");
            }
            cancel_binding(&name);
        }
    }
}

fn sync_cancelled(cx: &mut Cx) -> bool {
    let n = quinn_lower::cancelled_count() as f64;
    let flag = script_eval!(cx, {
        fn quinn_missing() { 0 }
        if ok{mod.quinn_set_cancelled} == nil { quinn_missing() } else { mod.quinn_set_cancelled(#(n)) }
    });
    script_number(flag).round() as i64 == 2
}

fn script_number(value: ScriptValue) -> f64 {
    // Integer literals lower to i32/u32, not f64. `as_f64` misses those and
    // the job loop would see a count of 0 even when the script returned 4.
    value.as_number().unwrap_or(0.0)
}

fn script_text(cx: &mut Cx, value: ScriptValue) -> String {
    if let Some(number) = value.as_number() {
        if number.is_finite() && number.fract() == 0.0 && number.abs() < 1e15 {
            return format!("{}", number as i64);
        }
        return number.to_string();
    }
    if let Some(flag) = value.as_bool() {
        return if flag { "true" } else { "false" }.into();
    }
    let mut text = String::new();
    cx.with_vm(|vm| {
        vm.bx.heap.cast_to_string(value, &mut text);
    });
    text
}

fn poll_jobs(cx: &mut Cx) {
    let count_value = script_eval!(cx, {
        if ok{mod.quinn_job_count} == nil { 0 } else { mod.quinn_job_count() }
    });
    let count = script_number(count_value).round() as usize;
    if std::env::var("QUINN_JOB_LOG").is_ok() {
        eprintln!("job count n={count}");
    }
    for index in 0..count {
        let i = index as f64;
        let gen_value = script_eval!(cx, { mod.quinn_job_gen(#(i)) });
        let gen = script_number(gen_value).round() as u64;
        if gen == 0 {
            continue;
        }
        let key_value = script_eval!(cx, { mod.quinn_job_key(#(i)) });
        let key = script_text(cx, key_value);
        if key.is_empty() {
            continue;
        }
        let already = STARTED.with(|started| started.borrow().contains(&(key.clone(), gen)));
        if already {
            continue;
        }
        let func_value = script_eval!(cx, { mod.quinn_job_fn(#(i)) });
        let func = script_text(cx, func_value);
        let arg_value = script_eval!(cx, { mod.quinn_job_arg(#(i)) });
        let arg = script_text(cx, arg_value);
        let plan_value = script_eval!(cx, {
            fn quinn_missing() { "" }
            if ok{mod.quinn_job_plan} == nil { quinn_missing() } else { mod.quinn_job_plan(#(i)) }
        });
        let plan = script_text(cx, plan_value);
        if std::env::var("QUINN_JOB_LOG").is_ok() {
            eprintln!("quinn job {key} fn {func} plan {plan} gen {gen} arg {arg}");
        }
        STARTED.with(|started| {
            started.borrow_mut().insert((key.clone(), gen));
        });
        spawn_job(func, plan, key, gen, arg);
    }
}

fn spawn_job(func: String, plan: String, binding: String, gen: u64, arg: String) {
    cancel_binding(&binding);
    let job_arg = quinn_lower::JobArg::from_text(arg);
    let binding_task = binding.clone();
    let handle = job_runtime().spawn(quinn_lower::track(async move {
        let result = quinn_lower::run_registered(&func, &plan, job_arg).await;
        clear_flight(&binding_task, gen);
        deliver(binding_task, gen, result);
    }));
    flights()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .insert(
            binding,
            Flight {
                gen,
                abort: handle.abort_handle(),
            },
        );
}

fn deliver(name: String, gen: u64, result: Result<JobValue, String>) {
    inbox()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .push(Delivery { name, gen, result });
    SignalToUI::set_ui_signal();
}

fn drain_jobs(cx: &mut Cx) {
    let batch: Vec<Delivery> = inbox()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .drain(..)
        .collect();
    if batch.is_empty() {
        return;
    }
    let mut retry = Vec::new();
    let mut applied_any = false;
    for item in batch {
        let phase = if item.result.is_ok() {
            "ready".to_string()
        } else {
            "error".to_string()
        };
        let (json, err) = match &item.result {
            Ok(value) => (value.to_json(), String::new()),
            Err(err) => ("\"\"".to_string(), err.clone()),
        };
        let name = item.name.clone();
        let gen = item.gen as f64;
        let applied = name.clone();
        // Both branches are calls. A literal on one side and a call on the
        // other unbalances the script stack, so the apply ran and the Rust
        // side still saw nil and retried the same delivery forever.
        let flag = script_eval!(cx, {
            let raw = #(json)
            let value = raw.parse_json()
            fn quinn_missing() { 0 }
            if ok{mod.quinn_apply} == nil { quinn_missing() } else { mod.quinn_apply(#(applied), #(gen), #(phase), value, #(err)) }
        });
        match script_number(flag).round() as i64 {
            2 => {
                applied_any = true;
                if std::env::var("QUINN_JOB_LOG").is_ok() {
                    eprintln!("quinn job {name} gen {} applied", item.gen);
                }
            }
            // Stale generation, or the binding left the tree. Consumed.
            1 => {
                if std::env::var("QUINN_JOB_LOG").is_ok() {
                    eprintln!("quinn job {name} gen {} stale", item.gen);
                }
            }
            _ => {
                if std::env::var("QUINN_JOB_LOG").is_ok() {
                    eprintln!(
                        "quinn job {name} gen {} held ({})",
                        item.gen,
                        script_text(cx, flag)
                    );
                }
                retry.push(item);
            }
        }
    }
    if applied_any {
        script_eval!(cx, {
            ui.host.render()
            true
        });
    }
    if !retry.is_empty() {
        inbox()
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .splice(0..0, retry);
    }
}

/// Install the frame probe on a shell whose widgets already came from script.
///
/// Unlike [`startup`], this does not mount a Rust `view!`. Clicks are script
/// `on_click` closures. The host view is the `host` name in that script.
pub fn boot_script(cx: &mut Cx, ui: &WidgetRef) {
    let host = ui.view(cx, ids!(host));
    if host.is_empty() {
        panic!("quinn host view was not created; the shell script did not register `host`");
    }
    crate::probe::run_widget_bench(&host);
    crate::probe::install(cx);
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
        session_slot().with(|slot| {
            *slot.borrow_mut() = Some(Session { runtime, owner });
        });
        if let Some(hook) = crate::probe::take_after_mount() {
            hook();
        }
        crate::probe::run_widget_bench(&host);
    });
    // Ready is the first presented frame, not the end of `startup`. The hook
    // runs from `Cx::post_draw_hook`, after this frame's draw lists are
    // recorded and before `eglSwapBuffers` in the same paint.
    crate::probe::install(cx);
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
