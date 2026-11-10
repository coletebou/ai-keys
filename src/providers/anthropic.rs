use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use super::{CreateOpts, CreatedKey, KeyInfo, KeyProvider, KeyStatus};

const BASE_URL: &str = "https://api.anthropic.com/v1/organizations/api_keys";

pub struct AnthropicProvider {
    admin_key: String,
    client: Client,
}

#[derive(Debug, Deserialize)]
struct ListResponse {
    data: Vec<ApiKey>,
}

#[derive(Debug, Deserialize)]
struct ApiKey {
    id: String,
    name: String,
    status: String,
    created_at: Option<String>,
}

impl AnthropicProvider {
    pub fn new(admin_key: String) -> Self {
        Self {
            admin_key,
            client: Client::new(),
        }
    }

    async fn check_error(&self, response: reqwest::Response) -> Result<reqwest::Response> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }

        let body = response.text().await.unwrap_or_default();
        bail!("Anthropic API error ({}): {}", status, body);
    }
}

fn extract_date(datetime: &Option<String>) -> Option<String> {
    datetime
        .as_ref()
        .and_then(|dt| dt.get(..10).map(|s| s.to_string()))
}

fn parse_status(status: &str) -> KeyStatus {
    match status {
        "active" => KeyStatus::Active,
        "inactive" => KeyStatus::Disabled,
        _ => KeyStatus::Unknown,
    }
}

#[async_trait]
impl KeyProvider for AnthropicProvider {
    fn supports_create(&self) -> bool {
        false
    }

    async fn list(&self) -> Result<Vec<KeyInfo>> {
        let url = format!("{}?limit=100", BASE_URL);
        let resp = self
            .client
            .get(&url)
            .header("x-api-key", &self.admin_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .context("failed to reach Anthropic API")?;

        let resp = self.check_error(resp).await?;
        let body: ListResponse = resp
            .json()
            .await
            .context("failed to parse Anthropic key list")?;

        let keys = body
            .data
            .into_iter()
            .map(|k| KeyInfo {
                id: k.id,
                name: k.name,
                status: parse_status(&k.status),
                created: extract_date(&k.created_at),
                limit: None,
                used: None,
            })
            .collect();

        Ok(keys)
    }

    async fn create(&self, _name: &str, _opts: &CreateOpts) -> Result<CreatedKey> {
        bail!(
            "Anthropic does not support programmatic key creation. \
             Visit https://console.anthropic.com/settings/keys"
        );
    }

    async fn revoke(&self, key_id: &str) -> Result<()> {
        let url = format!("{}/{}", BASE_URL, key_id);
        let body = serde_json::json!({ "status": "inactive" });

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.admin_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .context("failed to reach Anthropic API")?;

        self.check_error(resp).await?;
        Ok(())
    }
}
