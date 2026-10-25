//! Upstream Leptos [`reactive_graph`](https://github.com/leptos-rs/leptos), pinned
//! to commit `5ca3a9bf90cbcc0561dc4323f8018e18dce5906f`.
//!
//! Native code uses [`ImmediateEffect`] because Makepad does not run an async
//! executor on the UI thread. [`effect::Effect`] and [`effect::RenderEffect`]
//! schedule through `any_spawner` and would not flush.

pub use reactive_graph;

pub use reactive_graph::effect::ImmediateEffect;
pub use reactive_graph::owner::{on_cleanup, Owner};
pub use reactive_graph::signal::signal;

pub mod prelude {
    pub use reactive_graph::owner::{on_cleanup, Owner};
    pub use reactive_graph::prelude::*;
    pub use reactive_graph::signal::signal;
}

#[cfg(test)]
mod tests {
    use super::*;
    use reactive_graph::prelude::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn immediate_effect_runs_synchronously_and_stops_when_owner_drops() {
        let runs = Arc::new(AtomicUsize::new(0));
        let owner = Owner::new();
        let (count, set_count) = owner.with(|| signal(0));
        let runs2 = runs.clone();
        owner.with(|| {
            ImmediateEffect::new_scoped(move || {
                let _ = count.get();
                runs2.fetch_add(1, Ordering::SeqCst);
            });
        });
        assert_eq!(runs.load(Ordering::SeqCst), 1);
        set_count.set(1);
        assert_eq!(runs.load(Ordering::SeqCst), 2);
        drop(owner);
        set_count.set(2);
        assert_eq!(runs.load(Ordering::SeqCst), 2);
    }
}
