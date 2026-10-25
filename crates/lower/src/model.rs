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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// A value a handler can compute. Names are signals, `let` bindings, or the
/// input closure's parameter.
#[derive(Clone, Debug, PartialEq)]
pub enum HxExpr {
    Int(i64),
    Bool(bool),
    Str(String),
    Name(String),
    Not(Box<HxExpr>),
    Binary(BinOp, Box<HxExpr>, Box<HxExpr>),
    Concat(Vec<HxExpr>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum HandlerStmt {
    Let { name: String, expr: HxExpr },
    Set { signal: String, expr: HxExpr },
    Push { items: String, id: HxExpr, label: HxExpr },
    Pop { items: String },
    Reverse { items: String },
    /// Drop the item whose `id` equals `key`.
    RemoveKey { items: String, key: HxExpr },
    /// Exchange the two items whose `id`s equal `a` and `b`.
    SwapKey { items: String, a: HxExpr, b: HxExpr },
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
    /// Host handler: statements against `mod.state`.
    Body(Vec<HandlerStmt>),
    /// Row handler: signal writes are per-row, keyed by `id`.
    Row(Vec<HandlerStmt>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassWhen {
    pub token: String,
    pub signal: String,
}

/// `style:prop = if signal { "yes" } else { "no" }`.
#[derive(Clone, Debug, PartialEq)]
pub struct StyleWhen {
    pub prop: String,
    pub signal: String,
    pub yes: String,
    pub no: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct El {
    pub name: String,
    pub tag: String,
    pub class: String,
    /// Static `style="..."`. Merged after classes.
    pub style: String,
    pub class_when: Vec<ClassWhen>,
    pub style_when: Vec<StyleWhen>,
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
        /// Signals declared on the row component. They live under `state.r_<name>`.
        row_signals: Vec<Signal>,
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
