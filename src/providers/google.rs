use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Command;

use super::{CreateOpts, CreatedKey, KeyInfo, KeyProvider, KeyStatus};

pub struct GoogleProvider {
    project_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GcloudKey {
    uid: String,
    display_name: Option<String>,
    create_time: Option<String>,
}

impl GoogleProvider {
    pub fn new(project_id: Option<String>) -> Self {
        Self { project_id }
    }

    fn check_gcloud() -> Result<()> {
        let output = Command::new("which")
            .arg("gcloud")
            .output()
            .context("failed to check for gcloud CLI")?;

        if !output.status.success() {
            bail!(
                "gcloud CLI not found. Install it from https://cloud.google.com/sdk/docs/install"
            );
        }
        Ok(())
    }

    fn run_gcloud(&self, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new("gcloud");
        cmd.args(args);

        if let Some(ref project) = self.project_id {
            cmd.arg(format!("--project={}", project));
        }

        let output = cmd
            .output()
            .context("failed to run gcloud command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("gcloud error: {}", stderr.trim());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

fn extract_date(datetime: &Option<String>) -> Option<String> {
    datetime
        .as_ref()
        .and_then(|dt| dt.get(..10).map(|s| s.to_string()))
}

#[async_trait]
impl KeyProvider for GoogleProvider {
    fn name(&self) -> &str {
        "google"
    }

    fn supports_create(&self) -> bool {
        true
    }

    async fn list(&self) -> Result<Vec<KeyInfo>> {
        Self::check_gcloud()?;

        let output = self.run_gcloud(&["services", "api-keys", "list", "--format=json"])?;

        if output.trim().is_empty() {
            return Ok(Vec::new());
        }

        let gcloud_keys: Vec<GcloudKey> =
            serde_json::from_str(&output).context("failed to parse gcloud key list")?;

        let keys = gcloud_keys
            .into_iter()
            .map(|k| KeyInfo {
                id: k.uid,
                name: k.display_name.unwrap_or_else(|| "(unnamed)".to_string()),
                status: KeyStatus::Active,
                created: extract_date(&k.create_time),
                limit: None,
                used: None,
            })
            .collect();

        Ok(keys)
    }

    async fn create(&self, name: &str, opts: &CreateOpts) -> Result<CreatedKey> {
        Self::check_gcloud()?;

        let display_name_arg = format!("--display-name={}", name);
        let mut args = vec![
            "services",
            "api-keys",
            "create",
            &display_name_arg,
            "--format=json",
        ];

        let restrict_arg;
        if let Some(ref api) = opts.restrict_api {
            restrict_arg = format!("--api-target=service={}", api);
            args.push(&restrict_arg);
        }

        let output = self.run_gcloud(&args)?;

        let key_string = output
            .lines()
            .find(|l| l.contains("keyString"))
            .and_then(|l| {
                let parts: Vec<&str> = l.split('"').collect();
                parts.get(3).map(|s| s.to_string())
            })
            .unwrap_or_else(|| "(key in gcloud output)".to_string());

        Ok(CreatedKey {
            name: name.to_string(),
            key: key_string,
            details: Vec::new(),
        })
    }

    async fn revoke(&self, key_id: &str) -> Result<()> {
        Self::check_gcloud()?;

        self.run_gcloud(&["services", "api-keys", "delete", key_id, "--quiet"])?;
        Ok(())
    }
}
