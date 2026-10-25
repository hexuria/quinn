use std::any::Any;

use quinn_styles::Style;

/// Tags the markup crate understands. Each one is a Makepad widget prototype,
/// not an HTML element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    Div,
    Span,
    H1,
    H2,
    P,
    Label,
    Button,
    Input,
}

impl Tag {
    pub fn is_text(self) -> bool {
        matches!(
            self,
            Self::Span | Self::H1 | Self::H2 | Self::P | Self::Label
        )
    }

    pub fn defaults(self) -> Style {
        let mut style = Style::default();
        let ink = quinn_styles::Rgba::new(28, 25, 23);
        match self {
            Self::Div => {
                style.flow = Some(quinn_styles::FlowSpec::Column);
                style.width = Some(quinn_styles::SizeSpec::Fill);
                style.height = Some(quinn_styles::SizeSpec::Fit);
            }
            Self::H1 => {
                style.color = Some(ink);
                style.font_size = Some(28.0);
                style.width = Some(quinn_styles::SizeSpec::Fill);
                style.height = Some(quinn_styles::SizeSpec::Fit);
            }
            Self::H2 => {
                style.color = Some(ink);
                style.font_size = Some(22.0);
                style.width = Some(quinn_styles::SizeSpec::Fill);
                style.height = Some(quinn_styles::SizeSpec::Fit);
            }
            Self::Span | Self::P | Self::Label => {
                style.color = Some(ink);
                style.font_size = Some(16.0);
                style.width = Some(quinn_styles::SizeSpec::Fit);
                style.height = Some(quinn_styles::SizeSpec::Fit);
            }
            Self::Button => {
                style.color = Some(quinn_styles::Rgba::new(246, 243, 236));
                style.font_size = Some(16.0);
                style.background = Some(quinn_styles::Rgba::new(28, 25, 23));
                style.radius = Some(8.0);
                style.padding = quinn_styles::Edges::all(10.0);
                style.width = Some(quinn_styles::SizeSpec::Fit);
                style.height = Some(quinn_styles::SizeSpec::Fit);
            }
            Self::Input => {
                style.color = Some(ink);
                style.font_size = Some(16.0);
                style.background = Some(quinn_styles::Rgba::new(255, 255, 255));
                style.border_width = Some(1.0);
                style.border_color = Some(quinn_styles::Rgba::new(214, 211, 209));
                style.radius = Some(6.0);
                style.padding = quinn_styles::Edges::all(8.0);
                style.width = Some(quinn_styles::SizeSpec::Fill);
                style.height = Some(quinn_styles::SizeSpec::Px(36.0));
            }
        }
        style
    }
}

#[derive(Clone, Debug)]
pub struct NodeSpec {
    pub tag: Tag,
    pub style: Style,
    pub text: String,
    pub placeholder: String,
}

/// Renderer operations. Implementations run on the UI thread.
pub trait Surface {
    fn create(&mut self, parent: Option<u64>, spec: &NodeSpec) -> u64;
    fn remove(&mut self, id: u64);
    fn set_text(&mut self, id: u64, text: &str);
    fn text(&self, id: u64) -> String;
    fn set_style(&mut self, id: u64, style: &Style);
    fn set_placeholder(&mut self, id: u64, text: &str);
    fn set_order(&mut self, parent: u64, children: &[u64]);
    fn redraw(&mut self);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[derive(Clone, Debug)]
struct MemNode {
    parent: Option<u64>,
    #[allow(dead_code)]
    tag: Tag,
    text: String,
    placeholder: String,
    style: Style,
    alive: bool,
    children: Vec<u64>,
}

/// In-memory surface for lifecycle tests. Ids start at 1 and are never reused.
#[derive(Default)]
pub struct MemorySurface {
    next: u64,
    nodes: std::collections::HashMap<u64, MemNode>,
    pub log: Vec<String>,
    pub redraws: usize,
}

impl MemorySurface {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alive(&self, id: u64) -> bool {
        self.nodes.get(&id).is_some_and(|n| n.alive)
    }

    pub fn children(&self, id: u64) -> Vec<u64> {
        self.nodes
            .get(&id)
            .map(|n| n.children.clone())
            .unwrap_or_default()
    }
}

impl Surface for MemorySurface {
    fn create(&mut self, parent: Option<u64>, spec: &NodeSpec) -> u64 {
        self.next += 1;
        let id = self.next;
        if let Some(parent) = parent {
            if let Some(node) = self.nodes.get_mut(&parent) {
                node.children.push(id);
            }
        }
        self.log
            .push(format!("create {id} {:?} parent={parent:?}", spec.tag));
        self.nodes.insert(
            id,
            MemNode {
                parent,
                tag: spec.tag,
                text: spec.text.clone(),
                placeholder: spec.placeholder.clone(),
                style: spec.style.clone(),
                alive: true,
                children: Vec::new(),
            },
        );
        id
    }

    fn remove(&mut self, id: u64) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.alive = false;
            let parent = node.parent;
            if let Some(parent) = parent {
                if let Some(parent_node) = self.nodes.get_mut(&parent) {
                    parent_node.children.retain(|child| *child != id);
                }
            }
        }
        self.log.push(format!("remove {id}"));
    }

    fn set_text(&mut self, id: u64, text: &str) {
        if let Some(node) = self.nodes.get_mut(&id) {
            if node.text == text {
                self.log.push(format!("set_text {id} skipped"));
                return;
            }
            node.text = text.to_string();
        }
        self.log.push(format!("set_text {id} {text}"));
    }

    fn text(&self, id: u64) -> String {
        self.nodes
            .get(&id)
            .map(|n| n.text.clone())
            .unwrap_or_default()
    }

    fn set_style(&mut self, id: u64, style: &Style) {
        if let Some(node) = self.nodes.get_mut(&id) {
            if &node.style == style {
                return;
            }
            node.style = style.clone();
        }
        self.log.push(format!("set_style {id}"));
    }

    fn set_placeholder(&mut self, id: u64, text: &str) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.placeholder = text.to_string();
        }
        self.log.push(format!("placeholder {id} {text}"));
    }

    fn set_order(&mut self, parent: u64, children: &[u64]) {
        if let Some(node) = self.nodes.get_mut(&parent) {
            node.children = children.to_vec();
        }
        self.log.push(format!("order {parent} {children:?}"));
    }

    fn redraw(&mut self) {
        self.redraws += 1;
        self.log.push("redraw".into());
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
