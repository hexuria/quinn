//! Makepad host. Widget creation goes through `ScriptVm::eval` and
//! `WidgetRef::script_from_value` (`Apply::New`). Child order is the public
//! `View::children` vec that draw and events already walk, plus
//! `CxWidgetExt::widget_tree_insert_child` for the lookup graph.
//!
//! `Cx` is UI-thread-only. [`with_cx`] publishes the current `&mut Cx` for the
//! object-safe [`quinn_core::Surface`] methods, which cannot take a `Cx` argument.

mod control;
mod host;
mod list;
mod probe;
mod widgets;

pub use control::drive_control;
pub use host::{
    boot_script, census_host, drive_shell, eval_hot_file, load_bundled_view, load_hot_view,
    load_hot_view_with, on_actions, reclaim_script_heap, startup, with_current_cx, with_cx,
    WidgetCensus,
};
pub use makepad_widgets;
pub use probe::{
    after_mount, flush_now, rss_kib, session_stats, set_frame_driver, set_inspector,
    set_live_shell, SessionStats,
};
pub use quinn_lower::{accept_gen, cancelled_count, job, JobArg, JobValue};

use std::cell::RefCell;

use quinn_core::{Owner, Runtime};

pub struct Session {
    pub runtime: Runtime,
    /// Kept alive so component signals survive for the process.
    #[allow(dead_code)]
    owner: Owner,
}

std::thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

pub(crate) fn session_slot() -> &'static std::thread::LocalKey<RefCell<Option<Session>>> {
    &SESSION
}
