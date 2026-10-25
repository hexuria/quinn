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
pub use quinn_makepad::{
    after_mount, boot_script, census_host, drive_shell, eval_hot_file, flush_now, load_hot_view,
    makepad_widgets,
    on_actions, reclaim_script_heap, rss_kib, session_stats, set_frame_driver, set_inspector,
    set_live_shell, startup, with_current_cx, with_cx, SessionStats, WidgetCensus,
};

/// Marker the lowerer sees in a component body. It emits `quinn_retain_step`
/// and `quinn_retain_reset` into the hot script. The running example drives
/// those functions from Rust; this call is not executed.
pub fn retain_probe() {}

pub mod prelude {
    pub use crate::{
        component, display_text, el, empty, for_each, fragment, on_cleanup, run, scope_owner, show,
        signal, view, DynView, El, IntoView, Owner, Tag,
    };
    pub use quinn_reactive::prelude::*;
}
