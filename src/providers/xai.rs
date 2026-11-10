use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use super::{CreateOpts, CreatedKey, KeyInfo, KeyProvider, KeyStatus};

const BASE_URL: &str = "https://management-api.x.ai/auth/teams";

pub struct XaiProvider {
    management_key: String,
    team_id: String,
    client: Client,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListResponse {
    api_keys: Vec<ApiKey>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiKey {
    api_key_id: String,
    name: Option<String>,
    created_at: Option<String>,
    #[allow(dead_code)]
    redacted_api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateResponse {
    api_key_id: String,
    api_key: String,
    name: Option<String>,
}

impl XaiProvider {
    pub fn new(management_key: String, team_id: String) -> Self {
        Self {
            management_key,
            team_id,
            client: Client::new(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.management_key)
    }

    fn team_url(&self) -> String {
        format!("{}/{}/api-keys", BASE_URL, self.team_id)
    }

    async fn check_error(&self, response: reqwest::Response) -> Result<reqwest::Response> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }

        let body = response.text().await.unwrap_or_default();
        bail!("xAI API error ({}): {}", status, body);
    }
}

fn extract_date(datetime: &Option<String>) -> Option<String> {
    datetime
        .as_ref()
        .and_then(|dt| dt.get(..10).map(|s| s.to_string()))
}

#[async_trait]
impl KeyProvider for XaiProvider {
    fn supports_create(&self) -> bool {
        true
    }

    async fn list(&self) -> Result<Vec<KeyInfo>> {
        let resp = self
            .client
            .get(self.team_url())
            .header("Authorization", self.auth_header())
            .send()
            .await
            .context("failed to reach xAI API")?;

        let resp = self.check_error(resp).await?;
        let body: ListResponse = resp.json().await.context("failed to parse xAI key list")?;

        let keys = body
            .api_keys
            .into_iter()
            .map(|k| KeyInfo {
                id: k.api_key_id,
                name: k.name.unwrap_or_else(|| "(unnamed)".to_string()),
                status: KeyStatus::Active,
                created: extract_date(&k.created_at),
                limit: None,
                used: None,
            })
            .collect();

        Ok(keys)
    }

    async fn create(&self, name: &str, opts: &CreateOpts) -> Result<CreatedKey> {
        let acls = opts
            .acl
            .as_deref()
            .map(|a| {
                a.split(',')
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                vec![
                    "api-key:model:*".to_string(),
                    "api-key:endpoint:*".to_string(),
                ]
            });

        let body = serde_json::json!({
            "name": name,
            "acls": acls,
        });

        let resp = self
            .client
            .post(self.team_url())
            .header("Authorization", self.auth_header())
            .json(&body)
            .send()
            .await
            .context("failed to reach xAI API")?;

        let resp = self.check_error(resp).await?;
        let created: CreateResponse = resp
            .json()
            .await
            .context("failed to parse xAI create response")?;

        let details = vec![("Key ID".to_string(), created.api_key_id)];

        Ok(CreatedKey {
            name: created.name.unwrap_or_else(|| name.to_string()),
            key: created.api_key,
            details,
        })
    }

    async fn revoke(&self, key_id: &str) -> Result<()> {
        let url = format!("{}/{}", self.team_url(), key_id);

        let resp = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .context("failed to reach xAI API")?;

        self.check_error(resp).await?;
        Ok(())
    }
}
