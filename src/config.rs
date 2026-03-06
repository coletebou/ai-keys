use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub machines: HashMap<String, MachineConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub management_key_env: Option<String>,
    pub team_id: Option<String>,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineConfig {
    pub host: String,
    #[serde(default = "default_secrets_file")]
    pub secrets_file: String,
}

fn default_secrets_file() -> String {
    "~/.env_secrets".to_string()
}

impl Default for Config {
    fn default() -> Self {
        let mut providers = HashMap::new();

        providers.insert(
            "openrouter".to_string(),
            ProviderConfig {
                management_key_env: Some("OPENROUTER_MGMT_KEY".to_string()),
                team_id: None,
                project_id: None,
            },
        );

        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                management_key_env: Some("OPENAI_ADMIN_KEY".to_string()),
                team_id: None,
                project_id: None,
            },
        );

        providers.insert(
            "xai".to_string(),
            ProviderConfig {
                management_key_env: Some("XAI_MGMT_KEY".to_string()),
                team_id: Some("default".to_string()),
                project_id: None,
            },
        );

        providers.insert(
            "google".to_string(),
            ProviderConfig {
                management_key_env: None,
                team_id: None,
                project_id: None,
            },
        );

        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                management_key_env: Some("ANTHROPIC_ADMIN_KEY".to_string()),
                team_id: None,
                project_id: None,
            },
        );

        Config {
            providers,
            machines: HashMap::new(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
        config_dir.join("ai-keys").join("config.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            let config: Config =
                toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating directory {}", parent.display()))?;
        }
        let content =
            toml::to_string_pretty(self).context("serializing config to TOML")?;
        fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    pub fn get_management_key(&self, provider: &str) -> Result<Option<String>> {
        let provider_config = match self.providers.get(provider) {
            Some(c) => c,
            None => bail!("unknown provider: {}", provider),
        };

        match &provider_config.management_key_env {
            Some(env_var) => match std::env::var(env_var) {
                Ok(val) if !val.is_empty() => Ok(Some(val)),
                _ => Ok(None),
            },
            None => Ok(None),
        }
    }

    pub fn init() -> Result<()> {
        let path = Self::config_path();
        if path.exists() {
            bail!(
                "config file already exists at {}\nEdit it directly or delete it to re-initialize.",
                path.display()
            );
        }

        let config = Config::default();
        config.save()?;

        println!("Created config at: {}", path.display());
        println!();
        println!("Next steps:");
        println!("  1. Edit the config to add your management keys as env vars");
        println!("  2. Export those env vars in your shell (e.g. in ~/.env_secrets)");
        println!("  3. Run `ai-keys list` to verify connectivity");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_all_providers() {
        let config = Config::default();
        let expected = ["openrouter", "openai", "xai", "google", "anthropic"];
        for provider in &expected {
            assert!(
                config.providers.contains_key(*provider),
                "missing provider: {}",
                provider
            );
        }
        assert_eq!(config.providers.len(), 5);
    }

    #[test]
    fn test_parse_config_toml() {
        let toml_str = r#"
[providers.openrouter]
management_key_env = "MY_OR_KEY"

[machines.dev-box]
host = "10.0.0.1"
secrets_file = "/etc/secrets"
"#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.providers["openrouter"].management_key_env.as_deref(),
            Some("MY_OR_KEY")
        );
        assert_eq!(config.machines["dev-box"].host, "10.0.0.1");
        assert_eq!(config.machines["dev-box"].secrets_file, "/etc/secrets");
    }
}
