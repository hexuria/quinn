//! A closed style subset. Values map onto Makepad `Walk`, `Layout`, and draw
//! fields. This is not CSS: there is no inheritance, cascade beyond
//! source-order utility classes, flex-grow, grid, or positioning.

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
        format!("#x{:02x}{:02x}{:02x}", self.r, self.g, self.b)
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeSpec {
    Fill,
    Fit,
    Px(f64),
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
    pub gap: Option<f64>,
    pub padding: Edges,
    pub margin: Edges,
    pub main_align: Option<AlignSpec>,
    pub cross_align: Option<AlignSpec>,
    pub width: Option<SizeSpec>,
    pub height: Option<SizeSpec>,
    pub color: Option<Rgba>,
    pub font_size: Option<f64>,
    pub background: Option<Rgba>,
    pub border_width: Option<f64>,
    pub border_color: Option<Rgba>,
    pub radius: Option<f64>,
}

impl Style {
    pub fn merge_from(&mut self, over: &Style) {
        if over.flow.is_some() {
            self.flow = over.flow;
        }
        if over.gap.is_some() {
            self.gap = over.gap;
        }
        self.padding.merge(over.padding);
        self.margin.merge(over.margin);
        if over.main_align.is_some() {
            self.main_align = over.main_align;
        }
        if over.cross_align.is_some() {
            self.cross_align = over.cross_align;
        }
        if over.width.is_some() {
            self.width = over.width;
        }
        if over.height.is_some() {
            self.height = over.height;
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
        if over.border_width.is_some() {
            self.border_width = over.border_width;
        }
        if over.border_color.is_some() {
            self.border_color = over.border_color;
        }
        if over.radius.is_some() {
            self.radius = over.radius;
        }
    }
}

/// Tailwind-like utilities. Spacing numbers are multiplied by 4px (`gap-4` = 16).
/// Unknown tokens are ignored so a class string can be extended later without
/// breaking the ones this milestone understands.
pub fn parse_classes(input: &str) -> Style {
    let mut style = Style::default();
    for token in input.split_whitespace() {
        apply_token(&mut style, token);
    }
    style
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
    match token {
        "flex" | "flex-row" => style.flow = Some(FlowSpec::Row),
        "flex-col" => style.flow = Some(FlowSpec::Column),
        "items-start" => style.cross_align = Some(AlignSpec::Start),
        "items-center" => style.cross_align = Some(AlignSpec::Center),
        "items-end" => style.cross_align = Some(AlignSpec::End),
        "justify-start" => style.main_align = Some(AlignSpec::Start),
        "justify-center" => style.main_align = Some(AlignSpec::Center),
        "justify-end" => style.main_align = Some(AlignSpec::End),
        "w-full" => style.width = Some(SizeSpec::Fill),
        "w-fit" => style.width = Some(SizeSpec::Fit),
        "h-full" => style.height = Some(SizeSpec::Fill),
        "h-fit" => style.height = Some(SizeSpec::Fit),
        "rounded" | "rounded-md" => style.radius = Some(8.0),
        "rounded-sm" => style.radius = Some(4.0),
        "rounded-lg" => style.radius = Some(12.0),
        "rounded-full" => style.radius = Some(999.0),
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
        _ => {
            if let Some(rest) = token.strip_prefix("gap-") {
                if let Some(px) = spacing(rest) {
                    style.gap = Some(px);
                }
            } else if let Some(rest) = token.strip_prefix("p-") {
                if let Some(px) = spacing(rest) {
                    style.padding = Edges::all(px);
                }
            } else if let Some(rest) = token.strip_prefix("px-") {
                if let Some(px) = spacing(rest) {
                    style.padding.left = Some(px);
                    style.padding.right = Some(px);
                }
            } else if let Some(rest) = token.strip_prefix("py-") {
                if let Some(px) = spacing(rest) {
                    style.padding.top = Some(px);
                    style.padding.bottom = Some(px);
                }
            } else if let Some(rest) = token.strip_prefix("pt-") {
                if let Some(px) = spacing(rest) {
                    style.padding.top = Some(px);
                }
            } else if let Some(rest) = token.strip_prefix("pr-") {
                if let Some(px) = spacing(rest) {
                    style.padding.right = Some(px);
                }
            } else if let Some(rest) = token.strip_prefix("pb-") {
                if let Some(px) = spacing(rest) {
                    style.padding.bottom = Some(px);
                }
            } else if let Some(rest) = token.strip_prefix("pl-") {
                if let Some(px) = spacing(rest) {
                    style.padding.left = Some(px);
                }
            } else if let Some(rest) = token.strip_prefix("m-") {
                if let Some(px) = spacing(rest) {
                    style.margin = Edges::all(px);
                }
            } else if let Some(rest) = token.strip_prefix("mx-") {
                if let Some(px) = spacing(rest) {
                    style.margin.left = Some(px);
                    style.margin.right = Some(px);
                }
            } else if let Some(rest) = token.strip_prefix("my-") {
                if let Some(px) = spacing(rest) {
                    style.margin.top = Some(px);
                    style.margin.bottom = Some(px);
                }
            } else if let Some(color) = token.strip_prefix("bg-").and_then(palette) {
                style.background = Some(color);
            } else if let Some(color) = token.strip_prefix("text-").and_then(palette) {
                style.color = Some(color);
            } else if let Some(color) = token.strip_prefix("border-").and_then(palette) {
                style.border_color = Some(color);
            }
        }
    }
}

fn spacing(token: &str) -> Option<f64> {
    if token == "px" {
        return Some(1.0);
    }
    token.parse::<f64>().ok().map(|n| n * 4.0)
}

fn palette(name: &str) -> Option<Rgba> {
    Some(match name {
        "white" => Rgba::new(255, 255, 255),
        "black" => Rgba::new(0, 0, 0),
        "slate-900" => Rgba::new(15, 23, 42),
        "slate-600" => Rgba::new(71, 85, 105),
        "slate-100" => Rgba::new(241, 245, 249),
        "stone-900" => Rgba::new(28, 25, 23),
        "stone-100" => Rgba::new(245, 245, 244),
        "amber-700" => Rgba::new(180, 83, 9),
        "amber-600" => Rgba::new(217, 119, 6),
        "amber-100" => Rgba::new(254, 243, 199),
        _ => return None,
    })
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
        "width" => style.width = parse_size(value),
        "height" => style.height = parse_size(value),
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
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Some(Rgba::new(r, g, b));
        }
    }
    None
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
}
