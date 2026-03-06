# ai-keys

[![CI](https://github.com/coletebou/ai-keys/actions/workflows/ci.yml/badge.svg)](https://github.com/coletebou/ai-keys/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/ai-keys.svg)](https://crates.io/crates/ai-keys)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Manage API keys across AI providers from the terminal.**

Stop juggling dashboards. Create, list, revoke, and audit API keys for OpenRouter, OpenAI, xAI, Google, and Anthropic from a single CLI.

## Features

- **Unified interface** for all major AI providers
- **Create keys** with provider-specific options (usage limits, project scoping, ACLs)
- **List and audit** keys across all providers at once
- **Revoke keys** instantly from the terminal
- **Sync metadata** across providers
- **Status dashboard** showing key health at a glance
- **Cross-platform** binaries for Linux and macOS

## Supported Providers

| Provider | Create | List | Revoke |
|----------|--------|------|--------|
| OpenRouter | :white_check_mark: | :white_check_mark: | :white_check_mark: |
| OpenAI | :white_check_mark: | :white_check_mark: | :white_check_mark: |
| xAI | :white_check_mark: | :white_check_mark: | :white_check_mark: |
| Google | :white_check_mark: | :white_check_mark: | :white_check_mark: |
| Anthropic | :white_check_mark: | :white_check_mark: | :white_check_mark: |

## Install

### From crates.io

```bash
cargo install ai-keys
```

### Prebuilt binaries

Download the latest release from [GitHub Releases](https://github.com/coletebou/ai-keys/releases) for your platform:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

### From source

```bash
git clone https://github.com/coletebou/ai-keys.git
cd ai-keys
cargo install --path .
```

## Quick Start

```bash
# Initialize configuration
ai-keys init

# Set your provider API keys as environment variables
export OPENROUTER_API_KEY="sk-or-..."
export OPENAI_API_KEY="sk-..."
export XAI_API_KEY="xai-..."
export GOOGLE_API_KEY="..."
export ANTHROPIC_API_KEY="sk-ant-..."

# List all keys across providers
ai-keys list

# List keys for a specific provider
ai-keys list openrouter

# Create a new key with a usage limit
ai-keys create openrouter my-project-key --limit 50.00

# Create a key scoped to a project
ai-keys create openai dev-key --project proj_abc123

# Revoke a key
ai-keys revoke openai key_abc123

# Sync key metadata
ai-keys sync
```

## Configuration

ai-keys stores its configuration at `~/.config/ai-keys/config.toml`:

```toml
[providers.openrouter]
api_key_env = "OPENROUTER_API_KEY"

[providers.openai]
api_key_env = "OPENAI_API_KEY"

[providers.xai]
api_key_env = "XAI_API_KEY"

[providers.google]
api_key_env = "GOOGLE_API_KEY"

[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"
```

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT -- see [LICENSE](LICENSE) for details.
