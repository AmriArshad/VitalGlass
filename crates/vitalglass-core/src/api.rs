//! Thin client for the Google Health API v4 (`https://health.googleapis.com`).
//!
//! Methods return the raw JSON items (`serde_json::Value`); [`crate::models`]
//! turns them into typed structs. Every request retries with exponential
//! backoff on 429/503/504, and every list-style call follows `nextPageToken`.

use std::time::Duration;

use chrono::{DateTime, Datelike, NaiveDate, SecondsFormat, Utc};
use reqwest::{Method, StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::Error;

const BASE_URL: &str = "https://health.googleapis.com/v4/users/me";

/// Retries after the first attempt. Delays are 1, 2, 4, 8, 16 s plus jitter,
/// so a request gives up after roughly half a minute.
const MAX_RETRIES: u32 = 5;

pub struct HealthClient {
    http: reqwest::Client,
    access_token: String,
}

impl HealthClient {
    pub fn new(http: reqwest::Client, access_token: String) -> Self {
        Self { http, access_token }
    }

    /// `GET identity`: confirms the Google account is linked to Google
    /// Health. Fails with [`Error::AccountNotLinked`] if it isn't.
    pub async fn identity(&self) -> Result<Value, Error> {
        self.request(Method::GET, "/identity", &[], None).await
    }

    /// `GET settings`: the user's unit preferences and time zone.
    pub async fn settings(&self) -> Result<Value, Error> {
        self.request(Method::GET, "/settings", &[], None).await
    }

    /// `GET pairedDevices`: watches/scales with battery and last sync time.
    pub async fn paired_devices(&self) -> Result<Vec<Value>, Error> {
        self.paginate(Method::GET, "/pairedDevices", None, &[], "pairedDevices")
            .await
    }

    /// `GET dataTypes/{data_type}/dataPoints`: raw data points, newest first.
    ///
    /// `data_type` is the hyphenated URL form (`daily-resting-heart-rate`);
    /// `filter` uses the underscored form
    /// (`daily_resting_heart_rate.date >= "2026-10-01"`).
    pub async fn list(&self, data_type: &str, filter: &str, page_size: Option<u32>) -> Result<Vec<Value>, Error> {
        let path = format!("/dataTypes/{data_type}/dataPoints");
        let mut query = vec![("filter", filter.to_owned())];
        if let Some(size) = page_size {
            query.push(("pageSize", size.to_string()));
        }
        self.paginate(Method::GET, &path, None, &query, "dataPoints").await
    }

    /// `POST dataTypes/{data_type}/dataPoints:dailyRollUp`
    ///
    /// Daily totals over civil (local calendar) days from `start` up to but
    /// not including `end`. The caller keeps the range within the type's
    /// limit (14 or 90 days).
    pub async fn daily_roll_up(&self, data_type: &str, start: NaiveDate, end: NaiveDate) -> Result<Vec<Value>, Error> {
        let path = format!("/dataTypes/{data_type}/dataPoints:dailyRollUp");
        let body = json!({
            "range": { "start": civil(start), "end": civil(end) },
            "windowSizeDays": 1,
            "pageSize": 10000,
        });
        self.paginate(Method::POST, &path, Some(body), &[], "rollupDataPoints")
            .await
    }

    /// `POST dataTypes/{data_type}/dataPoints:rollUp`
    ///
    /// Buckets of `window` over physical (UTC) time, e.g. 5-minute heart
    /// rate. `window` must be at least the type's storage resolution.
    pub async fn roll_up(
        &self,
        data_type: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        window: Duration,
    ) -> Result<Vec<Value>, Error> {
        let path = format!("/dataTypes/{data_type}/dataPoints:rollUp");
        let body = json!({
            "range": { "startTime": rfc3339(start), "endTime": rfc3339(end) },
            "windowSize": format!("{}s", window.as_secs()),
            "pageSize": 10000,
        });
        self.paginate(Method::POST, &path, Some(body), &[], "rollupDataPoints")
            .await
    }

    /// Repeat a request, following `nextPageToken`, and collect `items_key`
    /// from every page. GET requests carry the token as a query parameter,
    /// POST requests in the body.
    async fn paginate(
        &self,
        method: Method,
        path: &str,
        mut body: Option<Value>,
        query: &[(&str, String)],
        items_key: &str,
    ) -> Result<Vec<Value>, Error> {
        let mut query = query.to_vec();
        let mut items = Vec::new();
        loop {
            let page = self.request(method.clone(), path, &query, body.as_ref()).await?;
            if let Some(page_items) = page.get(items_key).and_then(Value::as_array) {
                items.extend(page_items.iter().cloned());
            }
            let Some(token) = page
                .get("nextPageToken")
                .and_then(Value::as_str)
                .filter(|t| !t.is_empty())
            else {
                return Ok(items);
            };
            match body.as_mut() {
                Some(b) => b["pageToken"] = json!(token),
                None => {
                    query.retain(|(k, _)| *k != "pageToken");
                    query.push(("pageToken", token.to_owned()));
                }
            }
        }
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&Value>,
    ) -> Result<Value, Error> {
        let mut attempt = 0;
        loop {
            let mut req = self
                .http
                .request(method.clone(), format!("{BASE_URL}{path}"))
                .bearer_auth(&self.access_token)
                .query(query);
            if let Some(body) = body {
                req = req.json(body);
            }
            let resp = req.send().await?;
            let status = resp.status();
            if status.is_success() {
                return Ok(resp.json().await?);
            }
            if is_retryable(status) && attempt < MAX_RETRIES {
                let delay = retry_after(&resp).unwrap_or_else(|| backoff(attempt));
                log::warn!("{status} from {path}, retrying in {delay:?}");
                tokio::time::sleep(delay).await;
                attempt += 1;
                continue;
            }
            let text = resp.text().await.unwrap_or_default();
            return Err(api_error(status, &text));
        }
    }
}

/// 429 is rate limiting; 500/503/504 are transient server-side failures
/// (seen in practice on `exercise` lists during the first real sync).
fn is_retryable(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS
            | StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

/// 2^attempt seconds plus up to 1 s of random jitter, so retries from
/// parallel requests don't line up.
fn backoff(attempt: u32) -> Duration {
    Duration::from_secs(1 << attempt) + Duration::from_millis(rand::random_range(0..1000))
}

/// Honour a `Retry-After: <seconds>` header if Google sends one.
fn retry_after(resp: &reqwest::Response) -> Option<Duration> {
    let secs: u64 = resp.headers().get("retry-after")?.to_str().ok()?.parse().ok()?;
    Some(Duration::from_secs(secs.min(60)))
}

/// `CivilDateTime` with only the date set (time defaults to midnight).
fn civil(date: NaiveDate) -> Value {
    json!({ "date": { "year": date.year(), "month": date.month(), "day": date.day() } })
}

fn rfc3339(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Google's standard error envelope:
/// `{"error": {"code": 400, "status": "FAILED_PRECONDITION", "message": "...",
///   "details": [{"@type": "...ErrorInfo", "reason": "ACCOUNT_NOT_LINKED"}]}}`
#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    #[serde(default)]
    status: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    details: Vec<ErrorDetail>,
}

#[derive(Deserialize)]
struct ErrorDetail {
    reason: Option<String>,
}

fn api_error(status: StatusCode, text: &str) -> Error {
    let Ok(ErrorEnvelope { error }) = serde_json::from_str::<ErrorEnvelope>(text) else {
        return Error::Api {
            status: status.as_u16(),
            status_text: String::new(),
            message: text.to_owned(),
        };
    };
    let reason = error.details.iter().find_map(|d| d.reason.as_deref());
    match (status, reason) {
        (_, Some("ACCOUNT_NOT_LINKED")) => Error::AccountNotLinked,
        (StatusCode::UNAUTHORIZED, _) => Error::Unauthorized,
        (StatusCode::FORBIDDEN, _) => Error::PermissionDenied(error.message),
        _ => Error::Api {
            status: status.as_u16(),
            status_text: error.status,
            message: error.message,
        },
    }
}
