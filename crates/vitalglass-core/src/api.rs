//! Thin client for the Google Health API v4 (`https://health.googleapis.com`).
//!
//! Responses are returned as `serde_json::Value` for now; typed models come
//! with the sync/store layer once we've seen real payloads.

use chrono::{Datelike, NaiveDate};
use reqwest::{Method, StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::Error;

const BASE_URL: &str = "https://health.googleapis.com/v4/users/me";

pub struct HealthClient {
    http: reqwest::Client,
    access_token: String,
}

impl HealthClient {
    pub fn new(http: reqwest::Client, access_token: String) -> Self {
        Self { http, access_token }
    }

    /// `GET /v4/users/me/identity`: confirms the Google account is linked
    /// to Google Health. Fails with [`Error::AccountNotLinked`] if it isn't.
    pub async fn identity(&self) -> Result<Value, Error> {
        self.request(Method::GET, "/identity", None).await
    }

    /// `POST /v4/users/me/dataTypes/{data_type}/dataPoints:dailyRollUp`
    ///
    /// Daily totals over civil (local calendar) days from `start` up to but
    /// not including `end`. `data_type` is the hyphenated URL form, e.g.
    /// `active-zone-minutes`. Follows `nextPageToken` and returns every
    /// `rollupDataPoints` entry in one list.
    pub async fn daily_roll_up(&self, data_type: &str, start: NaiveDate, end: NaiveDate) -> Result<Vec<Value>, Error> {
        let path = format!("/dataTypes/{data_type}/dataPoints:dailyRollUp");
        let mut body = json!({
            "range": { "start": civil(start), "end": civil(end) },
            "windowSizeDays": 1,
        });
        let mut points = Vec::new();
        loop {
            let page = self.request(Method::POST, &path, Some(&body)).await?;
            if let Some(items) = page.get("rollupDataPoints").and_then(Value::as_array) {
                points.extend(items.iter().cloned());
            }
            match page.get("nextPageToken").and_then(Value::as_str) {
                Some(token) if !token.is_empty() => body["pageToken"] = json!(token),
                _ => return Ok(points),
            }
        }
    }

    async fn request(&self, method: Method, path: &str, body: Option<&Value>) -> Result<Value, Error> {
        let mut req = self
            .http
            .request(method, format!("{BASE_URL}{path}"))
            .bearer_auth(&self.access_token);
        if let Some(body) = body {
            req = req.json(body);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if status.is_success() {
            return Ok(resp.json().await?);
        }
        let text = resp.text().await.unwrap_or_default();
        Err(api_error(status, &text))
    }
}

/// `CivilDateTime` with only the date set (time defaults to midnight).
fn civil(date: NaiveDate) -> Value {
    json!({ "date": { "year": date.year(), "month": date.month(), "day": date.day() } })
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
