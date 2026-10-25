//! Component lifecycle on top of a single [`Surface`].
//!
//! The surface is the renderer. Makepad implements it. Tests use
//! [`MemorySurface`]. There is no second widget tree: nodes here are the
//! reactive owners, listeners, and child order for one surface widget each.

mod builder;
mod runtime;
mod surface;

#[cfg(test)]
mod tests;

pub use builder::{
    display_text, el, empty, for_each, fragment, scope_owner, show, DynView, El, IntoView,
};
pub use quinn_styles::{
    parse_classes, parse_inline, AlignSpec, Edges, FlowSpec, Rgba, SizeSpec, Style,
};
pub use runtime::{Runtime, UiEvent};
pub use surface::{MemorySurface, NodeSpec, Surface, Tag};

pub use quinn_reactive::{on_cleanup, signal, ImmediateEffect, Owner};
