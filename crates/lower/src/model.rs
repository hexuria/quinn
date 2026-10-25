//! The view, after `view!` has been read and before Makepad script is emitted.

#[derive(Clone, Debug, PartialEq)]
pub enum Init {
    Int(i64),
    Bool(bool),
    Str(String),
    Items(Vec<ItemSeed>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ItemSeed {
    pub id: i64,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Signal {
    pub read: String,
    pub write: String,
    pub init: Init,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Lit(String),
    /// `state.<signal>`
    Sig(String),
    IfBool {
        signal: String,
        yes: String,
        no: String,
    },
    /// Empty string shows `empty`, otherwise the signal's own text.
    IfEmpty { signal: String, empty: String },
    /// Function parameter of a keyed row (`label`).
    Param(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    Inc(String),
    Toggle(String),
    Clear(String),
    Pop(String),
    Reverse(String),
    Push { next: String, items: String },
    /// Increment the keyed row count captured by `id`.
    RowInc,
}

#[derive(Clone, Debug, PartialEq)]
pub struct El {
    pub name: String,
    pub tag: String,
    pub class: String,
    pub parts: Vec<Part>,
    pub placeholder: Option<String>,
    pub value: Option<String>,
    pub click: Option<Act>,
    pub input: Option<String>,
    pub children: Vec<ViewNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ViewNode {
    El(El),
    Show {
        signal: String,
        /// `mod.state` field that is 1 while the branch is mounted.
        live_flag: String,
        /// Fields zeroed when the branch is not mounted.
        reset: Vec<String>,
        child: Box<ViewNode>,
    },
    For {
        items: String,
        var: String,
        key_field: String,
        fn_name: String,
        /// Item fields passed after `id`, in order.
        params: Vec<String>,
        child: El,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub signals: Vec<Signal>,
    pub frame: Option<String>,
    pub root_class: String,
    pub nodes: Vec<ViewNode>,
    /// Emit the keyed-count helpers.
    pub keyed: bool,
    /// Emit the retain step used by `QUINN_RETAIN`.
    pub retain: bool,
}

impl Model {
    pub fn signal(&self, name: &str) -> Option<&Signal> {
        self.signals.iter().find(|signal| signal.read == name)
    }
}
