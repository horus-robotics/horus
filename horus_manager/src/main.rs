use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::generate;
use colored::*;
use horus_core::error::{HorusError, HorusResult};
use std::fs;
use std::io;
use std::path::PathBuf;

// Use modules from the library instead of redeclaring them
use horus_manager::{commands, dashboard, dashboard_tui, registry, workspace};

#[derive(Parser)]
#[command(name = "horus")]
#[command(about = "HORUS - Hybrid Optimized Robotics Unified System")]
#[command(version = "0.1.0")]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new HORUS project
    New {
        /// Project name
        name: String,
        /// Output directory (optional, defaults to current directory)
        #[arg(short = 'o', long = "output")]
        path: Option<PathBuf>,
        /// Use Python
        #[arg(short = 'p', long = "python", conflicts_with_all = ["rust", "c"])]
        python: bool,
        /// Use Rust
        #[arg(short = 'r', long = "rust", conflicts_with_all = ["python", "c"])]
        rust: bool,
        /// Use C
        #[arg(short = 'c', long = "c", conflicts_with_all = ["python", "rust"])]
        c: bool,
        /// Use Rust with macros
        #[arg(short = 'm', long = "macro", conflicts_with_all = ["python", "c"])]
        use_macro: bool,
    },

    /// Run a HORUS project or file
    Run {
        /// File to run (optional, auto-detects if not specified)
        file: Option<PathBuf>,

        /// Build only, don't run
        #[arg(short = 'b', long = "build-only")]
        build_only: bool,

        /// Build in release mode
        #[arg(short = 'r', long = "release")]
        release: bool,

        /// Clean build (remove cache)
        #[arg(short = 'c', long = "clean")]
        clean: bool,

        /// Deploy to remote robot (hostname, IP, or robot ID)
        #[arg(short = 'R', long = "remote", value_name = "ROBOT")]
        remote: Option<String>,

        /// Additional arguments to pass to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Open the unified HORUS dashboard (web-based, auto-opens browser)
    Dashboard {
        /// Port for web dashboard (default: 3000)
        #[arg(value_name = "PORT", default_value = "3000")]
        port: u16,

        /// Use Terminal UI mode instead of web
        #[arg(short = 't', long = "tui")]
        tui: bool,
    },

    /// Package management
    Pkg {
        #[command(subcommand)]
        command: PkgCommands,
    },

    /// Environment management (freeze/restore)
    Env {
        #[command(subcommand)]
        command: EnvCommands,
    },

    /// Authentication commands
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },

    /// Generate shell completion scripts
    #[command(hide = true)]
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Show version information
    Version,
}

#[derive(Subcommand)]
enum PkgCommands {
    /// Install a package from registry
    Install {
        /// Package name to install
        package: String,
        /// Specific package version (optional)
        #[arg(short = 'v', long = "ver")]
        ver: Option<String>,
        /// Install to global cache (shared across projects)
        #[arg(short = 'g', long = "global")]
        global: bool,
        /// Target workspace/project name (if not in workspace)
        #[arg(short = 't', long = "target")]
        target: Option<String>,
        /// Install as development dependency
        #[arg(short = 'd', long = "dev")]
        dev: bool,
    },

    /// Remove an installed package
    Remove {
        /// Package name to remove
        package: String,
        /// Remove from global cache
        #[arg(short = 'g', long = "global")]
        global: bool,
        /// Target workspace/project name
        #[arg(short = 't', long = "target")]
        target: Option<String>,
    },

    /// List installed packages or search registry
    List {
        /// Search query (optional)
        query: Option<String>,
        /// List global cache packages
        #[arg(short = 'g', long = "global")]
        global: bool,
        /// List all (local + global)
        #[arg(short = 'a', long = "all")]
        all: bool,
    },

    /// Publish package to registry
    Publish {
        /// Also generate freeze file
        #[arg(long)]
        freeze: bool,
    },

    /// Unpublish a package from the registry
    Unpublish {
        /// Package name to unpublish
        package: String,
        /// Package version to unpublish (required)
        #[arg(short = 'v', long = "version", required = true)]
        version: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum EnvCommands {
    /// Freeze current environment to a manifest file
    Freeze {
        /// Output file path (default: horus-freeze.yaml)
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,

        /// Publish environment to registry for sharing by ID
        #[arg(long)]
        publish: bool,
    },

    /// Restore environment from freeze file or registry ID
    Restore {
        /// Path to freeze file or environment ID
        source: String,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Login to HORUS registry with GitHub
    Login {
        /// Use GitHub OAuth authentication
        #[arg(long)]
        github: bool,
    },
    /// Generate API key after GitHub login
    GenerateKey {
        /// Name for the API key
        #[arg(long)]
        name: Option<String>,
        /// Environment (e.g., 'laptop', 'ci-cd')
        #[arg(long)]
        environment: Option<String>,
    },
    /// Logout from HORUS registry
    Logout,
    /// Show current authenticated user
    Whoami,
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run_command(cli.command) {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run_command(command: Commands) -> HorusResult<()> {
    match command {
        Commands::New {
            name,
            path,
            python,
            rust,
            c,
            use_macro,
        } => {
            let language = if python {
                "python"
            } else if rust {
                "rust"
            } else if c {
                "c"
            } else if use_macro {
                "rust" // -m implies Rust with macros
            } else {
                "" // Will use interactive prompt
            };

            commands::new::create_new_project(name, path, language.to_string(), use_macro)
                .map_err(|e| HorusError::Config(e.to_string()))
        }

        Commands::Run {
            file,
            build_only,
            release,
            clean,
            remote,
            args,
        } => {
            if let Some(robot_addr) = remote {
                // Remote execution mode
                commands::remote::execute_remote(&robot_addr, file)
                    .map_err(|e| HorusError::Config(e.to_string()))
            } else if build_only {
                // Build-only mode - compile but don't execute
                commands::run::execute_build_only(file, release, clean)
                    .map_err(|e| HorusError::Config(e.to_string()))
            } else {
                // Normal run mode (build if needed, then run)
                commands::run::execute_run(file, args, release, clean)
                    .map_err(|e| HorusError::Config(e.to_string()))
            }
        }

        Commands::Dashboard { port, tui } => {
            if tui {
                println!("{} Opening HORUS Terminal UI dashboard...", "→".cyan());
                // Launch TUI dashboard
                dashboard_tui::TuiDashboard::run().map_err(|e| HorusError::Config(e.to_string()))
            } else {
                // Default: Launch web dashboard and auto-open browser
                println!(
                    "{} Starting HORUS web dashboard on http://localhost:{}...",
                    "→".cyan(),
                    port
                );
                println!("  {} Opening browser...", "→".dimmed());
                println!(
                    "  {} Use 'horus dashboard -t' for Terminal UI",
                    "Tip:".dimmed()
                );

                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(dashboard::run(port))
                    .map_err(|e| {
                        let err_str = e.to_string();
                        if err_str.contains("Address already in use") || err_str.contains("os error 98") {
                            HorusError::Config(format!(
                                "Port {} is already in use.\n  {} Try a different port: horus dashboard <PORT>\n  {} Example: horus dashboard {}",
                                port,
                                "→".cyan(),
                                "→".cyan(),
                                port + 1
                            ))
                        } else {
                            HorusError::Config(err_str)
                        }
                    })
            }
        }

        Commands::Pkg { command } => {
            match command {
                PkgCommands::Install {
                    package,
                    ver,
                    global,
                    target,
                    dev,
                } => {
                    if dev {
                        println!("  {} Dev dependencies not yet supported", "Note:".yellow());
                    }

                    // Determine installation target
                    let install_target = if global {
                        // Explicit global install
                        workspace::InstallTarget::Global
                    } else if let Some(target_name) = target {
                        // Explicit target workspace
                        let registry = workspace::WorkspaceRegistry::load()
                            .map_err(|e| HorusError::Config(e.to_string()))?;

                        let ws = registry.find_by_name(&target_name).ok_or_else(|| {
                            HorusError::Config(format!("Workspace '{}' not found", target_name))
                        })?;

                        workspace::InstallTarget::Local(ws.path.clone())
                    } else {
                        // Auto-detect or interactive
                        workspace::detect_or_select_workspace(true)
                            .map_err(|e| HorusError::Config(e.to_string()))?
                    };

                    // Install package with target
                    let client = registry::RegistryClient::new();
                    client
                        .install_to_target(&package, ver.as_deref(), install_target)
                        .map_err(|e| HorusError::Config(e.to_string()))
                }

                PkgCommands::Remove {
                    package,
                    global,
                    target,
                } => {
                    println!("{} Removing {}...", "→".cyan(), package.yellow());

                    let remove_dir = if global {
                        // Remove from global cache
                        let home = dirs::home_dir().ok_or_else(|| {
                            HorusError::Config("Could not find home directory".to_string())
                        })?;
                        let global_cache = home.join(".horus/cache");

                        // Find versioned directory
                        let mut found = None;
                        if global_cache.exists() {
                            for entry in fs::read_dir(&global_cache)
                                .map_err(|e| HorusError::Config(e.to_string()))?
                            {
                                let entry = entry.map_err(|e| HorusError::Config(e.to_string()))?;
                                let name = entry.file_name().to_string_lossy().to_string();
                                if name == package || name.starts_with(&format!("{}@", package)) {
                                    found = Some(entry.path());
                                    break;
                                }
                            }
                        }
                        found.ok_or_else(|| {
                            HorusError::Config(format!(
                                "Package {} not found in global cache",
                                package
                            ))
                        })?
                    } else if let Some(target_name) = target {
                        // Remove from specific workspace
                        let registry = workspace::WorkspaceRegistry::load()
                            .map_err(|e| HorusError::Config(e.to_string()))?;
                        let ws = registry.find_by_name(&target_name).ok_or_else(|| {
                            HorusError::Config(format!("Workspace '{}' not found", target_name))
                        })?;
                        ws.path.join(".horus/packages").join(&package)
                    } else {
                        // Remove from current workspace
                        if let Some(root) = workspace::find_workspace_root() {
                            root.join(".horus/packages").join(&package)
                        } else {
                            PathBuf::from(".horus/packages").join(&package)
                        }
                    };

                    if !remove_dir.exists() {
                        println!("❌ Package {} is not installed", package);
                        return Ok(());
                    }

                    // Remove package directory
                    std::fs::remove_dir_all(&remove_dir).map_err(|e| {
                        HorusError::Config(format!("Failed to remove package: {}", e))
                    })?;

                    println!("✅ Removed {} from {}", package, remove_dir.display());

                    Ok(())
                }

                PkgCommands::List { query, global, all } => {
                    let client = registry::RegistryClient::new();

                    if let Some(q) = query {
                        // Search registry marketplace
                        println!(
                            "{} Searching registry marketplace for '{}'...",
                            "→".cyan(),
                            q
                        );
                        let results = client
                            .search(&q)
                            .map_err(|e| HorusError::Config(e.to_string()))?;

                        if results.is_empty() {
                            println!("❌ No packages found in marketplace matching '{}'", q);
                        } else {
                            println!(
                                "\n{} Found {} package(s) in marketplace:\n",
                                "✓".green(),
                                results.len()
                            );
                            for pkg in results {
                                println!(
                                    "  {} {} - {}",
                                    pkg.name.yellow().bold(),
                                    pkg.version.dimmed(),
                                    pkg.description.unwrap_or_default()
                                );
                            }
                        }
                    } else if all {
                        // List both local and global packages
                        let home = dirs::home_dir().ok_or_else(|| {
                            HorusError::Config("Could not find home directory".to_string())
                        })?;
                        let global_cache = home.join(".horus/cache");

                        // Show local packages
                        println!("{} Local packages:\n", "→".cyan());
                        let packages_dir = if let Some(root) = workspace::find_workspace_root() {
                            root.join(".horus/packages")
                        } else {
                            PathBuf::from(".horus/packages")
                        };

                        if packages_dir.exists() {
                            let mut has_local = false;
                            for entry in fs::read_dir(&packages_dir)
                                .map_err(|e| HorusError::Config(e.to_string()))?
                            {
                                let entry = entry.map_err(|e| HorusError::Config(e.to_string()))?;
                                if entry
                                    .file_type()
                                    .map_err(|e| HorusError::Config(e.to_string()))?
                                    .is_dir()
                                {
                                    has_local = true;
                                    let name = entry.file_name().to_string_lossy().to_string();
                                    println!("  📦 {}", name.yellow());
                                }
                            }
                            if !has_local {
                                println!("  No local packages");
                            }
                        } else {
                            println!("  No local packages");
                        }

                        // Show global packages
                        println!("\n{} Global cache packages:\n", "→".cyan());
                        if global_cache.exists() {
                            let mut has_global = false;
                            for entry in fs::read_dir(&global_cache)
                                .map_err(|e| HorusError::Config(e.to_string()))?
                            {
                                let entry = entry.map_err(|e| HorusError::Config(e.to_string()))?;
                                if entry
                                    .file_type()
                                    .map_err(|e| HorusError::Config(e.to_string()))?
                                    .is_dir()
                                {
                                    has_global = true;
                                    let name = entry.file_name().to_string_lossy().to_string();
                                    println!("  🌐 {}", name.yellow());
                                }
                            }
                            if !has_global {
                                println!("  No global packages");
                            }
                        } else {
                            println!("  No global packages");
                        }
                    } else if global {
                        // List global cache packages
                        println!("{} Global cache packages:\n", "→".cyan());
                        let home = dirs::home_dir().ok_or_else(|| {
                            HorusError::Config("Could not find home directory".to_string())
                        })?;
                        let global_cache = home.join(".horus/cache");

                        if !global_cache.exists() {
                            println!("  No global packages yet");
                            return Ok(());
                        }

                        for entry in fs::read_dir(&global_cache)
                            .map_err(|e| HorusError::Config(e.to_string()))?
                        {
                            let entry = entry.map_err(|e| HorusError::Config(e.to_string()))?;
                            if entry
                                .file_type()
                                .map_err(|e| HorusError::Config(e.to_string()))?
                                .is_dir()
                            {
                                let name = entry.file_name().to_string_lossy().to_string();
                                println!("  🌐 {}", name.yellow());
                            }
                        }
                    } else {
                        // List local workspace packages (default)
                        let packages_dir = if let Some(root) = workspace::find_workspace_root() {
                            root.join(".horus/packages")
                        } else {
                            PathBuf::from(".horus/packages")
                        };

                        println!("{} Local packages:\n", "→".cyan());

                        if !packages_dir.exists() {
                            println!("  No packages installed yet");
                            return Ok(());
                        }

                        for entry in fs::read_dir(&packages_dir)
                            .map_err(|e| HorusError::Config(e.to_string()))?
                        {
                            let entry = entry.map_err(|e| HorusError::Config(e.to_string()))?;
                            if entry
                                .file_type()
                                .map_err(|e| HorusError::Config(e.to_string()))?
                                .is_dir()
                            {
                                let name = entry.file_name().to_string_lossy().to_string();

                                // Try to read metadata.json
                                let metadata_path = entry.path().join("metadata.json");
                                if metadata_path.exists() {
                                    if let Ok(content) = fs::read_to_string(&metadata_path) {
                                        if let Ok(metadata) =
                                            serde_json::from_str::<serde_json::Value>(&content)
                                        {
                                            let version =
                                                metadata["version"].as_str().unwrap_or("unknown");
                                            println!("  {} {}", name.yellow(), version.dimmed());
                                            continue;
                                        }
                                    }
                                }

                                println!("  {}", name.yellow());
                            }
                        }
                    }

                    Ok(())
                }

                PkgCommands::Publish { freeze } => {
                    let client = registry::RegistryClient::new();
                    client
                        .publish(None)
                        .map_err(|e| HorusError::Config(e.to_string()))?;

                    // If --freeze flag is set, also generate freeze file
                    if freeze {
                        println!("\n{} Generating freeze file...", "→".cyan());
                        let manifest = client
                            .freeze()
                            .map_err(|e| HorusError::Config(e.to_string()))?;

                        let freeze_file = "horus-freeze.yaml";
                        let yaml = serde_yaml::to_string(&manifest)
                            .map_err(|e| HorusError::Config(e.to_string()))?;
                        std::fs::write(freeze_file, yaml)
                            .map_err(|e| HorusError::Config(e.to_string()))?;

                        println!("✅ Environment also frozen to {}", freeze_file);
                    }

                    Ok(())
                }

                PkgCommands::Unpublish {
                    package,
                    version,
                    yes,
                } => {
                    use std::io::{self, Write};

                    println!(
                        "{} Unpublishing {} v{}...",
                        "→".cyan(),
                        package.yellow(),
                        version.yellow()
                    );

                    // Confirmation prompt (unless --yes flag is set)
                    if !yes {
                        println!(
                            "\n{} This action is {} and will:",
                            "Warning:".yellow().bold(),
                            "IRREVERSIBLE".red().bold()
                        );
                        println!("  • Delete {} v{} from the registry", package, version);
                        println!("  • Make this version unavailable for download");
                        println!("  • Cannot be undone");
                        println!(
                            "\n{} Consider using 'yank' instead for temporary removal",
                            "Tip:".dimmed()
                        );

                        print!("\nType the package name '{}' to confirm: ", package);
                        io::stdout().flush().unwrap();

                        let mut confirmation = String::new();
                        io::stdin().read_line(&mut confirmation).map_err(|e| {
                            HorusError::Config(format!("Failed to read input: {}", e))
                        })?;

                        if confirmation.trim() != package {
                            println!("❌ Package name mismatch. Unpublish cancelled.");
                            return Ok(());
                        }
                    }

                    // Call unpublish API
                    let client = registry::RegistryClient::new();
                    client
                        .unpublish(&package, &version)
                        .map_err(|e| HorusError::Config(e.to_string()))?;

                    println!(
                        "\n✅ Successfully unpublished {} v{}",
                        package.green(),
                        version.green()
                    );
                    println!("   The package is no longer available on the registry");

                    Ok(())
                }
            }
        }

        Commands::Env { command } => {
            match command {
                EnvCommands::Freeze { output, publish } => {
                    println!("{} Freezing current environment...", "→".cyan());

                    let client = registry::RegistryClient::new();
                    let manifest = client
                        .freeze()
                        .map_err(|e| HorusError::Config(e.to_string()))?;

                    // Save to local file
                    let freeze_file = output.unwrap_or_else(|| PathBuf::from("horus-freeze.yaml"));
                    let yaml = serde_yaml::to_string(&manifest)
                        .map_err(|e| HorusError::Config(e.to_string()))?;
                    std::fs::write(&freeze_file, yaml)
                        .map_err(|e| HorusError::Config(e.to_string()))?;

                    println!("✅ Environment frozen to {}", freeze_file.display());
                    println!("   ID: {}", manifest.horus_id);
                    println!("   Packages: {}", manifest.packages.len());

                    // Publish to registry if requested
                    if publish {
                        println!();
                        client
                            .upload_environment(&manifest)
                            .map_err(|e| HorusError::Config(e.to_string()))?;
                    } else {
                        println!("\n{} To share this environment:", "Tip:".dimmed());
                        println!("   1. File: horus env restore {}", freeze_file.display());
                        println!("   2. Registry: horus env freeze --publish");
                    }

                    Ok(())
                }

                EnvCommands::Restore { source } => {
                    println!("{} Restoring environment from {}...", "→".cyan(), source);

                    let client = registry::RegistryClient::new();

                    // Check if source is a file path or environment ID
                    if source.ends_with(".yaml")
                        || source.ends_with(".yml")
                        || PathBuf::from(&source).exists()
                    {
                        // It's a file path
                        let content = fs::read_to_string(&source).map_err(|e| {
                            HorusError::Config(format!("Failed to read freeze file: {}", e))
                        })?;

                        let manifest: registry::EnvironmentManifest =
                            serde_yaml::from_str(&content).map_err(|e| {
                                HorusError::Config(format!("Failed to parse freeze file: {}", e))
                            })?;

                        println!("📦 Found {} packages to restore", manifest.packages.len());

                        // Install each package from the manifest
                        for pkg in &manifest.packages {
                            println!("  Installing {} v{}...", pkg.name, pkg.version);
                            client
                                .install(&pkg.name, Some(&pkg.version))
                                .map_err(|e| HorusError::Config(e.to_string()))?;
                        }

                        println!("✅ Environment restored from {}", source);
                        println!("   ID: {}", manifest.horus_id);
                        println!("   Packages: {}", manifest.packages.len());
                    } else {
                        // It's an environment ID from registry
                        client
                            .restore_environment(&source)
                            .map_err(|e| HorusError::Config(e.to_string()))?;
                    }

                    Ok(())
                }
            }
        }

        Commands::Auth { command } => match command {
            AuthCommands::Login { github } => commands::github_auth::login(github),
            AuthCommands::GenerateKey { name, environment } => {
                commands::github_auth::generate_key(name, environment)
            }
            AuthCommands::Logout => commands::github_auth::logout(),
            AuthCommands::Whoami => commands::github_auth::whoami(),
        },

        Commands::Completion { shell } => {
            // Hidden command used by install.sh for automatic completion setup
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            generate(shell, &mut cmd, bin_name, &mut io::stdout());
            Ok(())
        }

        Commands::Version => {
            horus_manager::version::print_version_info();
            Ok(())
        }
    }
}
