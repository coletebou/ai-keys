use colored::Colorize;

use crate::providers::{CreatedKey, KeyInfo, KeyStatus};

const DIVIDER_WIDTH: usize = 58;

fn divider() -> String {
    "\u{2500}".repeat(DIVIDER_WIDTH)
}

fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Dashboard showing all provider statuses.
/// Each tuple: (provider_name, has_management_key, key_count_or_none, supports_create)
pub fn print_status_dashboard(statuses: &[(String, bool, Option<usize>, bool)]) {
    println!();
    println!("{}", "AI Keys Status".bold());
    println!("{}", divider().dimmed());

    for (name, has_key, count, supports_create) in statuses {
        let display_name = title_case(name);
        let check = if *has_key {
            "\u{2713}".green().to_string()
        } else {
            "\u{2717}".red().to_string()
        };

        let key_label = if name == "google" {
            "gcloud auth"
        } else {
            "management key"
        };

        let status_text = if *has_key {
            format!("{} {}", check, key_label)
        } else {
            format!("{} no {}", check, key_label)
        };

        let count_text = match count {
            Some(n) if *has_key => {
                let suffix = if !supports_create {
                    " (list/revoke only)"
                } else {
                    ""
                };
                format!("{:>3} keys active{}", n, suffix)
            }
            _ => String::new(),
        };

        println!(
            "  {:<14} {:<24} {}",
            display_name.bold(),
            status_text,
            count_text
        );
    }

    println!();
}

/// Table of keys for a single provider.
pub fn print_key_list(provider: &str, keys: &[KeyInfo]) {
    let display_name = title_case(provider);

    println!();
    println!("{}", format!("{} Keys", display_name).bold());
    println!("{}", divider().dimmed());

    if keys.is_empty() {
        println!("  {}", "No keys found.".dimmed());
        println!();
        return;
    }

    println!(
        "  {:<18}{:<10}{:<12}{:<12}{}",
        "NAME".dimmed(),
        "STATUS".dimmed(),
        "LIMIT".dimmed(),
        "USED".dimmed(),
        "CREATED".dimmed(),
    );

    for key in keys {
        let status_str = match key.status {
            KeyStatus::Active => "active".green().to_string(),
            KeyStatus::Disabled => "disabled".red().to_string(),
            KeyStatus::Unknown => "unknown".yellow().to_string(),
        };

        let limit = key.limit.as_deref().unwrap_or("-");
        let used = key.used.as_deref().unwrap_or("-");
        let created = key.created.as_deref().unwrap_or("-");

        // Truncate name to fit column
        let name = if key.name.len() > 16 {
            format!("{}...", &key.name[..13])
        } else {
            key.name.clone()
        };

        println!(
            "  {:<18}{:<10}{:<12}{:<12}{}",
            name, status_str, limit, used, created,
        );
    }

    println!();
}

/// Display a newly created key with copy warning.
pub fn print_created_key(provider: &str, created: &CreatedKey) {
    println!();
    println!(
        "{} Created {} key {:?}",
        "\u{2713}".green(),
        title_case(provider),
        created.name,
    );
    println!();
    println!("  {}    {}", "Key:".dimmed(), created.key.bold());

    for (label, value) in &created.details {
        println!("  {}  {}", format!("{}:", label).dimmed(), value);
    }

    println!();
    println!(
        "  {}",
        "\u{26a0} Copy this key now \u{2014} it won't be shown again.".yellow()
    );
    println!(
        "  Run {} to distribute to your machines.",
        "`ai-keys sync`".cyan()
    );
    println!();
}

/// Message when a provider doesn't support programmatic key creation.
pub fn print_create_unsupported(provider: &str, url: &str) {
    let display_name = title_case(provider);

    println!();
    println!(
        "{} {} does not support programmatic key creation.",
        "\u{2717}".red(),
        display_name,
    );
    println!();
    println!("  Create keys manually:");
    println!("  {} {}", "\u{2192}".bold(), url.cyan());
    println!();
    println!(
        "  Once created, {} will show it.",
        format!("`ai-keys list {}`", provider).cyan()
    );
    println!();
}

/// Confirmation that a key was revoked.
pub fn print_revoked(provider: &str, id: &str) {
    println!();
    println!(
        "{} Revoked {} key {}",
        "\u{2713}".green(),
        title_case(provider),
        id.bold(),
    );
    println!();
}
