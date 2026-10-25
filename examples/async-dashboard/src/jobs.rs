//! Compiled async fns the dashboard's `view.rs` names.
//!
//! `view.rs` is not compiled. The lowerer copies the function ident
//! (`fetch_note`, `save_place`) into the hot script. [`install`] registers
//! the real future under that name. Editing markup or classes in `view.rs`
//! hot-reloads. Editing a function in this file is Rust, so it needs a
//! rebuild — the same split Makepad has between `script_mod!` and Rust.
//!
//! `QUINN_HTTP=1` with `--features net` GETs a public JSON list. Any failure,
//! and every build without that variable, uses the local city list so CI
//! does not need a network.

use std::time::Duration;

use quinn::{job, JobArg, JobValue};

#[derive(serde::Serialize)]
struct City {
    id: u32,
    name: String,
}

pub fn install() {
    static ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        job("fetch_note", |_arg: JobArg| async move {
            tokio::time::sleep(Duration::from_millis(jitter(1600, 2200))).await;
            Ok(JobValue::from("Ada's note"))
        });
        job("fetch_feed", |_arg: JobArg| async move {
            tokio::time::sleep(Duration::from_millis(jitter(280, 520))).await;
            Err("feed dropped".to_string())
        });
        job("fetch_place", |arg: JobArg| {
            let city = arg.as_string();
            async move {
                tokio::time::sleep(Duration::from_millis(jitter(320, 700))).await;
                Ok(JobValue::from(format!("{city} desk")))
            }
        });
        job("save_place", |arg: JobArg| {
            let place = arg.as_string();
            async move {
                tokio::time::sleep(Duration::from_millis(jitter(220, 480))).await;
                Ok(JobValue::from(format!("Saved {place}")))
            }
        });
        job("race_place", |arg: JobArg| {
            let place = arg.as_string();
            async move {
                // Davao is the fast request. A later dispatch still wins even
                // when an older, slower one finishes second.
                let wait = if place == "Davao" { 180 } else { 700 };
                tokio::time::sleep(Duration::from_millis(wait)).await;
                Ok(JobValue::from(format!("{place} won")))
            }
        });
        job("fetch_cities", |_arg: JobArg| async move {
            tokio::time::sleep(Duration::from_millis(jitter(450, 800))).await;
            load_cities().await
        });
        job("boom", |_arg: JobArg| async move {
            tokio::time::sleep(Duration::from_millis(jitter(200, 400))).await;
            panic!("boom");
        });
    });
}

fn jitter(min: u64, max: u64) -> u64 {
    let tick = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.subsec_nanos() as u64)
        .unwrap_or(0);
    min + tick % (max - min + 1)
}

async fn load_cities() -> Result<JobValue, String> {
    #[cfg(feature = "net")]
    if std::env::var("QUINN_HTTP").is_ok() {
        match http_cities().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                eprintln!("quinn http fallback: {err}");
            }
        }
    }
    Ok(local_cities())
}

fn local_cities() -> JobValue {
    let cities = vec![
        City {
            id: 1,
            name: "Manila".into(),
        },
        City {
            id: 2,
            name: "Cebu".into(),
        },
        City {
            id: 3,
            name: "Davao".into(),
        },
    ];
    JobValue::from_serialize(&cities).unwrap_or_else(|_| JobValue::from("cities"))
}

#[cfg(feature = "net")]
async fn http_cities() -> Result<JobValue, String> {
    let body = tokio::task::spawn_blocking(|| {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(8))
            .build();
        agent
            .get("https://jsonplaceholder.typicode.com/users")
            .call()
            .map_err(|err| err.to_string())?
            .into_string()
            .map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())??;
    let users: Vec<serde_json::Value> = serde_json::from_str(&body).map_err(|err| err.to_string())?;
    let cities: Vec<City> = users
        .into_iter()
        .filter_map(|user| {
            Some(City {
                id: user.get("id")?.as_u64()? as u32,
                name: user.get("name")?.as_str()?.to_string(),
            })
        })
        .take(8)
        .collect();
    if cities.is_empty() {
        return Err("the user list was empty".into());
    }
    JobValue::from_serialize(&cities)
}
