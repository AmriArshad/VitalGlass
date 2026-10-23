//! Phase 1 spike: sign in, check the account is linked, and print the last
//! 7 days of steps from `dailyRollUp` as JSON.
//!
//! Run with `cargo run -p vitalglass-cli`. Progress goes to stderr and the
//! JSON to stdout, so `cargo run -p vitalglass-cli > out.json` works.
//! Tokens are never printed or saved; you sign in on every run.

use anyhow::Context;
use chrono::{Days, Local};
use serde_json::json;
use vitalglass_core::api::HealthClient;
use vitalglass_core::config::ClientConfig;
use vitalglass_core::{Error, auth};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = ClientConfig::load()?;
    let http = reqwest_client()?;

    let tokens = auth::login(&client, &http).await?;
    eprintln!("Signed in. Access token valid until {}.", tokens.expires_at);
    let missing = tokens.missing_scopes();
    if !missing.is_empty() {
        eprintln!("Not granted: {} (related data will be unavailable)", missing.join(", "));
    }

    // Prove the refresh path works now, rather than finding out in a week.
    match &tokens.refresh_token {
        Some(rt) => {
            auth::refresh(&client, &http, rt)
                .await
                .context("refreshing the access token")?;
            eprintln!("Refresh token works.");
        }
        None => eprintln!("Warning: Google returned no refresh token."),
    }

    let api = HealthClient::new(http, tokens.access_token);

    let identity = match api.identity().await {
        Err(e @ Error::AccountNotLinked) => anyhow::bail!(e),
        other => other.context("GET identity")?,
    };

    // Last 7 civil days including today: [today - 6, tomorrow).
    let today = Local::now().date_naive();
    let start = today - Days::new(6);
    let end = today + Days::new(1);
    let steps = api
        .daily_roll_up("steps", start, end)
        .await
        .context("steps dailyRollUp")?;

    let out = json!({ "identity": identity, "steps": steps });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn reqwest_client() -> anyhow::Result<vitalglass_core::reqwest::Client> {
    Ok(vitalglass_core::reqwest::Client::builder()
        .user_agent(concat!("vitalglass/", env!("CARGO_PKG_VERSION")))
        .build()?)
}
