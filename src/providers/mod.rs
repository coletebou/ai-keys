pub mod anthropic;
pub mod google;
pub mod openai;
pub mod openrouter;
pub mod xai;

use anyhow::{bail, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    pub id: String,
    pub name: String,
    pub status: KeyStatus,
    pub created: Option<String>,
    pub limit: Option<String>,
    pub used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyStatus {
    Active,
    Disabled,
    Unknown,
}

impl fmt::Display for KeyStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyStatus::Active => write!(f, "active"),
            KeyStatus::Disabled => write!(f, "disabled"),
            KeyStatus::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Default)]
pub struct CreateOpts {
    pub limit: Option<f64>,
    pub reset: Option<String>,
    pub project: Option<String>,
    pub acl: Option<String>,
    pub restrict_api: Option<String>,
}

#[derive(Debug)]
pub struct CreatedKey {
    pub name: String,
    pub key: String,
    pub details: Vec<(String, String)>,
}

#[async_trait]
pub trait KeyProvider: Send + Sync {
    fn name(&self) -> &str;
    fn supports_create(&self) -> bool;
    async fn list(&self) -> Result<Vec<KeyInfo>>;
    async fn create(&self, name: &str, opts: &CreateOpts) -> Result<CreatedKey>;
    async fn revoke(&self, key_id: &str) -> Result<()>;
}

pub fn get_provider(name: &str, config: &Config) -> Result<Box<dyn KeyProvider>> {
    let mgmt_key = config.get_management_key(name)?;

    match name {
        "openrouter" => {
            let key = mgmt_key.ok_or_else(|| {
                anyhow::anyhow!(
                    "no management key found for openrouter. Set the OPENROUTER_MGMT_KEY env var."
                )
            })?;
            Ok(Box::new(openrouter::OpenRouterProvider::new(key)))
        }
        "openai" => {
            let key = mgmt_key.ok_or_else(|| {
                anyhow::anyhow!(
                    "no management key found for openai. Set the OPENAI_ADMIN_KEY env var."
                )
            })?;
            Ok(Box::new(openai::OpenAiProvider::new(key)))
        }
        "xai" => {
            let key = mgmt_key.ok_or_else(|| {
                anyhow::anyhow!(
                    "no management key found for xai. Set the XAI_MGMT_KEY env var."
                )
            })?;
            let team_id = config
                .providers
                .get("xai")
                .and_then(|p| p.team_id.clone())
                .unwrap_or_else(|| "default".to_string());
            Ok(Box::new(xai::XaiProvider::new(key, team_id)))
        }
        "anthropic" => {
            let key = mgmt_key.ok_or_else(|| {
                anyhow::anyhow!(
                    "no management key found for anthropic. Set the ANTHROPIC_ADMIN_KEY env var."
                )
            })?;
            Ok(Box::new(anthropic::AnthropicProvider::new(key)))
        }
        "google" => {
            let project_id = config
                .providers
                .get("google")
                .and_then(|p| p.project_id.clone());
            Ok(Box::new(google::GoogleProvider::new(project_id)))
        }
        _ => bail!("unknown provider: {}", name),
    }
}
