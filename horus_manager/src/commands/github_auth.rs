use anyhow::{anyhow, Result};
use colored::*;
use dirs::home_dir;
use horus_core::error::{HorusError, HorusResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct AuthConfig {
    api_key: String,
    registry_url: String,
    github_username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiKeyResponse {
    key: String,
    name: String,
    prefix: String,
    environment: Option<String>,
}

/// Get the path to the auth config file
fn auth_config_path() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    let config_dir = home.join(".horus");

    // Create .horus directory if it doesn't exist
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    Ok(config_dir.join("auth.json"))
}

/// Login to the HORUS registry with GitHub
pub fn login(github: bool) -> HorusResult<()> {
    let registry_url = get_registry_url();

    if github {
        // GitHub OAuth flow
        println!("{} Logging in to HORUS registry with GitHub...", "→".cyan());
        println!();
        println!(
            "{} Opening browser for GitHub authentication...",
            "→".cyan()
        );
        println!("  {} {}/auth/github", "URL:".dimmed(), registry_url);
        println!();

        // Open browser for GitHub OAuth
        let auth_url = format!("{}/auth/github", registry_url);
        if open::that(&auth_url).is_err() {
            println!("{} Could not open browser automatically.", "!".yellow());
            println!("Please visit: {}", auth_url.cyan());
        }

        println!();
        println!(
            "{} After authenticating with GitHub:",
            "Next steps:".green()
        );
        println!("  1. You'll be redirected back to the registry");
        println!("  2. Copy the displayed instructions");
        println!("  3. Run: {}", "horus auth generate-key".cyan());
        println!();
        println!("{} GitHub authentication initiated!", "✓".green());
    } else {
        println!("{} GitHub authentication is required.", "!".yellow());
        println!();
        println!("Run: {}", "horus auth login --github".cyan());
    }

    Ok(())
}

/// Generate API key after GitHub authentication
pub fn generate_key(name: Option<String>, environment: Option<String>) -> HorusResult<()> {
    println!("{} Generating API key...", "→".cyan());

    let registry_url = get_registry_url();
    let key_name = name.unwrap_or_else(|| {
        // Generate default name based on hostname
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        format!("{}-{}", hostname, chrono::Utc::now().timestamp())
    });

    println!();
    println!(
        "{} This requires you to be logged in via GitHub first.",
        "Note:".yellow()
    );
    println!(
        "  If you haven't logged in yet, run: {}",
        "horus auth login --github".cyan()
    );
    println!();

    // Prompt for manual key entry (since we need the GitHub session)
    println!("After logging in via GitHub, the registry will show an API key generation page.");
    println!("Visit: {}/dashboard/keys", registry_url.cyan());
    println!();
    println!("Generate a key with:");
    println!("  {} {}", "Name:".dimmed(), key_name);
    if let Some(env) = &environment {
        println!("  {} {}", "Environment:".dimmed(), env);
    }
    println!();

    print!("Enter the generated API key: ");
    io::stdout().flush().unwrap();

    let mut api_key = String::new();
    io::stdin()
        .read_line(&mut api_key)
        .map_err(|e| HorusError::Config(format!("Failed to read input: {}", e)))?;

    let api_key = api_key.trim().to_string();

    // Validate token format
    if !api_key.starts_with("horus_key_") {
        return Err(HorusError::Config(
            "Invalid token format. Token should start with 'horus_key_'".to_string(),
        ));
    }

    // Save auth config
    let config = AuthConfig {
        api_key: api_key.clone(),
        registry_url: registry_url.clone(),
        github_username: None, // Will be populated on first use
    };

    let config_path = auth_config_path().map_err(|e| HorusError::Config(e.to_string()))?;

    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| HorusError::Config(format!("Failed to serialize config: {}", e)))?;

    fs::write(&config_path, config_json)
        .map_err(|e| HorusError::Config(format!("Failed to save auth config: {}", e)))?;

    println!();
    println!("{} API key saved successfully!", "✓".green());
    println!("  {} {}", "Registry:".dimmed(), registry_url);
    println!(
        "  {} {}",
        "Config saved to:".dimmed(),
        config_path.display()
    );
    println!();
    println!(
        "{} You can now publish packages with: {}",
        "Tip:".yellow(),
        "horus publish".cyan()
    );

    Ok(())
}

/// Logout from the HORUS registry
pub fn logout() -> HorusResult<()> {
    println!("{} Logging out from HORUS registry...", "→".cyan());

    let config_path = auth_config_path().map_err(|e| HorusError::Config(e.to_string()))?;

    if config_path.exists() {
        fs::remove_file(&config_path)
            .map_err(|e| HorusError::Config(format!("Failed to remove auth config: {}", e)))?;

        println!("{} Successfully logged out!", "✓".green());
        println!("  {} API key removed from local storage", "•".dimmed());
    } else {
        println!("{} Not currently logged in", "!".yellow());
    }

    Ok(())
}

/// Show current authenticated user
pub fn whoami() -> HorusResult<()> {
    let config_path = auth_config_path().map_err(|e| HorusError::Config(e.to_string()))?;

    if !config_path.exists() {
        println!("{} Not logged in", "!".yellow());
        println!();
        println!("To authenticate:");
        println!("  1. Run: {}", "horus auth login --github".cyan());
        println!("  2. Then: {}", "horus auth generate-key".cyan());
        return Ok(());
    }

    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| HorusError::Config(format!("Failed to read auth config: {}", e)))?;

    let config: AuthConfig = serde_json::from_str(&config_content)
        .map_err(|e| HorusError::Config(format!("Failed to parse auth config: {}", e)))?;

    // Try to fetch user info from registry
    let registry_url = config.registry_url.clone();
    let client = reqwest::blocking::Client::new();

    match client
        .get(format!("{}/api/auth/whoami", registry_url))
        .header("Authorization", format!("Bearer {}", config.api_key))
        .send()
    {
        Ok(response) if response.status().is_success() => {
            if let Ok(user_info) = response.json::<serde_json::Value>() {
                println!("{} Current authentication:", "→".cyan());
                println!("  {} {}", "Registry:".dimmed(), registry_url);

                if let Some(username) = user_info["github_username"].as_str() {
                    println!("  {} @{}", "GitHub User:".dimmed(), username.green());
                }
                if let Some(email) = user_info["email"].as_str() {
                    println!("  {} {}", "Email:".dimmed(), email);
                }
                if let Some(packages) = user_info["packages_published"].as_u64() {
                    println!("  {} {}", "Packages Published:".dimmed(), packages);
                }

                if let Some(keys) = user_info["api_keys"].as_array() {
                    println!();
                    println!("  {} ({} active)", "API Keys:".dimmed(), keys.len());
                    for key in keys {
                        if let (Some(name), Some(prefix)) =
                            (key["name"].as_str(), key["prefix"].as_str())
                        {
                            println!("    • {} ({})", name, prefix.dimmed());
                        }
                    }
                }
            }
        }
        _ => {
            // Fallback to showing local config
            println!("{} Current authentication:", "→".cyan());
            println!("  {} {}", "Registry:".dimmed(), registry_url);

            // Show token prefix only
            let token_prefix = config.api_key.chars().take(15).collect::<String>() + "...";
            println!("  {} {}", "API Token:".dimmed(), token_prefix.green());
            println!();
            println!(
                "  {} Could not fetch user details from registry",
                "Note:".yellow()
            );
        }
    }

    println!();
    println!(
        "{} To manage API keys, visit: {}/dashboard/keys",
        "Tip:".yellow(),
        registry_url
    );

    Ok(())
}

/// Get the current auth token (used by other commands)
pub fn get_auth_token() -> Option<String> {
    // First check environment variable
    if let Ok(token) = std::env::var("HORUS_API_KEY") {
        return Some(token);
    }

    // Then check config file
    if let Ok(config_path) = auth_config_path() {
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<AuthConfig>(&content) {
                    return Some(config.api_key);
                }
            }
        }
    }

    None
}

/// Get the registry URL
pub fn get_registry_url() -> String {
    // First check environment variable
    if let Ok(url) = std::env::var("HORUS_REGISTRY_URL") {
        return url;
    }

    // Then check config file
    if let Ok(config_path) = auth_config_path() {
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<AuthConfig>(&content) {
                    return config.registry_url;
                }
            }
        }
    }

    // Default
    "http://localhost:3001".to_string()
}
