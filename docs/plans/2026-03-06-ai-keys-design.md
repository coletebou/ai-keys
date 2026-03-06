# ai-keys Design Document

> Standalone CLI for managing API keys across multiple AI providers.

## Problem

Managing API keys across 5+ AI providers (Anthropic, OpenAI, Google, xAI, OpenRouter) is manual and tedious. Keys are created one-by-one in web dashboards, shared across services that should have independent keys, and syncing keys to multiple machines requires manual SSH and file editing.

## Goals

- Create, list, and revoke API keys across providers from the terminal
- Per-service key isolation (e.g., separate Anthropic keys for Claude Code, AICLI, Badland API, each badclaw tenant)
- Sync keys to multiple machines via SSH
- Provider-native API calls for full feature access (spending limits, ACLs, API restrictions)
- OSS-ready from day one (MIT license, CI, prebuilt binaries)

## Non-Goals

- Not a secrets manager (use Bitwarden, 1Password, etc. for that)
- Not a gateway or proxy (use OpenRouter for that)
- Does not make inference API calls

---

## CLI Interface

```
ai-keys                          # Status dashboard (provider health, key counts)
ai-keys list [provider]          # List keys from admin APIs
ai-keys create <provider> <name> # Create a named key
ai-keys revoke <provider> <id>   # Deactivate/delete a key
ai-keys sync                     # Push keys to machines via SSH
ai-keys init                     # Generate default config
```

### Provider Names

`openrouter`, `openai`, `xai`, `google`, `anthropic`

### Provider-Specific Flags for `create`

| Provider | Flags |
|----------|-------|
| OpenRouter | `--limit <credits>`, `--reset daily\|weekly\|monthly` |
| OpenAI | `--project <name>` |
| xAI | `--acl <permissions>` |
| Google | `--restrict-api <service>` |
| Anthropic | N/A (create not supported, prints Console URL) |

---

## Config

Location: `~/.config/ai-keys/config.toml`

```toml
[providers.openrouter]
management_key_env = "OPENROUTER_MGMT_KEY"

[providers.openai]
management_key_env = "OPENAI_ADMIN_KEY"

[providers.xai]
management_key_env = "XAI_MGMT_KEY"
team_id = "default"

[providers.google]
project_id = "my-gcp-project"

[providers.anthropic]
management_key_env = "ANTHROPIC_ADMIN_KEY"

[machines]
cole-pc = { host = "cole-pc", secrets_file = "~/.env_secrets" }
badclaw = { host = "badclaw", secrets_file = "~/.env_secrets" }
hz-prod = { host = "nebulatio-hz-prod", secrets_file = "~/.env_secrets" }
roc-xe102101 = { host = "roc-xe102101", secrets_file = "~/.env_secrets" }
```

Management key values live in `~/.env_secrets` (env vars). Config only stores env var names and provider metadata.

---

## Architecture

### Provider Trait

```rust
trait KeyProvider {
    async fn list(&self) -> Result<Vec<KeyInfo>>;
    async fn create(&self, name: &str, opts: &CreateOpts) -> Result<CreatedKey>;
    async fn revoke(&self, key_id: &str) -> Result<()>;
    fn supports_create(&self) -> bool;
}
```

### Provider API Endpoints

| Provider | Base URL | Auth |
|----------|----------|------|
| OpenRouter | `https://openrouter.ai/api/v1/keys` | Bearer management key |
| OpenAI | `https://api.openai.com/v1/organization/...` | Admin API key |
| xAI | `https://management-api.x.ai/auth/teams/{id}/api-keys` | Management key |
| Google | `gcloud services api-keys create` | gcloud CLI auth |
| Anthropic | `https://api.anthropic.com/v1/organizations/api_keys` | `sk-ant-admin-*` key |

### Sync Mechanism

`ai-keys sync` SSHs to each configured machine and appends/updates key entries in the target `secrets_file`. Uses `ssh <host>` with existing SSH config (Tailscale, etc.).

---

## Project Structure

```
ai-keys/
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── providers/
│   │   ├── mod.rs
│   │   ├── openrouter.rs
│   │   ├── openai.rs
│   │   ├── xai.rs
│   │   ├── google.rs
│   │   └── anthropic.rs
│   ├── sync.rs
│   └── display.rs
├── Cargo.toml
├── README.md
├── CONTRIBUTING.md
├── LICENSE (MIT)
├── CHANGELOG.md
├── .github/
│   ├── workflows/
│   │   ├── ci.yml
│   │   └── release.yml
│   └── ISSUE_TEMPLATE/
│       ├── bug_report.md
│       └── feature_request.md
└── docs/
    └── providers.md
```

### Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI parsing |
| `reqwest` | HTTP client for admin APIs |
| `serde` / `serde_json` | Serialization |
| `toml` | Config file |
| `tokio` | Async runtime |
| `colored` | Terminal output |
| `dirs` | Config path resolution |
| `anyhow` | Error handling |

### Release Pipeline

Tag `vX.Y.Z` triggers GitHub Actions:
- Build for `x86_64-linux`, `aarch64-linux`, `x86_64-apple-darwin`, `aarch64-apple-darwin`
- Attach binaries to GitHub Release
- Also installable via `cargo install ai-keys`

---

## Output Examples

### Status Dashboard (`ai-keys`)

```
AI Keys Status
──────────────────────────────────────────────────
  OpenRouter    ✓ management key    12 keys active
  OpenAI        ✓ management key     3 keys active
  xAI           ✓ management key     2 keys active
  Google        ✓ gcloud auth        5 keys active
  Anthropic     ✓ management key     4 keys active (list/revoke only)
```

### List Keys (`ai-keys list openrouter`)

```
OpenRouter Keys
──────────────────────────────────────────────────────────────
NAME              STATUS    LIMIT    USED     CREATED
badclaw-cole      active    $10/mo   $3.21    2026-03-01
badclaw-karl      active    $10/mo   $1.05    2026-03-01
badland-api       active    $50/mo   $12.40   2026-02-15
aicli             active    —        $0.80    2026-03-06
```

### Create Key (`ai-keys create openrouter badclaw-lux --limit 10 --reset monthly`)

```
✓ Created OpenRouter key "badclaw-lux"

  Key:    sk-or-v1-abc123...def456
  Limit:  $10.00/month (resets 1st of month UTC)

  ⚠ Copy this key now — it won't be shown again.
  Run `ai-keys sync` to distribute to your machines.
```

### Unsupported Create (`ai-keys create anthropic badland-api`)

```
✗ Anthropic does not support programmatic key creation.

  Create keys manually:
  → https://console.anthropic.com/settings/keys

  Tip: Use workspaces to organize keys by service.
  Once created, `ai-keys list anthropic` will show it.
```
