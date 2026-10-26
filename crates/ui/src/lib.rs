//! Component library source. `components.rs` is lowered with the app's
//! `view.rs`. Cargo does not compile it. Apps import the names and the host
//! passes [`SOURCE`] into the lowerer.

pub const SOURCE: &str = include_str!("../components.rs");
