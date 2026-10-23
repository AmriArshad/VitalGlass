//! `vitalglass-cli`: sync and inspect the local cache without the UI.
//!
//! ```text
//! vitalglass-cli sync [--hot]   fetch from Google Health into SQLite
//! vitalglass-cli summary        print what's in the cache
//! vitalglass-cli status         account and per-type sync state
//! ```
//!
//! Uses the same keyring sign-in as the desktop app; opens the browser
//! only if there's no saved session. Set `RUST_LOG=debug` for more output.

use anyhow::{Context, bail};
use chrono::{DateTime, Days, Local, Utc};
use vitalglass_core::config::ClientConfig;
use vitalglass_core::session::Session;
use vitalglass_core::store::{self, Store};
use vitalglass_core::{Error, sync};

const USAGE: &str = "usage: vitalglass-cli <sync [--hot] | summary | status>";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["sync"] => cmd_sync(sync::Mode::Full).await,
        ["sync", "--hot"] => cmd_sync(sync::Mode::HotOnly).await,
        ["summary"] => cmd_summary(),
        ["status"] => cmd_status(),
        _ => bail!(USAGE),
    }
}

async fn cmd_sync(mode: sync::Mode) -> anyhow::Result<()> {
    let mut session = signed_in_session().await?;
    let store = Store::open_default()?;
    eprintln!("Syncing into {}", store::default_path()?.display());

    let started = std::time::Instant::now();
    let results = sync::run(&mut session, &store, mode, &mut |line| eprintln!("  {line}")).await?;
    let failed: Vec<_> = results.iter().filter(|r| r.status != "ok").collect();
    eprintln!("Done in {:.1}s.", started.elapsed().as_secs_f64());
    for r in failed {
        eprintln!("  {} {}: {}", r.status, r.data_type, r.message.as_deref().unwrap_or(""));
    }
    Ok(())
}

/// Restore from the keyring, falling back to a browser sign-in.
async fn signed_in_session() -> anyhow::Result<Session> {
    let mut session = Session::new(ClientConfig::load()?)?;
    match session.restore().await {
        Ok(true) => {}
        Ok(false) | Err(Error::SessionExpired) => {
            eprintln!("No saved session; signing in.");
            session.sign_in().await?;
        }
        Err(e) => return Err(e.into()),
    }
    if let Some(t) = session.tokens()
        && !t.missing_scopes().is_empty()
    {
        eprintln!("Not granted: {}", t.missing_scopes().join(", "));
    }
    Ok(session)
}

fn cmd_summary() -> anyhow::Result<()> {
    let store = Store::open_default()?;
    let today = Local::now().date_naive();
    let tomorrow = today + Days::new(1);
    let week_ago = today - Days::new(6);

    println!("Devices");
    for d in store.devices()? {
        println!(
            "  {} ({}), battery {}, last sync {}",
            d.product.as_deref().unwrap_or("?"),
            d.device_type.as_deref().unwrap_or("?").to_lowercase(),
            d.battery_level.map_or("?".into(), |b| format!("{b}%")),
            d.last_sync_utc.as_deref().map_or("never".into(), ago)
        );
    }

    println!("\nLast 7 days     steps     km  floors  AZM   kcal | RHR   HRV  SpO2    BR  VO2");
    let activity = store.daily_activity(week_ago, tomorrow)?;
    let vitals = store.daily_vitals(week_ago, tomorrow)?;
    let mut day = week_ago;
    while day < tomorrow {
        let date = day.to_string();
        let a = activity.iter().find(|a| a.date == date);
        let v = vitals.iter().find(|v| v.date == date);
        let azm = a.map(|a| a.azm_fat_burn.unwrap_or(0) + a.azm_cardio.unwrap_or(0) + a.azm_peak.unwrap_or(0));
        println!(
            "  {date} {:>7} {:>6} {:>7} {:>4} {:>6} | {:>3} {:>5} {:>5} {:>5} {:>4}",
            opt(a.and_then(|a| a.steps), |s| s.to_string()),
            opt(a.and_then(|a| a.distance_m), |m| format!("{:.2}", m / 1000.0)),
            opt(a.and_then(|a| a.floors), |f| f.to_string()),
            opt(azm, |z| z.to_string()),
            opt(a.and_then(|a| a.calories_kcal), |k| format!("{k:.0}")),
            opt(v.and_then(|v| v.resting_hr), |r| r.to_string()),
            opt(v.and_then(|v| v.hrv_ms), |h| format!("{h:.1}")),
            opt(v.and_then(|v| v.spo2_avg), |s| format!("{s:.1}")),
            opt(v.and_then(|v| v.resp_rate), |b| format!("{b:.1}")),
            opt(v.and_then(|v| v.vo2max), |x| format!("{x:.0}")),
        );
        day = day + Days::new(1);
    }

    println!();
    match store.latest_main_sleep()? {
        Some(s) => println!(
            "Last sleep ({}): {} asleep, {} to {}; deep {}, light {}, REM {}, awake {}",
            s.civil_end_date,
            mins(s.minutes_asleep),
            local_time(&s.start_utc),
            local_time(&s.end_utc),
            mins(s.minutes_deep),
            mins(s.minutes_light),
            mins(s.minutes_rem),
            mins(s.minutes_awake)
        ),
        None => println!("No sleep recorded."),
    }

    println!("\nRecent workouts");
    let workouts = store.recent_exercises(5)?;
    if workouts.is_empty() {
        println!("  none");
    }
    for e in workouts {
        println!(
            "  {} {:<18} {:>7}  avg HR {:>3}  {:>4} kcal  {}",
            local_time(&e.start_utc),
            e.display_name.or(e.exercise_type).unwrap_or_default(),
            mins(e.active_duration_s.map(|s| s / 60)),
            opt(e.avg_hr, |h| h.to_string()),
            opt(e.calories_kcal, |k| format!("{k:.0}")),
            opt(e.distance_m, |m| format!("{:.2} km", m / 1000.0)),
        );
    }

    let start = Utc::now() - chrono::Duration::hours(24);
    let hr = store.heart_rate(&start.to_rfc3339(), &Utc::now().to_rfc3339())?;
    if let (Some(lo), Some(hi)) = (
        hr.iter().map(|b| b.bpm_avg).reduce(f64::min),
        hr.iter().map(|b| b.bpm_avg).reduce(f64::max),
    ) {
        println!(
            "\nHeart rate, last 24 h: {} five-minute buckets, {lo:.0}–{hi:.0} bpm",
            hr.len()
        );
    }

    let weights = store.weight(today - Days::new(sync::HISTORY_DAYS), tomorrow)?;
    if let Some(w) = weights.last() {
        println!("Latest weight: {:.1} kg on {}", w.grams / 1000.0, w.civil_date);
    }
    Ok(())
}

fn cmd_status() -> anyhow::Result<()> {
    let store = Store::open_default().context("opening the cache")?;
    println!(
        "{:<30} {:<18} {:<12} last success",
        "data type", "status", "history from"
    );
    for s in store.sync_states()? {
        println!(
            "{:<30} {:<18} {:<12} {}",
            s.data_type,
            s.status,
            s.backfilled_from.unwrap_or_default(),
            s.last_success_utc.as_deref().map_or("never".into(), ago)
        );
        if let Some(m) = s.message {
            println!("    {m}");
        }
    }
    Ok(())
}

fn opt<T>(v: Option<T>, f: impl Fn(T) -> String) -> String {
    v.map_or_else(|| "-".into(), f)
}

/// `437` minutes -> `7h17m`.
fn mins(m: Option<i64>) -> String {
    opt(m, |m| format!("{}h{:02}m", m / 60, m % 60))
}

/// RFC 3339 UTC -> local `Oct 24 06:40`.
fn local_time(utc: &str) -> String {
    DateTime::parse_from_rfc3339(utc)
        .map(|t| t.with_timezone(&Local).format("%b %d %H:%M").to_string())
        .unwrap_or_else(|_| utc.to_owned())
}

/// RFC 3339 UTC -> `Oct 24 10:42 (12 min ago)`.
fn ago(utc: &str) -> String {
    let Ok(t) = DateTime::parse_from_rfc3339(utc) else {
        return utc.to_owned();
    };
    let mins = (Utc::now() - t.with_timezone(&Utc)).num_minutes();
    let rel = match mins {
        ..1 => "just now".into(),
        1..60 => format!("{mins} min ago"),
        60..1440 => format!("{} h ago", mins / 60),
        _ => format!("{} days ago", mins / 1440),
    };
    format!("{} ({rel})", local_time(utc))
}
