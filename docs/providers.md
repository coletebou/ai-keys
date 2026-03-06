# Provider Setup Guide

Per-provider instructions for configuring `ai-keys`.

---

## OpenRouter

**Auth:** Management API key (separate from your inference key).

### Getting your management key

1. Go to [openrouter.ai/settings/management-keys](https://openrouter.ai/settings/management-keys)
2. Create a new management key
3. Export it:
   ```bash
   export OPENROUTER_MGMT_KEY="sk-or-mgmt-..."
   ```

### Commands

```bash
# List all keys
ai-keys list openrouter

# Create a key with a $50/month usage limit
ai-keys create openrouter prod-service --limit 50 --reset monthly

# Create a key with a one-time $10 limit (no reset)
ai-keys create openrouter experiment --limit 10

# Revoke by key hash
ai-keys revoke openrouter sk_hash_abc123
```

### Provider-specific flags

| Flag | Description |
|------|-------------|
| `--limit <USD>` | Maximum spend in USD |
| `--reset <interval>` | Reset interval for the limit (e.g., `monthly`) |

### Gotchas

- The management key is different from your regular API key. Regular keys cannot manage other keys.
- Key IDs in the list output are hashes, not the full key value. Use the hash when revoking.

---

## OpenAI

**Auth:** Admin API key (requires Organization Admin role).

### Getting your admin key

1. Go to [platform.openai.com/settings/organization/admin-keys](https://platform.openai.com/settings/organization/admin-keys)
2. Create a new admin key
3. Export it:
   ```bash
   export OPENAI_ADMIN_KEY="sk-admin-..."
   ```

### How it works

OpenAI does not have standalone API keys via the Admin API. Instead, `ai-keys` manages **service accounts** within projects:

- **List** enumerates all projects and their service accounts
- **Create** creates a new service account in a project, which returns an API key
- **Revoke** deletes a service account (and its associated key)

### Commands

```bash
# List all service accounts across projects
ai-keys list openai

# Create a service account in a specific project
ai-keys create openai my-service --project proj_abc123

# Revoke a service account
ai-keys revoke openai sa_id_123
```

### Provider-specific flags

| Flag | Description |
|------|-------------|
| `--project <id>` | Project ID to create the service account in (required for create) |

### Gotchas

- You need the **Admin** role in your organization, not just Member.
- The `--project` flag is required for `create`. Without it, the command will fail.
- The key value is only shown once at creation time. It is saved to the local ledger automatically.

---

## xAI (Grok)

**Auth:** Management API key.

### Getting your management key

1. Go to [console.x.ai/team/default/management-keys](https://console.x.ai/team/default/management-keys)
2. Create a new management key
3. Export it:
   ```bash
   export XAI_MGMT_KEY="xai-mgmt-..."
   ```

### Commands

```bash
# List all keys
ai-keys list xai

# Create a named key
ai-keys create xai my-agent

# Revoke by key ID
ai-keys revoke xai key_id_123
```

### Config options

If you have multiple teams, set the team ID in your config:

```toml
[providers.xai]
management_key_env = "XAI_MGMT_KEY"
team_id = "my-team-id"    # defaults to "default"
```

### Gotchas

- The management key is separate from regular API keys, similar to OpenRouter.
- Team ID defaults to `"default"` if not specified.

---

## Google (Gemini)

**Auth:** `gcloud` CLI (no API key needed in your environment).

### Prerequisites

1. Install the [Google Cloud SDK](https://cloud.google.com/sdk/docs/install)
2. Authenticate:
   ```bash
   gcloud auth login
   ```
3. (Optional) Set a default project:
   ```bash
   gcloud config set project my-project-id
   ```

### Commands

```bash
# List all API keys in the project
ai-keys list google

# Create a named key
ai-keys create google gemini-prod

# Revoke by key UID
ai-keys revoke google uid_abc123
```

### Config options

Override the default GCP project in your config:

```toml
[providers.google]
project_id = "my-gcp-project"
```

If `project_id` is not set, `ai-keys` uses your `gcloud` default project.

### Gotchas

- Requires `gcloud` to be installed and on your PATH.
- You must be authenticated (`gcloud auth login`) before running any commands.
- The Google provider shells out to `gcloud` rather than using an HTTP API directly.
- API key management requires the `apikeys.keys.*` IAM permissions on the project.

---

## Anthropic (Claude)

**Auth:** Admin API key.

**Note:** Anthropic does not support programmatic key creation. `ai-keys` supports listing and revoking only.

### Getting your admin key

1. Go to [console.anthropic.com/settings/admin-keys](https://console.anthropic.com/settings/admin-keys)
2. Create a new admin key
3. Export it:
   ```bash
   export ANTHROPIC_ADMIN_KEY="sk-ant-admin-..."
   ```

### Commands

```bash
# List all keys in the organization
ai-keys list anthropic

# Revoke (disable) a key by ID
ai-keys revoke anthropic key_id_123
```

### Gotchas

- **No create support.** The Anthropic API does not expose a key creation endpoint. If you try `ai-keys create anthropic ...`, you will be directed to the web console.
- Revoking a key sets its status to `inactive`. This is not reversible via `ai-keys`.
- The admin key is different from a regular API key. You need the Admin API key specifically, which can manage other keys in your organization.
