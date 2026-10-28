//! The view, after `view!` has been read and before Makepad script is emitted.

#[derive(Clone, Debug, PartialEq)]
pub enum Init {
    Int(i64),
    Bool(bool),
    Str(String),
    Items(Vec<ItemSeed>),
    /// Nested script object. Fields may themselves be objects.
    Obj(Vec<(String, Init)>),
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
    IfEmpty {
        signal: String,
        empty: String,
    },
    /// Function parameter of a keyed row (`label`).
    Param(String),
    /// Enumerate index of a `<For let:index>`. A row-function parameter, not an item field.
    Index(String),
    /// `mod.ctx.<key>`
    Ctx(String),
    /// `state.<job>_value`. A resource or action result.
    Job(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
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
    /// A tuple or array written in a handler. Emitted as a script array.
    List(Vec<HxExpr>),
    /// A struct literal. Field order is source order.
    Obj(Vec<(String, HxExpr)>),
    /// `min` / `max` / `clamp`. Emitted as script helpers.
    Call {
        name: String,
        args: Vec<HxExpr>,
    },
    /// `if cond { then } else { els }` used as a value.
    If {
        cond: Box<HxExpr>,
        then: Box<HxExpr>,
        els: Box<HxExpr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum HandlerStmt {
    Let {
        name: String,
        expr: HxExpr,
    },
    Set {
        signal: String,
        expr: HxExpr,
    },
    Push {
        items: String,
        id: HxExpr,
        label: HxExpr,
    },
    Pop {
        items: String,
    },
    Reverse {
        items: String,
    },
    /// Order the items by `field` (`id` or `label`). Labels compare without
    /// regard to case; equal keys keep their order.
    SortBy {
        items: String,
        field: String,
        descending: bool,
    },
    /// Drop the item whose `id` equals `key`.
    RemoveKey {
        items: String,
        key: HxExpr,
    },
    /// Exchange the two items whose `id`s equal `a` and `b`.
    SwapKey {
        items: String,
        a: HxExpr,
        b: HxExpr,
    },
    /// `state.route = path` and render. Not a URL bar.
    Navigate(HxExpr),
    /// `action.dispatch(arg)`. Starts one client job. Not a server call.
    Dispatch {
        action: String,
        arg: HxExpr,
    },
    /// `note.refetch()`. Bumps the generation and sets phase back to loading.
    /// The previous value stays so a `<Transition>` can keep showing it.
    Refetch {
        job: String,
    },
    /// Flip `state.quinn_dark`. The shell signal behind `dark:` classes.
    ToggleTheme,
    /// `set_density(0|1|2)`. 0 compact, 1 standard, 2 touch.
    SetDensity(i64),
    /// `if cond { ... } else { ... }`. Both branches are statement lists.
    If {
        cond: HxExpr,
        then_branch: Vec<HandlerStmt>,
        else_branch: Vec<HandlerStmt>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    Inc(String),
    Toggle(String),
    Clear(String),
    Pop(String),
    Reverse(String),
    Push {
        next: String,
        items: String,
    },
    /// Increment the keyed row count captured by `id`.
    RowInc,
    /// Host handler: statements against `mod.state`.
    Body(Vec<HandlerStmt>),
    /// Row handler: signal writes are per-row, keyed by `id`.
    Row(Vec<HandlerStmt>),
}

/// `bind:width="level*2-7"` → `quinn_clamp(state.level * 2 + -7, 0, 100000)`.
#[derive(Clone, Debug, PartialEq)]
pub struct WidthBind {
    pub signal: String,
    pub factor: f64,
    pub bias: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassWhen {
    pub token: String,
    /// The attribute was `class:dark:...`. It applies only while `quinn_dark` is on.
    pub dark: bool,
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
    /// `bind:value`. The input omits `text:` so a reload does not move the caret.
    pub bind: Option<String>,
    /// `bind:width="signal*factor+bias"`. Width in pixels from a number signal.
    pub bind_width: Option<WidthBind>,
    /// `mod.ctx.key = expr` statements emitted just before this widget.
    pub provides: Vec<(String, String)>,
    /// `<Icon name="...">`: a glyph from `quinn-makepad/resources/icons`.
    pub icon: Option<String>,
    /// `on:keydown`. The host forwards Escape; `ev.key()` reads the key name.
    pub keydown: Option<Act>,
    /// `<input type="range">`: min, max, step.
    pub range: Option<[f64; 3]>,
    /// `<Slider step=move || signal.get()>`: the step follows a number
    /// signal instead of the literal in `range`.
    pub range_step: Option<String>,
    /// `data-popover`: drawn in an overlay under the control before it,
    /// flipped above it near the bottom edge.
    pub popover: bool,
    /// `<button widget="QButton">`: a library template from
    /// `quinn-makepad/src/widgets.rs`, instantiated as it is. The template
    /// owns every face, label, border, and state; the element only sets
    /// text, handlers, size, and the dark / selected / disabled flags.
    pub widget: Option<String>,
    /// `selected=move || signal.get()` on a library widget.
    pub selected: Option<String>,
    /// `invalid=true` on a library field: the danger border and ring.
    pub invalid: bool,
    /// `descending=move || signal.get()` on a sort header: the direction
    /// chevron after its label points down while the signal is true.
    pub descending: Option<String>,
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
        /// `let:index` on `<For>`. The row function receives this and `quinn_odd`.
        index: Option<String>,
        /// Item fields passed after `id`, in order.
        params: Vec<String>,
        /// Signals declared on the row component. They live under `state.r_<name>`.
        row_signals: Vec<Signal>,
        child: El,
    },
    /// Child view of a component, emitted as its own script function.
    /// An empty `fn_name` is a fragment: the nodes are inlined, not wrapped.
    Slot {
        fn_name: String,
        nodes: Vec<ViewNode>,
        /// Script expression passed into a `ChildrenFn`. `None` is `children()`.
        arg: Option<String>,
    },
    /// In-app routes. `state.route` picks one child. Not `leptos_router`.
    Routes {
        routes: Vec<(String, Vec<ViewNode>)>,
    },
    /// Fallback while any named job is `"loading"`. `transition` keeps a
    /// settled value on screen and shows the fallback beside it.
    Suspense {
        jobs: Vec<String>,
        transition: bool,
        fallback: Vec<ViewNode>,
        child: Box<ViewNode>,
    },
    /// Fallback when any named job's phase is `"error"`.
    Boundary {
        jobs: Vec<String>,
        fallback: Vec<ViewNode>,
        child: Box<ViewNode>,
    },
    /// Lifted to the window overlay. Not a DOM portal.
    Portal {
        nodes: Vec<ViewNode>,
    },
}

/// One slot of a packed job argument. The script builds a JSON array or object.
#[derive(Clone, Debug, PartialEq)]
pub enum PackItem {
    Signal(String),
    Lit(String),
    /// `state.{job}_source`.
    Source,
    /// `state.{job}_arg`.
    Dispatch,
}

/// Where a compiled async fn's argument comes from.
///
/// One value stays a plain script value so `as_string()` still sees `"Cebu"`.
/// Several values are a JSON array. A struct literal is a JSON object.
#[derive(Clone, Debug, PartialEq)]
pub enum ArgSrc {
    /// The fn takes no argument.
    None,
    /// `signal.get()` read in the closure. Also a refetch dependency.
    Signal(String),
    /// The resource fetcher's closure parameter. The value is the source snapshot.
    Source,
    /// `action.dispatch(arg)` stores the argument and the fn receives it.
    Dispatch,
    /// A string literal in the closure.
    Lit(String),
    /// JSON array of inputs. `JobArg::get` reads one slot.
    List(Vec<PackItem>),
    /// JSON object. `JobArg::deserialize` reads it.
    Obj(Vec<(String, PackItem)>),
}

/// A client job. `view.rs` is not compiled. A single call names one registered
/// Rust fn. A chain (`let a = f(x).await?; g(a).await`) keeps each fn in Rust
/// and stores a plan whose key is those idents (`chain:f+g`), not a source
/// line, so a markup edit above the closure does not change which fns run.
/// `deps` are signals that bump `gen` and set the phase back to loading.
/// An action reads captured signals when it is dispatched; they are not deps.
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub name: String,
    pub kind: JobKind,
    pub func: String,
    pub arg: ArgSrc,
    /// Empty for one call. Otherwise `name@spec;name@spec`.
    pub plan: String,
    pub deps: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobKind {
    Local,
    Resource,
    Action,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub signals: Vec<Signal>,
    pub frame: Option<String>,
    pub root_class: String,
    /// The root `<div on:keydown>`. The root is the host, not a node.
    pub root_keydown: Option<Act>,
    pub nodes: Vec<ViewNode>,
    /// Emit the keyed-count helpers.
    pub keyed: bool,
    /// Emit the retain step used by `QUINN_RETAIN`.
    pub retain: bool,
    /// Seed `mod.ctx` because a component provides or reads it.
    pub has_ctx: bool,
    /// Root `provide_context` lines, emitted at the start of `on_render`.
    pub ctx_init: Vec<(String, String)>,
    /// Client jobs. The executor reads `mod.quinn_job_*` and writes phase fields.
    pub jobs: Vec<Job>,
}

impl Model {
    pub fn signal(&self, name: &str) -> Option<&Signal> {
        self.signals.iter().find(|signal| signal.read == name)
    }
}
