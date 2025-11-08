use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::generate;
use colored::*;
use horus_core::error::{HorusError, HorusResult};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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

        /// Additional arguments to pass to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Validate horus.yaml and check for issues
    Check {
        /// Path to horus.yaml (default: ./horus.yaml)
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,

        /// Only show errors, suppress warnings
        #[arg(short = 'q', long = "quiet")]
        quiet: bool,
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

    /// Simulation tools (sim2d, sim3d)
    Sim {
        #[command(subcommand)]
        command: SimCommands,
    },

    /// Generate shell completion scripts
    #[command(hide = true)]
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
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
        /// Package version to unpublish
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
        #[arg(short = 'p', long = "publish")]
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
    /// Login to HORUS registry (requires GitHub)
    Login,
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

#[derive(Subcommand)]
enum SimCommands {
    /// Run 2D simulator (sim2d)
    #[command(name = "2d")]
    Sim2d {
        /// World configuration file
        #[arg(long)]
        world: Option<PathBuf>,

        /// World image file (PNG, JPG, PGM) - occupancy grid
        #[arg(long)]
        world_image: Option<PathBuf>,

        /// Resolution in meters per pixel (for world image)
        #[arg(long)]
        resolution: Option<f32>,

        /// Obstacle threshold (0-255, darker = obstacle)
        #[arg(long)]
        threshold: Option<u8>,

        /// Robot configuration file
        #[arg(long)]
        robot: Option<PathBuf>,

        /// HORUS topic for velocity commands
        #[arg(long, default_value = "cmd_vel")]
        topic: String,

        /// Robot name for logging
        #[arg(long, default_value = "robot")]
        name: String,

        /// Run in headless mode (no GUI)
        #[arg(long)]
        headless: bool,
    },

    /// Run 3D simulator (sim3d) - Coming soon
    #[command(name = "3d")]
    Sim3d {
        /// Headless mode (no rendering)
        #[arg(long)]
        headless: bool,

        /// Random seed for deterministic simulation
        #[arg(long)]
        seed: Option<u64>,
    },
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
            args,
        } => {
            if build_only {
                // Build-only mode - compile but don't execute
                commands::run::execute_build_only(file, release, clean)
                    .map_err(|e| HorusError::Config(e.to_string()))
            } else {
                // Normal run mode (build if needed, then run)
                commands::run::execute_run(file, args, release, clean)
                    .map_err(|e| HorusError::Config(e.to_string()))
            }
        }

        Commands::Check { file, quiet } => {
            use horus_manager::commands::run::parse_horus_yaml_dependencies_v2;
            use horus_manager::dependency_resolver::DependencySource;
            use std::collections::HashSet;

            let horus_yaml_path = file.unwrap_or_else(|| PathBuf::from("horus.yaml"));

            if !horus_yaml_path.exists() {
                println!(
                    "{} File not found at: {}",
                    "✗".red(),
                    horus_yaml_path.display()
                );
                return Err(HorusError::Config("File not found".to_string()));
            }

            // Check if it's a source file (.rs, .py, .c, .cpp) or horus.yaml
            let extension = horus_yaml_path.extension().and_then(|s| s.to_str());

            match extension {
                Some("rs") => {
                    // Check Rust file
                    println!("{} Checking Rust file: {}\n", "".cyan(), horus_yaml_path.display());

                    print!("  {} Parsing Rust syntax... ", "".cyan());
                    let content = fs::read_to_string(&horus_yaml_path)?;

                    // Use syn to parse Rust code
                    match syn::parse_file(&content) {
                        Ok(_) => {
                            println!("{}", "".green());
                            println!("\n{} Syntax check passed!", "✓".green().bold());
                        }
                        Err(e) => {
                            println!("{}", "".red());
                            println!("\n{} Syntax error:", "✗".red().bold());
                            println!("  {}", e);
                            return Err(HorusError::Config(format!("Rust syntax error: {}", e)));
                        }
                    }
                    return Ok(());
                }
                Some("py") => {
                    // Check Python file
                    println!("{} Checking Python file: {}\n", "".cyan(), horus_yaml_path.display());

                    print!("  {} Parsing Python syntax... ", "".cyan());

                    // Use python3 to check syntax
                    let output = std::process::Command::new("python3")
                        .arg("-m")
                        .arg("py_compile")
                        .arg(&horus_yaml_path)
                        .output();

                    match output {
                        Ok(result) if result.status.success() => {
                            println!("{}", "".green());
                            println!("\n{} Syntax check passed!", "✓".green().bold());
                        }
                        Ok(result) => {
                            println!("{}", "".red());
                            let error = String::from_utf8_lossy(&result.stderr);
                            println!("\n{} Syntax error:", "✗".red().bold());
                            println!("  {}", error);
                            return Err(HorusError::Config(format!("Python syntax error: {}", error)));
                        }
                        Err(e) => {
                            println!("{}", "⚠".yellow());
                            println!("\n{} Could not check Python syntax (python3 not found): {}", "⚠".yellow(), e);
                        }
                    }
                    return Ok(());
                }
                Some("c") | Some("cpp") | Some("cc") | Some("cxx") => {
                    // Check C/C++ file
                    println!("{} Checking C/C++ file: {}\n", "".cyan(), horus_yaml_path.display());

                    print!("  {} Parsing C/C++ syntax... ", "".cyan());

                    let compiler = if extension == Some("cpp") || extension == Some("cc") || extension == Some("cxx") {
                        "g++"
                    } else {
                        "gcc"
                    };

                    // Use gcc/g++ to check syntax only
                    let output = std::process::Command::new(compiler)
                        .arg("-fsyntax-only")
                        .arg(&horus_yaml_path)
                        .output();

                    match output {
                        Ok(result) if result.status.success() => {
                            println!("{}", "".green());
                            println!("\n{} Syntax check passed!", "✓".green().bold());
                        }
                        Ok(result) => {
                            println!("{}", "".red());
                            let error = String::from_utf8_lossy(&result.stderr);
                            println!("\n{} Syntax error:", "✗".red().bold());
                            println!("  {}", error);
                            return Err(HorusError::Config(format!("C/C++ syntax error: {}", error)));
                        }
                        Err(e) => {
                            println!("{}", "⚠".yellow());
                            println!("\n{} Could not check C/C++ syntax ({} not found): {}", "⚠".yellow(), compiler, e);
                        }
                    }
                    return Ok(());
                }
                _ => {
                    // Assume it's horus.yaml or yaml file
                    println!("{} Checking {}...\n", "".cyan(), horus_yaml_path.display());
                }
            }

            let mut errors = Vec::new();
            let mut warn_msgs = Vec::new();
            let base_dir = horus_yaml_path.parent().unwrap_or(Path::new("."));

            // 1. YAML Syntax Validation
            print!("  {} Validating YAML syntax... ", "".cyan());
            let yaml_content = match fs::read_to_string(&horus_yaml_path) {
                Ok(content) => {
                    println!("{}", "".green());
                    content
                }
                Err(e) => {
                    println!("{}", "".red());
                    errors.push(format!("Cannot read file: {}", e));
                    String::new()
                }
            };

            let yaml_value: Option<serde_yaml::Value> = if !yaml_content.is_empty() {
                match serde_yaml::from_str(&yaml_content) {
                    Ok(val) => Some(val),
                    Err(e) => {
                        errors.push(format!("Invalid YAML syntax: {}", e));
                        None
                    }
                }
            } else {
                None
            };

            // 2. Required Fields Check
            if let Some(ref yaml) = yaml_value {
                print!("  {} Checking required fields... ", "".cyan());
                let mut missing_fields = Vec::new();

                if yaml.get("name").is_none() {
                    missing_fields.push("name");
                }
                if yaml.get("version").is_none() {
                    missing_fields.push("version");
                }

                if missing_fields.is_empty() {
                    println!("{}", "".green());
                } else {
                    println!("{}", "".red());
                    errors.push(format!(
                        "Missing required fields: {}",
                        missing_fields.join(", ")
                    ));
                }

                // Optional fields warning
                if !quiet {
                    if yaml.get("description").is_none() {
                        warn_msgs.push("Optional field missing: description".to_string());
                    }
                    if yaml.get("author").is_none() {
                        warn_msgs.push("Optional field missing: author".to_string());
                    }
                }

                // License warning (encourage projects to declare their license)
                print!("  {} Checking license field... ", "✓".cyan());
                let missing_license_warning = "No license specified. Consider adding a license field (e.g., Apache-2.0, BSD-3-Clause).";
                if let Some(license) = yaml.get("license").and_then(|l| l.as_str()) {
                    if license.trim().is_empty() {
                        println!("{}", "⚠".yellow());
                        warn_msgs.push(missing_license_warning.to_string());
                    } else {
                        println!("{} ({})", "✓".green(), license.dimmed());
                    }
                } else {
                    println!("{}", "⚠".yellow());
                    warn_msgs.push(missing_license_warning.to_string());
                }

                // Language validation
                print!("  {} Validating language field... ", "".cyan());
                if let Some(language) = yaml.get("language").and_then(|l| l.as_str()) {
                    if language == "rust" || language == "python" || language == "cpp" {
                        println!("{}", "".green());
                    } else {
                        println!("{}", "".red());
                        errors.push(format!(
                            "Invalid language '{}' - must be: rust, python, or cpp",
                            language
                        ));
                    }
                } else {
                    println!("{}", "".red());
                    errors.push(
                        "Missing or invalid 'language' field - must be: rust, python, or cpp"
                            .to_string(),
                    );
                }

                // Version format validation
                print!("  {} Validating version format... ", "".cyan());
                if let Some(version_str) = yaml.get("version").and_then(|v| v.as_str()) {
                    use semver::Version;
                    match Version::parse(version_str) {
                        Ok(_) => println!("{}", "".green()),
                        Err(e) => {
                            println!("{}", "".red());
                            errors.push(format!(
                                "Invalid version format '{}': {} (must be valid semver like 0.1.0)",
                                version_str, e
                            ));
                        }
                    }
                } else if yaml.get("version").is_some() {
                    println!("{}", "".red());
                    errors.push("Version field must be a string".to_string());
                }

                // Project name validation
                print!("  {} Validating project name... ", "".cyan());
                if let Some(name) = yaml.get("name").and_then(|n| n.as_str()) {
                    let mut name_issues = Vec::new();

                    if name.is_empty() {
                        name_issues.push("name cannot be empty");
                    }
                    if name.contains(' ') {
                        name_issues.push("name cannot contain spaces");
                    }
                    if name
                        .chars()
                        .any(|c| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
                    {
                        name_issues.push(
                            "name can only contain letters, numbers, hyphens, and underscores",
                        );
                    }

                    if name_issues.is_empty() {
                        println!("{}", "".green());
                        // Warn if uppercase
                        if !quiet && name.chars().any(|c| c.is_uppercase()) {
                            warn_msgs.push(format!(
                                "Project name '{}' contains uppercase - consider using lowercase",
                                name
                            ));
                        }
                    } else {
                        println!("{}", "".red());
                        for issue in name_issues {
                            errors.push(format!("Invalid project name: {}", issue));
                        }
                    }
                }

                // Main file existence check
                print!("  {} Checking for main file... ", "".cyan());
                if let Some(language) = yaml.get("language").and_then(|l| l.as_str()) {
                    let main_files = match language {
                        "rust" => vec!["main.rs", "src/main.rs"],
                        "python" => vec!["main.py"],
                        "cpp" => vec!["main.cpp"],
                        _ => vec![],
                    };

                    let mut found = false;
                    for main_file in &main_files {
                        let path = base_dir.join(main_file);
                        if path.exists() {
                            println!("{}", "".green());
                            found = true;
                            break;
                        }
                    }

                    if !found && !main_files.is_empty() {
                        println!("{}", "".yellow());
                        if !quiet {
                            warn_msgs.push(format!(
                                "No main file found - expected one of: {}",
                                main_files.join(", ")
                            ));
                        }
                    }
                } else {
                    println!("{}", "⊘".dimmed());
                }
            }

            // 3. Parse Dependencies
            print!("  {} Parsing dependencies... ", "".cyan());
            let dep_specs =
                match parse_horus_yaml_dependencies_v2(horus_yaml_path.to_str().unwrap()) {
                    Ok(specs) => {
                        println!("{}", "".green());
                        specs
                    }
                    Err(e) => {
                        println!("{}", "".red());
                        errors.push(format!("Failed to parse dependencies: {}", e));
                        Vec::new()
                    }
                };

            // 4. Check for Duplicates
            if !dep_specs.is_empty() {
                print!("  {} Checking for duplicates... ", "".cyan());
                let mut seen = HashSet::new();
                let mut duplicates = Vec::new();

                for spec in &dep_specs {
                    if !seen.insert(&spec.name) {
                        duplicates.push(spec.name.clone());
                    }
                }

                if duplicates.is_empty() {
                    println!("{}", "".green());
                } else {
                    println!("{}", "".red());
                    errors.push(format!("Duplicate dependencies: {}", duplicates.join(", ")));
                }
            }

            // 5. Validate Path Dependencies
            println!("\n  {} Checking path dependencies...", "".cyan());
            let mut path_deps_found = false;

            for spec in &dep_specs {
                use horus_manager::dependency_resolver::DependencySource;
                if let DependencySource::Path(ref path) = spec.source {
                    path_deps_found = true;
                    let resolved_path = if path.is_absolute() {
                        path.clone()
                    } else {
                        base_dir.join(path)
                    };

                    if resolved_path.exists() {
                        if resolved_path.is_dir() {
                            println!("    {} {} ({})", "".green(), spec.name, path.display());
                        } else {
                            println!(
                                "    {} {} ({}) - Not a directory",
                                "✗".red(),
                                spec.name,
                                path.display()
                            );
                            errors.push(format!(
                                "Path dependency '{}' is not a directory: {}",
                                spec.name,
                                path.display()
                            ));
                        }
                    } else {
                        println!(
                            "    {} {} ({}) - Path not found",
                            "✗".red(),
                            spec.name,
                            path.display()
                        );
                        errors.push(format!(
                            "Path dependency '{}' not found: {}",
                            spec.name,
                            path.display()
                        ));
                    }
                }
            }

            if !path_deps_found {
                println!("    {} No path dependencies", "".dimmed());
            }

            // 6. Circular Dependency Detection (Simple Check)
            println!("\n  {} Checking for circular dependencies...", "".cyan());
            let mut circular_found = false;

            for spec in &dep_specs {
                use horus_manager::dependency_resolver::DependencySource;
                if let DependencySource::Path(ref path) = spec.source {
                    let resolved_path = if path.is_absolute() {
                        path.clone()
                    } else {
                        base_dir.join(path)
                    };

                    // Check if target has horus.yaml
                    let target_yaml = resolved_path.join("horus.yaml");
                    if target_yaml.exists() {
                        // Check if it references us back
                        if let Ok(target_deps) =
                            parse_horus_yaml_dependencies_v2(target_yaml.to_str().unwrap())
                        {
                            let our_name = yaml_value
                                .as_ref()
                                .and_then(|y| y.get("name"))
                                .and_then(|n| n.as_str())
                                .unwrap_or("");

                            for target_dep in target_deps {
                                if target_dep.name == our_name {
                                    if let DependencySource::Path(_) = target_dep.source {
                                        circular_found = true;
                                        errors.push(format!(
                                            "Circular dependency detected: {} -> {} -> {}",
                                            our_name, spec.name, our_name
                                        ));
                                        println!(
                                            "    {} Circular: {} ↔ {}",
                                            "✗".red(),
                                            our_name,
                                            spec.name
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !circular_found {
                println!("    {} No circular dependencies", "".green());
            }

            // 7. Version Constraint Validation
            print!("\n  {} Validating version constraints... ", "".cyan());

            for spec in &dep_specs {
                // Version requirement is already parsed in DependencySpec
                // If it parsed successfully, it's valid
                // But we can check for common mistakes
                if spec.requirement.to_string() == "*" {
                    if !quiet {
                        warn_msgs.push(format!("Dependency '{}' uses wildcard version (*) - consider pinning to a specific version", spec.name));
                    }
                }
            }

            println!("{}", "".green());

            // 8. Workspace Structure Check
            print!("\n  {} Checking workspace structure... ", "".cyan());
            let base_dir = horus_yaml_path.parent().unwrap_or_else(|| Path::new("."));
            let horus_dir = base_dir.join(".horus");

            if horus_dir.exists() && horus_dir.is_dir() {
                println!("{}", "".green());
            } else {
                println!("{}", "".yellow());
                if !quiet {
                    warn_msgs.push(
                        "No .horus/ workspace directory found - will be created on first run"
                            .to_string(),
                    );
                }
            }

            // 9. Dependency Installation Check
            print!("  {} Checking installed dependencies... ", "".cyan());
            if horus_dir.exists() {
                let packages_dir = horus_dir.join("packages");
                if packages_dir.exists() {
                    let mut missing_deps = Vec::new();

                    for spec in &dep_specs {
                        match &spec.source {
                            DependencySource::Registry => {
                                let package_dir = packages_dir.join(&spec.name);
                                if !package_dir.exists() {
                                    missing_deps.push(spec.name.clone());
                                }
                            }
                            DependencySource::Path(_) => {
                                // Path deps checked separately
                            }
                        }
                    }

                    if missing_deps.is_empty() {
                        println!("{}", "".green());
                    } else {
                        println!("{}", "".yellow());
                        if !missing_deps.is_empty() {
                            if !quiet {
                                warn_msgs.push(format!(
                                    "Missing dependencies: {} (run 'horus run' to install)",
                                    missing_deps.join(", ")
                                ));
                            }
                        }
                    }
                } else {
                    println!("{}", "".yellow());
                    if !quiet {
                        warn_msgs.push(
                            "No packages directory - dependencies not installed yet".to_string(),
                        );
                    }
                }
            } else {
                println!("{}", "⊘".dimmed());
            }

            // 10. Toolchain Check
            print!("  {} Checking toolchain... ", "".cyan());
            if let Some(ref yaml) = yaml_value {
                if let Some(language) = yaml.get("language").and_then(|l| l.as_str()) {
                    let toolchain_available = match language {
                        "rust" => std::process::Command::new("rustc")
                            .arg("--version")
                            .output()
                            .map(|o| o.status.success())
                            .unwrap_or(false),
                        "python" => std::process::Command::new("python3")
                            .arg("--version")
                            .output()
                            .map(|o| o.status.success())
                            .unwrap_or(false),
                        "cpp" => std::process::Command::new("g++")
                            .arg("--version")
                            .output()
                            .map(|o| o.status.success())
                            .unwrap_or(false),
                        _ => false,
                    };

                    if toolchain_available {
                        println!("{}", "".green());
                    } else {
                        println!("{}", "".red());
                        errors.push(format!(
                            "Required toolchain for '{}' not found in PATH",
                            language
                        ));
                    }
                } else {
                    println!("{}", "⊘".dimmed());
                }
            } else {
                println!("{}", "⊘".dimmed());
            }

            // 11. Code Validation (optional, can be slow)
            print!("  {} Validating code syntax... ", "".cyan());
            if let Some(ref yaml) = yaml_value {
                if let Some(language) = yaml.get("language").and_then(|l| l.as_str()) {
                    match language {
                        "rust" => {
                            // Check for Cargo.toml or main.rs
                            let has_cargo = base_dir.join("Cargo.toml").exists();
                            let has_main = base_dir.join("main.rs").exists()
                                || base_dir.join("src/main.rs").exists();

                            if has_cargo || has_main {
                                let check_result = std::process::Command::new("cargo")
                                    .arg("check")
                                    .arg("--quiet")
                                    .current_dir(base_dir)
                                    .output();

                                match check_result {
                                    Ok(output) if output.status.success() => {
                                        println!("{}", "".green());
                                    }
                                    Ok(_) => {
                                        println!("{}", "".red());
                                        errors.push("Rust code has compilation errors (run 'cargo check' for details)".to_string());
                                    }
                                    Err(_) => {
                                        println!("{}", "".yellow());
                                        if !quiet {
                                            warn_msgs.push("Could not run 'cargo check' - skipping code validation".to_string());
                                        }
                                    }
                                }
                            } else {
                                println!("{}", "⊘".dimmed());
                            }
                        }
                        "python" => {
                            // Check main.py syntax
                            let main_py = base_dir.join("main.py");
                            if main_py.exists() {
                                let check_result = std::process::Command::new("python3")
                                    .arg("-m")
                                    .arg("py_compile")
                                    .arg(&main_py)
                                    .output();

                                match check_result {
                                    Ok(output) if output.status.success() => {
                                        println!("{}", "".green());
                                    }
                                    Ok(_) => {
                                        println!("{}", "".red());
                                        errors.push("Python code has syntax errors".to_string());
                                    }
                                    Err(_) => {
                                        println!("{}", "".yellow());
                                        if !quiet {
                                            warn_msgs.push(
                                                "Could not validate Python syntax".to_string(),
                                            );
                                        }
                                    }
                                }
                            } else {
                                println!("{}", "⊘".dimmed());
                            }
                        }
                        "cpp" => {
                            // Check main.cpp syntax
                            let main_cpp = base_dir.join("main.cpp");
                            if main_cpp.exists() {
                                // Get horus_cpp include path from cache
                                let home_dir =
                                    std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                                let horus_cpp_include =
                                    format!("{}/.horus/cache/horus_cpp@0.1.0/include", home_dir);

                                // Run g++ syntax check
                                let check_result = std::process::Command::new("g++")
                                    .arg("-fsyntax-only")
                                    .arg("-std=c++17")
                                    .arg(&format!("-I{}", horus_cpp_include))
                                    .arg(&main_cpp)
                                    .output();

                                match check_result {
                                    Ok(output) if output.status.success() => {
                                        println!("{}", "✓".green());

                                        // Additional API validation
                                        if let Ok(content) = fs::read_to_string(&main_cpp) {
                                            // Check for common API mistakes
                                            if content.contains("try_recv(") {
                                                if !quiet {
                                                    warn_msgs.push("Found try_recv() - this method was removed, use recv() instead".to_string());
                                                }
                                            }
                                            if content.contains("Publisher<")
                                                && !content.contains("Publisher<Twist>")
                                                && !content.contains("Publisher<Pose>")
                                            {
                                                if !quiet {
                                                    warn_msgs.push("Custom message types in Publisher<T> are not supported - only Twist and Pose".to_string());
                                                }
                                            }
                                            if content.contains("Subscriber<")
                                                && !content.contains("Subscriber<Twist>")
                                                && !content.contains("Subscriber<Pose>")
                                            {
                                                if !quiet {
                                                    warn_msgs.push("Custom message types in Subscriber<T> are not supported - only Twist and Pose".to_string());
                                                }
                                            }
                                        }
                                    }
                                    Ok(output) => {
                                        println!("{}", "✗".red());
                                        let stderr = String::from_utf8_lossy(&output.stderr);
                                        errors.push(format!(
                                            "C++ code has compilation errors:\n{}",
                                            stderr
                                        ));
                                    }
                                    Err(_) => {
                                        println!("{}", "⚠".yellow());
                                        if !quiet {
                                            warn_msgs.push("Could not run g++ - skipping C++ syntax validation".to_string());
                                        }
                                    }
                                }
                            } else {
                                println!("{}", "⊘".dimmed());
                            }
                        }
                        _ => {
                            println!("{}", "⊘".dimmed());
                        }
                    }
                } else {
                    println!("{}", "⊘".dimmed());
                }
            } else {
                println!("{}", "⊘".dimmed());
            }

            // 12. HORUS System Check
            print!("\n  {} Checking HORUS installation... ", "".cyan());
            let horus_version = env!("CARGO_PKG_VERSION");
            println!("v{}", horus_version.dimmed());

            // 13. Registry Connectivity
            print!("  {} Checking registry connectivity... ", "".cyan());
            // Simple connectivity check - try to connect to registry
            let registry_available = std::process::Command::new("ping")
                .arg("-c")
                .arg("1")
                .arg("-W")
                .arg("1")
                .arg("registry.horus.rs")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if registry_available {
                println!("{}", "".green());
            } else {
                println!("{}", "⊘".dimmed());
                if !quiet {
                    warn_msgs
                        .push("Registry not reachable - package installation may fail".to_string());
                }
            }

            // 14. System Requirements Check
            print!("  {} Checking system requirements... ", "".cyan());
            let mut sys_issues = Vec::new();

            // Check /dev/shm for shared memory
            #[cfg(target_os = "linux")]
            {
                let shm_path = std::path::Path::new("/dev/shm");
                if !shm_path.exists() {
                    sys_issues.push("/dev/shm not available");
                } else if let Ok(metadata) = std::fs::metadata(shm_path) {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = metadata.permissions().mode();
                    if mode & 0o777 != 0o777 {
                        sys_issues.push("/dev/shm permissions restrictive");
                    }
                }
            }

            if sys_issues.is_empty() {
                println!("{}", "".green());
            } else {
                println!("{}", "".yellow());
                for issue in sys_issues {
                    if !quiet {
                        warn_msgs.push(format!("System issue: {}", issue));
                    }
                }
            }

            // 15. API Usage Check (basic pattern matching)
            print!("  {} Checking API usage... ", "".cyan());
            if let Some(ref yaml) = yaml_value {
                if let Some(language) = yaml.get("language").and_then(|l| l.as_str()) {
                    match language {
                        "rust" => {
                            // Check if using HORUS dependencies
                            let uses_horus = dep_specs
                                .iter()
                                .any(|spec| spec.name == "horus" || spec.name == "horus_macros");

                            if uses_horus {
                                // Quick check for common patterns in main.rs or src/main.rs
                                let main_paths =
                                    vec![base_dir.join("main.rs"), base_dir.join("src/main.rs")];

                                let mut has_scheduler = false;
                                for main_path in main_paths {
                                    if main_path.exists() {
                                        if let Ok(content) = std::fs::read_to_string(&main_path) {
                                            has_scheduler = content.contains("Scheduler::new")
                                                || content.contains("scheduler.register");
                                            if has_scheduler {
                                                break;
                                            }
                                        }
                                    }
                                }

                                if has_scheduler {
                                    println!("{}", "".green());
                                } else {
                                    println!("{}", "".yellow());
                                    if !quiet {
                                        warn_msgs.push("HORUS dependency found but no Scheduler usage detected".to_string());
                                    }
                                }
                            } else {
                                println!("{}", "⊘".dimmed());
                            }
                        }
                        "python" => {
                            let uses_horus = dep_specs.iter().any(|spec| spec.name == "horus_py");

                            if uses_horus {
                                let main_py = base_dir.join("main.py");
                                if main_py.exists() {
                                    if let Ok(content) = std::fs::read_to_string(&main_py) {
                                        if content.contains("import horus")
                                            || content.contains("from horus")
                                        {
                                            println!("{}", "".green());
                                        } else {
                                            println!("{}", "".yellow());
                                            if !quiet {
                                                warn_msgs.push("horus_py dependency but no 'import horus' found".to_string());
                                            }
                                        }
                                    } else {
                                        println!("{}", "⊘".dimmed());
                                    }
                                } else {
                                    println!("{}", "⊘".dimmed());
                                }
                            } else {
                                println!("{}", "⊘".dimmed());
                            }
                        }
                        "cpp" => {
                            let uses_horus = dep_specs.iter().any(|spec| spec.name == "horus_cpp");

                            if uses_horus {
                                let main_cpp = base_dir.join("main.cpp");
                                if main_cpp.exists() {
                                    if let Ok(content) = std::fs::read_to_string(&main_cpp) {
                                        if content.contains("#include <horus.hpp>") {
                                            println!("{}", "".green());
                                        } else {
                                            println!("{}", "".yellow());
                                            if !quiet {
                                                warn_msgs.push("horus_cpp dependency but no '#include <horus.hpp>' found".to_string());
                                            }
                                        }
                                    } else {
                                        println!("{}", "⊘".dimmed());
                                    }
                                } else {
                                    println!("{}", "⊘".dimmed());
                                }
                            } else {
                                println!("{}", "⊘".dimmed());
                            }
                        }
                        _ => {
                            println!("{}", "⊘".dimmed());
                        }
                    }
                } else {
                    println!("{}", "⊘".dimmed());
                }
            } else {
                println!("{}", "⊘".dimmed());
            }

            // Print Summary
            println!();
            if !quiet {
                if warn_msgs.is_empty() {
                    println!("{} No warnings detected.", "✓".green());
                } else {
                    println!("{} {} warning(s):", "⚠".yellow(), warn_msgs.len());
                    for warn in &warn_msgs {
                        println!("  - {}", warn);
                    }
                }
                println!();
            }

            if errors.is_empty() {
                println!("{} All checks passed!", "".green().bold());
                Ok(())
            } else {
                println!("{} {} error(s) found:\n", "✗".red().bold(), errors.len());
                for (i, err) in errors.iter().enumerate() {
                    println!("  {}. {}", i + 1, err);
                }
                println!();
                Err(HorusError::Config("Validation failed".to_string()))
            }
        }

        Commands::Dashboard { port, tui } => {
            if tui {
                println!("{} Opening HORUS Terminal UI dashboard...", "".cyan());
                // Launch TUI dashboard
                dashboard_tui::TuiDashboard::run().map_err(|e| HorusError::Config(e.to_string()))
            } else {
                // Default: Launch web dashboard and auto-open browser
                println!(
                    "{} Starting HORUS web dashboard on http://localhost:{}...",
                    "".cyan(),
                    port
                );
                println!("  {} Opening browser...", "".dimmed());
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
                                "".cyan(),
                                "".cyan(),
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
                } => {
                    use horus_manager::yaml_utils::{
                        add_path_dependency_to_horus_yaml, is_path_like,
                        read_package_name_from_path,
                    };

                    // Check if package is actually a path
                    if is_path_like(&package) {
                        // Path dependency installation
                        if global {
                            return Err(HorusError::Config(
                                "Cannot install path dependencies globally. Path dependencies must be local.".to_string()
                            ));
                        }

                        println!(
                            "{} Installing path dependency: {}",
                            "".cyan(),
                            package.green()
                        );

                        // Resolve path
                        let path = PathBuf::from(&package);
                        let absolute_path = if path.is_absolute() {
                            path.clone()
                        } else {
                            std::env::current_dir()
                                .map_err(|e| HorusError::Config(e.to_string()))?
                                .join(&path)
                        };

                        // Verify path exists and is a directory
                        if !absolute_path.exists() {
                            return Err(HorusError::Config(format!(
                                "Path does not exist: {}",
                                absolute_path.display()
                            )));
                        }
                        if !absolute_path.is_dir() {
                            return Err(HorusError::Config(format!(
                                "Path is not a directory: {}",
                                absolute_path.display()
                            )));
                        }

                        // Read package name from the path
                        let package_name = read_package_name_from_path(&absolute_path)
                            .map_err(|e| HorusError::Config(e.to_string()))?;

                        println!(
                            "  {} Detected package name: {}",
                            "".cyan(),
                            package_name.cyan()
                        );

                        // Determine installation target
                        let install_target = if let Some(target_name) = target {
                            let registry = workspace::WorkspaceRegistry::load()
                                .map_err(|e| HorusError::Config(e.to_string()))?;
                            let ws = registry.find_by_name(&target_name).ok_or_else(|| {
                                HorusError::Config(format!("Workspace '{}' not found", target_name))
                            })?;
                            workspace::InstallTarget::Local(ws.path.clone())
                        } else {
                            workspace::detect_or_select_workspace(true)
                                .map_err(|e| HorusError::Config(e.to_string()))?
                        };

                        // Install using install_from_path
                        let client = registry::RegistryClient::new();
                        let workspace_path = match &install_target {
                            workspace::InstallTarget::Local(p) => p.clone(),
                            _ => unreachable!(), // Already blocked global above
                        };

                        // Pass None for base_dir - CLI paths are resolved relative to current_dir
                        client
                            .install_from_path(&package_name, &absolute_path, install_target, None)
                            .map_err(|e| HorusError::Config(e.to_string()))?;

                        // Update horus.yaml with path dependency
                        let horus_yaml_path = workspace_path.join("horus.yaml");
                        if horus_yaml_path.exists() {
                            if let Err(e) = add_path_dependency_to_horus_yaml(
                                &horus_yaml_path,
                                &package_name,
                                &package, // Use original path as provided by user
                            ) {
                                println!("  {} Failed to update horus.yaml: {}", "".yellow(), e);
                            } else {
                                println!("  {} Updated horus.yaml", "".green());
                            }
                        }

                        println!("{} Path dependency installed successfully!", "".green());
                        Ok(())
                    } else {
                        // Registry dependency installation (existing logic)
                        let install_target = if global {
                            workspace::InstallTarget::Global
                        } else if let Some(target_name) = target {
                            let registry = workspace::WorkspaceRegistry::load()
                                .map_err(|e| HorusError::Config(e.to_string()))?;
                            let ws = registry.find_by_name(&target_name).ok_or_else(|| {
                                HorusError::Config(format!("Workspace '{}' not found", target_name))
                            })?;
                            workspace::InstallTarget::Local(ws.path.clone())
                        } else {
                            workspace::detect_or_select_workspace(true)
                                .map_err(|e| HorusError::Config(e.to_string()))?
                        };

                        let client = registry::RegistryClient::new();
                        client
                            .install_to_target(&package, ver.as_deref(), install_target.clone())
                            .map_err(|e| HorusError::Config(e.to_string()))?;

                        // Update horus.yaml if installing locally
                        if let workspace::InstallTarget::Local(workspace_path) = install_target {
                            let horus_yaml_path = workspace_path.join("horus.yaml");
                            if horus_yaml_path.exists() {
                                let version = ver.as_deref().unwrap_or("latest");
                                if let Err(e) =
                                    horus_manager::yaml_utils::add_dependency_to_horus_yaml(
                                        &horus_yaml_path,
                                        &package,
                                        version,
                                    )
                                {
                                    println!(
                                        "  {} Failed to update horus.yaml: {}",
                                        "".yellow(),
                                        e
                                    );
                                }
                            }
                        }

                        Ok(())
                    }
                }

                PkgCommands::Remove {
                    package,
                    global,
                    target,
                } => {
                    println!("{} Removing {}...", "".cyan(), package.yellow());

                    // Track workspace path for horus.yaml update
                    let workspace_path = if global {
                        None
                    } else if let Some(target_name) = &target {
                        let registry = workspace::WorkspaceRegistry::load()
                            .map_err(|e| HorusError::Config(e.to_string()))?;
                        let ws = registry.find_by_name(target_name).ok_or_else(|| {
                            HorusError::Config(format!("Workspace '{}' not found", target_name))
                        })?;
                        Some(ws.path.clone())
                    } else {
                        workspace::find_workspace_root()
                    };

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
                    } else if let Some(target_name) = &target {
                        // Remove from specific workspace
                        let registry = workspace::WorkspaceRegistry::load()
                            .map_err(|e| HorusError::Config(e.to_string()))?;
                        let ws = registry.find_by_name(target_name).ok_or_else(|| {
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

                    // Check for system package reference first
                    let packages_dir = if global {
                        let home = dirs::home_dir().ok_or_else(|| {
                            HorusError::Config("Could not find home directory".to_string())
                        })?;
                        home.join(".horus/cache")
                    } else if let Some(target_name) = &target {
                        let registry = workspace::WorkspaceRegistry::load()
                            .map_err(|e| HorusError::Config(e.to_string()))?;
                        let ws = registry.find_by_name(target_name).ok_or_else(|| {
                            HorusError::Config(format!("Workspace '{}' not found", target_name))
                        })?;
                        ws.path.join(".horus/packages")
                    } else {
                        if let Some(root) = workspace::find_workspace_root() {
                            root.join(".horus/packages")
                        } else {
                            PathBuf::from(".horus/packages")
                        }
                    };

                    let system_ref = packages_dir.join(format!("{}.system.json", package));
                    if system_ref.exists() {
                        // Read to determine package type
                        let content = fs::read_to_string(&system_ref).map_err(|e| {
                            HorusError::Config(format!("Failed to read system reference: {}", e))
                        })?;
                        let metadata: serde_json::Value =
                            serde_json::from_str(&content).map_err(|e| {
                                HorusError::Config(format!(
                                    "Failed to parse system reference: {}",
                                    e
                                ))
                            })?;

                        // Remove reference file
                        fs::remove_file(&system_ref).map_err(|e| {
                            HorusError::Config(format!("Failed to remove system reference: {}", e))
                        })?;

                        // If it's a cargo package, also remove bin symlink
                        if let Some(pkg_type) = metadata.get("package_type") {
                            if pkg_type == "CratesIO" {
                                let bin_dir = if let Some(root) = workspace::find_workspace_root() {
                                    root.join(".horus/bin")
                                } else {
                                    PathBuf::from(".horus/bin")
                                };
                                let bin_link = bin_dir.join(&package);
                                if bin_link.exists() || bin_link.read_link().is_ok() {
                                    fs::remove_file(&bin_link).map_err(|e| {
                                        HorusError::Config(format!(
                                            "Failed to remove binary link: {}",
                                            e
                                        ))
                                    })?;
                                    println!(" Removed binary link for {}", package);
                                }
                            }
                        }

                        println!(" Removed system package reference for {}", package);

                        // Update horus.yaml if removing from local workspace
                        if let Some(ws_path) = workspace_path {
                            let horus_yaml_path = ws_path.join("horus.yaml");
                            if horus_yaml_path.exists() {
                                let mut content = fs::read_to_string(&horus_yaml_path)
                                    .map_err(|e| HorusError::Config(e.to_string()))?;

                                // Remove package from dependencies list
                                let lines: Vec<&str> = content.lines().collect();
                                let mut new_lines = Vec::new();
                                let mut in_deps = false;

                                for line in lines {
                                    if line.trim() == "dependencies:" {
                                        in_deps = true;
                                        new_lines.push(line);
                                    } else if in_deps && line.starts_with("  -") {
                                        let dep = line.trim_start_matches("  -").trim();
                                        if dep != package
                                            && !dep.starts_with(&format!("{}@", package))
                                        {
                                            new_lines.push(line);
                                        }
                                    } else {
                                        if in_deps && !line.starts_with("  ") {
                                            in_deps = false;
                                        }
                                        new_lines.push(line);
                                    }
                                }

                                content = new_lines.join("\n") + "\n";
                                fs::write(&horus_yaml_path, content)
                                    .map_err(|e| HorusError::Config(e.to_string()))?;
                            }
                        }

                        return Ok(());
                    }

                    if !remove_dir.exists() {
                        println!(" Package {} is not installed", package);
                        return Ok(());
                    }

                    // Remove package directory
                    std::fs::remove_dir_all(&remove_dir).map_err(|e| {
                        HorusError::Config(format!("Failed to remove package: {}", e))
                    })?;

                    println!(" Removed {} from {}", package, remove_dir.display());

                    // Update horus.yaml if removing from local workspace
                    if let Some(ws_path) = workspace_path {
                        let horus_yaml_path = ws_path.join("horus.yaml");
                        if horus_yaml_path.exists() {
                            if let Err(e) =
                                horus_manager::yaml_utils::remove_dependency_from_horus_yaml(
                                    &horus_yaml_path,
                                    &package,
                                )
                            {
                                println!("  {} Failed to update horus.yaml: {}", "".yellow(), e);
                            }
                        }
                    }

                    Ok(())
                }

                PkgCommands::List { query, global, all } => {
                    let client = registry::RegistryClient::new();

                    if let Some(q) = query {
                        // Search registry marketplace
                        println!(
                            "{} Searching registry marketplace for '{}'...",
                            "".cyan(),
                            q
                        );
                        let results = client
                            .search(&q)
                            .map_err(|e| HorusError::Config(e.to_string()))?;

                        if results.is_empty() {
                            println!(" No packages found in marketplace matching '{}'", q);
                        } else {
                            println!(
                                "\n{} Found {} package(s) in marketplace:\n",
                                "".green(),
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
                        println!("{} Local packages:\n", "".cyan());
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
                                let entry_path = entry.path();

                                // Skip if it's a metadata file
                                if entry_path.extension().and_then(|s| s.to_str()) == Some("json") {
                                    continue;
                                }

                                if entry
                                    .file_type()
                                    .map_err(|e| HorusError::Config(e.to_string()))?
                                    .is_dir()
                                    || entry
                                        .file_type()
                                        .map_err(|e| HorusError::Config(e.to_string()))?
                                        .is_symlink()
                                {
                                    has_local = true;
                                    let name = entry.file_name().to_string_lossy().to_string();

                                    // Check for path dependency metadata
                                    let path_meta =
                                        packages_dir.join(format!("{}.path.json", name));
                                    if path_meta.exists() {
                                        if let Ok(content) = fs::read_to_string(&path_meta) {
                                            if let Ok(metadata) =
                                                serde_json::from_str::<serde_json::Value>(&content)
                                            {
                                                let version =
                                                    metadata["version"].as_str().unwrap_or("dev");
                                                let path = metadata["source_path"]
                                                    .as_str()
                                                    .unwrap_or("unknown");
                                                println!(
                                                    "   {} {} {} {}",
                                                    name.yellow(),
                                                    version.dimmed(),
                                                    "(path:".dimmed(),
                                                    format!("{})", path).dimmed()
                                                );
                                                continue;
                                            }
                                        }
                                    }

                                    // Check for system package metadata
                                    let system_meta =
                                        packages_dir.join(format!("{}.system.json", name));
                                    if system_meta.exists() {
                                        if let Ok(content) = fs::read_to_string(&system_meta) {
                                            if let Ok(metadata) =
                                                serde_json::from_str::<serde_json::Value>(&content)
                                            {
                                                let version = metadata["version"]
                                                    .as_str()
                                                    .unwrap_or("unknown");
                                                println!(
                                                    "   {} {} {}",
                                                    name.yellow(),
                                                    version.dimmed(),
                                                    "(system)".dimmed()
                                                );
                                                continue;
                                            }
                                        }
                                    }

                                    // Check for regular metadata.json
                                    let metadata_path = entry_path.join("metadata.json");
                                    if metadata_path.exists() {
                                        if let Ok(content) = fs::read_to_string(&metadata_path) {
                                            if let Ok(metadata) =
                                                serde_json::from_str::<serde_json::Value>(&content)
                                            {
                                                let version = metadata["version"]
                                                    .as_str()
                                                    .unwrap_or("unknown");
                                                println!(
                                                    "   {} {} {}",
                                                    name.yellow(),
                                                    version.dimmed(),
                                                    "(registry)".dimmed()
                                                );
                                                continue;
                                            }
                                        }
                                    }

                                    // Fallback: just show name
                                    println!("   {}", name.yellow());
                                }
                            }
                            if !has_local {
                                println!("  No local packages");
                            }
                        } else {
                            println!("  No local packages");
                        }

                        // Show global packages
                        println!("\n{} Global cache packages:\n", "".cyan());
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
                        println!("{} Global cache packages:\n", "".cyan());
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

                        println!("{} Local packages:\n", "".cyan());

                        if !packages_dir.exists() {
                            println!("  No packages installed yet");
                            return Ok(());
                        }

                        for entry in fs::read_dir(&packages_dir)
                            .map_err(|e| HorusError::Config(e.to_string()))?
                        {
                            let entry = entry.map_err(|e| HorusError::Config(e.to_string()))?;
                            let entry_path = entry.path();

                            // Skip if it's a metadata file
                            if entry_path.extension().and_then(|s| s.to_str()) == Some("json") {
                                continue;
                            }

                            if entry
                                .file_type()
                                .map_err(|e| HorusError::Config(e.to_string()))?
                                .is_dir()
                                || entry
                                    .file_type()
                                    .map_err(|e| HorusError::Config(e.to_string()))?
                                    .is_symlink()
                            {
                                let name = entry.file_name().to_string_lossy().to_string();

                                // Check for path dependency metadata
                                let path_meta = packages_dir.join(format!("{}.path.json", name));
                                if path_meta.exists() {
                                    if let Ok(content) = fs::read_to_string(&path_meta) {
                                        if let Ok(metadata) =
                                            serde_json::from_str::<serde_json::Value>(&content)
                                        {
                                            let version =
                                                metadata["version"].as_str().unwrap_or("dev");
                                            let path = metadata["source_path"]
                                                .as_str()
                                                .unwrap_or("unknown");
                                            println!(
                                                "  {} {} {} {}",
                                                name.yellow(),
                                                version.dimmed(),
                                                "(path:".dimmed(),
                                                format!("{})", path).dimmed()
                                            );
                                            continue;
                                        }
                                    }
                                }

                                // Check for system package metadata
                                let system_meta =
                                    packages_dir.join(format!("{}.system.json", name));
                                if system_meta.exists() {
                                    if let Ok(content) = fs::read_to_string(&system_meta) {
                                        if let Ok(metadata) =
                                            serde_json::from_str::<serde_json::Value>(&content)
                                        {
                                            let version =
                                                metadata["version"].as_str().unwrap_or("unknown");
                                            println!(
                                                "  {} {} {}",
                                                name.yellow(),
                                                version.dimmed(),
                                                "(system)".dimmed()
                                            );
                                            continue;
                                        }
                                    }
                                }

                                // Try to read metadata.json (registry packages)
                                let metadata_path = entry_path.join("metadata.json");
                                if metadata_path.exists() {
                                    if let Ok(content) = fs::read_to_string(&metadata_path) {
                                        if let Ok(metadata) =
                                            serde_json::from_str::<serde_json::Value>(&content)
                                        {
                                            let version =
                                                metadata["version"].as_str().unwrap_or("unknown");
                                            println!(
                                                "  {} {} {}",
                                                name.yellow(),
                                                version.dimmed(),
                                                "(registry)".dimmed()
                                            );
                                            continue;
                                        }
                                    }
                                }

                                // Fallback: just show name
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
                        println!("\n{} Generating freeze file...", "".cyan());
                        let manifest = client
                            .freeze()
                            .map_err(|e| HorusError::Config(e.to_string()))?;

                        let freeze_file = "horus-freeze.yaml";
                        let yaml = serde_yaml::to_string(&manifest)
                            .map_err(|e| HorusError::Config(e.to_string()))?;
                        std::fs::write(freeze_file, yaml)
                            .map_err(|e| HorusError::Config(e.to_string()))?;

                        println!(" Environment also frozen to {}", freeze_file);
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
                        "".cyan(),
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
                            println!(" Package name mismatch. Unpublish cancelled.");
                            return Ok(());
                        }
                    }

                    // Call unpublish API
                    let client = registry::RegistryClient::new();
                    client
                        .unpublish(&package, &version)
                        .map_err(|e| HorusError::Config(e.to_string()))?;

                    println!(
                        "\n Successfully unpublished {} v{}",
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
                    println!("{} Freezing current environment...", "".cyan());

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

                    println!(" Environment frozen to {}", freeze_file.display());
                    println!("   ID: {}", manifest.horus_id);
                    println!("   Packages: {}", manifest.packages.len());

                    // Publish to registry if requested
                    if publish {
                        // Validate: check for path dependencies before publishing
                        let has_path_deps = manifest
                            .packages
                            .iter()
                            .any(|pkg| matches!(pkg.source, registry::PackageSource::Path { .. }));

                        if has_path_deps {
                            println!(
                                "\n{} Cannot publish environment with path dependencies!",
                                "Error:".red()
                            );
                            println!("\nPath dependencies found:");
                            for pkg in &manifest.packages {
                                if let registry::PackageSource::Path { ref path } = pkg.source {
                                    println!("  • {} -> {}", pkg.name, path);
                                }
                            }
                            println!("\n{}", "Path dependencies are not portable and cannot be published to the registry.".yellow());
                            println!(
                                "{}",
                                "You can still save locally with: horus env freeze".yellow()
                            );
                            return Err(HorusError::Config(
                                "Cannot publish environment with path dependencies".to_string(),
                            ));
                        }

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
                    println!("{} Restoring environment from {}...", "".cyan(), source);

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

                        println!(" Found {} packages to restore", manifest.packages.len());

                        // Get workspace path for horus.yaml updates
                        let workspace_path = workspace::find_workspace_root();

                        // Install each package from the manifest
                        for pkg in &manifest.packages {
                            // Handle different package sources
                            match &pkg.source {
                                registry::PackageSource::System => {
                                    // Check if system package actually exists
                                    let exists = check_system_package_exists(&pkg.name);

                                    if exists {
                                        println!(
                                            "  {} {} v{} (system package - verified)",
                                            "✓".green(),
                                            pkg.name,
                                            pkg.version
                                        );
                                        continue;
                                    } else {
                                        println!(
                                            "\n  {} {} v{} (system package NOT found)",
                                            "⚠".yellow(),
                                            pkg.name,
                                            pkg.version
                                        );

                                        // Prompt user for what to do
                                        match prompt_missing_system_package(&pkg.name)? {
                                            MissingSystemChoice::InstallGlobal => {
                                                println!(
                                                    "  {} Installing to HORUS global cache...",
                                                    "↓".cyan()
                                                );
                                                client
                                                    .install_to_target(
                                                        &pkg.name,
                                                        Some(&pkg.version),
                                                        workspace::InstallTarget::Global,
                                                    )
                                                    .map_err(|e| {
                                                        HorusError::Config(e.to_string())
                                                    })?;
                                            }
                                            MissingSystemChoice::InstallLocal => {
                                                println!(
                                                    "  {} Installing to HORUS local...",
                                                    "↓".cyan()
                                                );
                                                client
                                                    .install(&pkg.name, Some(&pkg.version))
                                                    .map_err(|e| {
                                                        HorusError::Config(e.to_string())
                                                    })?;
                                            }
                                            MissingSystemChoice::Skip => {
                                                println!("  {} Skipped {}", "⊘".yellow(), pkg.name);
                                                continue;
                                            }
                                        }
                                    }
                                }
                                registry::PackageSource::Path { path } => {
                                    println!("  {} {} (path dependency)", "⚠".yellow(), pkg.name);
                                    println!("    Path: {}", path);
                                    println!("    {} Path dependencies are not portable across machines.", "Note:".dimmed());
                                    println!("    {} Please update horus.yaml with the correct path if needed.", "Tip:".dimmed());
                                    // Don't try to install - user must fix path manually
                                    continue;
                                }
                                _ => {
                                    // Registry, PyPI, CratesIO - use standard install
                                    println!("  Installing {} v{}...", pkg.name, pkg.version);
                                    client
                                        .install(&pkg.name, Some(&pkg.version))
                                        .map_err(|e| HorusError::Config(e.to_string()))?;
                                }
                            }

                            // Update horus.yaml if in a workspace
                            if let Some(ref ws_path) = workspace_path {
                                let yaml_path = ws_path.join("horus.yaml");
                                if yaml_path.exists() {
                                    if let Err(e) =
                                        horus_manager::yaml_utils::add_dependency_to_horus_yaml(
                                            &yaml_path,
                                            &pkg.name,
                                            &pkg.version,
                                        )
                                    {
                                        eprintln!(
                                            "  {} Failed to update horus.yaml: {}",
                                            "".yellow(),
                                            e
                                        );
                                    }
                                }
                            }
                        }

                        println!(" Environment restored from {}", source);
                        println!("   ID: {}", manifest.horus_id);
                        println!("   Packages: {}", manifest.packages.len());
                    } else {
                        // It's an environment ID from registry
                        // Fetch manifest and install manually to update horus.yaml
                        println!(" Fetching environment {}...", source);

                        let url = format!("{}/api/environments/{}", client.base_url(), source);

                        let response = client
                            .http_client()
                            .get(&url)
                            .send()
                            .map_err(|e| HorusError::Config(e.to_string()))?;

                        if !response.status().is_success() {
                            return Err(HorusError::Config(format!(
                                "Environment not found: {}",
                                source
                            )));
                        }

                        let manifest: registry::EnvironmentManifest = response
                            .json()
                            .map_err(|e| HorusError::Config(e.to_string()))?;

                        println!(" Found {} packages to restore", manifest.packages.len());

                        // Get workspace path for horus.yaml updates
                        let workspace_path = workspace::find_workspace_root();

                        // Install each package
                        for pkg in &manifest.packages {
                            // Handle different package sources
                            match &pkg.source {
                                registry::PackageSource::System => {
                                    // Check if system package actually exists
                                    let exists = check_system_package_exists(&pkg.name);

                                    if exists {
                                        println!(
                                            "  {} {} v{} (system package - verified)",
                                            "✓".green(),
                                            pkg.name,
                                            pkg.version
                                        );
                                        continue;
                                    } else {
                                        println!(
                                            "\n  {} {} v{} (system package NOT found)",
                                            "⚠".yellow(),
                                            pkg.name,
                                            pkg.version
                                        );

                                        // Prompt user for what to do
                                        match prompt_missing_system_package(&pkg.name)? {
                                            MissingSystemChoice::InstallGlobal => {
                                                println!(
                                                    "  {} Installing to HORUS global cache...",
                                                    "↓".cyan()
                                                );
                                                client
                                                    .install_to_target(
                                                        &pkg.name,
                                                        Some(&pkg.version),
                                                        workspace::InstallTarget::Global,
                                                    )
                                                    .map_err(|e| {
                                                        HorusError::Config(e.to_string())
                                                    })?;
                                            }
                                            MissingSystemChoice::InstallLocal => {
                                                println!(
                                                    "  {} Installing to HORUS local...",
                                                    "↓".cyan()
                                                );
                                                client
                                                    .install(&pkg.name, Some(&pkg.version))
                                                    .map_err(|e| {
                                                        HorusError::Config(e.to_string())
                                                    })?;
                                            }
                                            MissingSystemChoice::Skip => {
                                                println!("  {} Skipped {}", "⊘".yellow(), pkg.name);
                                                continue;
                                            }
                                        }
                                    }
                                }
                                registry::PackageSource::Path { path } => {
                                    println!("  {} {} (path dependency)", "⚠".yellow(), pkg.name);
                                    println!("    Path: {}", path);
                                    println!("    {} Path dependencies are not portable across machines.", "Note:".dimmed());
                                    println!("    {} Please update horus.yaml with the correct path if needed.", "Tip:".dimmed());
                                    // Don't try to install - user must fix path manually
                                    continue;
                                }
                                _ => {
                                    // Registry, PyPI, CratesIO - use standard install
                                    println!("  Installing {} v{}...", pkg.name, pkg.version);
                                    client
                                        .install(&pkg.name, Some(&pkg.version))
                                        .map_err(|e| HorusError::Config(e.to_string()))?;
                                }
                            }

                            // Update horus.yaml if in a workspace
                            if let Some(ref ws_path) = workspace_path {
                                let yaml_path = ws_path.join("horus.yaml");
                                if yaml_path.exists() {
                                    if let Err(e) =
                                        horus_manager::yaml_utils::add_dependency_to_horus_yaml(
                                            &yaml_path,
                                            &pkg.name,
                                            &pkg.version,
                                        )
                                    {
                                        eprintln!(
                                            "  {} Failed to update horus.yaml: {}",
                                            "".yellow(),
                                            e
                                        );
                                    }
                                }
                            }
                        }

                        println!(" Environment {} restored successfully!", source);
                    }

                    Ok(())
                }
            }
        }

        Commands::Auth { command } => match command {
            AuthCommands::Login => commands::github_auth::login(),
            AuthCommands::GenerateKey { name, environment } => {
                commands::github_auth::generate_key(name, environment)
            }
            AuthCommands::Logout => commands::github_auth::logout(),
            AuthCommands::Whoami => commands::github_auth::whoami(),
        },

        Commands::Sim { command } => match command {
            SimCommands::Sim2d {
                world,
                world_image,
                resolution,
                threshold,
                robot,
                topic,
                name,
                headless,
            } => {
                use std::env;
                use std::process::Command;

                println!("{} Starting sim2d...", "🎮".cyan());
                if headless {
                    println!("  Mode: Headless (no GUI)");
                }
                println!("  Topic: {}", topic);
                println!("  Robot name: {}", name);
                if let Some(ref world_path) = world {
                    println!("  World: {}", world_path.display());
                }
                if let Some(ref robot_path) = robot {
                    println!("  Robot config: {}", robot_path.display());
                }
                println!();

                // Find sim2d path relative to HORUS repo
                let horus_source = env::var("HORUS_SOURCE")
                    .or_else(|_| env::var("HOME").map(|h| format!("{}/.horus/cache/HORUS", h)))
                    .unwrap_or_else(|_| ".".to_string());

                let sim2d_path = format!("{}/horus_library/tools/sim2d", horus_source);

                // Build cargo run command with arguments
                let mut cmd = Command::new("cargo");
                cmd.current_dir(&sim2d_path)
                    .arg("run")
                    .arg("--release")
                    .arg("--");

                // Add optional arguments
                if let Some(ref world_path) = world {
                    cmd.arg("--world").arg(world_path);
                }
                if let Some(ref robot_path) = robot {
                    cmd.arg("--robot").arg(robot_path);
                }
                cmd.arg("--topic").arg(&topic);
                cmd.arg("--name").arg(&name);
                if headless {
                    cmd.arg("--headless");
                }

                println!("{} Launching sim2d...", "▶".green());
                println!();

                // Try to run pre-built binary first (fast path)
                let sim2d_binary = env::var("HOME")
                    .map(|h| format!("{}/.cargo/bin/sim2d", h))
                    .unwrap_or_else(|_| "sim2d".to_string());

                let status = if std::path::Path::new(&sim2d_binary).exists() {
                    // Run pre-built binary directly (instant launch!)
                    let mut binary_cmd = Command::new(&sim2d_binary);

                    // Add arguments
                    if let Some(ref w) = world {
                        binary_cmd.arg("--world").arg(w);
                    }
                    if let Some(ref w) = world_image {
                        binary_cmd.arg("--world_image").arg(w);
                        if let Some(res) = resolution {
                            binary_cmd.arg("--resolution").arg(res.to_string());
                        }
                        if let Some(thresh) = threshold {
                            binary_cmd.arg("--threshold").arg(thresh.to_string());
                        }
                    }
                    if let Some(ref r) = robot {
                        binary_cmd.arg("--robot").arg(r);
                    }
                    binary_cmd.arg("--topic").arg(&topic);
                    binary_cmd.arg("--name").arg(&name);
                    if headless {
                        binary_cmd.arg("--headless");
                    }

                    binary_cmd
                        .status()
                        .map_err(|e| HorusError::Config(format!("Failed to run sim2d: {}", e)))?
                } else {
                    // Fallback: compile and run from source
                    println!(
                        "{} Pre-built binary not found, compiling from source...",
                        "⚠️".yellow()
                    );
                    cmd.status()
                        .map_err(|e| HorusError::Config(format!("Failed to run sim2d: {}. Try running manually: cd {} && cargo run --release", e, sim2d_path)))?
                };

                // Execute and wait (reuse status variable from above)

                if !status.success() {
                    return Err(HorusError::Config(format!(
                        "sim2d exited with error code: {:?}",
                        status.code()
                    )));
                }

                Ok(())
            }
            SimCommands::Sim3d { headless, seed } => {
                println!("{} Starting sim3d...", "🎮".cyan());
                if headless {
                    println!("  Mode: Headless");
                }
                if let Some(s) = seed {
                    println!("  Seed: {}", s);
                }
                println!("\n{}", "⚠️  sim3d is planned for future release!".yellow());
                println!("See roadmap: https://docs.horus-registry.dev/roadmap");
                Ok(())
            }
        },

        Commands::Completion { shell } => {
            // Hidden command used by install.sh for automatic completion setup
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            generate(shell, &mut cmd, bin_name, &mut io::stdout());
            Ok(())
        }
    }
}

// Helper functions for system package detection during restore

#[derive(Debug, Clone, PartialEq)]
enum MissingSystemChoice {
    InstallGlobal,
    InstallLocal,
    Skip,
}

fn check_system_package_exists(package_name: &str) -> bool {
    use std::process::Command;

    // Try Python package detection
    let py_check = Command::new("python3")
        .args(&["-m", "pip", "show", package_name])
        .output();

    if let Ok(output) = py_check {
        if output.status.success() {
            return true;
        }
    }

    // Try Rust binary detection
    if let Some(home) = dirs::home_dir() {
        let cargo_bin = home.join(".cargo/bin").join(package_name);
        if cargo_bin.exists() {
            return true;
        }
    }

    false
}

fn prompt_missing_system_package(package_name: &str) -> Result<MissingSystemChoice, HorusError> {
    use std::io::{self, Write};

    println!(
        "\n  System package '{}' was expected but not found.",
        package_name
    );
    println!("  What would you like to do?");
    println!("    [1] Install to HORUS global cache (shared across projects)");
    println!("    [2] Install to HORUS local (this project only)");
    println!("    [3] Skip (you will install it manually later)");

    print!("\n  Choice [1-3]: ");
    io::stdout()
        .flush()
        .map_err(|e| HorusError::Config(e.to_string()))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| HorusError::Config(e.to_string()))?;

    match input.trim() {
        "1" => Ok(MissingSystemChoice::InstallGlobal),
        "2" => Ok(MissingSystemChoice::InstallLocal),
        "3" => Ok(MissingSystemChoice::Skip),
        _ => {
            println!("  Invalid choice, defaulting to Skip");
            Ok(MissingSystemChoice::Skip)
        }
    }
}
