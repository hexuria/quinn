//! A closed style subset. Values map onto Makepad `Walk`, `Layout`, and draw
//! fields. This is not a CSS cascade: later utilities in one class string win,
//! and there is no inheritance.
//!
//! Color names are the Tailwind v3 palette plus Apple system colors. See
//! `docs/design-system.md`. An unknown class is reported by [`unknown_classes`].

mod palette_data;

/// sRGB color. Alpha is kept for script patches; Makepad hex form is `#xRRGGBB`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn hex_script(self) -> String {
        if self.a == 255 {
            format!("#x{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    pub fn channels(self) -> (f32, f32, f32, f32) {
        (
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowSpec {
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignSpec {
    Start,
    Center,
    End,
}

impl AlignSpec {
    pub fn fraction(self) -> f64 {
        match self {
            Self::Start => 0.0,
            Self::Center => 0.5,
            Self::End => 1.0,
        }
    }
}

/// Cross-axis alignment. On a row this is Makepad `RowAlign`. `Stretch` is
/// not a `RowAlign` variant; the emitter turns an unset cross size into `Fill`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossAlign {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistributeSpec {
    Between,
    Around,
    Evenly,
}

impl DistributeSpec {
    pub fn script(self) -> &'static str {
        match self {
            Self::Between => "SpaceBetween",
            Self::Around => "SpaceAround",
            Self::Evenly => "SpaceEvenly",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelBase {
    Parent,
    Vw,
    Vh,
}

impl RelBase {
    pub fn script(self) -> &'static str {
        match self {
            Self::Parent => "Base.Parent",
            Self::Vw => "Base.Vw",
            Self::Vh => "Base.Vh",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeSpec {
    Fill,
    Fit,
    Px(f64),
    Rel {
        base: RelBase,
        factor: f64,
    },
    /// `Size::Fill { weight, shrink, basis }` with basis in pixels.
    Flex {
        weight: f64,
        shrink: f64,
        basis: f64,
    },
    /// Control height from `state.quinn_density`: compact 28, standard 36, touch 48.
    Control,
    /// Top bar: 48 on compact and standard, 56 on touch.
    Bar,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bound {
    Px(f64),
    Full,
}

/// Grow / shrink / basis applied on the parent's main axis at emit time.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlexSpec {
    pub grow: Option<f64>,
    pub shrink: Option<f64>,
    pub basis: Option<f64>,
}

impl FlexSpec {
    pub fn is_empty(self) -> bool {
        self.grow.is_none() && self.shrink.is_none() && self.basis.is_none()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Edges {
    pub left: Option<f64>,
    pub top: Option<f64>,
    pub right: Option<f64>,
    pub bottom: Option<f64>,
}

impl Edges {
    pub fn all(v: f64) -> Self {
        Self {
            left: Some(v),
            top: Some(v),
            right: Some(v),
            bottom: Some(v),
        }
    }

    pub fn any(self) -> bool {
        self.left.is_some() || self.top.is_some() || self.right.is_some() || self.bottom.is_some()
    }

    pub fn merge(&mut self, over: Edges) {
        if over.left.is_some() {
            self.left = over.left;
        }
        if over.top.is_some() {
            self.top = over.top;
        }
        if over.right.is_some() {
            self.right = over.right;
        }
        if over.bottom.is_some() {
            self.bottom = over.bottom;
        }
    }

    pub fn resolve(self) -> (f64, f64, f64, f64) {
        (
            self.left.unwrap_or(0.0),
            self.top.unwrap_or(0.0),
            self.right.unwrap_or(0.0),
            self.bottom.unwrap_or(0.0),
        )
    }
}

/// Resolved style. `None` means "leave the Makepad prototype value alone"
/// except where tag defaults fill a field in before class parsing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub flow: Option<FlowSpec>,
    pub wrap: bool,
    /// Shorthand `gap-*`: both axes, unless `gap_x` / `gap_y` override.
    pub gap: Option<f64>,
    pub gap_x: Option<f64>,
    pub gap_y: Option<f64>,
    pub padding: Edges,
    pub margin: Edges,
    pub main_align: Option<AlignSpec>,
    pub cross_align: Option<CrossAlign>,
    pub align_self: Option<CrossAlign>,
    pub distribute: Option<DistributeSpec>,
    pub width: Option<SizeSpec>,
    pub height: Option<SizeSpec>,
    /// The class string set this axis. Tag defaults do not.
    pub width_explicit: bool,
    pub height_explicit: bool,
    pub min_width: Option<Bound>,
    pub max_width: Option<Bound>,
    pub min_height: Option<Bound>,
    pub max_height: Option<Bound>,
    pub flex: FlexSpec,
    pub color: Option<Rgba>,
    pub font_size: Option<f64>,
    pub background: Option<Rgba>,
    pub hover_bg: Option<Rgba>,
    pub active_bg: Option<Rgba>,
    pub disabled_bg: Option<Rgba>,
    pub focus_bg: Option<Rgba>,
    pub hover_fg: Option<Rgba>,
    pub active_fg: Option<Rgba>,
    pub disabled_fg: Option<Rgba>,
    pub focus_fg: Option<Rgba>,
    /// Bare `disabled` class: the button starts in the disabled animator state.
    pub disabled: bool,
    /// `px-control`: horizontal padding follows density (8 / 12 / 16).
    pub control_pad: bool,
    /// `min-h-control`: minimum height follows the control density scale.
    pub min_control: bool,
    pub border_width: Option<f64>,
    pub border_color: Option<Rgba>,
    pub radius: Option<f64>,
    /// `grid-cols-N`. The div is a `Grid` when this is set.
    pub grid_cols: Option<u32>,
    pub col_span: Option<u32>,
    pub absolute: bool,
    /// Parent-relative insets. `left` / `top` are `abs_x` / `abs_y`.
    /// `right` / `bottom` stretch or pin the other edge. Not draw-space.
    pub abs_x: Option<f64>,
    pub abs_y: Option<f64>,
    pub abs_right: Option<f64>,
    pub abs_bottom: Option<f64>,
    /// Paint order among overlay siblings. Higher paints later.
    pub z_index: Option<i64>,
    pub clip: bool,
    pub scroll_x: bool,
    pub scroll_y: bool,
    /// OpenType weight. 500 medium, 600 semibold, 700 bold.
    pub font_weight: Option<f64>,
    /// `TextStyle.line_spacing` multiplier.
    pub line_height: Option<f64>,
    pub gradient: Option<Gradient>,
    pub shadow: Option<Shadow>,
    /// Sidebar material. The emitter uses Makepad's scene-blur view.
    pub vibrancy: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientAxis {
    Horizontal,
    Vertical,
}

/// Two-stop fill. `flip` swaps the stops (`to-l`, `to-t`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gradient {
    pub axis: GradientAxis,
    pub flip: bool,
    /// Set by `bg-gradient-to-*`. Color tokens leave this false so they do not reset the axis.
    pub dir_set: bool,
    pub from: Option<Rgba>,
    pub to: Option<Rgba>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub radius: f64,
    pub offset_y: f64,
    pub color: Rgba,
}

impl Style {
    pub fn merge_from(&mut self, over: &Style) {
        if over.flow.is_some() {
            self.flow = over.flow;
        }
        if over.wrap {
            self.wrap = true;
        }
        if over.gap.is_some() {
            self.gap = over.gap;
        }
        if over.gap_x.is_some() {
            self.gap_x = over.gap_x;
        }
        if over.gap_y.is_some() {
            self.gap_y = over.gap_y;
        }
        self.padding.merge(over.padding);
        self.margin.merge(over.margin);
        if over.main_align.is_some() {
            self.main_align = over.main_align;
        }
        if over.cross_align.is_some() {
            self.cross_align = over.cross_align;
        }
        if over.align_self.is_some() {
            self.align_self = over.align_self;
        }
        if over.distribute.is_some() {
            self.distribute = over.distribute;
        }
        if over.width.is_some() {
            self.width = over.width;
        }
        if over.width_explicit {
            self.width_explicit = true;
        }
        if over.height.is_some() {
            self.height = over.height;
        }
        if over.height_explicit {
            self.height_explicit = true;
        }
        if over.min_width.is_some() {
            self.min_width = over.min_width;
        }
        if over.max_width.is_some() {
            self.max_width = over.max_width;
        }
        if over.min_height.is_some() {
            self.min_height = over.min_height;
        }
        if over.max_height.is_some() {
            self.max_height = over.max_height;
        }
        if over.flex.grow.is_some() {
            self.flex.grow = over.flex.grow;
        }
        if over.flex.shrink.is_some() {
            self.flex.shrink = over.flex.shrink;
        }
        if over.flex.basis.is_some() {
            self.flex.basis = over.flex.basis;
        }
        if over.color.is_some() {
            self.color = over.color;
        }
        if over.font_size.is_some() {
            self.font_size = over.font_size;
        }
        if over.background.is_some() {
            self.background = over.background;
        }
        if over.hover_bg.is_some() {
            self.hover_bg = over.hover_bg;
        }
        if over.active_bg.is_some() {
            self.active_bg = over.active_bg;
        }
        if over.disabled_bg.is_some() {
            self.disabled_bg = over.disabled_bg;
        }
        if over.focus_bg.is_some() {
            self.focus_bg = over.focus_bg;
        }
        if over.hover_fg.is_some() {
            self.hover_fg = over.hover_fg;
        }
        if over.active_fg.is_some() {
            self.active_fg = over.active_fg;
        }
        if over.disabled_fg.is_some() {
            self.disabled_fg = over.disabled_fg;
        }
        if over.focus_fg.is_some() {
            self.focus_fg = over.focus_fg;
        }
        if over.disabled {
            self.disabled = true;
        }
        if over.control_pad {
            self.control_pad = true;
        }
        if over.min_control {
            self.min_control = true;
        }
        if over.border_width.is_some() {
            self.border_width = over.border_width;
        }
        if over.border_color.is_some() {
            self.border_color = over.border_color;
        }
        if over.radius.is_some() {
            self.radius = over.radius;
        }
        if over.grid_cols.is_some() {
            self.grid_cols = over.grid_cols;
        }
        if over.col_span.is_some() {
            self.col_span = over.col_span;
        }
        if over.absolute {
            self.absolute = true;
        }
        if over.abs_x.is_some() {
            self.abs_x = over.abs_x;
        }
        if over.abs_y.is_some() {
            self.abs_y = over.abs_y;
        }
        if over.abs_right.is_some() {
            self.abs_right = over.abs_right;
        }
        if over.abs_bottom.is_some() {
            self.abs_bottom = over.abs_bottom;
        }
        if over.z_index.is_some() {
            self.z_index = over.z_index;
        }
        if over.clip {
            self.clip = true;
        }
        if over.scroll_x {
            self.scroll_x = true;
        }
        if over.scroll_y {
            self.scroll_y = true;
        }
        if over.font_weight.is_some() {
            self.font_weight = over.font_weight;
        }
        if over.line_height.is_some() {
            self.line_height = over.line_height;
        }
        if let Some(over_g) = over.gradient {
            let slot = self.gradient.get_or_insert(Gradient {
                axis: GradientAxis::Horizontal,
                flip: false,
                dir_set: false,
                from: None,
                to: None,
            });
            if over_g.from.is_some() {
                slot.from = over_g.from;
            }
            if over_g.to.is_some() {
                slot.to = over_g.to;
            }
            if over_g.dir_set {
                slot.axis = over_g.axis;
                slot.flip = over_g.flip;
                slot.dir_set = true;
            }
        }
        if over.shadow.is_some() {
            self.shadow = over.shadow;
        }
        if over.vibrancy {
            self.vibrancy = true;
        }
    }

    pub fn main_gap(&self) -> Option<f64> {
        self.gap_x.or(self.gap)
    }

    pub fn cross_gap(&self) -> Option<f64> {
        self.gap_y.or(self.gap)
    }
}

/// Tailwind-like utilities. Spacing numbers are multiplied by 4px (`gap-4` = 16).
/// Unknown tokens do not change the style. [`unknown_classes`] lists them so
/// the lowerer can fail the build instead of dropping them.
pub fn parse_classes(input: &str) -> Style {
    parse_classes_in(input, false)
}

/// Like [`parse_classes`], but Apple system colors use their dark sRGB value
/// when `dark` is set. Tailwind hues stay the same.
pub fn parse_classes_in(input: &str, dark: bool) -> Style {
    palette_data::set_dark(dark);
    let mut style = Style::default();
    for token in input.split_whitespace() {
        apply_token(&mut style, token);
    }
    palette_data::set_dark(false);
    style
}

/// Class tokens that do not map to a utility. Variant prefixes (`dark:`,
/// `md:`, `hover:`) are stripped before the check. The returned strings are
/// the original tokens, prefixes included.
pub fn unknown_classes(class: &str) -> Vec<String> {
    let mut out = Vec::new();
    for token in class.split_whitespace() {
        let bare = strip_known_prefixes(token);
        if bare.is_empty() {
            continue;
        }
        if !utility_known(bare) {
            out.push(token.to_string());
        }
    }
    out
}

fn strip_known_prefixes(token: &str) -> &str {
    let mut rest = token;
    loop {
        let next = if let Some(next) = rest.strip_prefix("dark:") {
            next
        } else if let Some(next) = rest.strip_prefix("sm:") {
            next
        } else if let Some(next) = rest.strip_prefix("md:") {
            next
        } else if let Some(next) = rest.strip_prefix("lg:") {
            next
        } else if let Some(next) = rest.strip_prefix("xl:") {
            next
        } else if let Some(next) = rest.strip_prefix("bp800:") {
            next
        } else if let Some(next) = rest.strip_prefix("bp1120:") {
            next
        } else if let Some(next) = rest.strip_prefix("bp1440:") {
            next
        } else if let Some(next) = rest.strip_prefix("hover:") {
            next
        } else if let Some(next) = rest.strip_prefix("active:") {
            next
        } else if let Some(next) = rest.strip_prefix("focus:") {
            next
        } else if let Some(next) = rest.strip_prefix("disabled:") {
            next
        } else if let Some(next) = rest.strip_prefix("even:") {
            next
        } else if let Some(next) = rest.strip_prefix("odd:") {
            next
        } else {
            break;
        };
        rest = next;
    }
    rest
}

fn utility_known(token: &str) -> bool {
    // These match a real utility but do not change `Style::default`.
    const NEUTRAL: &[&str] = &[
        "flex-nowrap",
        "self-auto",
        "grid",
        "to-r",
        "to-l",
        "to-t",
        "to-b",
        "to-tr",
        "to-tl",
        "to-br",
        "to-bl",
    ];
    if NEUTRAL.contains(&token) {
        return true;
    }
    let mut style = Style::default();
    apply_token(&mut style, token);
    style != Style::default()
}

/// `key: value; key: value` inline styles. Lengths are raw pixels.
pub fn parse_inline(input: &str) -> Style {
    let mut style = Style::default();
    for part in input.split(';') {
        let Some((key, value)) = part.split_once(':') else {
            continue;
        };
        apply_inline(&mut style, key.trim(), value.trim());
    }
    style
}

fn apply_token(style: &mut Style, token: &str) {
    // `sm:` / `dark:` are shell conditions. They are not part of the base style.
    if shell_token(token).is_some() {
        return;
    }
    if let Some(rest) = token.strip_prefix("hover:") {
        apply_variant(style, Variant::Hover, rest);
        return;
    }
    if let Some(rest) = token.strip_prefix("active:") {
        apply_variant(style, Variant::Active, rest);
        return;
    }
    if let Some(rest) = token.strip_prefix("disabled:") {
        apply_variant(style, Variant::Disabled, rest);
        return;
    }
    if let Some(rest) = token.strip_prefix("focus:") {
        apply_variant(style, Variant::Focus, rest);
        return;
    }
    if let Some((prefix, value)) = arbitrary(token) {
        apply_arbitrary(style, prefix, value);
        return;
    }
    match token {
        "flex" | "flex-row" => style.flow = Some(FlowSpec::Row),
        "flex-col" => style.flow = Some(FlowSpec::Column),
        "flex-nowrap" => style.wrap = false,
        "flex-wrap" => {
            style.wrap = true;
            if style.flow.is_none() {
                style.flow = Some(FlowSpec::Row);
            }
        }
        "flex-1" => {
            style.flex.grow = Some(1.0);
            style.flex.shrink = Some(1.0);
            style.flex.basis = Some(0.0);
        }
        "flex-none" => {
            style.flex.grow = Some(0.0);
            style.flex.shrink = Some(0.0);
        }
        "grow" => style.flex.grow = Some(1.0),
        "shrink" => style.flex.shrink = Some(1.0),
        "shrink-0" => style.flex.shrink = Some(0.0),
        "items-start" => style.cross_align = Some(CrossAlign::Start),
        "items-center" => style.cross_align = Some(CrossAlign::Center),
        "items-end" => style.cross_align = Some(CrossAlign::End),
        "items-stretch" => style.cross_align = Some(CrossAlign::Stretch),
        "self-auto" => style.align_self = None,
        "self-start" => style.align_self = Some(CrossAlign::Start),
        "self-center" => style.align_self = Some(CrossAlign::Center),
        "self-end" => style.align_self = Some(CrossAlign::End),
        "self-stretch" => style.align_self = Some(CrossAlign::Stretch),
        "justify-start" => style.main_align = Some(AlignSpec::Start),
        "justify-center" => style.main_align = Some(AlignSpec::Center),
        "justify-end" => style.main_align = Some(AlignSpec::End),
        "justify-between" => style.distribute = Some(DistributeSpec::Between),
        "justify-around" => style.distribute = Some(DistributeSpec::Around),
        "justify-evenly" => style.distribute = Some(DistributeSpec::Evenly),
        "w-full" => set_width(style, SizeSpec::Fill),
        "w-fit" => set_width(style, SizeSpec::Fit),
        "w-screen" => set_width(
            style,
            SizeSpec::Rel {
                base: RelBase::Vw,
                factor: 1.0,
            },
        ),
        "h-full" => set_height(style, SizeSpec::Fill),
        "h-fit" => set_height(style, SizeSpec::Fit),
        "h-screen" => set_height(
            style,
            SizeSpec::Rel {
                base: RelBase::Vh,
                factor: 1.0,
            },
        ),
        "disabled" => style.disabled = true,
        "rounded" | "rounded-md" => style.radius = Some(8.0),
        "rounded-sm" => style.radius = Some(4.0),
        "rounded-lg" => style.radius = Some(12.0),
        "rounded-full" => style.radius = Some(999.0),
        "border-0" => style.border_width = Some(0.0),
        "border" => {
            style.border_width = Some(1.0);
            if style.border_color.is_none() {
                style.border_color = Some(Rgba::new(214, 211, 209));
            }
        }
        "border-2" => {
            style.border_width = Some(2.0);
            if style.border_color.is_none() {
                style.border_color = Some(Rgba::new(214, 211, 209));
            }
        }
        "text-xs" => style.font_size = Some(12.0),
        "text-sm" => style.font_size = Some(14.0),
        "text-base" => style.font_size = Some(16.0),
        "text-lg" => style.font_size = Some(18.0),
        "text-xl" => style.font_size = Some(20.0),
        "text-2xl" => style.font_size = Some(24.0),
        "text-3xl" => style.font_size = Some(30.0),
        // macOS type scale. Sizes are points. Line spacing is line-height / size.
        "text-large-title" => type_scale(style, 26.0, 400.0, 32.0 / 26.0),
        "text-title1" | "text-title" => type_scale(style, 22.0, 400.0, 26.0 / 22.0),
        "text-title2" => type_scale(style, 17.0, 400.0, 22.0 / 17.0),
        "text-title3" => type_scale(style, 15.0, 400.0, 20.0 / 15.0),
        "text-headline" => type_scale(style, 13.0, 600.0, 16.0 / 13.0),
        "text-body" => type_scale(style, 14.0, 400.0, 20.0 / 14.0),
        "text-control" => type_scale(style, 14.0, 500.0, 20.0 / 14.0),
        "text-reading" => type_scale(style, 16.0, 400.0, 24.0 / 16.0),
        "text-section" => type_scale(style, 18.0, 600.0, 24.0 / 18.0),
        "text-page" => type_scale(style, 24.0, 600.0, 32.0 / 24.0),
        "text-display" => type_scale(style, 32.0, 600.0, 40.0 / 32.0),
        "text-callout" => type_scale(style, 12.0, 400.0, 15.0 / 12.0),
        "text-subheadline" => type_scale(style, 11.0, 400.0, 14.0 / 11.0),
        "text-footnote" => type_scale(style, 10.0, 400.0, 13.0 / 10.0),
        "text-caption" => type_scale(style, 12.0, 400.0, 16.0 / 12.0),
        "text-caption2" => type_scale(style, 10.0, 500.0, 13.0 / 10.0),
        "h-control-mini" => set_height(style, SizeSpec::Px(16.0)),
        "h-control-small" => set_height(style, SizeSpec::Px(18.0)),
        "h-control" | "h-control-regular" => set_height(style, SizeSpec::Control),
        "h-control-large" => set_height(style, SizeSpec::Px(28.0)),
        "w-control" => set_width(style, SizeSpec::Control),
        "h-toolbar" => set_height(style, SizeSpec::Bar),
        "min-h-control" => style.min_control = true,
        "px-control" => {
            style.control_pad = true;
            style.padding.left = Some(12.0);
            style.padding.right = Some(12.0);
        },
        "rounded-control-sm" => style.radius = Some(5.0),
        "rounded-control" => style.radius = Some(6.0),
        "rounded-control-lg" => style.radius = Some(8.0),
        "rounded-window" => style.radius = Some(10.0),
        "rounded-sheet" => style.radius = Some(12.0),
        "shadow-control" => style.shadow = Some(shadow(8.0, 2.0)),
        "shadow-window" => style.shadow = Some(shadow(24.0, 8.0)),
        "ring" => {
            style.border_width = Some(2.0);
            style.border_color = palette("system-blue");
        }
        "absolute" => style.absolute = true,
        "inset-0" => {
            style.absolute = true;
            style.abs_x = Some(0.0);
            style.abs_y = Some(0.0);
            style.abs_right = Some(0.0);
            style.abs_bottom = Some(0.0);
            set_width(style, SizeSpec::Fill);
            set_height(style, SizeSpec::Fill);
        }
        "z-0" => style.z_index = Some(0),
        "z-10" => style.z_index = Some(10),
        "z-20" => style.z_index = Some(20),
        "z-30" => style.z_index = Some(30),
        "z-40" => style.z_index = Some(40),
        "z-50" => style.z_index = Some(50),
        "overflow-hidden" => style.clip = true,
        "overflow-x-auto" => style.scroll_x = true,
        "overflow-y-auto" => style.scroll_y = true,
        "overflow-auto" => {
            style.scroll_x = true;
            style.scroll_y = true;
        }
        "font-medium" => style.font_weight = Some(500.0),
        "font-semibold" => style.font_weight = Some(600.0),
        "font-bold" => style.font_weight = Some(700.0),
        "leading-none" => style.line_height = Some(1.0),
        "leading-tight" => style.line_height = Some(1.25),
        "leading-snug" => style.line_height = Some(1.375),
        "leading-normal" => style.line_height = Some(1.5),
        "leading-relaxed" => style.line_height = Some(1.625),
        "leading-loose" => style.line_height = Some(2.0),
        "vibrancy" => style.vibrancy = true,
        "shadow" | "shadow-sm" => style.shadow = Some(shadow(3.0, 1.0)),
        "shadow-md" => style.shadow = Some(shadow(8.0, 4.0)),
        "shadow-lg" => style.shadow = Some(shadow(16.0, 8.0)),
        "bg-gradient-to-r" => set_gradient_dir(style, GradientAxis::Horizontal, false),
        "bg-gradient-to-l" => set_gradient_dir(style, GradientAxis::Horizontal, true),
        "bg-gradient-to-b" => set_gradient_dir(style, GradientAxis::Vertical, false),
        "bg-gradient-to-t" => set_gradient_dir(style, GradientAxis::Vertical, true),
        _ => apply_prefixed(style, token),
    }
}

fn type_scale(style: &mut Style, size: f64, weight: f64, leading: f64) {
    style.font_size = Some(size);
    style.font_weight = Some(weight);
    style.line_height = Some(leading);
}

fn shadow(radius: f64, offset_y: f64) -> Shadow {
    Shadow {
        radius,
        offset_y,
        color: Rgba::new(0, 0, 0),
    }
}

fn gradient_slot(style: &mut Style) -> &mut Gradient {
    style.gradient.get_or_insert(Gradient {
        axis: GradientAxis::Horizontal,
        flip: false,
        dir_set: false,
        from: None,
        to: None,
    })
}

fn set_gradient_dir(style: &mut Style, axis: GradientAxis, flip: bool) {
    let slot = gradient_slot(style);
    slot.axis = axis;
    slot.flip = flip;
    slot.dir_set = true;
}

fn set_width(style: &mut Style, size: SizeSpec) {
    style.width = Some(size);
    style.width_explicit = true;
}

fn set_height(style: &mut Style, size: SizeSpec) {
    style.height = Some(size);
    style.height_explicit = true;
}

fn apply_prefixed(style: &mut Style, token: &str) {
    if let Some(rest) = token.strip_prefix("w-") {
        if let Some(factor) = fraction(rest) {
            set_width(
                style,
                SizeSpec::Rel {
                    base: RelBase::Parent,
                    factor,
                },
            );
        }
    } else if let Some(rest) = token.strip_prefix("h-") {
        if let Some(factor) = fraction(rest) {
            set_height(
                style,
                SizeSpec::Rel {
                    base: RelBase::Parent,
                    factor,
                },
            );
        }
    } else if let Some(rest) = token.strip_prefix("gap-x-") {
        if let Some(px) = spacing(rest) {
            style.gap_x = Some(px);
        }
    } else if let Some(rest) = token.strip_prefix("gap-y-") {
        if let Some(px) = spacing(rest) {
            style.gap_y = Some(px);
        }
    } else if let Some(rest) = token.strip_prefix("gap-") {
        if let Some(px) = spacing(rest) {
            style.gap = Some(px);
        }
    } else if let Some(rest) = token.strip_prefix("basis-") {
        if let Some(px) = spacing(rest) {
            style.flex.basis = Some(px);
        }
    } else if let Some(rest) = token.strip_prefix("grow-") {
        if let Some(n) = rest.parse::<f64>().ok() {
            style.flex.grow = Some(n);
        }
    } else if let Some(rest) = token.strip_prefix("shrink-") {
        if let Some(n) = rest.parse::<f64>().ok() {
            style.flex.shrink = Some(n);
        }
    } else if let Some(rest) = token.strip_prefix("min-w-") {
        style.min_width = bound_token(rest);
    } else if let Some(rest) = token.strip_prefix("max-w-") {
        style.max_width = bound_token(rest);
    } else if let Some(rest) = token.strip_prefix("min-h-") {
        style.min_height = bound_token(rest);
    } else if let Some(rest) = token.strip_prefix("max-h-") {
        style.max_height = bound_token(rest);
    } else if let Some(rest) = strip_edge(token, "p") {
        apply_padding(style, rest.0, rest.1);
    } else if let Some(rest) = strip_edge(token, "m") {
        apply_margin(style, rest.0, rest.1);
    } else if let Some(color) = token.strip_prefix("bg-").and_then(palette) {
        style.background = Some(color);
    } else if let Some(color) = token.strip_prefix("text-").and_then(palette) {
        style.color = Some(color);
    } else if let Some(color) = token.strip_prefix("border-").and_then(palette) {
        style.border_color = Some(color);
    } else if let Some(rest) = token.strip_prefix("grid-cols-") {
        if let Ok(n) = rest.parse::<u32>() {
            if n > 0 {
                style.grid_cols = Some(n);
            }
        }
    } else if let Some(rest) = token.strip_prefix("col-span-") {
        if let Ok(n) = rest.parse::<u32>() {
            if n > 0 {
                style.col_span = Some(n);
            }
        }
    } else if let Some(rest) = token.strip_prefix("left-") {
        if let Some(px) = spacing(rest) {
            style.abs_x = Some(px);
        }
    } else if let Some(rest) = token.strip_prefix("right-") {
        if let Some(px) = spacing(rest) {
            style.abs_right = Some(px);
        }
    } else if let Some(rest) = token.strip_prefix("top-") {
        if let Some(px) = spacing(rest) {
            style.abs_y = Some(px);
        }
    } else if let Some(rest) = token.strip_prefix("bottom-") {
        if let Some(px) = spacing(rest) {
            style.abs_bottom = Some(px);
        }
    } else if let Some(rest) = token.strip_prefix("z-") {
        if let Ok(n) = rest.parse::<i64>() {
            style.z_index = Some(n);
        }
    } else if let Some(rest) = token.strip_prefix("from-") {
        if let Some(color) = palette(rest) {
            gradient_slot(style).from = Some(color);
        }
    } else if let Some(rest) = token.strip_prefix("to-") {
        if matches!(rest, "r" | "l" | "t" | "b" | "tr" | "tl" | "br" | "bl") {
            return;
        }
        if let Some(color) = palette(rest) {
            gradient_slot(style).to = Some(color);
        }
    } else if let Some(rest) = token.strip_prefix("leading-") {
        if let Some(px) = spacing(rest) {
            style.line_height = Some(px / 16.0);
        }
    }
}

fn strip_edge<'a>(token: &'a str, prefix: &str) -> Option<(&'a str, &'a str)> {
    let rest = token.strip_prefix(prefix)?;
    if let Some(n) = rest.strip_prefix("x-") {
        return Some(("x", n));
    }
    if let Some(n) = rest.strip_prefix("y-") {
        return Some(("y", n));
    }
    if let Some(n) = rest.strip_prefix("t-") {
        return Some(("t", n));
    }
    if let Some(n) = rest.strip_prefix("r-") {
        return Some(("r", n));
    }
    if let Some(n) = rest.strip_prefix("b-") {
        return Some(("b", n));
    }
    if let Some(n) = rest.strip_prefix("l-") {
        return Some(("l", n));
    }
    let n = rest.strip_prefix('-')?;
    Some(("", n))
}

fn apply_padding(style: &mut Style, edge: &str, token: &str) {
    let Some(px) = spacing(token) else {
        return;
    };
    paint_edge(&mut style.padding, edge, px);
}

fn apply_margin(style: &mut Style, edge: &str, token: &str) {
    let Some(px) = spacing(token) else {
        return;
    };
    paint_edge(&mut style.margin, edge, px);
}

fn paint_edge(edges: &mut Edges, edge: &str, px: f64) {
    match edge {
        "" => *edges = Edges::all(px),
        "x" => {
            edges.left = Some(px);
            edges.right = Some(px);
        }
        "y" => {
            edges.top = Some(px);
            edges.bottom = Some(px);
        }
        "t" => edges.top = Some(px),
        "r" => edges.right = Some(px),
        "b" => edges.bottom = Some(px),
        "l" => edges.left = Some(px),
        _ => {}
    }
}

fn bound_token(token: &str) -> Option<Bound> {
    match token {
        "0" => Some(Bound::Px(0.0)),
        "full" => Some(Bound::Full),
        other => spacing(other).map(Bound::Px),
    }
}

fn fraction(token: &str) -> Option<f64> {
    let (num, den) = token.split_once('/')?;
    let num: f64 = num.parse().ok()?;
    let den: f64 = den.parse().ok()?;
    if den == 0.0 {
        return None;
    }
    Some(num / den)
}

#[derive(Clone, Copy)]
enum Variant {
    Hover,
    Active,
    Disabled,
    Focus,
}

/// Tailwind defaults: sm 640, md 768, lg 1024, xl 1280.
/// `dark:` reads `state.quinn_dark`. Not a media query.
#[derive(Clone, Debug, PartialEq)]
pub struct ShellWhen {
    pub dark: bool,
    pub min_width: Option<f64>,
    /// Utility after the variant prefixes, including `hover:` / `focus:`.
    pub token: String,
}

pub fn shell_variants(class: &str) -> Vec<ShellWhen> {
    class.split_whitespace().filter_map(shell_token).collect()
}

fn shell_token(token: &str) -> Option<ShellWhen> {
    let mut dark = false;
    let mut min_width = None;
    let mut rest = token;
    loop {
        if let Some(next) = rest.strip_prefix("dark:") {
            dark = true;
            rest = next;
            continue;
        }
        let bp = [
            ("sm:", 640.0),
            ("md:", 768.0),
            ("lg:", 1024.0),
            ("xl:", 1280.0),
            ("bp800:", 800.0),
            ("bp1120:", 1120.0),
            ("bp1440:", 1440.0),
        ];
        let mut hit = false;
        for (prefix, width) in bp {
            if let Some(next) = rest.strip_prefix(prefix) {
                min_width = Some(min_width.map_or(width, |have: f64| have.max(width)));
                rest = next;
                hit = true;
                break;
            }
        }
        if !hit {
            break;
        }
    }
    if !dark && min_width.is_none() {
        return None;
    }
    if rest.is_empty() {
        return None;
    }
    Some(ShellWhen {
        dark,
        min_width,
        token: rest.to_string(),
    })
}

fn apply_variant(style: &mut Style, variant: Variant, token: &str) {
    let mut painted = Style::default();
    apply_token(&mut painted, token);
    match variant {
        Variant::Hover => {
            if painted.background.is_some() {
                style.hover_bg = painted.background;
            }
            if painted.color.is_some() {
                style.hover_fg = painted.color;
            }
        }
        Variant::Active => {
            if painted.background.is_some() {
                style.active_bg = painted.background;
            }
            if painted.color.is_some() {
                style.active_fg = painted.color;
            }
        }
        Variant::Disabled => {
            if painted.background.is_some() {
                style.disabled_bg = painted.background;
            }
            if painted.color.is_some() {
                style.disabled_fg = painted.color;
            }
        }
        Variant::Focus => {
            if painted.background.is_some() {
                style.focus_bg = painted.background;
            }
            if painted.color.is_some() {
                style.focus_fg = painted.color;
            }
        }
    }
}

fn arbitrary<'a>(token: &'a str) -> Option<(&'a str, &'a str)> {
    let (prefix, rest) = token.split_once("-[")?;
    let value = rest.strip_suffix(']')?;
    if prefix.is_empty() || value.is_empty() {
        return None;
    }
    Some((prefix, value))
}

fn apply_arbitrary(style: &mut Style, prefix: &str, value: &str) {
    match prefix {
        "w" => {
            if let Some(px) = parse_px(value) {
                set_width(style, SizeSpec::Px(px));
            }
        }
        "h" => {
            if let Some(px) = parse_px(value) {
                set_height(style, SizeSpec::Px(px));
            }
        }
        "min-w" => style.min_width = parse_px(value).map(Bound::Px),
        "max-w" => style.max_width = parse_px(value).map(Bound::Px),
        "min-h" => style.min_height = parse_px(value).map(Bound::Px),
        "max-h" => style.max_height = parse_px(value).map(Bound::Px),
        "p" | "px" | "py" | "pt" | "pr" | "pb" | "pl" => {
            if let Some(px) = parse_px(value) {
                let edge = match prefix {
                    "px" => "x",
                    "py" => "y",
                    "pt" => "t",
                    "pr" => "r",
                    "pb" => "b",
                    "pl" => "l",
                    _ => "",
                };
                paint_edge(&mut style.padding, edge, px);
            }
        }
        "left" => {
            if let Some(px) = parse_px(value) {
                style.abs_x = Some(px);
            }
        }
        "right" => {
            if let Some(px) = parse_px(value) {
                style.abs_right = Some(px);
            }
        }
        "top" => {
            if let Some(px) = parse_px(value) {
                style.abs_y = Some(px);
            }
        }
        "bottom" => {
            if let Some(px) = parse_px(value) {
                style.abs_bottom = Some(px);
            }
        }
        "z" => {
            if let Ok(n) = value.parse::<i64>() {
                style.z_index = Some(n);
            }
        }
        "leading" => {
            if let Some(px) = parse_px(value) {
                style.line_height = Some(px / 16.0);
            } else if let Ok(n) = value.parse::<f64>() {
                style.line_height = Some(n);
            }
        }
        "m" | "mx" | "my" | "mt" | "mr" | "mb" | "ml" => {
            if let Some(px) = parse_px(value) {
                let edge = match prefix {
                    "mx" => "x",
                    "my" => "y",
                    "mt" => "t",
                    "mr" => "r",
                    "mb" => "b",
                    "ml" => "l",
                    _ => "",
                };
                paint_edge(&mut style.margin, edge, px);
            }
        }
        "gap" => style.gap = parse_px(value),
        "gap-x" => style.gap_x = parse_px(value),
        "gap-y" => style.gap_y = parse_px(value),
        "basis" => style.flex.basis = parse_px(value),
        "rounded" => style.radius = parse_px(value),
        "border" => style.border_width = parse_px(value),
        "text" => {
            if value.starts_with('#') {
                style.color = parse_color(value);
            } else {
                style.font_size = parse_px(value);
            }
        }
        "bg" => style.background = parse_color(value),
        "border-color" => style.border_color = parse_color(value),
        _ => {}
    }
}

fn spacing(token: &str) -> Option<f64> {
    if token == "px" {
        return Some(1.0);
    }
    token.parse::<f64>().ok().map(|n| n * 4.0)
}

fn palette(name: &str) -> Option<Rgba> {
    palette_data::color(name)
}

fn apply_inline(style: &mut Style, key: &str, value: &str) {
    match key {
        "color" => style.color = parse_color(value),
        "background" | "background-color" => style.background = parse_color(value),
        "font-size" => style.font_size = parse_px(value),
        "gap" => style.gap = parse_px(value),
        "radius" | "border-radius" => style.radius = parse_px(value),
        "border" | "border-width" => style.border_width = parse_px(value),
        "border-color" => style.border_color = parse_color(value),
        "padding" => {
            if let Some(px) = parse_px(value) {
                style.padding = Edges::all(px);
            }
        }
        "margin" => {
            if let Some(px) = parse_px(value) {
                style.margin = Edges::all(px);
            }
        }
        "width" => {
            if let Some(size) = parse_size(value) {
                set_width(style, size);
            }
        }
        "height" => {
            if let Some(size) = parse_size(value) {
                set_height(style, size);
            }
        }
        _ => {}
    }
}

fn parse_px(value: &str) -> Option<f64> {
    let value = value.trim().trim_end_matches("px");
    value.parse::<f64>().ok()
}

fn parse_size(value: &str) -> Option<SizeSpec> {
    match value {
        "fill" | "100%" => Some(SizeSpec::Fill),
        "fit" | "auto" => Some(SizeSpec::Fit),
        _ => parse_px(value).map(SizeSpec::Px),
    }
}

fn parse_color(value: &str) -> Option<Rgba> {
    if let Some(color) = palette(value) {
        return Some(color);
    }
    let hex = value.trim().trim_start_matches('#');
    if hex.len() == 3 {
        let chars: Vec<char> = hex.chars().collect();
        let expanded: String = chars.iter().flat_map(|ch| [*ch, *ch]).collect();
        return parse_hex6(&expanded);
    }
    if hex.len() == 6 {
        return parse_hex6(hex);
    }
    if hex.len() == 8 {
        let mut color = parse_hex6(&hex[..6])?;
        color.a = u8::from_str_radix(&hex[6..8], 16).ok()?;
        return Some(color);
    }
    None
}

fn parse_hex6(hex: &str) -> Option<Rgba> {
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Rgba::new(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_string_matches_the_counter_example() {
        let style = parse_classes("flex flex-col gap-4 p-4");
        assert_eq!(style.flow, Some(FlowSpec::Column));
        assert_eq!(style.gap, Some(16.0));
        assert_eq!(style.padding, Edges::all(16.0));
    }

    #[test]
    fn later_flow_token_wins() {
        let style = parse_classes("flex-col flex-row");
        assert_eq!(style.flow, Some(FlowSpec::Row));
    }

    #[test]
    fn inline_overrides_are_pixel_values() {
        let style = parse_inline("color: #1c1917; font-size: 20px; width: fill");
        assert_eq!(style.color, Some(Rgba::new(0x1c, 0x19, 0x17)));
        assert_eq!(style.font_size, Some(20.0));
        assert_eq!(style.width, Some(SizeSpec::Fill));
    }

    #[test]
    fn p0_layout_utilities() {
        let style = parse_classes(
            "flex flex-wrap gap-2 gap-y-4 justify-between items-center self-end flex-1 shrink-0 basis-4 min-w-8 max-w-full min-h-0 max-h-16 w-1/2 h-screen",
        );
        assert_eq!(style.flow, Some(FlowSpec::Row));
        assert!(style.wrap);
        assert_eq!(style.gap, Some(8.0));
        assert_eq!(style.gap_y, Some(16.0));
        assert_eq!(style.distribute, Some(DistributeSpec::Between));
        assert_eq!(style.cross_align, Some(CrossAlign::Center));
        assert_eq!(style.align_self, Some(CrossAlign::End));
        assert_eq!(style.flex.grow, Some(1.0));
        assert_eq!(style.flex.shrink, Some(0.0));
        assert_eq!(style.flex.basis, Some(16.0));
        assert_eq!(style.min_width, Some(Bound::Px(32.0)));
        assert_eq!(style.max_width, Some(Bound::Full));
        assert_eq!(style.min_height, Some(Bound::Px(0.0)));
        assert_eq!(style.max_height, Some(Bound::Px(64.0)));
        assert_eq!(
            style.width,
            Some(SizeSpec::Rel {
                base: RelBase::Parent,
                factor: 0.5
            })
        );
        assert_eq!(
            style.height,
            Some(SizeSpec::Rel {
                base: RelBase::Vh,
                factor: 1.0
            })
        );
        assert!(style.width_explicit && style.height_explicit);
    }

    #[test]
    fn per_side_spacing_and_arbitrary_values() {
        let style = parse_classes("px-4 py-2 mt-1 mr-3 ml-2 mb-5 mx-1");
        assert_eq!(style.padding.left, Some(16.0));
        assert_eq!(style.padding.right, Some(16.0));
        assert_eq!(style.padding.top, Some(8.0));
        assert_eq!(style.padding.bottom, Some(8.0));
        assert_eq!(style.margin.top, Some(4.0));
        assert_eq!(style.margin.bottom, Some(20.0));
        assert_eq!(style.margin.left, Some(4.0));
        assert_eq!(style.margin.right, Some(4.0));
        let arb = parse_classes("w-[13px] bg-[#1e293b] text-[#fff] p-[10px] m-[6px]");
        assert_eq!(arb.width, Some(SizeSpec::Px(13.0)));
        assert_eq!(arb.background, Some(Rgba::new(0x1e, 0x29, 0x3b)));
        assert_eq!(arb.color, Some(Rgba::new(255, 255, 255)));
        assert_eq!(arb.padding, Edges::all(10.0));
        assert_eq!(arb.margin, Edges::all(6.0));
    }

    #[test]
    fn hover_active_and_disabled_colors_stay_separate() {
        let style = parse_classes("bg-stone-900 hover:bg-amber-600 active:bg-amber-700 disabled:bg-slate-100 disabled:text-slate-600 hover:text-white");
        assert_eq!(style.background, Some(Rgba::new(28, 25, 23)));
        assert_eq!(style.hover_bg, Some(Rgba::new(217, 119, 6)));
        assert_eq!(style.active_bg, Some(Rgba::new(180, 83, 9)));
        assert_eq!(style.disabled_bg, Some(Rgba::new(241, 245, 249)));
        assert_eq!(style.disabled_fg, Some(Rgba::new(71, 85, 105)));
        assert_eq!(style.hover_fg, Some(Rgba::new(255, 255, 255)));
        assert!(!style.disabled);
        assert!(parse_classes("disabled").disabled);
    }

    #[test]
    fn wrap_column_grow_and_the_other_distributes() {
        let column = parse_classes("flex-col flex-wrap");
        assert_eq!(column.flow, Some(FlowSpec::Column));
        assert!(column.wrap);
        assert!(!parse_classes("flex-wrap flex-nowrap").wrap);
        let around = parse_classes("justify-around");
        assert_eq!(around.distribute, Some(DistributeSpec::Around));
        let evenly = parse_classes("justify-evenly");
        assert_eq!(evenly.distribute, Some(DistributeSpec::Evenly));
        let cross = parse_classes("items-stretch items-end self-stretch");
        assert_eq!(cross.cross_align, Some(CrossAlign::End));
        assert_eq!(cross.align_self, Some(CrossAlign::Stretch));
        assert_eq!(
            parse_classes("self-start").align_self,
            Some(CrossAlign::Start)
        );
        assert_eq!(
            parse_classes("self-center").align_self,
            Some(CrossAlign::Center)
        );
        let grow = parse_classes("grow shrink w-screen gap-x-3");
        assert_eq!(grow.flex.grow, Some(1.0));
        assert_eq!(grow.flex.shrink, Some(1.0));
        assert_eq!(
            grow.width,
            Some(SizeSpec::Rel {
                base: RelBase::Vw,
                factor: 1.0
            })
        );
        assert_eq!(grow.gap_x, Some(12.0));
        assert_eq!(grow.main_gap(), Some(12.0));
        assert_eq!(parse_classes("gap-y-1").cross_gap(), Some(4.0));
    }

    #[test]
    fn three_digit_hex_and_fractions() {
        let style = parse_classes("bg-[#abc] w-2/3 w-1/4");
        assert_eq!(style.background, Some(Rgba::new(0xaa, 0xbb, 0xcc)));
        assert_eq!(
            style.width,
            Some(SizeSpec::Rel {
                base: RelBase::Parent,
                factor: 0.25
            })
        );
    }

    #[test]
    fn p1_grid_overflow_type_gradient_and_shadow() {
        let style = parse_classes(
            "grid grid-cols-3 gap-2 col-span-2 absolute left-2 top-[10px] overflow-hidden overflow-y-auto font-bold leading-tight bg-gradient-to-r from-amber-600 to-stone-900 shadow-lg",
        );
        assert_eq!(style.grid_cols, Some(3));
        assert_eq!(style.col_span, Some(2));
        assert_eq!(style.gap, Some(8.0));
        assert!(style.absolute);
        assert_eq!(style.abs_x, Some(8.0));
        assert_eq!(style.abs_y, Some(10.0));
        assert!(style.clip);
        assert!(style.scroll_y);
        assert_eq!(style.font_weight, Some(700.0));
        assert_eq!(style.line_height, Some(1.25));
        let gradient = style.gradient.unwrap();
        assert_eq!(gradient.axis, GradientAxis::Horizontal);
        assert!(!gradient.flip);
        assert_eq!(gradient.from, Some(Rgba::new(217, 119, 6)));
        assert_eq!(gradient.to, Some(Rgba::new(28, 25, 23)));
        assert_eq!(style.shadow.unwrap().radius, 16.0);
        let medium = parse_classes("font-medium font-semibold leading-[1.4] leading-[20px] overflow-x-auto shadow-md bg-gradient-to-b");
        assert_eq!(medium.font_weight, Some(600.0));
        assert_eq!(medium.line_height, Some(20.0 / 16.0));
        assert!(medium.scroll_x);
        assert_eq!(medium.shadow.unwrap().radius, 8.0);
        assert_eq!(medium.gradient.unwrap().axis, GradientAxis::Vertical);
    }

    #[test]
    fn p2_breakpoints_dark_inset_z_and_focus_do_not_paint_the_base() {
        let style = parse_classes(
            "bg-stone-100 dark:bg-stone-900 md:flex-row sm:p-4 inset-0 z-40 focus:bg-amber-100",
        );
        assert_eq!(style.background, Some(Rgba::new(245, 245, 244)));
        assert_eq!(style.flow, None);
        assert!(style.absolute);
        assert_eq!(style.abs_x, Some(0.0));
        assert_eq!(style.abs_y, Some(0.0));
        assert_eq!(style.abs_right, Some(0.0));
        assert_eq!(style.abs_bottom, Some(0.0));
        assert_eq!(style.width, Some(SizeSpec::Fill));
        assert_eq!(style.height, Some(SizeSpec::Fill));
        assert_eq!(style.z_index, Some(40));
        assert_eq!(style.focus_bg, Some(Rgba::new(254, 243, 199)));
        let edge = parse_classes("absolute right-2 bottom-3 z-10");
        assert_eq!(edge.abs_right, Some(8.0));
        assert_eq!(edge.abs_bottom, Some(12.0));
        assert!(edge.abs_x.is_none());
        assert!(edge.abs_y.is_none());
        assert_eq!(edge.z_index, Some(10));
        let variants = shell_variants(
            "dark:bg-stone-900 md:flex-row dark:md:bg-white xl:z-50 hover:bg-amber-600",
        );
        assert_eq!(variants.len(), 4);
        assert!(variants[0].dark && variants[0].min_width.is_none());
        assert_eq!(variants[0].token, "bg-stone-900");
        assert!(!variants[1].dark);
        assert_eq!(variants[1].min_width, Some(768.0));
        assert!(variants[2].dark);
        assert_eq!(variants[2].min_width, Some(768.0));
        assert_eq!(variants[2].token, "bg-white");
        assert_eq!(variants[3].min_width, Some(1280.0));
        let ink = parse_classes("bg-[#1c191780]");
        assert_eq!(ink.background.unwrap().a, 0x80);
        assert_eq!(ink.background.unwrap().hex_script(), "#1c191780");
    }

    #[test]
    fn tailwind_palette_apple_tokens_and_unknown_classes() {
        let sky = parse_classes("bg-sky-100 text-rose-950");
        assert_eq!(sky.background, Some(Rgba::new(224, 242, 254)));
        assert_eq!(sky.color, Some(Rgba::new(76, 5, 25)));
        assert!(unknown_classes("bg-sky-100 text-body").is_empty());
        assert_eq!(
            unknown_classes("bg-not-a-color md:text-nope"),
            vec!["bg-not-a-color".to_string(), "md:text-nope".to_string()]
        );
        let blue = parse_classes("bg-system-blue text-label");
        assert_eq!(blue.background, Some(Rgba::new(0, 122, 255)));
        assert_eq!(blue.color.unwrap().a, 217);
        let dark = parse_classes_in("bg-system-blue text-label", true);
        assert_eq!(dark.background, Some(Rgba::new(10, 132, 255)));
        assert_eq!(dark.color.unwrap().r, 255);
        let headline = parse_classes("text-headline h-control-regular rounded-control");
        assert_eq!(headline.font_size, Some(13.0));
        assert_eq!(headline.font_weight, Some(600.0));
        assert_eq!(headline.height, Some(SizeSpec::Control));
        assert_eq!(headline.radius, Some(6.0));
        let body = parse_classes("text-body text-control text-reading text-section text-page text-display text-caption");
        assert_eq!(parse_classes("text-body").font_size, Some(14.0));
        assert_eq!(parse_classes("text-control").font_weight, Some(500.0));
        assert_eq!(parse_classes("text-reading").font_size, Some(16.0));
        assert_eq!(parse_classes("text-section").font_size, Some(18.0));
        assert_eq!(parse_classes("text-page").font_size, Some(24.0));
        assert_eq!(parse_classes("text-display").font_size, Some(32.0));
        assert_eq!(parse_classes("text-caption").font_size, Some(12.0));
        let _ = body;
        let accent = parse_classes("bg-accent text-on-accent border-control");
        assert_eq!(accent.background, Some(Rgba::new(29, 78, 216)));
        assert_eq!(accent.color, Some(Rgba::new(255, 255, 255)));
        assert_eq!(accent.border_color, Some(Rgba::new(100, 116, 139)));
        let canvas = parse_classes_in("bg-canvas text-secondary", true);
        assert_eq!(canvas.background, Some(Rgba::new(15, 23, 42)));
        assert_eq!(canvas.color, Some(Rgba::new(203, 213, 225)));
        assert_eq!(
            parse_classes_in("text-muted", true).color,
            Some(Rgba::new(148, 163, 184))
        );
        let pad = parse_classes("px-control min-h-control h-toolbar w-control");
        assert!(pad.control_pad);
        assert!(pad.min_control);
        assert_eq!(pad.height, Some(SizeSpec::Bar));
        assert_eq!(pad.width, Some(SizeSpec::Control));
        let bp = shell_variants("bp800:w-[64px] bp1120:w-[224px] bp1440:flex");
        assert_eq!(bp[0].min_width, Some(800.0));
        assert_eq!(bp[1].min_width, Some(1120.0));
        assert_eq!(bp[2].min_width, Some(1440.0));
        assert!(unknown_classes("bg-canvas text-primary bp800:bg-surface").is_empty());
        assert!(unknown_classes("grid flex-nowrap to-r").is_empty());
        let hairline = parse_classes("border-0");
        assert_eq!(hairline.border_width, Some(0.0));
    }
}
