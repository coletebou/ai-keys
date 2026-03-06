# ai-keys

[![CI](https://github.com/coletebou/ai-keys/actions/workflows/ci.yml/badge.svg)](https://github.com/coletebou/ai-keys/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/ai-keys.svg)](https://crates.io/crates/ai-keys)

**One CLI to manage API keys across every AI provider.**

Stop juggling five dashboards. `ai-keys` lets you create, list, revoke, and distribute API keys for OpenRouter, OpenAI, xAI, Google, and Anthropic -- all from the terminal.

## Features

- **Unified interface** -- one command set for all major AI providers
- **Create keys** with provider-specific options: usage limits, reset intervals, project scoping, ACLs
- **List and audit** keys across every configured provider in a single call
- **Revoke keys** instantly without opening a browser
- **SSH-based sync** -- distribute keys to remote machines automatically
- **Status dashboard** -- run `ai-keys` with no arguments for a health overview
- **Local ledger** -- created keys are tracked locally with env var mappings
- **Cross-platform** -- prebuilt binaries for Linux (x86_64, aarch64) and macOS (Intel, Apple Silicon)

## Supported Providers

| Provider | Create | List | Revoke | Auth Method |
|----------|--------|------|--------|-------------|
| OpenRouter | ✅ | ✅ | ✅ | Management API key |
| OpenAI | ✅ | ✅ | ✅ | Admin API key |
| xAI (Grok) | ✅ | ✅ | ✅ | Management key |
| Google (Gemini) | ✅ | ✅ | ✅ | `gcloud` CLI |
| Anthropic (Claude) | ❌ | ✅ | ✅ | Admin API key |

Anthropic does not expose a key-creation endpoint; `ai-keys` will point you to the console instead.

## Install

### From source (recommended for now)

```bash
cargo install --git https://github.com/coletebou/ai-keys
```

### Prebuilt binaries

Download the latest release from [GitHub Releases](https://github.com/coletebou/ai-keys/releases) for your platform:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

### From crates.io

```bash
cargo install ai-keys
```

## Quick Start

```bash
# Generate config file
ai-keys init

# Set management keys (init output shows URLs for each provider)
export OPENROUTER_MGMT_KEY="sk-or-mgmt-..."
export OPENAI_ADMIN_KEY="sk-admin-..."
export XAI_MGMT_KEY="xai-mgmt-..."
export ANTHROPIC_ADMIN_KEY="sk-ant-admin-..."
# Google uses gcloud CLI auth -- run: gcloud auth login

# Show status dashboard
ai-keys

# List all keys across providers
ai-keys list

# List keys for a single provider
ai-keys list openrouter

# Create a named key with a monthly usage limit
ai-keys create openrouter my-service --limit 10 --reset monthly

# Create a key scoped to an OpenAI project
ai-keys create openai staging-key --project proj_abc123

# Revoke a key by ID
ai-keys revoke openai key_abc123

# Distribute keys to configured machines
ai-keys sync
```

## Configuration

`ai-keys init` generates `~/.config/ai-keys/config.toml`:

```toml
[providers.openrouter]
management_key_env = "OPENROUTER_MGMT_KEY"

[providers.openai]
management_key_env = "OPENAI_ADMIN_KEY"

[providers.xai]
management_key_env = "XAI_MGMT_KEY"
team_id = "default"

[providers.google]
# No management key needed -- uses gcloud CLI auth
# project_id = "my-gcp-project"    # optional, overrides gcloud default

[providers.anthropic]
management_key_env = "ANTHROPIC_ADMIN_KEY"

# Machines for `ai-keys sync` to distribute keys to
[machines.prod-server]
host = "10.0.0.1"
secrets_file = "~/.env_secrets"      # default

[machines.dev-box]
host = "dev.example.com"
secrets_file = "/opt/app/.env"
```

## Sync

The `sync` command distributes all keys from the local ledger to remote machines over SSH.

When you create a key, `ai-keys` saves it to a local ledger (`~/.config/ai-keys/keys.json`) with an auto-generated environment variable name (e.g., `OPENROUTER_MY_SERVICE`). Running `ai-keys sync` then:

1. Reads the remote machine's secrets file via SSH
2. Updates or appends each key as an `export VAR="value"` line
3. Writes the file back

This keeps secrets files on all your machines in sync without manual copy-paste.

### Prerequisites

- SSH access to each machine (key-based auth recommended)
- At least one `[machines.*]` section in your config

## Provider Setup

See [docs/providers.md](docs/providers.md) for detailed per-provider setup instructions, including where to get management keys, required permissions, and provider-specific flags.

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT -- see [LICENSE](LICENSE) for details.
