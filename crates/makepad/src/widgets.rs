//! Widgets the lowerer targets where stock Makepad falls short of the design
//! system: a button with a separated focus ring, and a slider that takes
//! drag, arrow keys, and Tab.

use makepad_widgets::makepad_script::ScriptFnRef;
use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets_internal.*
    use mod.widgets.*

    // The ring sits inside the button's own rect: 2 units of focus color at
    // the edge, 2 units of separation, then the face. Focus never changes
    // the outer geometry.
    mod.widgets.QuinnButton = mod.widgets.Button{
        draw_bg +: {
            ring_color: uniform(#x1d4ed8)
            ring_gap: uniform(#xffffff)
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                sdf.box(
                    self.border_size,
                    self.border_size,
                    self.rect_size.x - self.border_size * 2.,
                    self.rect_size.y - self.border_size * 2.,
                    self.border_radius
                )
                sdf.fill_keep(self.face_fill())
                sdf.stroke(self.face_stroke(), self.border_size)
                if self.focus > 0.0 {
                    sdf.box(1., 1., self.rect_size.x - 2., self.rect_size.y - 2., self.border_radius)
                    sdf.stroke(vec4(self.ring_color.rgb, self.ring_color.a * self.focus), 2.)
                    sdf.box(3., 3., self.rect_size.x - 6., self.rect_size.y - 6., max(self.border_radius - 2., 0.))
                    sdf.stroke(vec4(self.ring_gap.rgb, self.ring_gap.a * self.focus), 2.)
                }
                return sdf.result
            }
        }
    }

    // Flat face and boundary from the lowered tokens; focus adds the same
    // separated ring as QuinnButton instead of recoloring the boundary.
    mod.widgets.QuinnTextInput = mod.widgets.TextInput{
        draw_bg +: {
            ring_color: uniform(#x1d4ed8)
            ring_gap: uniform(#xffffff)
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                sdf.box(
                    self.border_size,
                    self.border_size,
                    self.rect_size.x - self.border_size * 2.,
                    self.rect_size.y - self.border_size * 2.,
                    self.border_radius
                )
                sdf.fill_keep(mix(self.color, self.color_disabled, self.disabled))
                sdf.stroke(mix(self.border_color, self.border_color_disabled, self.disabled), self.border_size)
                if self.focus > 0.0 {
                    sdf.box(1., 1., self.rect_size.x - 2., self.rect_size.y - 2., self.border_radius)
                    sdf.stroke(vec4(self.ring_color.rgb, self.ring_color.a * self.focus), 2.)
                    sdf.box(3., 3., self.rect_size.x - 6., self.rect_size.y - 6., max(self.border_radius - 2., 0.))
                    sdf.stroke(vec4(self.ring_gap.rgb, self.ring_gap.a * self.focus), 2.)
                }
                return sdf.result
            }
        }
    }

    set_type_default() do #(DrawQuinnSlider::script_shader(vm)){
        ..mod.draw.DrawQuad
    }

    mod.widgets.QuinnLayerBase = #(QuinnLayer::register_widget(vm))
    mod.widgets.QuinnLayer = set_type_default() do mod.widgets.QuinnLayerBase{
        width: Fill
        height: Fill
        flow: Overlay
    }

    mod.widgets.QuinnPopupBase = #(QuinnPopup::register_widget(vm))
    mod.widgets.QuinnPopup = set_type_default() do mod.widgets.QuinnPopupBase{
        width: Fit
        height: Fit
        flow: Down
    }

    mod.widgets.QuinnSliderBase = #(QuinnSlider::register_widget(vm))
    mod.widgets.QuinnSlider = set_type_default() do mod.widgets.QuinnSliderBase{
        width: 240
        height: 36
        draw_bg +: {
            track_color: uniform(#xcbd5e1)
            fill_color: uniform(#x1d4ed8)
            thumb_color: uniform(#xffffff)
            thumb_border: uniform(#x64748b)
            ring_color: uniform(#x1d4ed8)
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                let cy = self.rect_size.y * 0.5
                let x0 = 12.
                let x1 = self.rect_size.x - 12.
                let cx = mix(x0, x1, self.slide_pos)
                sdf.box(0., cy - 2., self.rect_size.x, 4., 2.)
                sdf.fill(self.track_color)
                sdf.box(0., cy - 2., cx, 4., 2.)
                sdf.fill(self.fill_color)
                sdf.circle(cx, cy, 11.)
                sdf.stroke(vec4(self.ring_color.rgb, self.ring_color.a * self.focus), 2.)
                sdf.circle(cx, cy, 8.)
                sdf.fill_keep(self.thumb_color)
                sdf.stroke(self.thumb_border, 1.)
                return sdf.result
            }
        }
    }
}

/// Hosts `<Portal>` content in its own overlay draw list. While anything
/// inside registers a tab stop (an open dialog), Tab stays inside.
#[derive(Script, Widget)]
pub struct QuinnLayer {
    #[source]
    source: ScriptObjectRef,
    #[deref]
    view: View,
    #[rust]
    draw_list: Option<DrawList2d>,
}

impl ScriptHook for QuinnLayer {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        self.draw_list = Some(DrawList2d::script_new(vm));
    }
}

impl Widget for QuinnLayer {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, _walk: Walk) -> DrawStep {
        let draw_list = self.draw_list.as_mut().unwrap();
        draw_list.begin_overlay_reuse(cx);
        let size = cx.current_pass_size();
        cx.begin_root_turtle(size, Layout::flow_overlay());
        let _ = self.view.draw_all(cx, scope);
        cx.end_pass_sized_turtle();
        let draw_list = self.draw_list.as_mut().unwrap();
        draw_list.end(cx);
        let id = draw_list.id();
        let modal = !CxDraw::nav_scope_stops(cx, id).is_empty();
        CxDraw::set_nav_trap(cx, id, modal);
        DrawStep::done()
    }
}

/// `data-popover`: a menu or popover panel drawn above everything, just
/// under the control before it. Near the bottom edge it flips above that
/// control; near the side edges it shifts to keep 8 units of clearance.
#[derive(Script, Widget)]
pub struct QuinnPopup {
    #[source]
    source: ScriptObjectRef,
    #[deref]
    view: View,
    /// Distance from the natural slot up to the top of the trigger, plus the
    /// same gap again above it: where the panel's bottom goes when flipped.
    #[live(52.0)]
    anchor_offset: f64,
    #[rust]
    draw_list: Option<DrawList2d>,
    #[rust]
    size: Vec2d,
}

const EDGE: f64 = 8.0;

impl ScriptHook for QuinnPopup {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        self.draw_list = Some(DrawList2d::script_new(vm));
    }
}

impl QuinnPopup {
    fn place(&self, slot: Vec2d, pass: Vec2d) -> Vec2d {
        let mut pos = slot;
        let above = slot.y - self.anchor_offset - self.size.y;
        if slot.y + self.size.y + EDGE > pass.y && above >= EDGE {
            pos.y = above;
        }
        let right = pass.x - EDGE - self.size.x;
        pos.x = pos.x.min(right).max(EDGE);
        pos
    }
}

impl Widget for QuinnPopup {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let slot = cx.walk_turtle(Walk::new(Size::Fixed(0.0), Size::Fixed(0.0))).pos;
        let pass = cx.current_pass_size();
        let pos = self.place(slot, pass);
        let draw_list = self.draw_list.as_mut().unwrap();
        draw_list.begin_overlay_reuse(cx);
        cx.begin_root_turtle(pass, Layout::flow_overlay());
        let _ = self.view.draw_walk(cx, scope, walk.with_abs_pos(pos));
        cx.end_pass_sized_turtle();
        self.draw_list.as_mut().unwrap().end(cx);
        let size = self.view.area().rect(cx).size;
        if size != self.size {
            self.size = size;
            self.view.redraw(cx);
        }
        DrawStep::done()
    }
}

#[derive(Script, ScriptHook)]
#[repr(C)]
pub struct DrawQuinnSlider {
    #[deref]
    draw_super: DrawQuad,
    #[live]
    slide_pos: f32,
    #[live]
    focus: f32,
}

/// `<input type="range" min max step bind:value>`. The thumb's focus ring
/// needs 12 units each side, so the track is inset by that much.
#[derive(Script, ScriptHook, Widget)]
pub struct QuinnSlider {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[walk]
    walk: Walk,
    #[layout]
    layout: Layout,
    #[redraw]
    #[live]
    draw_bg: DrawQuinnSlider,
    #[live(0.0)]
    min: f64,
    #[live(100.0)]
    max: f64,
    #[live(1.0)]
    step: f64,
    /// Shift + arrow.
    #[live(10.0)]
    big_step: f64,
    #[live]
    value: f64,
    #[live]
    on_change: ScriptFnRef,
    #[rust]
    focused: bool,
}

const INSET: f64 = 12.0;

impl QuinnSlider {
    fn value_at(&self, x: f64, rect: Rect) -> f64 {
        let span = (rect.size.x - INSET * 2.0).max(1.0);
        let t = ((x - rect.pos.x - INSET) / span).clamp(0.0, 1.0);
        self.min + t * (self.max - self.min)
    }

    fn commit(&mut self, cx: &mut Cx, raw: f64) {
        let stepped = if self.step > 0.0 {
            self.min + ((raw - self.min) / self.step).round() * self.step
        } else {
            raw
        };
        let next = stepped.clamp(self.min.min(self.max), self.max.max(self.min));
        if next == self.value {
            return;
        }
        self.value = next;
        self.draw_bg.redraw(cx);
        let uid = self.widget_uid();
        cx.widget_to_script_call(
            uid,
            NIL,
            self.source.clone(),
            self.on_change.clone(),
            &[ScriptValue::from(next)],
        );
    }
}

impl Widget for QuinnSlider {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, _scope: &mut Scope) {
        let area = self.draw_bg.area();
        match event.hits(cx, area) {
            Hit::KeyFocus(_) => {
                self.focused = true;
                self.draw_bg.redraw(cx);
            }
            Hit::KeyFocusLost(_) => {
                self.focused = false;
                self.draw_bg.redraw(cx);
            }
            Hit::KeyDown(ke) => {
                let step = if ke.modifiers.shift {
                    self.big_step
                } else {
                    self.step
                };
                let next = match ke.key_code {
                    KeyCode::ArrowLeft | KeyCode::ArrowDown => Some(self.value - step),
                    KeyCode::ArrowRight | KeyCode::ArrowUp => Some(self.value + step),
                    KeyCode::Home => Some(self.min),
                    KeyCode::End => Some(self.max),
                    _ => None,
                };
                if let Some(next) = next {
                    self.commit(cx, next);
                }
            }
            Hit::FingerHoverIn(_) => cx.set_cursor(MouseCursor::Hand),
            Hit::FingerDown(fe) if fe.is_primary_hit() => {
                cx.set_key_focus(area);
                let next = self.value_at(fe.abs.x, fe.rect);
                self.commit(cx, next);
            }
            Hit::FingerMove(fe) => {
                let next = self.value_at(fe.abs.x, fe.rect);
                self.commit(cx, next);
            }
            _ => {}
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        let range = (self.max - self.min).abs().max(f64::EPSILON);
        self.draw_bg.slide_pos = ((self.value - self.min) / range).clamp(0.0, 1.0) as f32;
        self.draw_bg.focus = if self.focused { 1.0 } else { 0.0 };
        self.draw_bg.draw_walk(cx, walk);
        cx.add_nav_stop(self.draw_bg.area(), NavRole::Slider, Inset::default());
        DrawStep::done()
    }
}
