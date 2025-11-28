//! Package management commands including plugin support
//!
//! This module provides helper functions for pkg subcommands that
//! involve plugin management.

use crate::plugins::{
    CommandInfo, Compatibility, PluginEntry, PluginRegistry, PluginResolver, PluginScope,
    PluginSource, VerificationStatus, HORUS_VERSION,
};
use anyhow::{anyhow, Result};
use chrono::Utc;
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};

/// Detect if a package has CLI plugin capabilities
///
/// Returns Some(PluginMetadata) if the package provides CLI extensions
pub fn detect_plugin_metadata(package_dir: &Path) -> Option<PluginMetadata> {
    // Check for horus.yaml with plugin configuration
    let horus_yaml = package_dir.join("horus.yaml");
    if horus_yaml.exists() {
        if let Ok(content) = fs::read_to_string(&horus_yaml) {
            if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                if let Some(plugin) = yaml.get("plugin") {
                    return parse_plugin_yaml(plugin, &yaml, package_dir);
                }
            }
        }
    }

    // Check for Cargo.toml with [package.metadata.horus] plugin config
    let cargo_toml = package_dir.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            if let Ok(toml) = content.parse::<toml::Table>() {
                if let Some(metadata) = toml
                    .get("package")
                    .and_then(|p| p.get("metadata"))
                    .and_then(|m| m.get("horus"))
                {
                    return parse_plugin_toml(metadata, &toml, package_dir);
                }
            }
        }
    }

    // Check for bin directory with horus-* binaries
    let bin_dir = package_dir.join("bin");
    if bin_dir.exists() {
        if let Ok(entries) = fs::read_dir(&bin_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("horus-") && is_executable(&entry.path()) {
                        let command = name.strip_prefix("horus-").unwrap().to_string();
                        return Some(PluginMetadata {
                            command,
                            binary: entry.path(),
                            package_name: package_dir
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            version: detect_version(package_dir)
                                .unwrap_or_else(|| "0.0.0".to_string()),
                            commands: vec![],
                            compatibility: Compatibility::default(),
                            permissions: vec![],
                        });
                    }
                }
            }
        }
    }

    None
}

/// Plugin metadata extracted from package
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub command: String,
    pub binary: PathBuf,
    pub package_name: String,
    pub version: String,
    pub commands: Vec<CommandInfo>,
    pub compatibility: Compatibility,
    pub permissions: Vec<String>,
}

fn parse_plugin_yaml(
    plugin: &serde_yaml::Value,
    yaml: &serde_yaml::Value,
    package_dir: &Path,
) -> Option<PluginMetadata> {
    let command = plugin.get("command")?.as_str()?.to_string();
    let binary_rel = plugin.get("binary")?.as_str()?;
    let binary = package_dir.join(binary_rel);

    let package_name = yaml
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("unknown")
        .to_string();

    let version = yaml
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();

    let commands = plugin
        .get("subcommands")
        .and_then(|s| s.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| {
                    let name = item.get("name")?.as_str()?.to_string();
                    let description = item
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some(CommandInfo { name, description })
                })
                .collect()
        })
        .unwrap_or_default();

    let compatibility = plugin
        .get("compatibility")
        .map(|c| Compatibility {
            horus_min: c
                .get("horus")
                .and_then(|h| h.as_str())
                .and_then(|s| s.split(',').next())
                .map(|s| s.trim_start_matches(">=").trim().to_string())
                .unwrap_or_else(|| "0.1.0".to_string()),
            horus_max: c
                .get("horus")
                .and_then(|h| h.as_str())
                .and_then(|s| s.split(',').nth(1))
                .map(|s| s.trim_start_matches('<').trim().to_string())
                .unwrap_or_else(|| "2.0.0".to_string()),
            platforms: c
                .get("platforms")
                .and_then(|p| p.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .unwrap_or_default();

    let permissions = plugin
        .get("permissions")
        .and_then(|p| p.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Some(PluginMetadata {
        command,
        binary,
        package_name,
        version,
        commands,
        compatibility,
        permissions,
    })
}

fn parse_plugin_toml(
    metadata: &toml::Value,
    toml: &toml::Table,
    package_dir: &Path,
) -> Option<PluginMetadata> {
    let cli_extension = metadata.get("cli_extension")?.as_bool()?;
    if !cli_extension {
        return None;
    }

    let command = metadata.get("command_name")?.as_str()?.to_string();
    let default_binary = format!("horus-{}", command);
    let binary_name = metadata
        .get("binary")
        .and_then(|b| b.as_str())
        .unwrap_or(&default_binary);

    // Look for binary in various locations
    let binary = find_binary(package_dir, binary_name)?;

    let package_name = toml
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("unknown")
        .to_string();

    let version = toml
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();

    let commands = metadata
        .get("subcommands")
        .and_then(|s| s.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let name = item.get("name")?.as_str()?.to_string();
                    let description = item
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some(CommandInfo { name, description })
                })
                .collect()
        })
        .unwrap_or_default();

    Some(PluginMetadata {
        command,
        binary,
        package_name,
        version,
        commands,
        compatibility: Compatibility::default(),
        permissions: vec![],
    })
}

fn find_binary(package_dir: &Path, binary_name: &str) -> Option<PathBuf> {
    // Check various locations
    let candidates = [
        package_dir.join("bin").join(binary_name),
        package_dir.join("target/release").join(binary_name),
        package_dir.join("target/debug").join(binary_name),
        package_dir.join(binary_name),
    ];

    candidates
        .into_iter()
        .find(|candidate| candidate.exists() && is_executable(candidate))
}

fn detect_version(package_dir: &Path) -> Option<String> {
    // Try horus.yaml
    let horus_yaml = package_dir.join("horus.yaml");
    if horus_yaml.exists() {
        if let Ok(content) = fs::read_to_string(&horus_yaml) {
            if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                if let Some(version) = yaml.get("version").and_then(|v| v.as_str()) {
                    return Some(version.to_string());
                }
            }
        }
    }

    // Try Cargo.toml
    let cargo_toml = package_dir.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            if let Ok(toml) = content.parse::<toml::Table>() {
                if let Some(version) = toml
                    .get("package")
                    .and_then(|p| p.get("version"))
                    .and_then(|v| v.as_str())
                {
                    return Some(version.to_string());
                }
            }
        }
    }

    // Try metadata.json
    let metadata_json = package_dir.join("metadata.json");
    if metadata_json.exists() {
        if let Ok(content) = fs::read_to_string(&metadata_json) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(version) = json.get("version").and_then(|v| v.as_str()) {
                    return Some(version.to_string());
                }
            }
        }
    }

    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = path.metadata() {
        metadata.permissions().mode() & 0o111 != 0
    } else {
        false
    }
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.exists()
}

/// Register a plugin after package installation
pub fn register_plugin_after_install(
    package_dir: &Path,
    source: PluginSource,
    is_global: bool,
    project_root: Option<&Path>,
) -> Result<Option<String>> {
    let metadata = match detect_plugin_metadata(package_dir) {
        Some(m) => m,
        None => return Ok(None), // Not a plugin package
    };

    // Create plugin entry
    let checksum = PluginRegistry::calculate_checksum(&metadata.binary)?;

    // Clone commands for later display
    let commands_for_display = metadata.commands.clone();

    let entry = PluginEntry {
        package: metadata.package_name.clone(),
        version: metadata.version.clone(),
        source,
        binary: metadata.binary.clone(),
        checksum,
        signature: None,
        installed_at: Utc::now(),
        installed_by: HORUS_VERSION.to_string(),
        compatibility: metadata.compatibility,
        commands: metadata.commands,
        permissions: metadata.permissions,
    };

    // Create symlink in bin directory
    let bin_dir = if is_global {
        PluginRegistry::global_bin_dir()?
    } else {
        project_root
            .map(PluginRegistry::project_bin_dir)
            .ok_or_else(|| anyhow!("No project root for local plugin"))?
    };

    fs::create_dir_all(&bin_dir)?;
    let symlink_path = bin_dir.join(format!("horus-{}", metadata.command));

    // Remove existing symlink
    if symlink_path.exists() || symlink_path.symlink_metadata().is_ok() {
        fs::remove_file(&symlink_path).ok();
    }

    // Create new symlink
    #[cfg(unix)]
    std::os::unix::fs::symlink(&metadata.binary, &symlink_path)?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&metadata.binary, &symlink_path)?;

    // Update plugin registry
    let mut resolver = PluginResolver::new()?;

    if is_global {
        resolver
            .global_mut()
            .register_plugin(&metadata.command, entry);
        resolver.save_global()?;
    } else if let Some(root) = project_root {
        let project_name = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let project_registry = resolver.get_or_create_project(project_name);
        project_registry.register_plugin(&metadata.command, entry);

        let path = PluginRegistry::project_path(root);
        project_registry.save_to(&path)?;
    }

    println!(
        "   {} Registered CLI plugin: {}",
        "🔌".cyan(),
        format!("horus {}", metadata.command).green()
    );

    if !commands_for_display.is_empty() {
        println!("      Commands:");
        for cmd in &commands_for_display {
            println!("        • {} - {}", cmd.name, cmd.description.dimmed());
        }
    }

    Ok(Some(metadata.command))
}

/// Unregister a plugin when package is removed
pub fn unregister_plugin(
    command: &str,
    is_global: bool,
    project_root: Option<&Path>,
) -> Result<()> {
    let mut resolver = PluginResolver::new()?;

    // Remove from registry
    if is_global {
        if resolver.global_mut().unregister_plugin(command).is_some() {
            resolver.save_global()?;
        }
    } else if let Some(root) = project_root {
        if let Some(project) = resolver.project_mut() {
            if project.unregister_plugin(command).is_some() {
                let path = PluginRegistry::project_path(root);
                project.save_to(&path)?;
            }
        }
    }

    // Remove symlink from bin directory
    let bin_dir = if is_global {
        PluginRegistry::global_bin_dir()?
    } else {
        project_root
            .map(PluginRegistry::project_bin_dir)
            .ok_or_else(|| anyhow!("No project root"))?
    };

    let symlink_path = bin_dir.join(format!("horus-{}", command));
    if symlink_path.exists() || symlink_path.symlink_metadata().is_ok() {
        fs::remove_file(&symlink_path)?;
    }

    println!(
        "   {} Unregistered plugin: {}",
        "🔌".dimmed(),
        command.yellow()
    );

    Ok(())
}

/// Enable a disabled plugin
pub fn enable_plugin(command: &str) -> Result<()> {
    let mut resolver = PluginResolver::new()?;

    // Get project root before mutable borrow
    let project_root = resolver.project_root().map(|p| p.to_path_buf());

    // Try project first, then global
    if let Some(project) = resolver.project_mut() {
        if project.is_disabled(command) {
            project.enable_plugin(command)?;
            if let Some(root) = &project_root {
                let path = PluginRegistry::project_path(root);
                project.save_to(&path)?;
            }
            println!("{} Plugin '{}' enabled", "✓".green(), command.green());
            return Ok(());
        }
    }

    if resolver.global().is_disabled(command) {
        resolver.global_mut().enable_plugin(command)?;
        resolver.save_global()?;
        println!("{} Plugin '{}' enabled", "✓".green(), command.green());
        return Ok(());
    }

    Err(anyhow!("Plugin '{}' is not disabled", command))
}

/// Disable a plugin without uninstalling
pub fn disable_plugin(command: &str, reason: Option<&str>) -> Result<()> {
    let mut resolver = PluginResolver::new()?;
    let reason = reason.unwrap_or("Disabled by user");

    // Get project root before mutable borrow
    let project_root = resolver.project_root().map(|p| p.to_path_buf());

    // Try project first, then global
    if let Some(project) = resolver.project_mut() {
        if project.get_plugin(command).is_some() {
            project.disable_plugin(command, reason)?;
            if let Some(root) = &project_root {
                let path = PluginRegistry::project_path(root);
                project.save_to(&path)?;
            }
            println!(
                "{} Plugin '{}' disabled (package still installed)",
                "✓".yellow(),
                command.yellow()
            );
            return Ok(());
        }
    }

    if resolver.global().get_plugin(command).is_some() {
        resolver.global_mut().disable_plugin(command, reason)?;
        resolver.save_global()?;
        println!(
            "{} Plugin '{}' disabled (package still installed)",
            "✓".yellow(),
            command.yellow()
        );
        return Ok(());
    }

    Err(anyhow!("Plugin '{}' not found", command))
}

/// Verify plugin integrity
pub fn verify_plugins(plugin_name: Option<&str>) -> Result<()> {
    let resolver = PluginResolver::new()?;
    let results = resolver.verify_all();

    if results.is_empty() {
        println!("{} No plugins installed", "ℹ".cyan());
        return Ok(());
    }

    println!("{} Verifying plugins...\n", "🔍".cyan());

    let mut all_valid = true;

    for result in &results {
        // Filter by name if specified
        if let Some(name) = plugin_name {
            if result.command != name {
                continue;
            }
        }

        let scope_str = match result.scope {
            PluginScope::Global => "(global)".dimmed(),
            PluginScope::Project => "(project)".dimmed(),
        };

        match result.status {
            VerificationStatus::Valid => {
                println!(
                    "  {} {} {} checksum OK",
                    "✓".green(),
                    result.command.green(),
                    scope_str
                );
            }
            VerificationStatus::ChecksumMismatch => {
                all_valid = false;
                println!(
                    "  {} {} {} checksum MISMATCH",
                    "✗".red(),
                    result.command.red(),
                    scope_str
                );
            }
            VerificationStatus::Error => {
                all_valid = false;
                println!(
                    "  {} {} {} error: {}",
                    "✗".red(),
                    result.command.red(),
                    scope_str,
                    result.error.as_deref().unwrap_or("unknown error")
                );
            }
        }
    }

    println!();

    if all_valid {
        println!("{} All plugins verified successfully", "✓".green().bold());
        Ok(())
    } else {
        Err(anyhow!(
            "Some plugins failed verification. Run 'horus pkg install <package>' to reinstall."
        ))
    }
}

/// Restore plugins from lock file
pub fn restore_plugins(include_global: bool) -> Result<()> {
    let resolver = PluginResolver::new()?;

    println!("{} Restoring plugins from lock files...\n", "📦".cyan());

    let mut restored = 0;
    let mut failed = 0;

    // Restore project plugins
    if let Some(project) = resolver.project() {
        println!("{}Project plugins:", "".cyan());
        for (cmd, entry) in &project.plugins {
            if entry.binary.exists() {
                println!(
                    "  {} {} v{} - already present",
                    "✓".green(),
                    cmd,
                    entry.version
                );
            } else {
                println!(
                    "  {} {} v{} - binary missing, reinstall required",
                    "✗".yellow(),
                    cmd,
                    entry.version
                );
                failed += 1;
            }
            restored += 1;
        }
        if project.plugins.is_empty() {
            println!("  No project plugins");
        }
    }

    // Restore global plugins
    if include_global {
        println!("\n{}Global plugins:", "".cyan());
        for (cmd, entry) in &resolver.global().plugins {
            if entry.binary.exists() {
                println!(
                    "  {} {} v{} - already present",
                    "✓".green(),
                    cmd,
                    entry.version
                );
            } else {
                println!(
                    "  {} {} v{} - binary missing, reinstall required",
                    "✗".yellow(),
                    cmd,
                    entry.version
                );
                failed += 1;
            }
            restored += 1;
        }
        if resolver.global().plugins.is_empty() {
            println!("  No global plugins");
        }
    }

    println!();

    if failed > 0 {
        println!(
            "{} {} plugins need reinstallation. Run 'horus pkg install <package>' for each.",
            "⚠".yellow(),
            failed
        );
    } else if restored > 0 {
        println!(
            "{} All {} plugins are present",
            "✓".green().bold(),
            restored
        );
    }

    Ok(())
}

/// List installed plugins
pub fn list_plugins(show_global: bool, show_project: bool) -> Result<()> {
    let resolver = PluginResolver::new()?;

    let mut has_output = false;

    // Project plugins
    if show_project {
        if let Some(project) = resolver.project() {
            if !project.plugins.is_empty() {
                has_output = true;
                println!("{} Project plugins:\n", "🔌".cyan());
                for (cmd, entry) in &project.plugins {
                    let status = if entry.binary.exists() {
                        "✓".green()
                    } else {
                        "✗".red()
                    };
                    println!(
                        "  {} {}  {} v{}",
                        status,
                        cmd.green(),
                        entry.package.dimmed(),
                        entry.version.dimmed()
                    );

                    if !entry.commands.is_empty() {
                        for subcmd in &entry.commands {
                            println!(
                                "      {} {} - {}",
                                "•".dimmed(),
                                subcmd.name,
                                subcmd.description.dimmed()
                            );
                        }
                    }
                }
            }

            // Show disabled
            if !project.disabled.is_empty() {
                println!("\n  {} Disabled:", "⊘".dimmed());
                for (cmd, info) in &project.disabled {
                    println!(
                        "    {} {} - {}",
                        cmd.dimmed(),
                        info.plugin.version.dimmed(),
                        info.reason.dimmed()
                    );
                }
            }
        }
    }

    // Global plugins
    if show_global {
        if !resolver.global().plugins.is_empty() {
            if has_output {
                println!();
            }
            has_output = true;
            println!("{} Global plugins:\n", "🔌".cyan());
            for (cmd, entry) in &resolver.global().plugins {
                let status = if entry.binary.exists() {
                    "✓".green()
                } else {
                    "✗".red()
                };
                println!(
                    "  {} {}  {} v{}",
                    status,
                    cmd.green(),
                    entry.package.dimmed(),
                    entry.version.dimmed()
                );

                if !entry.commands.is_empty() {
                    for subcmd in &entry.commands {
                        println!(
                            "      {} {} - {}",
                            "•".dimmed(),
                            subcmd.name,
                            subcmd.description.dimmed()
                        );
                    }
                }
            }
        }

        // Show disabled
        if !resolver.global().disabled.is_empty() {
            println!("\n  {} Disabled:", "⊘".dimmed());
            for (cmd, info) in &resolver.global().disabled {
                println!(
                    "    {} {} - {}",
                    cmd.dimmed(),
                    info.plugin.version.dimmed(),
                    info.reason.dimmed()
                );
            }
        }
    }

    if !has_output {
        println!("{} No plugins installed", "ℹ".cyan());
        println!(
            "\n  Install plugins with: {}",
            "horus pkg install <package>".cyan()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_plugin_metadata_none() {
        let temp_dir = TempDir::new().unwrap();
        let result = detect_plugin_metadata(temp_dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_version_from_horus_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_path = temp_dir.path().join("horus.yaml");
        fs::write(&yaml_path, "name: test\nversion: 1.2.3\n").unwrap();

        let version = detect_version(temp_dir.path());
        assert_eq!(version, Some("1.2.3".to_string()));
    }
}
