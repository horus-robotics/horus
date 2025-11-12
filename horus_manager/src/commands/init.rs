//! HORUS initialization command
//!
//! Handles workspace initialization and certificate setup

use anyhow::{Context, Result};
use colored::*;

/// Run the init command
pub fn run_init(
    setup_certs: bool,
    regenerate_certs: bool,
    workspace_name: Option<String>,
) -> Result<()> {
    // If setup-certs or regenerate-certs flag is provided, handle certificate setup
    if setup_certs || regenerate_certs {
        return setup_certificates(regenerate_certs);
    }

    // Otherwise, initialize workspace
    initialize_workspace(workspace_name)
}

/// Initialize a HORUS workspace in the current directory
fn initialize_workspace(workspace_name: Option<String>) -> Result<()> {
    println!("{}", "Initializing HORUS workspace".cyan().bold());
    println!();

    // Register workspace using existing workspace module
    crate::workspace::register_current_workspace(workspace_name)?;

    println!();
    println!("{}", "Workspace initialized successfully!".green().bold());
    println!();
    println!("Next steps:");
    println!("  1. Create a new project: {}", "horus new my_robot".yellow());
    println!(
        "  2. Install packages:     {}",
        "horus pkg install <package>".yellow()
    );
    println!(
        "  3. Start dashboard:      {}",
        "horus dashboard".yellow()
    );
    println!();

    // Check if mkcert is available and suggest certificate setup
    if !crate::security::TlsConfig::is_mkcert_installed()
        || !crate::security::TlsConfig::is_mkcert_ca_installed()
    {
        println!("{}", "Tip:".cyan().bold());
        println!(
            "   For trusted HTTPS in dashboard (no browser warnings), run:"
        );
        println!("   {}", "horus init --setup-certs".yellow());
        println!();
    }

    Ok(())
}

/// Setup or regenerate TLS certificates
fn setup_certificates(regenerate: bool) -> Result<()> {
    let tls_config = crate::security::TlsConfig::default_paths()
        .context("Failed to get certificate paths")?;

    if regenerate {
        println!(
            "{}",
            "Regenerating TLS certificates".cyan().bold()
        );
        println!();

        // Remove existing certificates
        if tls_config.certificates_exist() {
            println!("   Removing existing certificates...");
            let _ = std::fs::remove_file(&tls_config.cert_path);
            let _ = std::fs::remove_file(&tls_config.key_path);
            println!("   Removed old certificates");
            println!();
        }
    }

    // Setup certificates with mkcert
    tls_config.setup_trusted_certificates()?;

    Ok(())
}
