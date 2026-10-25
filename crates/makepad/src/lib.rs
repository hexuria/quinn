//! Makepad host. Widget creation goes through `ScriptVm::eval` and
//! `WidgetRef::script_from_value` (`Apply::New`). Child order is the public
//! `View::children` vec that draw and events already walk, plus
//! `CxWidgetExt::widget_tree_insert_child` for the lookup graph.
//!
//! `Cx` is UI-thread-only. [`with_cx`] publishes the current `&mut Cx` for the
//! object-safe [`lm_core::Surface`] methods, which cannot take a `Cx` argument.

mod host;

pub use host::{on_actions, startup, with_cx};
pub use makepad_widgets;

use std::cell::RefCell;

use lm_core::{IntoView, Owner, Runtime};

pub struct Session {
    pub runtime: Runtime,
    owner: Owner,
}

std::thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

pub(crate) fn session_slot() -> &'static std::thread::LocalKey<RefCell<Option<Session>>> {
    &SESSION
}
