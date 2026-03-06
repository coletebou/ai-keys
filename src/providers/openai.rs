use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use super::{CreateOpts, CreatedKey, KeyInfo, KeyProvider, KeyStatus};

const BASE_URL: &str = "https://api.openai.com/v1/organization";

pub struct OpenAiProvider {
    admin_key: String,
    client: Client,
}

#[derive(Debug, Deserialize)]
struct ProjectList {
    data: Vec<Project>,
}

#[derive(Debug, Deserialize)]
struct Project {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct ServiceAccountList {
    data: Vec<ServiceAccount>,
}

#[derive(Debug, Deserialize)]
struct ServiceAccount {
    id: String,
    name: String,
    created_at: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct CreateSaResponse {
    id: String,
    name: String,
    api_key: CreatedApiKey,
}

#[derive(Debug, Deserialize)]
struct CreatedApiKey {
    value: String,
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    created_at: Option<u64>,
}

impl OpenAiProvider {
    pub fn new(admin_key: String) -> Self {
        Self {
            admin_key,
            client: Client::new(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.admin_key)
    }

    async fn check_error(&self, response: reqwest::Response, label: &str) -> Result<reqwest::Response> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }

        let body = response.text().await.unwrap_or_default();
        bail!("OpenAI {} error ({}): {}", label, status, body);
    }

    async fn list_projects(&self) -> Result<Vec<Project>> {
        let url = format!("{}/projects?limit=100", BASE_URL);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .context("failed to reach OpenAI API")?;

        let resp = self.check_error(resp, "list projects").await?;
        let body: ProjectList = resp.json().await.context("failed to parse OpenAI project list")?;
        Ok(body.data)
    }

    async fn list_service_accounts(&self, project_id: &str) -> Result<Vec<ServiceAccount>> {
        let url = format!("{}/projects/{}/service_accounts?limit=100", BASE_URL, project_id);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .context("failed to reach OpenAI API")?;

        let resp = self.check_error(resp, "list service accounts").await?;
        let body: ServiceAccountList = resp
            .json()
            .await
            .context("failed to parse OpenAI service account list")?;
        Ok(body.data)
    }
}

fn format_timestamp(ts: Option<u64>) -> Option<String> {
    ts.map(|t| {
        let secs = t;
        let days = secs / 86400;
        let years = days / 365 + 1970;
        let remaining_days = days % 365;
        let month = remaining_days / 30 + 1;
        let day = remaining_days % 30 + 1;
        format!("{:04}-{:02}-{:02}", years, month, day)
    })
}

#[async_trait]
impl KeyProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn supports_create(&self) -> bool {
        true
    }

    async fn list(&self) -> Result<Vec<KeyInfo>> {
        let projects = self.list_projects().await?;
        let mut keys = Vec::new();

        for project in &projects {
            let accounts = self.list_service_accounts(&project.id).await?;
            for sa in accounts {
                keys.push(KeyInfo {
                    id: format!("{}/{}", project.id, sa.id),
                    name: format!("{}/{}", project.name, sa.name),
                    status: KeyStatus::Active,
                    created: format_timestamp(sa.created_at),
                    limit: None,
                    used: None,
                });
            }
        }

        Ok(keys)
    }

    async fn create(&self, name: &str, opts: &CreateOpts) -> Result<CreatedKey> {
        let projects = self.list_projects().await?;
        if projects.is_empty() {
            bail!("no projects found in OpenAI organization");
        }

        let project = if let Some(ref proj_name) = opts.project {
            projects
                .iter()
                .find(|p| p.name.eq_ignore_ascii_case(proj_name) || p.id == *proj_name)
                .ok_or_else(|| anyhow::anyhow!("project '{}' not found", proj_name))?
        } else {
            &projects[0]
        };

        let url = format!("{}/projects/{}/service_accounts", BASE_URL, project.id);
        let body = serde_json::json!({ "name": name });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&body)
            .send()
            .await
            .context("failed to reach OpenAI API")?;

        let resp = self.check_error(resp, "create service account").await?;
        let created: CreateSaResponse = resp
            .json()
            .await
            .context("failed to parse OpenAI create response")?;

        let details = vec![
            ("Project".to_string(), project.name.clone()),
            ("SA ID".to_string(), created.id),
        ];

        Ok(CreatedKey {
            name: created.name,
            key: created.api_key.value,
            details,
        })
    }

    async fn revoke(&self, key_id: &str) -> Result<()> {
        let parts: Vec<&str> = key_id.splitn(2, '/').collect();
        if parts.len() != 2 {
            bail!("invalid key ID format: expected 'project_id/sa_id', got '{}'", key_id);
        }
        let (project_id, sa_id) = (parts[0], parts[1]);

        let url = format!(
            "{}/projects/{}/service_accounts/{}",
            BASE_URL, project_id, sa_id
        );

        let resp = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .context("failed to reach OpenAI API")?;

        self.check_error(resp, "delete service account").await?;
        Ok(())
    }
}
