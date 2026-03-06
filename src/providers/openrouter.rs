use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use super::{CreateOpts, CreatedKey, KeyInfo, KeyProvider, KeyStatus};

const BASE_URL: &str = "https://openrouter.ai/api/v1/keys";

pub struct OpenRouterProvider {
    management_key: String,
    client: Client,
}

#[derive(Debug, Deserialize)]
struct ListResponse {
    data: Vec<ApiKey>,
}

#[derive(Debug, Deserialize)]
struct ApiKey {
    hash: String,
    name: Option<String>,
    label: Option<String>,
    disabled: bool,
    limit: Option<f64>,
    limit_remaining: Option<f64>,
    usage: Option<f64>,
    created_at: Option<String>,
    limit_reset: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateResponse {
    data: CreatedApiKeyData,
    key: String,
}

#[derive(Debug, Deserialize)]
struct CreatedApiKeyData {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    error: Option<ApiErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: Option<String>,
}

impl OpenRouterProvider {
    pub fn new(management_key: String) -> Self {
        Self {
            management_key,
            client: Client::new(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.management_key)
    }

    async fn check_error(&self, response: reqwest::Response) -> Result<reqwest::Response> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }

        let body = response.text().await.unwrap_or_default();
        if let Ok(api_err) = serde_json::from_str::<ApiError>(&body) {
            if let Some(detail) = api_err.error {
                if let Some(msg) = detail.message {
                    bail!("OpenRouter API error ({}): {}", status, msg);
                }
            }
        }
        bail!("OpenRouter API error ({}): {}", status, body);
    }
}

fn format_limit(limit: Option<f64>, reset: &Option<String>) -> Option<String> {
    let amount = limit.filter(|&v| v > 0.0)?;
    let suffix = match reset.as_deref() {
        Some("monthly") => "/mo",
        Some("weekly") => "/wk",
        Some("daily") => "/day",
        Some(other) => {
            return Some(format!("${:.2}/{}", amount, other));
        }
        None => "",
    };
    Some(format!("${:.2}{}", amount, suffix))
}

fn format_usage(usage: Option<f64>) -> Option<String> {
    usage.map(|v| format!("${:.2}", v))
}

fn extract_date(datetime: &Option<String>) -> Option<String> {
    datetime.as_ref().and_then(|dt| {
        dt.get(..10).map(|s| s.to_string())
    })
}

#[async_trait]
impl KeyProvider for OpenRouterProvider {
    fn name(&self) -> &str {
        "openrouter"
    }

    fn supports_create(&self) -> bool {
        true
    }

    async fn list(&self) -> Result<Vec<KeyInfo>> {
        let response = self
            .client
            .get(BASE_URL)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .context("failed to reach OpenRouter API")?;

        let response = self.check_error(response).await?;
        let body: ListResponse = response
            .json()
            .await
            .context("failed to parse OpenRouter key list")?;

        let keys = body
            .data
            .into_iter()
            .map(|k| {
                let name = k
                    .name
                    .filter(|n| !n.is_empty())
                    .or(k.label.filter(|l| !l.is_empty()))
                    .unwrap_or_else(|| "(unnamed)".to_string());

                KeyInfo {
                    id: k.hash,
                    name,
                    status: if k.disabled {
                        KeyStatus::Disabled
                    } else {
                        KeyStatus::Active
                    },
                    created: extract_date(&k.created_at),
                    limit: format_limit(k.limit, &k.limit_reset),
                    used: format_usage(k.usage),
                }
            })
            .collect();

        Ok(keys)
    }

    async fn create(&self, name: &str, opts: &CreateOpts) -> Result<CreatedKey> {
        let mut body = serde_json::json!({ "name": name });

        if let Some(limit) = opts.limit {
            body["limit"] = serde_json::json!(limit);
        }
        if let Some(ref reset) = opts.reset {
            body["limit_reset"] = serde_json::json!(reset);
        }

        let response = self
            .client
            .post(BASE_URL)
            .header("Authorization", self.auth_header())
            .json(&body)
            .send()
            .await
            .context("failed to reach OpenRouter API")?;

        let response = self.check_error(response).await?;
        let created: CreateResponse = response
            .json()
            .await
            .context("failed to parse OpenRouter create response")?;

        let mut details = Vec::new();
        if let Some(limit) = opts.limit {
            let reset_label = opts.reset.as_deref().unwrap_or("no reset");
            details.push(("Limit".to_string(), format!("${:.2}/{}", limit, reset_label)));
        }

        Ok(CreatedKey {
            name: created.data.name.unwrap_or_else(|| name.to_string()),
            key: created.key,
            details,
        })
    }

    async fn revoke(&self, key_id: &str) -> Result<()> {
        let url = format!("{}/{}", BASE_URL, key_id);

        let response = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .context("failed to reach OpenRouter API")?;

        self.check_error(response).await?;
        Ok(())
    }
}
