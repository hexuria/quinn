//! Widgets the lowerer targets where stock Makepad falls short of the design
//! system: a button with a separated focus ring, and a slider that takes
//! drag, arrow keys, and Tab.

use makepad_widgets::makepad_script::ScriptFnRef;
use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets_internal.*
    use mod.widgets.*

    // The ring is 2 units of focus color on the button's own edge, so focus
    // never changes the outer geometry. A click does not take key focus: the
    // ring is for the keyboard.
    mod.widgets.QuinnButton = mod.widgets.Button{
        grab_key_focus: false
        draw_bg +: {
            ring_color: uniform(#x1d4ed8)
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
                }
                return sdf.result
            }
        }
    }

    // The library Button. Each variant is a template: its faces, labels, and
    // borders for every state are written here once, light and dark, and the
    // lowerer only names the template. `dark`, `selected`, and `inactive`
    // (disabled at creation; the animator's disabled track starts at off) are
    // set per instance from state; hover, down, and focus are the animator's.
    // A click does not take key focus, so the ring is a keyboard ring. It is
    // drawn outside the face: the rect carries a `halo` of 4 on every side
    // (2 of gap, 2 of ring) and a margin of -4 takes it back out of layout,
    // so the face is the size the lowerer asked for. `corner` is the
    // visible radius of the face in pixels.
    mod.widgets.QButton = mod.widgets.Button{
        width: Fit
        height: 44
        margin: Inset{left: -4 top: -4 right: -4 bottom: -4}
        padding: Inset{left: 16 right: 16}
        align: Center
        grab_key_focus: false
        draw_text +: {
            dark: uniform(0.0)
            selected: uniform(0.0)
            inactive: uniform(0.0)
            color: #xffffff
            color_hover: #xffffff
            color_down: uniform(#xffffff)
            color_disabled: uniform(#x475569)
            color_selected: uniform(#x1e3a8a)
            color_dark: uniform(#x0f172a)
            color_hover_dark: uniform(#x0f172a)
            color_down_dark: uniform(#x0f172a)
            color_disabled_dark: uniform(#x94a3b8)
            color_selected_dark: uniform(#xeff6ff)
            text_style: TextStyle{font_size: 10.5 font_family: FontFamily{latin := FontMember{res: crate_resource("makepad_widgets:resources/Inter.ttf") asc: 0.0 desc: 0.0 weight: 500.0}}}
            get_color: fn() {
                return mix(self.color, self.color_dark, self.dark)
                    .mix(mix(self.color_selected, self.color_selected_dark, self.dark), self.selected)
                    .mix(mix(self.color_hover, self.color_hover_dark, self.dark), self.hover)
                    .mix(mix(self.color_down, self.color_down_dark, self.dark), self.down)
                    .mix(mix(self.color_disabled, self.color_disabled_dark, self.dark), max(self.disabled, self.inactive))
            }
        }
        draw_bg +: {
            dark: uniform(0.0)
            selected: uniform(0.0)
            inactive: uniform(0.0)
            halo: uniform(4.0)
            corner: uniform(6.0)
            border_size: uniform(0.0)
            color: uniform(#x1d4ed8)
            color_hover: uniform(#x1e40af)
            color_down: uniform(#x1e3a8a)
            color_disabled: uniform(#xe2e8f0)
            color_selected: uniform(#xdbeafe)
            color_dark: uniform(#x93c5fd)
            color_hover_dark: uniform(#xbfdbfe)
            color_down_dark: uniform(#xdbeafe)
            color_disabled_dark: uniform(#x1e293b)
            color_selected_dark: uniform(#x1e3a8a)
            border_color: uniform(#x00000000)
            border_color_dark: uniform(#x00000000)
            border_color_disabled: uniform(#x00000000)
            border_color_selected: uniform(#x00000000)
            border_color_selected_dark: uniform(#x00000000)
            ring_color: uniform(#x1d4ed8)
            ring_color_dark: uniform(#x93c5fd)
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                let fill = mix(self.color, self.color_dark, self.dark)
                    .mix(mix(self.color_selected, self.color_selected_dark, self.dark), self.selected)
                    .mix(mix(self.color_hover, self.color_hover_dark, self.dark), self.hover)
                    .mix(mix(self.color_down, self.color_down_dark, self.dark), self.down)
                    .mix(mix(self.color_disabled, self.color_disabled_dark, self.dark), max(self.disabled, self.inactive))
                let stroke = mix(self.border_color, self.border_color_dark, self.dark)
                    .mix(mix(self.border_color_selected, self.border_color_selected_dark, self.dark), self.selected)
                    .mix(self.border_color_disabled, max(self.disabled, self.inactive))
                let inset = self.halo + self.border_size * 0.5
                sdf.box(
                    inset,
                    inset,
                    self.rect_size.x - inset * 2.,
                    self.rect_size.y - inset * 2.,
                    self.corner * 0.5
                )
                sdf.fill_keep(fill)
                sdf.stroke(stroke, self.border_size)
                if self.focus > 0.0 {
                    let ring = mix(self.ring_color, self.ring_color_dark, self.dark)
                    sdf.box(1., 1., self.rect_size.x - 2., self.rect_size.y - 2., (self.corner + self.halo - 1.) * 0.5)
                    sdf.stroke(vec4(ring.rgb, ring.a * self.focus), 2.)
                }
                return sdf.result
            }
        }
    }

    mod.widgets.QButtonSecondary = mod.widgets.QButton{
        draw_text +: {
            color: #x0f172a
            color_hover: #x0f172a
            color_down: #x0f172a
            color_dark: #xf8fafc
            color_hover_dark: #xf8fafc
            color_down_dark: #xf8fafc
        }
        draw_bg +: {
            border_size: 1.0
            color: #xffffff
            color_hover: #xf1f5f9
            color_down: #xe2e8f0
            color_dark: #x111827
            color_hover_dark: #x1e293b
            color_down_dark: #x334155
            border_color: #x64748b
            border_color_dark: #x94a3b8
            border_color_selected: #x64748b
            border_color_selected_dark: #x94a3b8
            border_color_disabled: #xe2e8f0
        }
    }

    mod.widgets.QButtonGhost = mod.widgets.QButton{
        draw_text +: {
            color: #x0f172a
            color_hover: #x0f172a
            color_down: #x0f172a
            color_dark: #xf8fafc
            color_hover_dark: #xf8fafc
            color_down_dark: #xf8fafc
        }
        draw_bg +: {
            // Clear faces carry the hover hue so the fade in does not pass
            // through grey.
            color: #xf1f5f900
            color_hover: #xf1f5f9
            color_down: #xe2e8f0
            color_dark: #x1e293b00
            color_hover_dark: #x1e293b
            color_down_dark: #x334155
        }
    }

    mod.widgets.QButtonDanger = mod.widgets.QButton{
        draw_bg +: {
            color: #xb91c1c
            color_hover: #x991b1b
            color_down: #x7f1d1d
            color_dark: #xfca5a5
            color_hover_dark: #xfecaca
            color_down_dark: #xfee2e2
        }
    }

    // One segment of a segmented control, on the control's tinted track.
    // Unselected segments are the track; the selected one is a raised
    // surface with a hairline.
    mod.widgets.QSegment = mod.widgets.QButton{
        draw_text +: {
            color: #x0f172a
            color_hover: #x0f172a
            color_down: #x0f172a
            color_selected: #x0f172a
            color_dark: #xf8fafc
            color_hover_dark: #xf8fafc
            color_down_dark: #xf8fafc
            color_selected_dark: #xf8fafc
        }
        draw_bg +: {
            border_size: 1.0
            color: #xf1f5f900
            color_hover: #xe2e8f0
            color_down: #xcbd5e1
            color_selected: #xffffff
            color_dark: #x1e293b00
            color_hover_dark: #x334155
            color_down_dark: #x475569
            color_selected_dark: #x111827
            border_color_selected: #xcbd5e1
            border_color_selected_dark: #x334155
        }
    }

    // A labelled control drawn in one widget: the mark (switch track, check
    // box) at the start, the label after it, the whole row the target.
    // `selected` is the checked state; the lowerer sets it from `checked`.
    mod.widgets.QToggle = mod.widgets.Button{
        width: Fit
        height: 36
        margin: 0.
        align: Align{x: 0. y: 0.5}
        grab_key_focus: false
        draw_text +: {
            dark: uniform(0.0)
            selected: uniform(0.0)
            inactive: uniform(0.0)
            color: #x0f172a
            color_hover: #x0f172a
            color_dark: uniform(#xf8fafc)
            color_selected: uniform(#x0f172a)
            color_selected_dark: uniform(#xf8fafc)
            color_disabled: uniform(#x475569)
            color_disabled_dark: uniform(#x94a3b8)
            text_style: TextStyle{font_size: 10.5 font_family: FontFamily{latin := FontMember{res: crate_resource("makepad_widgets:resources/Inter.ttf") asc: 0.0 desc: 0.0 weight: 400.0}}}
            get_color: fn() {
                return mix(self.color, self.color_dark, self.dark)
                    .mix(mix(self.color_selected, self.color_selected_dark, self.dark), self.selected)
                    .mix(mix(self.color_disabled, self.color_disabled_dark, self.dark), max(self.disabled, self.inactive))
            }
        }
        draw_bg +: {
            dark: uniform(0.0)
            selected: uniform(0.0)
            inactive: uniform(0.0)
            accent: uniform(#x1d4ed8)
            accent_dark: uniform(#x93c5fd)
            accent_hover: uniform(#x1e40af)
            accent_hover_dark: uniform(#xbfdbfe)
            well: uniform(#xffffff)
            well_dark: uniform(#x111827)
            well_hover: uniform(#xf1f5f9)
            well_hover_dark: uniform(#x1e293b)
            edge: uniform(#x64748b)
            edge_dark: uniform(#x94a3b8)
            mark: uniform(#xffffff)
            mark_dark: uniform(#x0f172a)
            muted: uniform(#xe2e8f0)
            muted_dark: uniform(#x1e293b)
            ring_color: uniform(#x1d4ed8)
            ring_color_dark: uniform(#x93c5fd)
        }
    }

    // 40 by 24 track, 16 thumb, 4 from the ends; 2 units of room on the
    // start edge for the focus ring around the track.
    mod.widgets.QSwitch = mod.widgets.QToggle{
        padding: Inset{left: 52}
        draw_bg +: {
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                let off = max(self.disabled, self.inactive)
                let cy = self.rect_size.y * 0.5
                let accent = mix(self.accent, self.accent_dark, self.dark)
                    .mix(mix(self.accent_hover, self.accent_hover_dark, self.dark), self.hover)
                let well = mix(self.well_hover, self.well_hover_dark, self.dark)
                let track = mix(well, accent, self.selected)
                    .mix(mix(self.muted, self.muted_dark, self.dark), off)
                let edge = mix(self.edge, self.edge_dark, self.dark)
                sdf.box(2.5, cy - 11.5, 39., 23., 6.)
                sdf.fill_keep(track)
                sdf.stroke(vec4(edge.rgb, edge.a * (1.0 - self.selected)), 1.)
                let thumb_x = mix(14., 30., self.selected)
                sdf.circle(thumb_x, cy, 8.)
                sdf.fill_keep(mix(self.well, self.well_dark, self.dark))
                sdf.stroke(vec4(edge.rgb, edge.a * (1.0 - self.selected)), 1.)
                if self.focus > 0.0 {
                    let ring = mix(self.ring_color, self.ring_color_dark, self.dark)
                    sdf.box(1., cy - 13., 42., 26., 7.)
                    sdf.stroke(vec4(ring.rgb, ring.a * self.focus), 2.)
                }
                return sdf.result
            }
        }
    }

    // 18-unit box, radius 4, a drawn check; 2 units of room on the start
    // edge for the focus ring around the box.
    mod.widgets.QCheckbox = mod.widgets.QToggle{
        padding: Inset{left: 30}
        draw_bg +: {
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                let off = max(self.disabled, self.inactive)
                let cy = self.rect_size.y * 0.5
                let accent = mix(self.accent, self.accent_dark, self.dark)
                    .mix(mix(self.accent_hover, self.accent_hover_dark, self.dark), self.hover)
                let well = mix(self.well, self.well_dark, self.dark)
                    .mix(mix(self.well_hover, self.well_hover_dark, self.dark), self.hover)
                let face = mix(well, accent, self.selected)
                    .mix(mix(self.muted, self.muted_dark, self.dark), off)
                let edge = mix(self.edge, self.edge_dark, self.dark)
                sdf.box(2.5, cy - 8.5, 17., 17., 2.)
                sdf.fill_keep(face)
                sdf.stroke(vec4(edge.rgb, edge.a * (1.0 - self.selected)), 1.)
                if self.selected > 0.0 {
                    let mark = mix(self.mark, self.mark_dark, self.dark)
                    sdf.move_to(6.5, cy + 0.5)
                    sdf.line_to(9.5, cy + 3.5)
                    sdf.line_to(15.5, cy - 3.)
                    sdf.stroke(vec4(mark.rgb, mark.a * self.selected), 1.75)
                }
                if self.focus > 0.0 {
                    let ring = mix(self.ring_color, self.ring_color_dark, self.dark)
                    sdf.box(1., cy - 10., 20., 20., 3.)
                    sdf.stroke(vec4(ring.rgb, ring.a * self.focus), 2.)
                }
                return sdf.result
            }
        }
    }

    // A row face shared by navigation rows, menu items, and header cells:
    // clear at rest, tinted under the pointer, the selection fill when
    // selected. Focus is a 2-unit ring on the row's own edge.
    mod.widgets.QRow = mod.widgets.QToggle{
        draw_text +: {
            color_selected: #x1e3a8a
            color_selected_dark: #xeff6ff
        }
        draw_bg +: {
            corner: uniform(6.0)
            face_hover: uniform(#xe2e8f0)
            face_hover_dark: uniform(#x334155)
            face_down: uniform(#xcbd5e1)
            face_down_dark: uniform(#x475569)
            face_selected: uniform(#xdbeafe)
            face_selected_dark: uniform(#x1e3a8a)
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                let hover = mix(self.face_hover, self.face_hover_dark, self.dark)
                    .mix(mix(self.face_down, self.face_down_dark, self.dark), self.down)
                let selected = mix(self.face_selected, self.face_selected_dark, self.dark)
                let face = mix(vec4(hover.rgb, 0.0), hover, self.hover * (1.0 - self.selected))
                    .mix(selected, self.selected)
                sdf.box(0., 0., self.rect_size.x, self.rect_size.y, self.corner * 0.5)
                sdf.fill_keep(face)
                if self.focus > 0.0 {
                    let ring = mix(self.ring_color, self.ring_color_dark, self.dark)
                    sdf.box(1., 1., self.rect_size.x - 2., self.rect_size.y - 2., self.corner * 0.5)
                    sdf.stroke(vec4(ring.rgb, ring.a * self.focus), 2.)
                }
                return sdf.result
            }
        }
    }

    // A navigation row: icon and label. The lowerer sets the icon, its ink,
    // and the responsive layout from the component's class.
    mod.widgets.QNavItem = mod.widgets.QRow{
        width: Fill
        icon_walk: Walk{width: 16 height: 16}
        draw_text +: {
            // A long name ends in an ellipsis, as a sidebar row does. Text
            // wraps only in a wrapping flow, and there the label would drop
            // below the icon instead of beside it.
            text_overflow: Ellipsis
        }
    }

    // A menu item on a panel: a full-width row, body text.
    mod.widgets.QMenuItem = mod.widgets.QRow{
        width: Fill
        padding: Inset{left: 12 right: 12}
        draw_bg +: {
            face_hover: #xf1f5f9
            face_hover_dark: #x1e293b
            face_down: #xe2e8f0
            face_down_dark: #x334155
        }
    }

    // A column header that sorts: caption type in the secondary ink.
    mod.widgets.QHeader = mod.widgets.QRow{
        padding: Inset{left: 12 right: 12}
        draw_text +: {
            color: #x475569
            color_hover: #x475569
            color_dark: #xcbd5e1
            text_style: TextStyle{font_size: 9. font_family: FontFamily{latin := FontMember{res: crate_resource("makepad_widgets:resources/Inter.ttf") asc: 0.0 desc: 0.0 weight: 400.0}}}
        }
    }

    // A raised surface: face, hairline, and shadow, light and dark. Menus,
    // toasts, dialogs, and drawers sit on it. `corner` is the visible radius.
    mod.widgets.QPanel = mod.widgets.RoundedShadowView{
        draw_bg +: {
            dark: uniform(0.0)
            corner: uniform(12.0)
            face: uniform(#xffffff)
            face_dark: uniform(#x111827)
            edge: uniform(#xcbd5e1)
            edge_dark: uniform(#x334155)
            shade: uniform(#x0f172a24)
            shade_dark: uniform(#x00000099)
            border_size: 1.0
            shadow_radius: 16.0
            shadow_offset: vec2(0., 6.)
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size3)
                sdf.box(
                    self.sdf_rect_pos.x
                    self.sdf_rect_pos.y
                    self.sdf_rect_size.x
                    self.sdf_rect_size.y
                    self.corner * 0.5
                )
                if sdf.shape > -1.0 {
                    let m = self.shadow_radius
                    let o = self.shadow_offset + self.rect_shift
                    let v = GaussShadow.rounded_box_shadow(vec2(m) + o self.rect_size2+o self.pos * (self.rect_size3+vec2(m)) self.shadow_radius*0.5 self.corner)
                    sdf.clear(mix(self.shade, self.shade_dark, self.dark) * v)
                }
                sdf.fill_keep(mix(self.face, self.face_dark, self.dark))
                sdf.stroke(mix(self.edge, self.edge_dark, self.dark), self.border_size)
                return sdf.result
            }
        }
    }

    // A menu's panel: the same surface with a tighter corner.
    mod.widgets.QMenuPanel = mod.widgets.QPanel{
        draw_bg +: {
            corner: 8.0
        }
    }

    // Flat face and boundary from the lowered tokens. Focus is a 2-unit ring
    // on the field's own edge, over the boundary; a gap inside it read as a
    // white band on a tinted field.
    mod.widgets.QuinnTextInputBase = #(QuinnTextInput::register_widget(vm))
    mod.widgets.QuinnTextInput = set_type_default() do mod.widgets.QuinnTextInputBase{
        ..mod.widgets.TextInput,
        draw_bg +: {
            ring_color: uniform(#x1d4ed8)
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
                }
                return sdf.result
            }
        }
    }

    // The library text field. As with QButton, the template owns every face,
    // border, ink, and state, light and dark; the lowerer sets text, the
    // handler, size, and the `dark`, `invalid`, and `inactive` flags. Focus is
    // a 2-unit ring on the field's own edge, in the danger color when the
    // field is invalid. `corner` is the visible radius in pixels.
    mod.widgets.QInput = mod.widgets.QuinnTextInput{
        width: Fill
        height: 36
        margin: 0.
        // TextInput centers its line in the height left after padding, which
        // puts the capitals 1.25 high; 2.5 on top halves into that.
        padding: Inset{left: 12 top: 2.5 right: 12}
        draw_bg +: {
            dark: uniform(0.0)
            invalid: uniform(0.0)
            inactive: uniform(0.0)
            corner: uniform(6.0)
            border_size: uniform(1.0)
            color: #xffffff
            color_dark: uniform(#x111827)
            color_disabled: uniform(#xe2e8f0)
            color_disabled_dark: uniform(#x1e293b)
            border_color: uniform(#x64748b)
            border_color_dark: uniform(#x94a3b8)
            danger: uniform(#xb91c1c)
            danger_dark: uniform(#xfca5a5)
            ring_color: uniform(#x1d4ed8)
            ring_color_dark: uniform(#x93c5fd)
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                let off = max(self.disabled, self.inactive)
                let danger = mix(self.danger, self.danger_dark, self.dark)
                let fill = mix(self.color, self.color_dark, self.dark)
                    .mix(mix(self.color_disabled, self.color_disabled_dark, self.dark), off)
                let stroke = mix(self.border_color, self.border_color_dark, self.dark)
                    .mix(danger, self.invalid)
                let inset = self.border_size * 0.5
                sdf.box(
                    inset,
                    inset,
                    self.rect_size.x - inset * 2.,
                    self.rect_size.y - inset * 2.,
                    self.corner * 0.5
                )
                sdf.fill_keep(fill)
                sdf.stroke(stroke, self.border_size)
                if self.focus > 0.0 {
                    let ring = mix(self.ring_color, self.ring_color_dark, self.dark).mix(danger, self.invalid)
                    sdf.box(1., 1., self.rect_size.x - 2., self.rect_size.y - 2., self.corner * 0.5)
                    sdf.stroke(vec4(ring.rgb, ring.a * self.focus), 2.)
                }
                return sdf.result
            }
        }
        // Below the spread of TextInput, a drawer's own fields are not
        // visible to a shader written here, so the ones the animator drives
        // are declared again.
        draw_text +: {
            hover: instance(0.0)
            focus: instance(0.0)
            down: instance(0.0)
            empty: instance(0.0)
            disabled: instance(0.0)
            dark: uniform(0.0)
            inactive: uniform(0.0)
            color: #x0f172a
            color_dark: uniform(#xf8fafc)
            color_empty: uniform(#x5f6f85)
            color_empty_dark: uniform(#x94a3b8)
            color_disabled: uniform(#x475569)
            color_disabled_dark: uniform(#x94a3b8)
            text_style: TextStyle{font_size: 10.5 font_family: FontFamily{latin := FontMember{res: crate_resource("makepad_widgets:resources/Inter.ttf") asc: 0.0 desc: 0.0 weight: 400.0}}}
            get_color: fn() {
                return mix(self.color, self.color_dark, self.dark)
                    .mix(mix(self.color_empty, self.color_empty_dark, self.dark), self.empty)
                    .mix(mix(self.color_disabled, self.color_disabled_dark, self.dark), max(self.disabled, self.inactive))
            }
        }
        draw_cursor +: {
            focus: instance(0.0)
            down: instance(0.0)
            empty: instance(0.0)
            disabled: instance(0.0)
            blink: instance(0.0)
            border_radius: uniform(0.5)
            dark: uniform(0.0)
            color: uniform(#x0f172a)
            color_dark: uniform(#xf8fafc)
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.border_radius)
                let ink = mix(self.color, self.color_dark, self.dark)
                sdf.fill(vec4(ink.rgb, ink.a * (1.0 - self.blink) * self.focus))
                return sdf.result
            }
        }
        draw_selection +: {
            hover: instance(0.0)
            focus: instance(0.0)
            down: instance(0.0)
            empty: instance(0.0)
            disabled: instance(0.0)
            border_radius: uniform(2.0)
            dark: uniform(0.0)
            // The selection is painted over the text, so it is a tint, and
            // it goes with the focus.
            color: uniform(#x1d4ed840)
            color_dark: uniform(#x93c5fd59)
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.border_radius)
                let tint = mix(self.color, self.color_dark, self.dark)
                sdf.fill(vec4(tint.rgb, tint.a * self.focus))
                return sdf.result
            }
        }
    }

    // A field on a tinted bar: the search box.
    mod.widgets.QInputSubtle = mod.widgets.QInput{
        draw_bg +: {
            color: #xf1f5f9
            color_dark: #x1e293b
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

/// `<input>`: Makepad's `TextInput`, whose text the script sets on every
/// render. A plain field assignment keeps the caret where it was, so text
/// shorter than the caret (a bound field cleared while focused) made the next
/// IME update slice past the end and panic each frame. Text a render changes
/// goes through `set_text`, which clamps the caret.
#[derive(Script, Widget)]
pub struct QuinnTextInput {
    #[source]
    source: ScriptObjectRef,
    #[deref]
    input: TextInput,
    #[rust]
    before: String,
}

impl ScriptHook for QuinnTextInput {
    fn on_before_apply(
        &mut self,
        _vm: &mut ScriptVm,
        _apply: &Apply,
        _scope: &mut Scope,
        _value: ScriptValue,
    ) {
        self.before = self.input.text();
    }

    fn on_after_apply(
        &mut self,
        vm: &mut ScriptVm,
        _apply: &Apply,
        _scope: &mut Scope,
        _value: ScriptValue,
    ) {
        let text = self.input.text();
        if text != self.before {
            vm.with_cx_mut(|cx| self.input.set_text(cx, &text));
        }
    }
}

impl Widget for QuinnTextInput {
    fn script_call(
        &mut self,
        vm: &mut ScriptVm,
        method: LiveId,
        args: ScriptValue,
    ) -> ScriptAsyncResult {
        self.input.script_call(vm, method, args)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.input.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.input.draw_walk(cx, scope, walk)
    }

    fn text(&self) -> String {
        self.input.text()
    }

    fn set_text(&mut self, cx: &mut Cx, text: &str) {
        self.input.set_text(cx, text);
    }

    fn set_disabled(&mut self, cx: &mut Cx, disabled: bool) {
        self.input.set_disabled(cx, disabled);
    }

    fn disabled(&self, cx: &Cx) -> bool {
        self.input.disabled(cx)
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

std::thread_local! {
    /// Popups that drew visible on the last frame.
    static OPEN_POPUPS: std::cell::RefCell<Vec<WidgetUid>> = const { std::cell::RefCell::new(Vec::new()) };
    /// Popups the layer already gave the current event; their turn in the
    /// tree is skipped.
    static GIVEN: std::cell::RefCell<Vec<WidgetUid>> = const { std::cell::RefCell::new(Vec::new()) };
}

impl Widget for QuinnLayer {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        // A view hands events to its children last-first, and the first
        // widget to claim a press keeps it. A popup draws above the page but
        // sits earlier in the tree than the controls under it, so a field
        // below an open menu took the click. The layer is the host's last
        // child and sees every event first: it hands the event to each open
        // popup before the page gets it.
        GIVEN.with(|given| given.borrow_mut().clear());
        let open = OPEN_POPUPS.with(|open| open.borrow().clone());
        for uid in open {
            let popup = cx.widget_tree().widget(uid);
            if popup.is_empty() {
                continue;
            }
            popup.handle_event(cx, event, scope);
            GIVEN.with(|given| given.borrow_mut().push(uid));
        }
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
        let uid = self.view.widget_uid();
        let given = GIVEN.with(|given| {
            let mut given = given.borrow_mut();
            match given.iter().position(|have| *have == uid) {
                Some(at) => {
                    given.remove(at);
                    true
                }
                None => false,
            }
        });
        if !given {
            self.view.handle_event(cx, event, scope);
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let uid = self.view.widget_uid();
        OPEN_POPUPS.with(|open| {
            let mut open = open.borrow_mut();
            open.retain(|have| *have != uid);
            if self.view.visible {
                open.push(uid);
            }
        });
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
