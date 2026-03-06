use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub provider: String,
    pub name: String,
    pub env_var: String,
    pub value: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Ledger {
    #[serde(default)]
    pub keys: Vec<LedgerEntry>,
}

impl Ledger {
    fn path() -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
        config_dir.join("ai-keys").join("keys.json")
    }

    pub fn load() -> Result<Self> {
        let path = Self::path();
        if path.exists() {
            let content =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            let ledger: Ledger = serde_json::from_str(&content)
                .with_context(|| format!("parsing {}", path.display()))?;
            Ok(ledger)
        } else {
            Ok(Ledger::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating directory {}", parent.display()))?;
        }
        let content = serde_json::to_string_pretty(self).context("serializing ledger to JSON")?;
        fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    pub fn add_key(&mut self, provider: &str, name: &str, env_var: &str, value: &str) {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.keys.push(LedgerEntry {
            provider: provider.to_string(),
            name: name.to_string(),
            env_var: env_var.to_string(),
            value: value.to_string(),
            created_at: today,
        });
    }

    pub fn get_keys(&self) -> &[LedgerEntry] {
        &self.keys
    }
}

/// Generate an env var name from provider + key name.
/// E.g., "openrouter" + "badclaw-cole" = "OPENROUTER_BADCLAW_COLE"
pub fn make_env_var(provider: &str, name: &str) -> String {
    format!("{}_{}", provider, name)
        .to_uppercase()
        .replace('-', "_")
}

/// Sync all ledger keys to configured remote machines via SSH.
pub async fn sync_keys(config: &Config) -> Result<()> {
    let ledger = Ledger::load()?;
    let keys = ledger.get_keys();

    if keys.is_empty() {
        println!(
            "{} No keys in ledger. Create keys with {} first.",
            "!".yellow(),
            "`ai-keys create`".cyan()
        );
        return Ok(());
    }

    if config.machines.is_empty() {
        println!(
            "{} No machines configured. Add machines to {}",
            "!".yellow(),
            "~/.config/ai-keys/config.toml".cyan()
        );
        return Ok(());
    }

    println!();
    println!("{}", "Syncing keys to machines...".bold());
    println!();

    for (machine_name, machine_config) in &config.machines {
        match sync_to_machine(&machine_config.host, &machine_config.secrets_file, keys) {
            Ok(count) => {
                println!(
                    "  {} {}: {} keys synced",
                    "\u{2713}".green(),
                    machine_name.bold(),
                    count
                );
            }
            Err(e) => {
                println!(
                    "  {} {}: SSH failed: {}",
                    "\u{2717}".red(),
                    machine_name.bold(),
                    format!("{:#}", e).red()
                );
            }
        }
    }

    println!();
    Ok(())
}

fn sync_to_machine(host: &str, secrets_file: &str, keys: &[LedgerEntry]) -> Result<usize> {
    // Read the remote secrets file (ok if it doesn't exist)
    let output = Command::new("ssh")
        .args([host, &format!("cat {} 2>/dev/null", secrets_file)])
        .output()
        .with_context(|| format!("running ssh to {}", host))?;

    if !output.status.success() && !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Connection failures should be reported, but missing file is ok
        if stderr.contains("Connection refused")
            || stderr.contains("Could not resolve")
            || stderr.contains("Permission denied")
            || stderr.contains("No route to host")
            || stderr.contains("Connection timed out")
        {
            anyhow::bail!("{}", stderr.trim());
        }
    }

    let existing = String::from_utf8_lossy(&output.stdout).to_string();
    let mut lines: Vec<String> = existing.lines().map(|l| l.to_string()).collect();
    let mut synced = 0;

    for key in keys {
        let export_line = format!("export {}=\"{}\"", key.env_var, key.value);
        let prefix = format!("export {}=", key.env_var);

        // Update existing line or append
        let mut found = false;
        for line in &mut lines {
            if line.starts_with(&prefix) {
                *line = export_line.clone();
                found = true;
                break;
            }
        }
        if !found {
            lines.push(export_line);
        }
        synced += 1;
    }

    // Write back via SSH
    let content = lines.join("\n");
    let content = if content.ends_with('\n') {
        content
    } else {
        format!("{}\n", content)
    };

    let write_output = Command::new("ssh")
        .args([host, &format!("cat > {}", secrets_file)])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("spawning ssh to {}", host))?;

    use std::io::Write;
    let mut child = write_output;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(content.as_bytes())
            .context("writing to ssh stdin")?;
    }

    let result = child.wait_with_output().context("waiting for ssh")?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        anyhow::bail!("write failed: {}", stderr.trim());
    }

    Ok(synced)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_env_var() {
        assert_eq!(make_env_var("openrouter", "badclaw-cole"), "OPENROUTER_BADCLAW_COLE");
        assert_eq!(make_env_var("openai", "my-key"), "OPENAI_MY_KEY");
        assert_eq!(make_env_var("xai", "test"), "XAI_TEST");
    }

    #[test]
    fn test_ledger_add_key() {
        let mut ledger = Ledger::default();
        assert!(ledger.get_keys().is_empty());

        ledger.add_key("openrouter", "test-key", "OPENROUTER_TEST_KEY", "sk-123");
        assert_eq!(ledger.get_keys().len(), 1);
        assert_eq!(ledger.get_keys()[0].provider, "openrouter");
        assert_eq!(ledger.get_keys()[0].env_var, "OPENROUTER_TEST_KEY");
    }

    #[test]
    fn test_ledger_roundtrip() {
        let mut ledger = Ledger::default();
        ledger.add_key("openai", "test", "OPENAI_TEST", "sk-abc");

        let json = serde_json::to_string_pretty(&ledger).unwrap();
        let parsed: Ledger = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.keys.len(), 1);
        assert_eq!(parsed.keys[0].value, "sk-abc");
    }
}
