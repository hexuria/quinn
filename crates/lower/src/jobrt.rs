//! Values and futures behind `LocalResource`, `Resource`, and `Action`.
//!
//! `view.rs` is not compiled. The lowerer reads a function name out of the
//! closure. The example registers that name with [`job`]. A tokio worker in
//! `quinn-makepad` polls the future. Markup and classes still hot-reload.
//! The body of the async fn is ordinary Rust, so changing it needs a rebuild.
//! That split is the same one Makepad already has between `script_mod!` and
//! Rust.

use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll};

/// What a finished future may write into `state.{name}_value`.
///
/// Strings, numbers, bools, lists, and objects all become one script value.
/// A list of objects is what `<For each=move || cities.get()>` walks.
#[derive(Clone, Debug, PartialEq)]
pub enum JobValue {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    List(Vec<JobValue>),
    Obj(Vec<(String, JobValue)>),
}

impl JobValue {
    pub fn list(items: Vec<JobValue>) -> Self {
        Self::List(items)
    }

    pub fn obj(fields: Vec<(&str, JobValue)>) -> Self {
        Self::Obj(
            fields
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }

    /// `T` is any serde struct or `Vec`. Field order follows serde.
    pub fn from_serialize<T: serde::Serialize>(value: &T) -> Result<Self, String> {
        let json = serde_json::to_value(value).map_err(|err| err.to_string())?;
        Ok(Self::from_json(json))
    }

    pub fn from_json(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(flag) => Self::Bool(flag),
            serde_json::Value::Number(number) => Self::Num(number.as_f64().unwrap_or(0.0)),
            serde_json::Value::String(text) => Self::Str(text),
            serde_json::Value::Array(items) => {
                Self::List(items.into_iter().map(Self::from_json).collect())
            }
            serde_json::Value::Object(map) => Self::Obj(
                map.into_iter()
                    .map(|(key, value)| (key, Self::from_json(value)))
                    .collect(),
            ),
        }
    }

    /// JSON text. The shell parses it with the script `parse_json` method so
    /// a string interpolation cannot be mistaken for source.
    pub fn to_json(&self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Bool(true) => "true".into(),
            Self::Bool(false) => "false".into(),
            Self::Num(number) => format_num(*number),
            Self::Str(text) => format!("\"{}\"", json_escape(text)),
            Self::List(items) => {
                let body = items
                    .iter()
                    .map(Self::to_json)
                    .collect::<Vec<_>>()
                    .join(",");
                format!("[{body}]")
            }
            Self::Obj(fields) => {
                let body = fields
                    .iter()
                    .map(|(key, value)| format!("\"{}\":{}", json_escape(key), value.to_json()))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{{{body}}}")
            }
        }
    }

    /// Flat text for a headless label. Objects and lists use their JSON.
    pub fn display(&self) -> String {
        match self {
            Self::Str(text) => text.clone(),
            Self::Num(number) => format_num(*number),
            Self::Bool(true) => "true".into(),
            Self::Bool(false) => "false".into(),
            Self::Null => String::new(),
            Self::List(_) | Self::Obj(_) => self.to_json(),
        }
    }
}

impl From<&str> for JobValue {
    fn from(text: &str) -> Self {
        Self::Str(text.to_string())
    }
}

impl From<String> for JobValue {
    fn from(text: String) -> Self {
        Self::Str(text)
    }
}

impl From<bool> for JobValue {
    fn from(flag: bool) -> Self {
        Self::Bool(flag)
    }
}

impl From<f64> for JobValue {
    fn from(number: f64) -> Self {
        Self::Num(number)
    }
}

impl From<i64> for JobValue {
    fn from(number: i64) -> Self {
        Self::Num(number as f64)
    }
}

impl From<i32> for JobValue {
    fn from(number: i32) -> Self {
        Self::Num(number as f64)
    }
}

impl From<u32> for JobValue {
    fn from(number: u32) -> Self {
        Self::Num(number as f64)
    }
}

impl From<u64> for JobValue {
    fn from(number: u64) -> Self {
        Self::Num(number as f64)
    }
}

fn format_num(number: f64) -> String {
    if number.is_finite() && number.fract() == 0.0 && number.abs() < 1e15 {
        format!("{}", number as i64)
    } else {
        number.to_string()
    }
}

fn json_escape(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other if other.is_control() => out.push_str(&format!("\\u{:04x}", other as u32)),
            other => out.push(other),
        }
    }
    out
}

/// The one argument `dispatch`, a resource source, or a signal read produced.
#[derive(Clone, Debug, PartialEq)]
pub struct JobArg {
    raw: String,
}

impl JobArg {
    pub fn from_text(raw: impl Into<String>) -> Self {
        Self { raw: raw.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }

    pub fn as_string(&self) -> String {
        self.raw.clone()
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.raw.parse().ok()
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    /// One slot of a JSON array. Strings are unquoted. Objects and arrays
    /// stay JSON text. A plain string argument is slot 0.
    pub fn get(&self, index: usize) -> Option<String> {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&self.raw) {
            return value.get(index).map(slot_text);
        }
        if index == 0 {
            Some(self.raw.clone())
        } else {
            None
        }
    }

    /// `#[derive(Deserialize)]` struct, or any other JSON value.
    pub fn deserialize<T: serde::de::DeserializeOwned>(&self) -> Result<T, String> {
        serde_json::from_str(&self.raw).map_err(|err| err.to_string())
    }
}

fn slot_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// A finished future's `Ok` value. `JobValue` passes through. A serde value
/// becomes JSON, then a [`JobValue`], so a struct is not double-encoded.
pub trait IntoJobValue {
    fn into_job_value(self) -> Result<JobValue, String>;
}

impl IntoJobValue for JobValue {
    fn into_job_value(self) -> Result<JobValue, String> {
        Ok(self)
    }
}

impl<T> IntoJobValue for T
where
    T: serde::Serialize,
{
    fn into_job_value(self) -> Result<JobValue, String> {
        JobValue::from_serialize(&self)
    }
}

/// A newer dispatch or refetch wins. Generation 0 is "not running".
pub fn accept_gen(current: u64, delivered: u64) -> bool {
    delivered != 0 && current == delivered
}

pub fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        format!("panic: {text}")
    } else if let Some(text) = payload.downcast_ref::<String>() {
        format!("panic: {text}")
    } else {
        "panic".into()
    }
}

/// Poll `fut` inside `catch_unwind`. A panic becomes `Err` and is not polled
/// again, so one job cannot take down the frame.
pub struct Guard<F> {
    fut: F,
}

pub fn guard<F>(fut: F) -> Guard<F>
where
    F: Future<Output = Result<JobValue, String>>,
{
    Guard { fut }
}

impl<F> Future for Guard<F>
where
    F: Future<Output = Result<JobValue, String>>,
{
    type Output = Result<JobValue, String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // `fut` is pinned with `self`. The panic aborts this poll only.
        let fut = unsafe { self.map_unchecked_mut(|guard| &mut guard.fut) };
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| fut.poll(cx))) {
            Ok(poll) => poll,
            Err(payload) => Poll::Ready(Err(panic_message(payload.as_ref()))),
        }
    }
}

pub type BoxedJob = Pin<Box<dyn Future<Output = Result<JobValue, String>> + Send>>;
pub type JobRun = Arc<dyn Fn(JobArg) -> BoxedJob + Send + Sync>;

static CANCELLED: AtomicU64 = AtomicU64::new(0);

/// How many job futures were dropped before they finished.
///
/// Leptos does not have a cancelled-count method. Dropping the future is the
/// observation (`Resource` dispose, `on_cleanup`). This counts those drops
/// for the process. A future that reached `Ready` is not a cancel.
pub fn cancelled_count() -> u64 {
    CANCELLED.load(Ordering::Relaxed)
}

/// Counts a drop that happens before `poll` returns [`Poll::Ready`].
pub struct Track<F> {
    fut: F,
    done: bool,
}

pub fn track<F>(fut: F) -> Track<F> {
    Track { fut, done: false }
}

impl<F: Future> Future for Track<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // `fut` is pinned with `self`. `done` is written only after Ready.
        let this = unsafe { self.get_unchecked_mut() };
        let poll = unsafe { Pin::new_unchecked(&mut this.fut) }.poll(cx);
        if poll.is_ready() {
            this.done = true;
        }
        poll
    }
}

impl<F> Drop for Track<F> {
    fn drop(&mut self) {
        if !self.done {
            CANCELLED.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn registry() -> &'static Mutex<HashMap<String, JobRun>> {
    static REG: OnceLock<Mutex<HashMap<String, JobRun>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register `name` as the compiled body a lowered job will call.
///
/// The name is the function ident in `view.rs` (`fetch_note`, `save_place`).
/// Calling [`job`] again replaces that name. The future runs off the UI
/// thread. [`Guard`] turns a panic into `Err`.
///
/// `Ok` is anything [`IntoJobValue`] (a [`JobValue`], or a `Serialize` struct
/// or `String`). `Err` is anything [`Display`](std::fmt::Display). When only
/// one variant is constructed, name the other type
/// (`Ok::<JobValue, String>(...)` or a fn return type).
pub fn job<F, Fut, T, E>(name: &'static str, f: F)
where
    F: Fn(JobArg) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    T: IntoJobValue,
    E: std::fmt::Display,
{
    let run: JobRun = Arc::new(move |arg| {
        let fut = f(arg);
        Box::pin(guard(async move {
            match fut.await {
                Ok(value) => value.into_job_value(),
                Err(err) => Err(err.to_string()),
            }
        }))
    });
    registry()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .insert(name.to_string(), run);
}

/// Run one registered fn, or a `name@spec;name@spec` plan of them.
///
/// An empty plan calls `func` with `arg`. A plan calls each fn and stops on
/// the first `Err`. `prev` is the previous step. `sN` is an earlier step.
/// `N` is an index into `arg`.
pub async fn run_registered(func: &str, plan: &str, arg: JobArg) -> Result<JobValue, String> {
    if plan.is_empty() {
        return call_named(func, arg).await;
    }
    let steps = parse_plan(plan)?;
    let mut outputs = Vec::new();
    for (name, specs) in &steps {
        let input = step_arg(specs, &arg, &outputs)?;
        outputs.push(call_named(name, input).await?);
    }
    outputs.pop().ok_or_else(|| "empty job plan".to_string())
}

async fn call_named(name: &str, arg: JobArg) -> Result<JobValue, String> {
    let Some(run) = lookup(name) else {
        return Err(format!("no async fn `{name}`"));
    };
    run(arg).await
}

enum Spec {
    Index(usize),
    Prev,
    Step(usize),
}

fn parse_plan(plan: &str) -> Result<Vec<(String, Vec<Spec>)>, String> {
    let mut steps = Vec::new();
    for piece in plan.split(';') {
        if piece.is_empty() {
            return Err(format!("unsupported job plan: empty step in `{plan}`"));
        }
        let (name, spec) = match piece.split_once('@') {
            Some((name, spec)) => (name, spec),
            None => (piece, ""),
        };
        if name.is_empty() || !name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
            return Err(format!("unsupported job plan: bad step `{piece}`"));
        }
        let mut specs = Vec::new();
        if !spec.is_empty() {
            for token in spec.split(',') {
                specs.push(parse_spec(token, piece)?);
            }
        }
        steps.push((name.to_string(), specs));
    }
    if steps.is_empty() {
        return Err(format!("unsupported job plan: `{plan}`"));
    }
    Ok(steps)
}

fn parse_spec(token: &str, piece: &str) -> Result<Spec, String> {
    if token == "prev" {
        return Ok(Spec::Prev);
    }
    if let Some(rest) = token.strip_prefix('s') {
        if let Ok(index) = rest.parse::<usize>() {
            return Ok(Spec::Step(index));
        }
    }
    if let Ok(index) = token.parse::<usize>() {
        return Ok(Spec::Index(index));
    }
    Err(format!("unsupported job plan: bad input `{token}` in `{piece}`"))
}

fn step_arg(specs: &[Spec], arg: &JobArg, outputs: &[JobValue]) -> Result<JobArg, String> {
    if specs.is_empty() {
        return Ok(JobArg::from_text(""));
    }
    let mut values = Vec::new();
    for spec in specs {
        values.push(match spec {
            Spec::Index(index) => match arg.get(*index) {
                Some(text) => JobValue::Str(text),
                None if *index == 0 => JobValue::Str(arg.as_string()),
                None => return Err(format!("job argument {index} is missing")),
            },
            Spec::Prev => outputs
                .last()
                .cloned()
                .ok_or_else(|| "job plan used prev with no previous step".to_string())?,
            Spec::Step(index) => outputs.get(*index).cloned().ok_or_else(|| {
                format!("job plan used s{index} before that step finished")
            })?,
        });
    }
    Ok(arg_from_values(values))
}

fn arg_from_values(values: Vec<JobValue>) -> JobArg {
    if values.len() == 1 {
        return match values.into_iter().next().unwrap() {
            JobValue::Str(text) => JobArg::from_text(text),
            other => JobArg::from_text(other.to_json()),
        };
    }
    JobArg::from_text(JobValue::List(values).to_json())
}

pub fn lookup(name: &str) -> Option<JobRun> {
    registry()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .get(name)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::task::{RawWaker, RawWakerVTable, Waker};

    /// The cancel counter is process-wide. These two tests must not overlap.
    static CANCEL_TESTS: Mutex<()> = Mutex::new(());

    fn noop_waker() -> Waker {
        fn clone(_: *const ()) -> RawWaker {
            raw()
        }
        fn empty(_: *const ()) {}
        fn raw() -> RawWaker {
            RawWaker::new(std::ptr::null(), &VTABLE)
        }
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, empty, empty, empty);
        unsafe { Waker::from_raw(raw()) }
    }

    fn ready<F: Future>(fut: F) -> F::Output {
        let waker = noop_waker();
        let mut fut = std::pin::pin!(fut);
        let mut cx = Context::from_waker(&waker);
        for _ in 0..8 {
            if let Poll::Ready(value) = fut.as_mut().poll(&mut cx) {
                return value;
            }
        }
        panic!("future stayed pending");
    }

    struct SecondPollPanic {
        polls: u8,
    }

    impl Future for SecondPollPanic {
        type Output = Result<JobValue, String>;

        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.get_mut();
            if this.polls == 0 {
                this.polls = 1;
                return Poll::Pending;
            }
            panic!("second");
        }
    }

    #[test]
    fn panic_on_first_poll_is_an_error() {
        let result = ready(guard(async { panic!("boom") }));
        assert_eq!(result.unwrap_err(), "panic: boom");
    }

    #[test]
    fn panic_after_pending_is_an_error() {
        let result = ready(guard(SecondPollPanic { polls: 0 }));
        assert_eq!(result.unwrap_err(), "panic: second");
    }

    #[test]
    fn registered_panic_is_an_error() {
        job("quinn_test_boom", |_arg: JobArg| async {
            panic!("nope");
            #[allow(unreachable_code)]
            Ok::<JobValue, String>(JobValue::Null)
        });
        let run = lookup("quinn_test_boom").expect("registered");
        let result = ready(run(JobArg::from_text("Cebu")));
        assert_eq!(result.unwrap_err(), "panic: nope");
    }

    #[test]
    fn accept_gen_drops_stale_and_zero() {
        assert!(accept_gen(3, 3));
        assert!(!accept_gen(4, 3));
        assert!(!accept_gen(0, 0));
        assert!(!accept_gen(2, 0));
    }

    #[test]
    fn serde_struct_and_vec_become_script_json() {
        #[derive(serde::Serialize)]
        struct City {
            id: u32,
            name: String,
        }
        let value = JobValue::from_serialize(&vec![
            City {
                id: 1,
                name: "Cebu".into(),
            },
            City {
                id: 2,
                name: "Davao".into(),
            },
        ])
        .unwrap();
        assert_eq!(
            value.to_json(),
            r#"[{"id":1,"name":"Cebu"},{"id":2,"name":"Davao"}]"#
        );
        assert_eq!(JobValue::from(true).to_json(), "true");
        assert_eq!(JobValue::from(12_i64).to_json(), "12");
        assert_eq!(JobValue::from("Ada's note").to_json(), r#""Ada's note""#);
    }

    #[test]
    fn array_slots_and_struct_round_trip() {
        let arg = JobArg::from_text(r#"["ada","2"]"#);
        assert_eq!(arg.get(0).as_deref(), Some("ada"));
        assert_eq!(arg.get(1).as_deref(), Some("2"));
        assert_eq!(arg.get(2), None);
        assert_eq!(JobArg::from_text("Cebu").get(0).as_deref(), Some("Cebu"));

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Place {
            city: String,
            note: String,
        }
        let place: Place = JobArg::from_text(r#"{"city":"Cebu","note":"desk"}"#)
            .deserialize()
            .unwrap();
        assert_eq!(
            place,
            Place {
                city: "Cebu".into(),
                note: "desk".into(),
            }
        );
    }

    #[test]
    fn typed_ok_and_display_error() {
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Place {
            city: String,
            note: String,
        }
        #[derive(Debug)]
        struct StampError(&'static str);
        impl std::fmt::Display for StampError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "stamp: {}", self.0)
            }
        }
        job("quinn_test_stamp", |_arg: JobArg| async move {
            Ok::<Place, StampError>(Place {
                city: "Cebu".into(),
                note: "desk".into(),
            })
        });
        let value = ready(lookup("quinn_test_stamp").unwrap()(JobArg::from_text("")));
        assert_eq!(
            value.unwrap(),
            JobValue::obj(vec![
                ("city", JobValue::from("Cebu")),
                ("note", JobValue::from("desk")),
            ])
        );
        job("quinn_test_stamp_err", |_arg: JobArg| async move {
            Err::<Place, StampError>(StampError("empty city"))
        });
        let err =
            ready(lookup("quinn_test_stamp_err").unwrap()(JobArg::from_text(""))).unwrap_err();
        assert_eq!(err, "stamp: empty city");
    }

    #[test]
    fn finished_track_is_not_a_cancel_and_a_drop_is() {
        let _guard = CANCEL_TESTS.lock().unwrap_or_else(|err| err.into_inner());
        let before = cancelled_count();
        let mut done = std::pin::pin!(track(std::future::ready(())));
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(done.as_mut().poll(&mut cx).is_ready());
        drop(done);
        assert_eq!(cancelled_count(), before);
        drop(track(std::future::pending::<()>()));
        assert_eq!(cancelled_count(), before + 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn superseded_future_stops_incrementing() {
        let _guard = CANCEL_TESTS.lock().unwrap_or_else(|err| err.into_inner());
        use std::sync::atomic::AtomicUsize;
        let ticks = std::sync::Arc::new(AtomicUsize::new(0));
        let ticks_task = ticks.clone();
        let before = cancelled_count();
        let handle = tokio::spawn(track(async move {
            loop {
                ticks_task.fetch_add(1, Ordering::Relaxed);
                tokio::time::sleep(std::time::Duration::from_millis(15)).await;
            }
        }));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let mid = ticks.load(Ordering::Relaxed);
        assert!(mid >= 2, "counter should have started, got {mid}");
        handle.abort();
        let _ = handle.await;
        let stopped = ticks.load(Ordering::Relaxed);
        tokio::time::sleep(std::time::Duration::from_millis(60)).await;
        assert_eq!(
            ticks.load(Ordering::Relaxed),
            stopped,
            "aborted future kept incrementing"
        );
        assert!(cancelled_count() >= before + 1);
    }

    #[tokio::test]
    async fn chain_plan_runs_registered_fns_then_stops() {
        job("quinn_test_left", |arg: JobArg| async move {
            let a = arg.get(0).unwrap_or_default();
            let b = arg.get(1).unwrap_or_default();
            Ok::<JobValue, String>(JobValue::from(format!("{a}-{b}")))
        });
        job("quinn_test_right", |arg: JobArg| async move {
            Ok::<JobValue, String>(JobValue::from(format!("decor {}", arg.as_string())))
        });
        let arg = JobArg::from_text(r#"["ada","2"]"#);
        let value = run_registered(
            "chain:quinn_test_left+quinn_test_right",
            "quinn_test_left@0,1;quinn_test_right@prev",
            arg,
        )
        .await
        .unwrap();
        assert_eq!(value, JobValue::from("decor ada-2"));
        let missing = run_registered("quinn_test_missing", "", JobArg::from_text("")).await;
        assert!(missing.unwrap_err().contains("quinn_test_missing"));
    }
}
