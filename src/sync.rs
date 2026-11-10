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
    let raw = format!("{}_{}", provider, name).to_uppercase();
    // Replace any non-alphanumeric character with underscore, collapse runs
    let sanitized: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    // Collapse consecutive underscores
    let mut result = String::with_capacity(sanitized.len());
    let mut prev_underscore = false;
    for c in sanitized.chars() {
        if c == '_' {
            if !prev_underscore {
                result.push(c);
            }
            prev_underscore = true;
        } else {
            result.push(c);
            prev_underscore = false;
        }
    }
    result.trim_end_matches('_').to_string()
}

/// Sync all ledger keys to configured remote machines via SSH.
///
/// For each machine, runs up to two write paths:
/// - **env file** (legacy): a single shell-source file with `export NAME="value"` lines.
///   Skipped when `secrets_file` is empty.
/// - **per-key files** (modern): one file per key, mode 0400, suitable for systemd
///   `LoadCredential=` or Docker Compose `secrets:`. Skipped when `secrets_dir` is unset.
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
        let env_result = if !machine_config.secrets_file.is_empty() {
            Some(sync_envfile_to_machine(
                &machine_config.host,
                &machine_config.secrets_file,
                keys,
            ))
        } else {
            None
        };

        let dir_result = machine_config
            .secrets_dir
            .as_ref()
            .map(|dir| sync_perkey_files_to_machine(&machine_config.host, dir, keys));

        let mut summary: Vec<String> = Vec::new();
        let mut had_error = false;

        if let Some(r) = env_result {
            match r {
                Ok(n) => summary.push(format!("{} env keys", n)),
                Err(e) => {
                    summary.push(format!("env file failed: {:#}", e).red().to_string());
                    had_error = true;
                }
            }
        }
        if let Some(r) = dir_result {
            match r {
                Ok(n) => summary.push(format!("{} per-key files", n)),
                Err(e) => {
                    summary.push(format!("per-key files failed: {:#}", e).red().to_string());
                    had_error = true;
                }
            }
        }
        if summary.is_empty() {
            summary.push("nothing to do (no secrets_file or secrets_dir)".to_string());
        }

        let mark = if had_error {
            "\u{2717}".red()
        } else {
            "\u{2713}".green()
        };
        println!("  {} {}: {}", mark, machine_name.bold(), summary.join(", "));
    }

    println!();
    Ok(())
}

fn sync_envfile_to_machine(host: &str, secrets_file: &str, keys: &[LedgerEntry]) -> Result<usize> {
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

/// POSIX shell escape for command arguments. Wraps in single quotes when needed.
/// Safe characters pass through unquoted.
fn shell_escape(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "/_.-~=:".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Build the path `<dir>/<filename>`, handling the trailing-slash case.
fn join_remote_path(dir: &str, filename: &str) -> String {
    if dir.ends_with('/') {
        format!("{}{}", dir, filename)
    } else {
        format!("{}/{}", dir, filename)
    }
}

/// Write each ledger key as an individual file under `dir` on the remote host.
///
/// Each file is created via tmp-write + chmod 0400 + atomic rename, all in a single
/// SSH invocation per key. The directory is created with mode 0700 if it doesn't exist.
/// File ownership matches the SSH login user (root for badclaw); LoadCredential and
/// Compose secrets both consume root-owned 0400 files cleanly.
fn sync_perkey_files_to_machine(host: &str, dir: &str, keys: &[LedgerEntry]) -> Result<usize> {
    let dir_q = shell_escape(dir);
    let mkdir_cmd = format!(
        "umask 077 && mkdir -p {dir} && chmod 0700 {dir}",
        dir = dir_q,
    );

    let output = Command::new("ssh")
        .args([host, &mkdir_cmd])
        .output()
        .with_context(|| format!("ssh mkdir on {}", host))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("mkdir failed on {}: {}", host, stderr.trim());
    }

    let mut written = 0;
    for key in keys {
        write_perkey_file(host, dir, &key.env_var, &key.value)
            .with_context(|| format!("writing key {}", key.env_var))?;
        written += 1;
    }
    Ok(written)
}

fn write_perkey_file(host: &str, dir: &str, env_var: &str, value: &str) -> Result<()> {
    let target = join_remote_path(dir, env_var);
    let tmp = join_remote_path(dir, &format!(".tmp.{}", env_var));

    let cmd = format!(
        "umask 077 && cat > {tmp} && chmod 0400 {tmp} && mv {tmp} {target}",
        tmp = shell_escape(&tmp),
        target = shell_escape(&target),
    );

    let mut child = Command::new("ssh")
        .args([host, &cmd])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("spawning ssh to {}", host))?;

    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(value.as_bytes())
            .context("writing key value to ssh stdin")?;
    }

    let result = child.wait_with_output().context("waiting for ssh")?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        anyhow::bail!("write failed: {}", stderr.trim());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_env_var() {
        assert_eq!(
            make_env_var("openrouter", "badclaw-cole"),
            "OPENROUTER_BADCLAW_COLE"
        );
        assert_eq!(make_env_var("openai", "my-key"), "OPENAI_MY_KEY");
        assert_eq!(make_env_var("xai", "test"), "XAI_TEST");
        assert_eq!(
            make_env_var("xai", "Brett Imagine API"),
            "XAI_BRETT_IMAGINE_API"
        );
        assert_eq!(
            make_env_var("openai", "key--with---dashes"),
            "OPENAI_KEY_WITH_DASHES"
        );
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

    #[test]
    fn test_shell_escape_safe_chars_passthrough() {
        // Plain paths and env var names need no escaping.
        assert_eq!(
            shell_escape("/opt/badclaw/secrets/credstore"),
            "/opt/badclaw/secrets/credstore"
        );
        assert_eq!(
            shell_escape("ANTHROPIC_KELLEN_GURU"),
            "ANTHROPIC_KELLEN_GURU"
        );
        assert_eq!(shell_escape("foo.bar-baz_qux"), "foo.bar-baz_qux");
    }

    #[test]
    fn test_shell_escape_quotes_unsafe_chars() {
        // Spaces, $, and other special chars trigger quoting.
        assert_eq!(shell_escape("with space"), "'with space'");
        assert_eq!(shell_escape("$EVIL"), "'$EVIL'");
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn test_shell_escape_handles_embedded_single_quotes() {
        // Single quotes inside a single-quoted string need '\'' decomposition.
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn test_join_remote_path() {
        assert_eq!(join_remote_path("/etc/secrets", "FOO"), "/etc/secrets/FOO");
        assert_eq!(join_remote_path("/etc/secrets/", "FOO"), "/etc/secrets/FOO");
        assert_eq!(join_remote_path("/", "FOO"), "/FOO");
    }
}
