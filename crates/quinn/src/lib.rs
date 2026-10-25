//! Authoring crate. Markup expands to [`quinn_core`] builders. The `native`
//! feature mounts those builders in a Makepad window.

pub use quinn_core::{
    display_text, el, empty, for_each, fragment, scope_owner, show, DynView, El, IntoView,
    MemorySurface, Runtime, Tag,
};
pub use quinn_macros::{component, run, view};
pub use quinn_reactive::{on_cleanup, signal, ImmediateEffect, Owner};
pub use quinn_styles::{parse_classes, parse_inline, Style};

#[cfg(feature = "native")]
pub use quinn_makepad::{makepad_widgets, on_actions, startup};

pub mod prelude {
    pub use crate::{
        component, display_text, el, empty, for_each, fragment, on_cleanup, run, scope_owner, show,
        signal, view, DynView, El, IntoView, Owner, Tag,
    };
    pub use quinn_reactive::prelude::*;
}
