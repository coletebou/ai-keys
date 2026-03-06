# ai-keys Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a standalone Rust CLI (`ai-keys`) that manages API keys across AI providers (OpenRouter, OpenAI, xAI, Google, Anthropic) via their native admin APIs.

**Architecture:** Provider trait with per-provider implementations. Config in TOML. HTTP via reqwest, Google via gcloud subprocess. SSH-based key sync to remote machines. OSS repo with CI/CD from day one.

**Tech Stack:** Rust, clap 4, reqwest, serde/serde_json, toml, tokio, colored, dirs, anyhow

---

### Task 1: Project Scaffolding & OSS Files

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `LICENSE`
- Create: `README.md`
- Create: `CONTRIBUTING.md`
- Create: `CHANGELOG.md`
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`
- Create: `.github/ISSUE_TEMPLATE/bug_report.md`
- Create: `.github/ISSUE_TEMPLATE/feature_request.md`
- Create: `.gitignore`

**Step 1: Initialize git repo**

```bash
cd ~/code/ai-keys
git init
```

**Step 2: Create Cargo.toml**

```toml
[package]
name = "ai-keys"
version = "0.1.0"
edition = "2021"
description = "Manage API keys across AI providers from the terminal"
authors = ["Cole Tebou"]
license = "MIT"
repository = "https://github.com/coletebou/ai-keys"
readme = "README.md"
keywords = ["api-keys", "ai", "cli", "openai", "anthropic"]
categories = ["command-line-utilities"]

[[bin]]
name = "ai-keys"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
tokio = { version = "1", features = ["full"] }
colored = "2"
dirs = "5"
anyhow = "1"
```

**Step 3: Create minimal main.rs that parses args and prints help**

```rust
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ai-keys")]
#[command(version)]
#[command(about = "Manage API keys across AI providers from the terminal")]
#[command(author = "Cole Tebou")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// List keys from provider admin APIs
    List {
        /// Provider to list keys for (omit for all)
        provider: Option<String>,
    },
    /// Create a named API key
    Create {
        /// Provider (openrouter, openai, xai, google, anthropic)
        provider: String,
        /// Name for the new key
        name: String,
        /// Spending limit in credits (OpenRouter)
        #[arg(long)]
        limit: Option<f64>,
        /// Reset interval: daily, weekly, monthly (OpenRouter)
        #[arg(long)]
        reset: Option<String>,
        /// Project name (OpenAI)
        #[arg(long)]
        project: Option<String>,
        /// Access control list (xAI)
        #[arg(long)]
        acl: Option<String>,
        /// Restrict to specific API (Google)
        #[arg(long)]
        restrict_api: Option<String>,
    },
    /// Revoke/deactivate an API key
    Revoke {
        /// Provider
        provider: String,
        /// Key ID or hash to revoke
        id: String,
    },
    /// Sync keys to configured machines via SSH
    Sync,
    /// Generate default config file
    Init,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::List { provider }) => {
            println!("list {:?}", provider);
        }
        Some(Command::Create { provider, name, .. }) => {
            println!("create {} {}", provider, name);
        }
        Some(Command::Revoke { provider, id }) => {
            println!("revoke {} {}", provider, id);
        }
        Some(Command::Sync) => {
            println!("sync");
        }
        Some(Command::Init) => {
            println!("init");
        }
        None => {
            // Status dashboard (default when no subcommand)
            println!("status dashboard");
        }
    }

    Ok(())
}
```

**Step 4: Create LICENSE (MIT)**

Full MIT license text with `Copyright (c) 2026 Cole Tebou`.

**Step 5: Create README.md**

Include: badges (CI, crates.io, license), one-liner description, feature list, install instructions (`cargo install ai-keys` + prebuilt binaries), quickstart, supported providers table, config example, contributing link.

**Step 6: Create CONTRIBUTING.md**

Standard: fork, branch, PR, run `cargo test` before submitting, code of conduct reference.

**Step 7: Create CHANGELOG.md**

```markdown
# Changelog

## [Unreleased]

### Added
- Initial project scaffolding
- CLI argument parsing
```

**Step 8: Create .gitignore**

```
/target
*.swp
.env
.env_secrets
```

**Step 9: Create CI workflow `.github/workflows/ci.yml`**

Triggers on PR. Jobs: `cargo fmt --check`, `cargo clippy`, `cargo test`, across ubuntu-latest and macos-latest.

**Step 10: Create release workflow `.github/workflows/release.yml`**

Triggers on tag `v*`. Matrix build for 4 targets (x86_64-linux, aarch64-linux, x86_64-apple-darwin, aarch64-apple-darwin). Creates GitHub Release, attaches binaries.

**Step 11: Create issue templates**

Bug report (steps to reproduce, expected, actual, OS/version) and feature request (description, use case, alternatives considered).

**Step 12: Build and verify**

```bash
cargo build
cargo run -- --help
```

Expected: Help text with all subcommands listed.

**Step 13: Commit**

```bash
git add -A
git commit -m "feat: initial project scaffolding with CLI, OSS files, CI/CD"
```

---

### Task 2: Config System

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs`
- Test: `src/config.rs` (inline tests)

**Step 1: Write failing test for config loading**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_all_providers() {
        let config = Config::default();
        assert!(config.providers.contains_key("openrouter"));
        assert!(config.providers.contains_key("openai"));
        assert!(config.providers.contains_key("xai"));
        assert!(config.providers.contains_key("google"));
        assert!(config.providers.contains_key("anthropic"));
    }

    #[test]
    fn test_parse_config_toml() {
        let toml_str = r#"
[providers.openrouter]
management_key_env = "OPENROUTER_MGMT_KEY"

[machines]
myhost = { host = "myhost.local", secrets_file = "~/.env_secrets" }
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.providers["openrouter"].management_key_env.as_deref(),
            Some("OPENROUTER_MGMT_KEY")
        );
        assert_eq!(config.machines["myhost"].host, "myhost.local");
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test
```

Expected: FAIL — `Config` not defined.

**Step 3: Implement config.rs**

```rust
use anyhow::{Context, Result};
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
    pub team_id: Option<String>,       // xAI
    pub project_id: Option<String>,    // Google
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
        providers.insert("openrouter".into(), ProviderConfig {
            management_key_env: Some("OPENROUTER_MGMT_KEY".into()),
            team_id: None,
            project_id: None,
        });
        providers.insert("openai".into(), ProviderConfig {
            management_key_env: Some("OPENAI_ADMIN_KEY".into()),
            team_id: None,
            project_id: None,
        });
        providers.insert("xai".into(), ProviderConfig {
            management_key_env: Some("XAI_MGMT_KEY".into()),
            team_id: Some("default".into()),
            project_id: None,
        });
        providers.insert("google".into(), ProviderConfig {
            management_key_env: None,
            team_id: None,
            project_id: None,
        });
        providers.insert("anthropic".into(), ProviderConfig {
            management_key_env: Some("ANTHROPIC_ADMIN_KEY".into()),
            team_id: None,
            project_id: None,
        });

        Config {
            providers,
            machines: HashMap::new(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ai-keys")
            .join("config.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {:?}", path))?;
            let config: Config = toml::from_str(&content)
                .with_context(|| "Failed to parse config")?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn get_management_key(&self, provider: &str) -> Option<String> {
        self.providers.get(provider)?.management_key_env.as_ref()
            .and_then(|env_var| std::env::var(env_var).ok())
    }
}
```

**Step 4: Wire config into main.rs** — add `mod config;`, use in `Init` command.

**Step 5: Run tests**

```bash
cargo test
```

Expected: PASS

**Step 6: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat: add config system with provider and machine settings"
```

---

### Task 3: Provider Trait & Types

**Files:**
- Create: `src/providers/mod.rs`
- Modify: `src/main.rs`

**Step 1: Define the provider trait and shared types**

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};

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

impl std::fmt::Display for KeyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

#[async_trait::async_trait]
pub trait KeyProvider: Send + Sync {
    fn name(&self) -> &str;
    fn supports_create(&self) -> bool;
    async fn list(&self) -> Result<Vec<KeyInfo>>;
    async fn create(&self, name: &str, opts: &CreateOpts) -> Result<CreatedKey>;
    async fn revoke(&self, key_id: &str) -> Result<()>;
}
```

Note: Add `async-trait = "0.1"` to Cargo.toml dependencies.

**Step 2: Add `mod providers;` to main.rs**

**Step 3: Build and verify**

```bash
cargo build
```

Expected: compiles with no errors.

**Step 4: Commit**

```bash
git add src/providers/mod.rs src/main.rs Cargo.toml
git commit -m "feat: add KeyProvider trait and shared types"
```

---

### Task 4: OpenRouter Provider

**Files:**
- Create: `src/providers/openrouter.rs`
- Modify: `src/providers/mod.rs`

**Step 1: Implement OpenRouter provider**

Endpoints:
- `GET https://openrouter.ai/api/v1/keys` — list keys
- `POST https://openrouter.ai/api/v1/keys` — create key (body: `name`, `limit`, `limit_reset`)
- `DELETE https://openrouter.ai/api/v1/keys/{key_hash}` — delete key
- `PATCH https://openrouter.ai/api/v1/keys/{key_hash}` — update (disable)

Auth: `Authorization: Bearer {management_key}`

Response shape for list:
```json
{
  "data": [
    {
      "hash": "abc123",
      "name": "my-key",
      "label": null,
      "disabled": false,
      "limit": 10.0,
      "limit_remaining": 7.5,
      "usage": 2.5,
      "created_at": "2026-03-01T00:00:00Z"
    }
  ]
}
```

Response shape for create:
```json
{
  "data": {
    "hash": "abc123",
    "key": "sk-or-v1-actual-key-here",
    "name": "my-key"
  }
}
```

Implement `OpenRouterProvider` struct with `management_key: String` field. Implement `KeyProvider` trait.

Map `disabled: true` to `KeyStatus::Disabled`, `false` to `KeyStatus::Active`.

Format `limit` as `$X.XX/mo` (or `/day`, `/wk` based on `limit_reset`).

Format `used` as `$X.XX` from `usage` field.

**Step 2: Register in mod.rs** — add `pub mod openrouter;` and a factory function:

```rust
pub fn get_provider(name: &str, config: &Config) -> Result<Box<dyn KeyProvider>> {
    let mgmt_key = config.get_management_key(name);
    match name {
        "openrouter" => {
            let key = mgmt_key.ok_or_else(|| anyhow::anyhow!(
                "No management key for OpenRouter. Set {} env var.",
                config.providers.get("openrouter")
                    .and_then(|p| p.management_key_env.as_deref())
                    .unwrap_or("OPENROUTER_MGMT_KEY")
            ))?;
            Ok(Box::new(openrouter::OpenRouterProvider::new(key)))
        }
        _ => anyhow::bail!("Unknown provider: {}", name),
    }
}
```

**Step 3: Build and verify**

```bash
cargo build
```

**Step 4: Commit**

```bash
git add src/providers/
git commit -m "feat: add OpenRouter provider (list, create, revoke)"
```

---

### Task 5: Display Module & Status Dashboard

**Files:**
- Create: `src/display.rs`
- Modify: `src/main.rs`

**Step 1: Implement display helpers**

Functions:
- `print_status_dashboard(statuses: &[(String, bool, usize)])` — the default `ai-keys` view
- `print_key_list(provider: &str, keys: &[KeyInfo])` — formatted table
- `print_created_key(created: &CreatedKey)` — success message with key value
- `print_create_unsupported(provider: &str, url: &str)` — console URL redirect
- `print_revoked(provider: &str, id: &str)` — confirmation message

Use `colored` crate for checkmarks, colors, bold headers.

**Step 2: Wire the display module into main.rs** — replace placeholder `println!` calls with real logic. For the default (no subcommand) case, iterate over all providers, try to get management key, call `list()` to get count, display dashboard.

**Step 3: Build and verify**

```bash
cargo build
cargo run
```

Expected: Status dashboard (may show errors for unconfigured providers — that's fine).

**Step 4: Commit**

```bash
git add src/display.rs src/main.rs
git commit -m "feat: add display module and wire up status dashboard"
```

---

### Task 6: Remaining Providers (OpenAI, xAI, Anthropic, Google)

**Files:**
- Create: `src/providers/openai.rs`
- Create: `src/providers/xai.rs`
- Create: `src/providers/anthropic.rs`
- Create: `src/providers/google.rs`
- Modify: `src/providers/mod.rs`

**Step 1: Implement OpenAI provider**

Endpoints:
- List projects: `GET https://api.openai.com/v1/organization/projects`
- Create service account: `POST https://api.openai.com/v1/organization/projects/{project_id}/service_accounts` (body: `{"name": "..."}`) — response includes unredacted API key
- List service accounts: `GET https://api.openai.com/v1/organization/projects/{project_id}/service_accounts`
- Delete service account: `DELETE https://api.openai.com/v1/organization/projects/{project_id}/service_accounts/{sa_id}`

Auth: `Authorization: Bearer {admin_key}`

For `list`, iterate all projects and collect service accounts. For `create`, if `--project` is provided use that, otherwise use default project.

`supports_create() -> true`

**Step 2: Implement xAI provider**

Endpoints:
- List keys: `GET https://management-api.x.ai/auth/teams/{team_id}/api-keys`
- Create key: `POST https://management-api.x.ai/auth/teams/{team_id}/api-keys` (body: `{"name": "...", "acls": [...]}`)
- Delete key: `DELETE https://management-api.x.ai/auth/teams/{team_id}/api-keys/{key_id}`

Auth: `Authorization: Bearer {management_key}`

Uses `team_id` from config (defaults to "default").

`supports_create() -> true`

**Step 3: Implement Anthropic provider**

Endpoints:
- List keys: `GET https://api.anthropic.com/v1/organizations/api_keys` with headers `x-api-key: {admin_key}`, `anthropic-version: 2023-06-01`
- Deactivate key: `POST https://api.anthropic.com/v1/organizations/api_keys/{key_id}` with body `{"status": "inactive"}`

No create endpoint.

`supports_create() -> false`

In `create()`, return error with helpful message pointing to Console URL.

**Step 4: Implement Google provider**

Uses `gcloud` CLI subprocess (no HTTP — Google's API key management is via `gcloud services api-keys`):
- List: `gcloud services api-keys list --format=json`
- Create: `gcloud services api-keys create --display-name="{name}" --format=json` + optional `--api-target=service={restrict_api}`
- Delete: `gcloud services api-keys delete {key_id} --quiet`

Requires `gcloud` to be installed and authenticated. Check with `which gcloud` first, error if not found.

`supports_create() -> true`

**Step 5: Register all providers in mod.rs factory function**

```rust
match name {
    "openrouter" => { /* existing */ }
    "openai" => Ok(Box::new(openai::OpenAiProvider::new(key))),
    "xai" => Ok(Box::new(xai::XaiProvider::new(key, team_id))),
    "anthropic" => Ok(Box::new(anthropic::AnthropicProvider::new(key))),
    "google" => Ok(Box::new(google::GoogleProvider::new(project_id))),
    _ => anyhow::bail!("Unknown provider: {}", name),
}
```

**Step 6: Build and verify**

```bash
cargo build
```

**Step 7: Commit**

```bash
git add src/providers/
git commit -m "feat: add OpenAI, xAI, Anthropic, Google providers"
```

---

### Task 7: Sync Command

**Files:**
- Create: `src/sync.rs`
- Modify: `src/main.rs`

**Step 1: Implement sync module**

`ai-keys sync` reads the `[machines]` section from config. For each machine:
1. SSH to `host`
2. Read existing `secrets_file` content
3. For each key that needs syncing (tracked in a local `~/.config/ai-keys/keys.json` ledger), update or append the env var line
4. Write back the updated file

The ledger (`keys.json`) stores: `{ "OPENROUTER_BADCLAW_COLE": "sk-or-v1-...", ... }` — keys created via `ai-keys create` are auto-added to the ledger. User can also manually add entries.

Sync logic per machine:
```bash
ssh {host} "cat {secrets_file}" > /tmp/ai-keys-sync-tmp
# Update lines matching KEY_NAME= or append new ones
ssh {host} "cat > {secrets_file}" < updated_content
```

Use `std::process::Command` for SSH calls.

Print per-machine status: `✓ cole-pc: 3 keys synced`, `✗ badclaw: SSH failed`.

**Step 2: Wire into main.rs**

**Step 3: Build and verify**

```bash
cargo build
cargo run -- sync
```

Expected: error about no machines configured (if no config) or SSH attempts.

**Step 4: Commit**

```bash
git add src/sync.rs src/main.rs
git commit -m "feat: add sync command for SSH key distribution"
```

---

### Task 8: Init Command & Polish

**Files:**
- Modify: `src/main.rs`
- Modify: `src/config.rs`

**Step 1: Implement `ai-keys init`**

Generate default config at `~/.config/ai-keys/config.toml`. If file exists, warn and exit (don't overwrite). Print the config path and next steps (set management key env vars).

**Step 2: Polish the main.rs routing**

Wire all subcommands to their real implementations. Handle errors gracefully with colored output. For `create`, pass through provider-specific flags via `CreateOpts`.

**Step 3: Build, test, verify end-to-end**

```bash
cargo test
cargo build --release
cargo run -- --help
cargo run -- init
cargo run -- list openrouter  # (needs real mgmt key to work)
```

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: add init command and wire all subcommands"
```

---

### Task 9: README, Docs & First Release

**Files:**
- Modify: `README.md` (full content)
- Create: `docs/providers.md`

**Step 1: Write comprehensive README**

Sections: Hero/badges, Install (cargo install, prebuilt binaries, from source), Quickstart (init, set env vars, list, create), Supported Providers table, Config reference, Sync setup, Contributing, License.

**Step 2: Write docs/providers.md**

Per-provider setup guide: where to get management keys, what permissions are needed, example commands, gotchas.

**Step 3: Create GitHub repo and push**

```bash
cd ~/code/ai-keys
gh repo create coletebou/ai-keys --public --description "Manage API keys across AI providers from the terminal" --source .
git push -u origin master
```

**Step 4: Tag and release**

```bash
git tag -a v0.1.0 -m "Initial release"
git push origin v0.1.0
```

Wait for GitHub Actions release workflow to build and attach binaries.

**Step 5: Verify release**

```bash
gh release view v0.1.0
```

Expected: Release with 4 binary attachments.

**Step 6: Install on all machines**

```bash
cargo install ai-keys  # or download from release
```
