mod config;
mod display;
mod providers;
mod sync;

use clap::{Parser, Subcommand};
use colored::Colorize;

use config::Config;
use providers::{get_provider, CreateOpts};
use sync::{make_env_var, Ledger};

const PROVIDER_NAMES: &[&str] = &["openrouter", "openai", "xai", "google", "anthropic"];

const CONSOLE_URLS: &[(&str, &str)] = &[
    ("openai", "https://platform.openai.com/api-keys"),
    ("xai", "https://console.x.ai/team/default/api-keys"),
    ("google", "https://console.cloud.google.com/apis/credentials"),
    ("anthropic", "https://console.anthropic.com/settings/keys"),
];

fn console_url(provider: &str) -> &'static str {
    CONSOLE_URLS
        .iter()
        .find(|(name, _)| *name == provider)
        .map(|(_, url)| *url)
        .unwrap_or("https://example.com")
}

#[derive(Parser)]
#[command(
    name = "ai-keys",
    about = "Manage API keys across AI providers from the terminal",
    version,
    arg_required_else_help = false
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// List API keys, optionally filtered by provider
    List {
        /// Provider to filter by (openrouter, openai, xai, google, anthropic)
        provider: Option<String>,
    },

    /// Create a new API key
    Create {
        /// Provider (openrouter, openai, xai, google, anthropic)
        provider: String,

        /// Name for the new key
        name: String,

        /// Record an externally-minted key value instead of calling the provider API.
        /// Required for providers like Anthropic where the Admin API cannot create keys.
        /// When set, --limit/--reset/--project/--acl/--restrict-api are ignored.
        #[arg(long)]
        value: Option<String>,

        /// Usage limit in USD (provider-specific)
        #[arg(long)]
        limit: Option<f64>,

        /// Reset interval for usage limits (e.g. monthly)
        #[arg(long)]
        reset: Option<String>,

        /// Project or workspace to associate the key with
        #[arg(long)]
        project: Option<String>,

        /// Access control list (comma-separated permissions)
        #[arg(long)]
        acl: Option<String>,

        /// Restrict to specific API endpoints (comma-separated)
        #[arg(long)]
        restrict_api: Option<String>,
    },

    /// Revoke an existing API key
    Revoke {
        /// Provider (openrouter, openai, xai, google, anthropic)
        provider: String,

        /// Key ID to revoke
        id: String,
    },

    /// Distribute API keys to remote machines via SSH
    Sync,

    /// Initialize configuration file
    Init,
}

async fn run_dashboard(config: &Config) {
    let mut statuses: Vec<(String, bool, Option<usize>, bool)> = Vec::new();

    for &name in PROVIDER_NAMES {
        let has_key = config
            .get_management_key(name)
            .ok()
            .flatten()
            .is_some();

        let (count, supports_create) = if has_key {
            match get_provider(name, config) {
                Ok(provider) => {
                    let count = provider.list().await.ok().map(|keys| keys.len());
                    (count, provider.supports_create())
                }
                Err(_) => (None, false),
            }
        } else {
            (None, false)
        };

        statuses.push((name.to_string(), has_key, count, supports_create));
    }

    display::print_status_dashboard(&statuses);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            let config = Config::load()?;
            run_dashboard(&config).await;
        }
        Some(Command::Init) => {
            Config::init()?;
        }
        Some(command) => {
            let config = Config::load()?;
            match command {
                Command::List { provider } => {
                    let names: Vec<&str> = match &provider {
                        Some(p) => vec![p.as_str()],
                        None => PROVIDER_NAMES.to_vec(),
                    };

                    for name in names {
                        match get_provider(name, &config) {
                            Ok(p) => match p.list().await {
                                Ok(keys) => display::print_key_list(name, &keys),
                                Err(e) => eprintln!(
                                    "{} {}: {}",
                                    "\u{2717}".red(),
                                    name,
                                    format!("{:#}", e).red()
                                ),
                            },
                            Err(e) => eprintln!(
                                "{} {}: {}",
                                "\u{2717}".red(),
                                name,
                                format!("{:#}", e).dimmed()
                            ),
                        }
                    }
                }
                Command::Create {
                    provider,
                    name,
                    value,
                    limit,
                    reset,
                    project,
                    acl,
                    restrict_api,
                } => {
                    let env_var = make_env_var(&provider, &name);

                    // Externally-minted key path: skip provider API, write straight to ledger.
                    // This is the only viable path for providers without programmatic create
                    // (e.g. Anthropic Admin API is list/revoke only).
                    if let Some(val) = value {
                        let mut ledger = Ledger::load()?;
                        ledger.add_key(&provider, &name, &env_var, &val);
                        ledger.save()?;
                        println!();
                        println!(
                            "  {} Recorded {} ({}) to ledger as {}. Run {} to distribute.",
                            "\u{2713}".green(),
                            name.bold(),
                            provider,
                            env_var.bold(),
                            "`ai-keys sync`".cyan()
                        );
                        println!();
                        return Ok(());
                    }

                    let p = get_provider(&provider, &config)?;

                    if !p.supports_create() {
                        display::print_create_unsupported(&provider, console_url(&provider));
                        return Ok(());
                    }

                    let opts = CreateOpts {
                        limit,
                        reset,
                        project,
                        acl,
                        restrict_api,
                    };

                    let created = p.create(&name, &opts).await?;
                    display::print_created_key(&provider, &created);

                    let mut ledger = Ledger::load()?;
                    ledger.add_key(&provider, &name, &env_var, &created.key);
                    ledger.save()?;
                    println!(
                        "  {} Key saved to ledger as {}. Run {} to distribute.",
                        "\u{2713}".green(),
                        env_var.bold(),
                        "`ai-keys sync`".cyan()
                    );
                    println!();
                }
                Command::Revoke { provider, id } => {
                    let p = get_provider(&provider, &config)?;
                    p.revoke(&id).await?;
                    display::print_revoked(&provider, &id);
                }
                Command::Sync => {
                    sync::sync_keys(&config).await?;
                }
                Command::Init => unreachable!(),
            }
        }
    }

    Ok(())
}
