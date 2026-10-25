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

fn registry() -> &'static Mutex<HashMap<String, JobRun>> {
    static REG: OnceLock<Mutex<HashMap<String, JobRun>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register `name` as the compiled body a lowered job will call.
///
/// The name is the function ident in `view.rs` (`fetch_note`, `save_place`).
/// Calling [`job`] again replaces that name. The future runs off the UI
/// thread. [`Guard`] turns a panic into `Err`.
pub fn job<F, Fut>(name: &'static str, f: F)
where
    F: Fn(JobArg) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<JobValue, String>> + Send + 'static,
{
    let run: JobRun = Arc::new(move |arg| Box::pin(guard(f(arg))));
    registry()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .insert(name.to_string(), run);
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
        job("quinn_test_boom", |_arg: JobArg| async { panic!("nope") });
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
}
