mod config;

use clap::{Parser, Subcommand};
use colored::Colorize;

use config::Config;

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

    /// Sync key metadata across providers
    Sync,

    /// Initialize configuration file
    Init,
}

fn print_status_dashboard() {
    println!("{}", "ai-keys status dashboard".bold());
    println!();
    println!(
        "{}",
        "No configuration found. Run `ai-keys init` to get started.".dimmed()
    );
    println!();
    println!("{}", "Supported providers:".bold());
    println!("  {} OpenRouter", "●".green());
    println!("  {} OpenAI", "●".green());
    println!("  {} xAI", "●".green());
    println!("  {} Google", "●".green());
    println!("  {} Anthropic", "●".green());
    println!();
    println!(
        "Run {} for usage information.",
        "ai-keys --help".cyan()
    );
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            print_status_dashboard();
        }
        Some(Command::Init) => {
            Config::init()?;
        }
        Some(command) => {
            let _config = Config::load()?;
            match command {
                Command::List { provider } => {
                    match provider {
                        Some(p) => println!("Listing keys for provider: {}", p.bold()),
                        None => println!("Listing keys for all providers..."),
                    }
                    println!("{}", "(not yet implemented)".dimmed());
                }
                Command::Create { provider, name, .. } => {
                    println!(
                        "Creating key '{}' for provider: {}",
                        name.bold(),
                        provider.bold()
                    );
                    println!("{}", "(not yet implemented)".dimmed());
                }
                Command::Revoke { provider, id } => {
                    println!(
                        "Revoking key '{}' for provider: {}",
                        id.bold(),
                        provider.bold()
                    );
                    println!("{}", "(not yet implemented)".dimmed());
                }
                Command::Sync => {
                    println!("Syncing key metadata across providers...");
                    println!("{}", "(not yet implemented)".dimmed());
                }
                Command::Init => unreachable!(),
            }
        }
    }

    Ok(())
}
