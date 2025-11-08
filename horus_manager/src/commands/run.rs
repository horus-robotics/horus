use crate::dependency_resolver::DependencySpec;
use crate::version;
use anyhow::{anyhow, bail, Context, Result};
use colored::*;
use glob::glob;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
enum ExecutionTarget {
    File(PathBuf),
    Directory(PathBuf),
    Manifest(PathBuf),
    Multiple(Vec<PathBuf>),
}

/// Python package dependency for pip
#[derive(Debug, Clone)]
struct PipPackage {
    name: String,
    version: Option<String>, // None means latest
}

impl PipPackage {
    fn from_string(s: &str) -> Result<Self> {
        // Parse formats:
        // - "numpy>=1.24.0"
        // - "numpy==1.24.0"
        // - "numpy~=1.24"
        // - "numpy@1.24.0" (HORUS-style)
        // - "numpy"

        let s = s.trim();

        // Handle @ separator (HORUS-style: numpy@1.24.0)
        if let Some(at_pos) = s.find('@') {
            let name = s[..at_pos].trim().to_string();
            let version_str = s[at_pos + 1..].trim();
            let version = if !version_str.is_empty() {
                Some(format!("=={}", version_str))
            } else {
                None
            };
            return Ok(PipPackage { name, version });
        }

        // Handle comparison operators (>=, ==, ~=, etc.)
        let operators = [">=", "<=", "==", "~=", ">", "<", "!="];
        for op in &operators {
            if let Some(op_pos) = s.find(op) {
                let name = s[..op_pos].trim().to_string();
                let version = Some(s[op_pos..].trim().to_string());
                return Ok(PipPackage { name, version });
            }
        }

        // No version specified
        Ok(PipPackage {
            name: s.to_string(),
            version: None,
        })
    }

    fn requirement_string(&self) -> String {
        match &self.version {
            Some(v) => format!("{}{}", self.name, v),
            None => self.name.clone(),
        }
    }
}

/// Cargo package dependency for Rust binaries
#[derive(Debug, Clone)]
struct CargoPackage {
    name: String,
    version: Option<String>, // None means latest
    features: Vec<String>,   // Cargo features to enable
}

impl CargoPackage {
    fn from_string(s: &str) -> Result<Self> {
        // Parse formats: "bat@0.24.0:features=derive,serde" or "bat@0.24.0" or "bat"
        let s = s.trim();

        // Check for features
        let (pkg_part, features) = if let Some(colon_pos) = s.find(':') {
            let pkg = &s[..colon_pos];
            let features_part = &s[colon_pos + 1..];

            // Parse features=a,b,c
            let features = if let Some(equals_pos) = features_part.find('=') {
                let features_str = &features_part[equals_pos + 1..].trim();
                if features_str.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Empty features list (remove ':features=' or provide feature names)"
                    ));
                }
                features_str
                    .split(',')
                    .map(|f| f.trim().to_string())
                    .filter(|f| !f.is_empty()) // Filter out empty strings
                    .collect()
            } else if !features_part.is_empty() {
                return Err(anyhow::anyhow!(
                    "Invalid features syntax '{}' - use 'features=feat1,feat2'",
                    features_part
                ));
            } else {
                Vec::new()
            };
            (pkg, features)
        } else {
            (s, Vec::new())
        };

        // Parse name@version
        if let Some(at_pos) = pkg_part.find('@') {
            let name = pkg_part[..at_pos].trim().to_string();
            let version_str = pkg_part[at_pos + 1..].trim();

            // Validate package name
            if name.is_empty() {
                return Err(anyhow::anyhow!("Package name cannot be empty"));
            }

            let version = if !version_str.is_empty() {
                // Basic semver validation
                if version_str.contains(char::is_whitespace) {
                    return Err(anyhow::anyhow!(
                        "Version cannot contain whitespace: '{}'",
                        version_str
                    ));
                }
                // Check for common mistakes
                if version_str == "latest" {
                    return Err(anyhow::anyhow!("Version 'latest' is not valid - specify a version like '1.0' or omit version"));
                }
                Some(version_str.to_string())
            } else {
                None
            };
            return Ok(CargoPackage {
                name,
                version,
                features,
            });
        }

        // No version specified
        let name = pkg_part.trim().to_string();
        if name.is_empty() {
            return Err(anyhow::anyhow!("Package name cannot be empty"));
        }

        Ok(CargoPackage {
            name,
            version: None,
            features,
        })
    }

    #[allow(dead_code)]
    fn crate_spec(&self) -> String {
        match &self.version {
            Some(v) => format!("{}@{}", self.name, v),
            None => self.name.clone(),
        }
    }
}

pub fn execute_build_only(file: Option<PathBuf>, release: bool, clean: bool) -> Result<()> {
    // Handle clean build
    if clean {
        println!("{} Cleaning build cache...", "🧹".cyan());
        clean_build_cache()?;
    }

    let mode = if release { "release" } else { "debug" };
    println!(
        "{} Building project in {} mode (no execution)...",
        "".cyan(),
        mode.yellow()
    );

    // Resolve target file
    let target_file = match file {
        Some(f) => f,
        None => auto_detect_main_file()?,
    };

    let language = detect_language(&target_file)?;
    println!(
        "{} Detected: {} ({})",
        "".cyan(),
        target_file.display().to_string().green(),
        language.yellow()
    );

    // Ensure .horus directory exists
    ensure_horus_directory()?;

    // Run static analysis for Rust files
    if language == "rust" {
        use crate::static_analysis;
        // Non-fatal: warnings only, don't fail the build
        if let Err(e) = static_analysis::check_link_usage(&target_file) {
            eprintln!("⚠ Static analysis error: {}", e);
        }
    }

    // Build based on language
    match language.as_str() {
        "python" => {
            println!("{} Python is interpreted, no build needed", "[i]".blue());
            println!(
                "  {} File is ready to run: {}",
                "".cyan(),
                target_file.display()
            );
        }
        "c" => {
            setup_c_environment()?;

            // Determine output path
            let file_stem = target_file
                .file_stem()
                .context("Invalid file name")?
                .to_string_lossy();
            let suffix = if release { "_release" } else { "_debug" };
            let output_path = PathBuf::from(format!(".horus/cache/{}{}", file_stem, suffix));

            // Detect compiler
            let compiler = if Command::new("gcc").arg("--version").output().is_ok() {
                "gcc"
            } else if Command::new("clang").arg("--version").output().is_ok() {
                "clang"
            } else {
                bail!("No C compiler found. Please install gcc or clang.");
            };

            println!("{} Compiling with {}...", "".cyan(), compiler);
            compile_c_file(&target_file, &output_path, compiler, release)?;
            println!(
                "{} Successfully built: {}",
                "".green(),
                output_path.display().to_string().green()
            );
        }
        "rust" => {
            // Setup Rust build using Cargo in .horus workspace
            println!("{} Setting up Cargo workspace...", "".cyan());

            // Parse horus.yaml to get dependencies
            let dependencies = if Path::new("horus.yaml").exists() {
                parse_horus_yaml_dependencies("horus.yaml")?
            } else {
                HashSet::new()
            };

            // Split dependencies into HORUS packages, pip packages, cargo packages, and path dependencies
            let (horus_deps, _pip_packages, cargo_packages, path_deps) =
                split_dependencies_with_path_context(dependencies.clone(), Some("rust"));

            // Generate Cargo.toml in .horus/ that references source files in parent directory
            let cargo_toml_path = PathBuf::from(".horus/Cargo.toml");

            // Get relative path from .horus/ to the source file
            let source_relative_path = format!("../{}", target_file.display());

            let mut cargo_toml = format!(
                r#"[package]
name = "horus-project"
version = "0.1.0"
edition = "2021"

# Empty workspace to prevent inheriting parent workspace
[workspace]

[[bin]]
name = "horus-project"
path = "{}"

[dependencies]
"#,
                source_relative_path
            );

            // Find HORUS source directory
            let horus_source = find_horus_source_dir()?;
            println!(
                "  {} Using HORUS source: {}",
                "".cyan(),
                horus_source.display()
            );

            // Add HORUS dependencies from source
            for dep in &horus_deps {
                // Strip version from dependency name for path lookup
                let dep_name = if let Some(at_pos) = dep.find('@') {
                    &dep[..at_pos]
                } else {
                    dep.as_str()
                };

                let dep_path = horus_source.join(dep_name);

                if dep_path.exists() && dep_path.join("Cargo.toml").exists() {
                    cargo_toml.push_str(&format!(
                        "{} = {{ path = \"{}\" }}\n",
                        dep_name,
                        dep_path.display()
                    ));
                    println!(
                        "  {} Added dependency: {} -> {}",
                        "".cyan(),
                        dep,
                        dep_path.display()
                    );
                } else {
                    eprintln!(
                        "  {} Warning: dependency {} not found at {}",
                        "".yellow(),
                        dep,
                        dep_path.display()
                    );
                }
            }

            // Add cargo dependencies from crates.io
            for pkg in &cargo_packages {
                if !pkg.features.is_empty() {
                    // With features
                    if let Some(ref version) = pkg.version {
                        cargo_toml.push_str(&format!(
                            "{} = {{ version = \"{}\", features = [{}] }}\n",
                            pkg.name,
                            version,
                            pkg.features
                                .iter()
                                .map(|f| format!("\"{}\"", f))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    } else {
                        cargo_toml.push_str(&format!(
                            "{} = {{ version = \"*\", features = [{}] }}\n",
                            pkg.name,
                            pkg.features
                                .iter()
                                .map(|f| format!("\"{}\"", f))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                        eprintln!(
                            "  {} Warning: Using wildcard version for '{}' - specify a version for reproducibility",
                            "".yellow(),
                            pkg.name
                        );
                    }
                    println!(
                        "  {} Added crates.io dependency: {} (features: {})",
                        "".cyan(),
                        pkg.name,
                        pkg.features.join(", ")
                    );
                } else if let Some(ref version) = pkg.version {
                    cargo_toml.push_str(&format!("{} = \"{}\"\n", pkg.name, version));
                    println!(
                        "  {} Added crates.io dependency: {}@{}",
                        "".cyan(),
                        pkg.name,
                        version
                    );
                } else {
                    cargo_toml.push_str(&format!("{} = \"*\"\n", pkg.name));
                    eprintln!(
                        "  {} Warning: Using wildcard version for '{}' - specify a version for reproducibility",
                        "".yellow(),
                        pkg.name
                    );
                    eprintln!("     Example: 'cargo:{}@1.0' in horus.yaml", pkg.name);
                    println!("  {} Added crates.io dependency: {}", "".cyan(), pkg.name);
                }
            }

            // Add path dependencies
            for (pkg_name, pkg_path) in &path_deps {
                // Convert relative path from current directory to relative from .horus/
                let full_path = PathBuf::from("..").join(pkg_path);
                cargo_toml.push_str(&format!(
                    "{} = {{ path = \"{}\" }}\n",
                    pkg_name,
                    full_path.display()
                ));
                println!(
                    "  {} Added path dependency: {} -> {}",
                    "".cyan(),
                    pkg_name,
                    pkg_path
                );
            }

            // Also add dependencies directly from horus.yaml (in case some weren't parsed by resolve_dependencies)
            if Path::new("horus.yaml").exists() {
                if let Ok(yaml_content) = fs::read_to_string("horus.yaml") {
                    if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&yaml_content) {
                        if let Some(deps_value) = yaml.get("dependencies") {
                            if let serde_yaml::Value::Sequence(list) = deps_value {
                                for item in list {
                                    if let Some(dep_str) = parse_yaml_cargo_dependency(item) {
                                        cargo_toml.push_str(&format!("{}\n", dep_str));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            fs::write(&cargo_toml_path, cargo_toml)?;
            println!(
                "  {} Generated Cargo.toml (no source copying needed)",
                "".green()
            );

            // Run cargo build in .horus directory
            println!("{} Building with cargo...", "".cyan());
            let mut cmd = Command::new("cargo");
            cmd.arg("build");
            cmd.current_dir(".horus");

            if release {
                cmd.arg("--release");
            }

            let status = cmd.status()?;
            if !status.success() {
                bail!("Cargo build failed");
            }

            let profile = if release { "release" } else { "debug" };
            let binary_path = format!(".horus/target/{}/horus-project", profile);

            println!("{} Successfully built: {}", "".green(), binary_path.green());
        }
        _ => bail!("Unsupported language: {}", language),
    }

    Ok(())
}

pub fn execute_run(
    file: Option<PathBuf>,
    args: Vec<String>,
    release: bool,
    clean: bool,
) -> Result<()> {
    // Handle clean build
    if clean {
        eprintln!("{} Cleaning build cache...", "🧹".cyan());
        clean_build_cache()?;
    }

    let mode = if release { "release" } else { "debug" };
    eprintln!(
        "{} Starting HORUS runtime in {} mode...",
        "".cyan(),
        mode.yellow()
    );

    // Step 1: Resolve target(s) - file, directory, or pattern
    let execution_targets = match file {
        Some(f) => resolve_execution_target(f)?,
        None => vec![ExecutionTarget::File(auto_detect_main_file()?)],
    };

    // Step 2: Execute based on target type
    for target in execution_targets {
        match target {
            ExecutionTarget::File(file_path) => {
                execute_single_file(file_path, args.clone(), release, clean)?;
            }
            ExecutionTarget::Directory(dir_path) => {
                execute_directory(dir_path, args.clone(), release, clean)?;
            }
            ExecutionTarget::Manifest(manifest_path) => {
                execute_from_manifest(manifest_path, args.clone(), release, clean)?;
            }
            ExecutionTarget::Multiple(file_paths) => {
                execute_multiple_files(file_paths, args.clone(), release, clean)?;
            }
        }
    }

    Ok(())
}

fn execute_single_file(
    file_path: PathBuf,
    args: Vec<String>,
    release: bool,
    clean: bool,
) -> Result<()> {
    // Try to read session_id from horus.yaml, otherwise generate unique session ID
    let session_id = parse_session_id_from_horus_yaml("horus.yaml")
        .ok()
        .flatten()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    env::set_var("HORUS_SESSION_ID", &session_id);

    let language = detect_language(&file_path)?;

    eprintln!(
        "{} Detected: {} ({})",
        "".cyan(),
        file_path.display().to_string().green(),
        language.yellow()
    );
    eprintln!("  {} Session: {}", "".dimmed(), session_id.dimmed());

    // Load ignore patterns from horus.yaml if it exists
    let ignore = if Path::new("horus.yaml").exists() {
        parse_horus_yaml_ignore("horus.yaml").unwrap_or_default()
    } else {
        IgnorePatterns::default()
    };

    // Ensure .horus directory exists
    ensure_horus_directory()?;

    // Scan imports and resolve dependencies
    eprintln!("{} Scanning imports...", "".cyan());
    let dependencies = scan_imports(&file_path, &language, &ignore)?;

    // Run static analysis for Rust files
    if language == "rust" {
        use crate::static_analysis;
        // Non-fatal: warnings only, don't fail the build
        if let Err(e) = static_analysis::check_link_usage(&file_path) {
            eprintln!("⚠ Static analysis error: {}", e);
        }
    }

    if !dependencies.is_empty() {
        eprintln!("{} Found {} dependencies", "".cyan(), dependencies.len());

        // For Rust files, cargo dependencies are handled in Cargo.toml generation
        // So we filter them out here to avoid trying to `cargo install` library crates
        let dependencies_to_resolve = if language == "rust" {
            let (horus_pkgs, pip_pkgs, _cargo_pkgs) =
                split_dependencies_with_context(dependencies.clone(), Some(&language));
            // Reconstruct set with only HORUS and pip packages
            horus_pkgs
                .into_iter()
                .chain(pip_pkgs.into_iter().map(|p| {
                    if let Some(ref v) = p.version {
                        format!("pip:{}=={}", p.name, v)
                    } else {
                        format!("pip:{}", p.name)
                    }
                }))
                .collect()
        } else {
            dependencies
        };

        if !dependencies_to_resolve.is_empty() {
            resolve_dependencies(dependencies_to_resolve)?;
        }
    }

    // Setup environment
    setup_environment()?;

    // Execute
    eprintln!("{} Executing...\n", "".cyan());
    execute_with_scheduler(file_path, language, args, release, clean)?;

    // Clean up session directory
    cleanup_session(&session_id)?;

    Ok(())
}

fn execute_directory(
    dir_path: PathBuf,
    args: Vec<String>,
    release: bool,
    clean: bool,
) -> Result<()> {
    println!(
        "{} Executing from directory: {}",
        "".cyan(),
        dir_path.display().to_string().green()
    );

    let original_dir = env::current_dir()?;

    // Change to target directory
    env::set_current_dir(&dir_path).context(format!(
        "Failed to change to directory: {}",
        dir_path.display()
    ))?;

    let result = (|| -> Result<()> {
        // Auto-detect main file in this directory
        let main_file = auto_detect_main_file().context(format!(
            "No main file found in directory: {}",
            dir_path.display()
        ))?;

        // Execute the file in this context
        execute_single_file(main_file, args, release, clean)?;

        Ok(())
    })();

    // Always restore original directory
    env::set_current_dir(original_dir)?;

    result
}

fn execute_from_manifest(
    manifest_path: PathBuf,
    args: Vec<String>,
    release: bool,
    clean: bool,
) -> Result<()> {
    println!(
        "{} Executing from manifest: {}",
        "".cyan(),
        manifest_path.display().to_string().green()
    );

    match manifest_path.file_name().and_then(|s| s.to_str()) {
        Some("horus.yaml") => execute_from_horus_yaml(manifest_path, args, release, clean),
        Some("Cargo.toml") => execute_from_cargo_toml(manifest_path, args, release, clean),
        _ => bail!("Unsupported manifest type: {}", manifest_path.display()),
    }
}

fn execute_from_horus_yaml(
    manifest_path: PathBuf,
    args: Vec<String>,
    release: bool,
    clean: bool,
) -> Result<()> {
    // For now, find the main file in the same directory as horus.yaml
    let project_dir = manifest_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine project directory"))?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(project_dir)?;

    let result = (|| -> Result<()> {
        // Check if this is a C/C++ project with build system
        if Path::new("Makefile").exists() || Path::new("makefile").exists() {
            return execute_makefile_project(args, release, clean);
        }
        if Path::new("CMakeLists.txt").exists() {
            return execute_cmake_project(args, release, clean);
        }

        // Otherwise, auto-detect and run main file
        let main_file =
            auto_detect_main_file().context("No main file found in project directory")?;
        execute_single_file(main_file, args, release, clean)
    })();

    env::set_current_dir(original_dir)?;
    result
}

fn execute_from_cargo_toml(
    manifest_path: PathBuf,
    args: Vec<String>,
    release: bool,
    clean: bool,
) -> Result<()> {
    // Change to the directory containing Cargo.toml
    let project_dir = manifest_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine project directory"))?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(project_dir)?;

    let result = (|| -> Result<()> {
        // Ensure .horus directory exists
        ensure_horus_directory()?;

        // Parse Cargo.toml for HORUS dependencies
        println!("{} Scanning Cargo.toml dependencies...", "".cyan());
        let horus_deps = parse_cargo_dependencies("Cargo.toml")?;

        if !horus_deps.is_empty() {
            println!(
                "{} Found {} HORUS dependencies",
                "".cyan(),
                horus_deps.len()
            );
            resolve_dependencies(horus_deps)?;
        }

        // Setup environment with .horus libraries
        setup_environment()?;

        // For Rust projects, run cargo directly
        let project_name = get_project_name()?;
        let build_dir = if release { "release" } else { "debug" };
        let binary = format!("target/{}/{}", build_dir, project_name);

        if !Path::new(&binary).exists() || clean {
            println!(
                "{} Building Cargo project ({} mode)...",
                "".cyan(),
                build_dir
            );
            let mut cmd = Command::new("cargo");
            cmd.arg("build");
            if release {
                cmd.arg("--release");
            }

            let status = cmd.status()?;
            if !status.success() {
                bail!("Build failed");
            }
        }

        // Run the binary with environment
        println!("{} Executing Cargo project...\n", "".cyan());
        let mut cmd = Command::new(binary);
        cmd.args(args);
        let status = cmd.status()?;
        if !status.success() {
            bail!("Execution failed");
        }

        Ok(())
    })();

    env::set_current_dir(original_dir)?;
    result
}

fn execute_makefile_project(args: Vec<String>, release: bool, clean: bool) -> Result<()> {
    println!("{} Detected Makefile project", "".cyan());

    // Ensure .horus directory exists
    ensure_horus_directory()?;

    // Setup environment with .horus libraries
    setup_environment()?;

    // Clean if requested
    if clean {
        println!("{} Cleaning Makefile project...", "".cyan());
        Command::new("make").arg("clean").status().ok();
    }

    // Build the project
    let build_target = if release { "release" } else { "all" };
    println!(
        "{} Building Makefile project (target: {})...",
        "".cyan(),
        build_target
    );

    let mut cmd = Command::new("make");
    cmd.arg(build_target);

    let status = cmd.status()?;
    if !status.success() {
        bail!("Make build failed");
    }

    // Try to find and run the executable
    // Common patterns: ./bin/main, ./build/main, ./main
    let possible_executables = vec!["bin/main", "build/main", "main", "a.out"];

    for exe in &possible_executables {
        if Path::new(exe).exists() {
            println!("{} Running executable: {}\n", "".cyan(), exe.green());
            let mut cmd = Command::new(format!("./{}", exe));
            cmd.args(args);
            let status = cmd.status()?;
            if !status.success() {
                bail!("Execution failed");
            }
            return Ok(());
        }
    }

    println!(
        "{} Build succeeded but could not find executable",
        "".yellow()
    );
    println!("  {} Looked for: {:?}", "".dimmed(), possible_executables);
    Ok(())
}

fn execute_cmake_project(args: Vec<String>, release: bool, clean: bool) -> Result<()> {
    println!("{} Detected CMake project", "".cyan());

    // Ensure .horus directory exists
    ensure_horus_directory()?;

    // Setup environment with .horus libraries
    setup_environment()?;

    let build_dir = PathBuf::from("build");

    // Clean if requested
    if clean && build_dir.exists() {
        println!("{} Cleaning CMake build directory...", "".cyan());
        fs::remove_dir_all(&build_dir)?;
    }

    // Create build directory
    fs::create_dir_all(&build_dir)?;

    // Configure with CMake
    let build_type = if release { "Release" } else { "Debug" };
    println!("{} Configuring CMake ({} mode)...", "".cyan(), build_type);

    let mut cmd = Command::new("cmake");
    cmd.arg("..")
        .arg(format!("-DCMAKE_BUILD_TYPE={}", build_type))
        .current_dir(&build_dir);

    let status = cmd.status()?;
    if !status.success() {
        bail!("CMake configuration failed");
    }

    // Build
    println!("{} Building CMake project...", "".cyan());
    let mut cmd = Command::new("cmake");
    cmd.arg("--build").arg(".").current_dir(&build_dir);

    let status = cmd.status()?;
    if !status.success() {
        bail!("CMake build failed");
    }

    // Try to find and run the executable
    let possible_executables = vec![
        format!("build/{}", get_cmake_target_name()?),
        "build/main".to_string(),
        "build/app".to_string(),
    ];

    for exe in &possible_executables {
        if Path::new(exe).exists() {
            println!("{} Running executable: {}\n", "".cyan(), exe.green());
            let mut cmd = Command::new(format!("./{}", exe));
            cmd.args(args);
            let status = cmd.status()?;
            if !status.success() {
                bail!("Execution failed");
            }
            return Ok(());
        }
    }

    println!(
        "{} Build succeeded but could not find executable",
        "".yellow()
    );
    println!("  {} Looked for: {:?}", "".dimmed(), possible_executables);
    Ok(())
}

fn get_cmake_target_name() -> Result<String> {
    // Try to parse CMakeLists.txt for project name
    if let Ok(content) = fs::read_to_string("CMakeLists.txt") {
        for line in content.lines() {
            if line.trim().starts_with("project(") {
                if let Some(name_start) = line.find('(') {
                    if let Some(name_end) = line[name_start..].find(')') {
                        let name = line[name_start + 1..name_start + name_end]
                            .split_whitespace()
                            .next()
                            .unwrap_or("main");
                        return Ok(name.to_string());
                    }
                }
            }
            if line.trim().starts_with("add_executable(") {
                if let Some(name_start) = line.find('(') {
                    if let Some(name_end) = line[name_start..].find(')') {
                        let parts: Vec<&str> = line[name_start + 1..name_start + name_end]
                            .split_whitespace()
                            .collect();
                        if !parts.is_empty() {
                            return Ok(parts[0].to_string());
                        }
                    }
                }
            }
        }
    }
    Ok("main".to_string())
}

fn execute_multiple_files(
    file_paths: Vec<PathBuf>,
    args: Vec<String>,
    release: bool,
    clean: bool,
) -> Result<()> {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    // Generate unique session ID for this run
    let session_id = Uuid::new_v4().to_string();
    env::set_var("HORUS_SESSION_ID", &session_id);

    println!(
        "{} Executing {} files concurrently:",
        "".cyan(),
        file_paths.len()
    );
    println!("  {} Session: {}", "".dimmed(), session_id.dimmed());

    for (i, file_path) in file_paths.iter().enumerate() {
        let language = detect_language(file_path)?;
        println!(
            "  {} {} ({})",
            format!("{}.", i + 1).dimmed(),
            file_path.display().to_string().green(),
            language.yellow()
        );
    }

    // Phase 1: Build all files (batch Rust files for performance)
    println!("\n{} Phase 1: Building all files...", "".cyan());
    let mut executables = Vec::new();

    // Group files by language for optimized building
    let mut rust_files = Vec::new();
    let mut other_files = Vec::new();

    for file_path in &file_paths {
        let language = detect_language(file_path)?;
        if language == "rust" {
            rust_files.push(file_path.clone());
        } else {
            other_files.push((file_path.clone(), language));
        }
    }

    // Build all Rust files together in a single Cargo workspace (major optimization!)
    if !rust_files.is_empty() {
        if rust_files.len() == 1 {
            println!(
                "  {} Building {}...",
                "".cyan(),
                rust_files[0].display().to_string().green()
            );
        } else {
            println!(
                "  {} Building {} Rust files together (optimized)...",
                "".cyan(),
                rust_files.len().to_string().yellow()
            );
        }

        let rust_executables = build_rust_files_batch(rust_files, release, clean)?;
        executables.extend(rust_executables);
    }

    // Build other languages individually
    for (file_path, language) in other_files {
        println!(
            "  {} Building {}...",
            "".cyan(),
            file_path.display().to_string().green()
        );

        let exec_info = build_file_for_concurrent_execution(
            file_path, language, release, false, // Don't clean - already done if needed
        )?;

        executables.push(exec_info);
    }

    println!("{} All files built successfully!\n", "".green());

    // Phase 2: Execute all binaries concurrently
    println!("{} Phase 2: Starting all processes...", "".cyan());

    let running = Arc::new(AtomicBool::new(true));
    let children: Arc<Mutex<Vec<(String, std::process::Child)>>> = Arc::new(Mutex::new(Vec::new()));

    // Setup Ctrl+C handler with access to children
    let r = running.clone();
    let c = children.clone();
    ctrlc::set_handler(move || {
        println!("\n{} Shutting down all processes...", "".yellow());
        r.store(false, Ordering::SeqCst);

        // Kill all child processes
        if let Ok(mut children_lock) = c.lock() {
            for (name, child) in children_lock.iter_mut() {
                println!("  {} Terminating [{}]...", "".yellow(), name);
                let _ = child.kill();
            }
        }
    })
    .expect("Error setting Ctrl-C handler");

    let mut handles = Vec::new();

    // Spawn all processes
    for (i, exec_info) in executables.iter().enumerate() {
        let node_name = exec_info.name.clone();
        let color = get_color_for_index(i);

        let mut cmd = exec_info.create_command(&args);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => {
                // Handle stdout
                if let Some(stdout) = child.stdout.take() {
                    let name = node_name.clone();
                    let handle = std::thread::spawn(move || {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                println!("{} {}", format!("[{}]", name).color(color), line);
                            }
                        }
                    });
                    handles.push(handle);
                }

                // Handle stderr
                if let Some(stderr) = child.stderr.take() {
                    let name = node_name.clone();
                    let handle = std::thread::spawn(move || {
                        let reader = BufReader::new(stderr);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                eprintln!("{} {}", format!("[{}]", name).color(color), line);
                            }
                        }
                    });
                    handles.push(handle);
                }

                println!("  {} Started [{}]", "".green(), node_name.color(color));
                children.lock().unwrap().push((node_name, child));
            }
            Err(e) => {
                eprintln!("  {} Failed to start [{}]: {}", "".red(), node_name, e);
            }
        }
    }

    println!(
        "\n{} All processes running. Press Ctrl+C to stop.\n",
        "".green()
    );

    // Wait for all processes to complete (concurrent, checks running flag)
    loop {
        let mut all_done = true;
        let mut children_lock = children.lock().unwrap();

        // Check each child with try_wait (non-blocking)
        children_lock.retain_mut(|(name, child)| {
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Process exited
                    if !status.success() {
                        eprintln!(
                            "\n{} Process [{}] exited with code: {}",
                            "".yellow(),
                            name,
                            status.code().unwrap_or(-1)
                        );
                    }
                    false // Remove from list
                }
                Ok(None) => {
                    // Still running
                    all_done = false;
                    true // Keep in list
                }
                Err(e) => {
                    eprintln!("\n{} Error checking [{}]: {}", "".red(), name, e);
                    false // Remove from list
                }
            }
        });

        let still_running = !children_lock.is_empty();
        drop(children_lock);

        // Exit if all processes done or Ctrl+C was pressed and we killed them
        if all_done || (!running.load(Ordering::SeqCst) && !still_running) {
            break;
        }

        // Small sleep to avoid busy-waiting
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // Wait for output threads to finish
    for handle in handles {
        handle.join().ok();
    }

    if !running.load(Ordering::SeqCst) {
        println!("\n{} All processes stopped.", "".green());
    } else {
        println!("\n{} All processes completed.", "".green());
    }

    // Clean up session directory
    cleanup_session(&session_id)?;

    Ok(())
}

/// Clean up session-isolated shared memory directories
fn cleanup_session(session_id: &str) -> Result<()> {
    let session_dir = PathBuf::from(format!("/dev/shm/horus/sessions/{}", session_id));

    if session_dir.exists() {
        fs::remove_dir_all(&session_dir).with_context(|| {
            format!(
                "Failed to clean up session directory: {}",
                session_dir.display()
            )
        })?;
        println!("{} Cleaned up session memory", "".dimmed());
    }

    Ok(())
}

struct ExecutableInfo {
    name: String,
    command: String,
    args_override: Vec<String>,
}

impl ExecutableInfo {
    fn create_command(&self, user_args: &[String]) -> Command {
        let mut cmd = Command::new(&self.command);

        // Use override args if provided, otherwise use user args
        if !self.args_override.is_empty() {
            cmd.args(&self.args_override);
        } else {
            cmd.args(user_args);
        }

        cmd
    }
}

fn get_color_for_index(index: usize) -> &'static str {
    let colors = ["cyan", "green", "yellow", "magenta", "blue", "red"];
    colors[index % colors.len()]
}

/// Build multiple Rust files in a single Cargo workspace for optimal performance
fn build_rust_files_batch(
    file_paths: Vec<PathBuf>,
    release: bool,
    clean: bool,
) -> Result<Vec<ExecutableInfo>> {
    if file_paths.is_empty() {
        return Ok(Vec::new());
    }

    // Ensure .horus directory exists
    ensure_horus_directory()?;

    // Setup environment
    setup_environment()?;

    // Load ignore patterns from horus.yaml if it exists
    let ignore = if Path::new("horus.yaml").exists() {
        parse_horus_yaml_ignore("horus.yaml").unwrap_or_default()
    } else {
        IgnorePatterns::default()
    };

    // Find HORUS source directory
    let horus_source = find_horus_source_dir()?;

    // Collect all dependencies from all Rust files
    let mut all_dependencies = HashSet::new();
    for file_path in &file_paths {
        let dependencies = scan_imports(file_path, "rust", &ignore)?;
        all_dependencies.extend(dependencies);
    }

    // Resolve all dependencies once
    if !all_dependencies.is_empty() {
        resolve_dependencies(all_dependencies)?;
    }

    // Generate single Cargo.toml with multiple binary targets
    let cargo_toml_path = PathBuf::from(".horus/Cargo.toml");

    let mut cargo_toml = String::from(
        r#"[package]
name = "horus-multi-node"
version = "0.1.0"
edition = "2021"

"#,
    );

    // Add a [[bin]] entry for each Rust file
    let mut binary_names = Vec::new();
    for file_path in &file_paths {
        let name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("node")
            .to_string();

        let source_relative_path = format!("../{}", file_path.display());

        cargo_toml.push_str(&format!(
            r#"[[bin]]
name = "{}"
path = "{}"

"#,
            name, source_relative_path
        ));

        binary_names.push(name);
    }

    // Add dependencies section
    cargo_toml.push_str("[dependencies]\n");

    // Add HORUS core dependencies
    if horus_source.ends_with(".horus/cache") || horus_source.ends_with(".horus\\cache") {
        cargo_toml.push_str(&format!(
            "horus = {{ path = \"{}\" }}\n",
            horus_source.join("horus@0.1.0/horus").display()
        ));
        cargo_toml.push_str(&format!(
            "horus_library = {{ path = \"{}\" }}\n",
            horus_source.join("horus@0.1.0/horus_library").display()
        ));
    } else {
        cargo_toml.push_str(&format!(
            "horus = {{ path = \"{}\" }}\n",
            horus_source.join("horus").display()
        ));
        cargo_toml.push_str(&format!(
            "horus_library = {{ path = \"{}\" }}\n",
            horus_source.join("horus_library").display()
        ));
    }

    // Add dependencies from horus.yaml
    if Path::new("horus.yaml").exists() {
        if let Ok(yaml_content) = fs::read_to_string("horus.yaml") {
            if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&yaml_content) {
                if let Some(deps_value) = yaml.get("dependencies") {
                    if let serde_yaml::Value::Sequence(list) = deps_value {
                        for item in list {
                            if let Some(dep_str) = parse_yaml_cargo_dependency(item) {
                                cargo_toml.push_str(&format!("{}\n", dep_str));
                            }
                        }
                    }
                }
            }
        }
    }

    // Write the unified Cargo.toml
    fs::write(&cargo_toml_path, cargo_toml)?;

    // Clean if requested
    if clean {
        let mut clean_cmd = Command::new("cargo");
        clean_cmd.arg("clean").current_dir(".horus");
        clean_cmd.status().ok();
    }

    // Build all binaries with a single cargo build command
    let mut cmd = Command::new("cargo");
    cmd.arg("build").current_dir(".horus");
    if release {
        cmd.arg("--release");
    }

    let status = cmd.status()?;
    if !status.success() {
        bail!("Cargo build failed for batch Rust compilation");
    }

    // Create ExecutableInfo for each binary
    let profile = if release { "release" } else { "debug" };
    let mut executables = Vec::new();

    for name in binary_names {
        let binary_path = format!(".horus/target/{}/{}", profile, name);
        executables.push(ExecutableInfo {
            name,
            command: binary_path,
            args_override: Vec::new(),
        });
    }

    Ok(executables)
}

fn build_file_for_concurrent_execution(
    file_path: PathBuf,
    language: String,
    release: bool,
    clean: bool,
) -> Result<ExecutableInfo> {
    let name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("node")
        .to_string();

    // Ensure .horus directory exists
    ensure_horus_directory()?;

    // Load ignore patterns from horus.yaml if it exists
    let ignore = if Path::new("horus.yaml").exists() {
        parse_horus_yaml_ignore("horus.yaml").unwrap_or_default()
    } else {
        IgnorePatterns::default()
    };

    // Scan imports and resolve dependencies
    let dependencies = scan_imports(&file_path, &language, &ignore)?;
    if !dependencies.is_empty() {
        resolve_dependencies(dependencies)?;
    }

    // Setup environment
    setup_environment()?;

    match language.as_str() {
        "rust" => {
            // Build Rust file with Cargo
            let horus_source = find_horus_source_dir()?;
            let cargo_toml_path = PathBuf::from(".horus/Cargo.toml");
            let source_relative_path = format!("../{}", file_path.display());

            let mut cargo_toml = format!(
                r#"[package]
name = "horus-project-{}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{}"
path = "{}"

[dependencies]
"#,
                name, name, source_relative_path
            );

            // Add HORUS dependencies
            if horus_source.ends_with(".horus/cache") || horus_source.ends_with(".horus\\cache") {
                cargo_toml.push_str(&format!(
                    "horus = {{ path = \"{}\" }}\n",
                    horus_source.join("horus@0.1.0/horus").display()
                ));
                cargo_toml.push_str(&format!(
                    "horus_library = {{ path = \"{}\" }}\n",
                    horus_source.join("horus@0.1.0/horus_library").display()
                ));
            } else {
                cargo_toml.push_str(&format!(
                    "horus = {{ path = \"{}\" }}\n",
                    horus_source.join("horus").display()
                ));
                cargo_toml.push_str(&format!(
                    "horus_library = {{ path = \"{}\" }}\n",
                    horus_source.join("horus_library").display()
                ));
            }

            // Add dependencies from horus.yaml
            if Path::new("horus.yaml").exists() {
                if let Ok(yaml_content) = fs::read_to_string("horus.yaml") {
                    if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&yaml_content) {
                        if let Some(deps_value) = yaml.get("dependencies") {
                            if let serde_yaml::Value::Sequence(list) = deps_value {
                                for item in list {
                                    if let Some(dep_str) = parse_yaml_cargo_dependency(item) {
                                        cargo_toml.push_str(&format!("{}\n", dep_str));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            fs::write(&cargo_toml_path, cargo_toml)?;

            if clean {
                let mut clean_cmd = Command::new("cargo");
                clean_cmd.arg("clean").current_dir(".horus");
                clean_cmd.status().ok();
            }

            // Build with Cargo
            let mut cmd = Command::new("cargo");
            cmd.arg("build").current_dir(".horus");
            if release {
                cmd.arg("--release");
            }
            cmd.arg("--bin").arg(&name);

            let status = cmd.status()?;
            if !status.success() {
                bail!("Cargo build failed for {}", name);
            }

            let profile = if release { "release" } else { "debug" };
            let binary_path = format!(".horus/target/{}/{}", profile, name);

            Ok(ExecutableInfo {
                name,
                command: binary_path,
                args_override: Vec::new(),
            })
        }
        "python" => {
            // Python doesn't need building, just setup interpreter
            let python_cmd = detect_python_interpreter()?;
            setup_python_environment()?;

            Ok(ExecutableInfo {
                name,
                command: python_cmd,
                args_override: vec![file_path.to_string_lossy().to_string()],
            })
        }
        "c" => {
            // Compile C file
            let compiler = detect_c_compiler()?;
            let binary_name = generate_c_binary_name(&file_path, release)?;
            let cache_dir = PathBuf::from(".horus/cache");
            fs::create_dir_all(&cache_dir)?;
            let binary_path = cache_dir.join(&binary_name);

            compile_c_file(&file_path, &binary_path, &compiler, release)?;

            Ok(ExecutableInfo {
                name,
                command: binary_path.to_string_lossy().to_string(),
                args_override: Vec::new(),
            })
        }
        _ => bail!("Unsupported language: {}", language),
    }
}

fn resolve_execution_target(input: PathBuf) -> Result<Vec<ExecutionTarget>> {
    let input_str = input.to_string_lossy();

    // Check for glob patterns
    if input_str.contains('*') || input_str.contains('?') || input_str.contains('[') {
        return resolve_glob_pattern(&input_str);
    }

    if input.is_file() {
        // Check if it's a manifest file
        match input.extension().and_then(|s| s.to_str()) {
            Some("yaml") | Some("yml") => {
                if input.file_name().and_then(|s| s.to_str()) == Some("horus.yaml") {
                    return Ok(vec![ExecutionTarget::Manifest(input)]);
                }
            }
            Some("toml") => {
                if input.file_name().and_then(|s| s.to_str()) == Some("Cargo.toml") {
                    return Ok(vec![ExecutionTarget::Manifest(input)]);
                }
            }
            _ => {}
        }

        // Regular file
        return Ok(vec![ExecutionTarget::File(input)]);
    }

    if input.is_dir() {
        return Ok(vec![ExecutionTarget::Directory(input)]);
    }

    bail!("Target not found: {}", input.display())
}

fn resolve_glob_pattern(pattern: &str) -> Result<Vec<ExecutionTarget>> {
    // Load ignore patterns from horus.yaml if it exists
    let ignore = if Path::new("horus.yaml").exists() {
        parse_horus_yaml_ignore("horus.yaml").unwrap_or_default()
    } else {
        IgnorePatterns::default()
    };

    let mut files = Vec::new();

    for entry in glob(pattern).context("Failed to parse glob pattern")? {
        match entry {
            Ok(path) => {
                if path.is_file() && !ignore.should_ignore_file(&path) {
                    // Only include executable file types
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        if matches!(ext, "rs" | "py" | "c" | "cc" | "cpp" | "horus") {
                            files.push(path);
                        }
                    }
                }
            }
            Err(e) => eprintln!("Warning: Glob error: {}", e),
        }
    }

    if files.is_empty() {
        bail!("No executable files found matching pattern: {}\n\n{}\n  {} Supported extensions: {}\n  {} Check pattern: {}",
            pattern.green(),
            "No matches found:".yellow(),
            "•".cyan(), ".rs, .py, .c, .cc, .cpp, .horus".green(),
            "•".cyan(), "Use quotes around patterns like \"nodes/*.py\"".dimmed()
        );
    }

    if files.len() == 1 {
        Ok(vec![ExecutionTarget::File(
            files.into_iter().next().unwrap(),
        )])
    } else {
        Ok(vec![ExecutionTarget::Multiple(files)])
    }
}

fn auto_detect_main_file() -> Result<PathBuf> {
    // Load ignore patterns from horus.yaml if it exists
    let ignore = if Path::new("horus.yaml").exists() {
        parse_horus_yaml_ignore("horus.yaml").unwrap_or_default()
    } else {
        IgnorePatterns::default()
    };

    // Check for main files in priority order
    let candidates = [
        "main.rs",
        "main.py",
        "main.c",
        "src/main.rs",
        "src/main.py",
        "src/main.c",
    ];

    for candidate in &candidates {
        let path = PathBuf::from(candidate);
        if path.exists() && !ignore.should_ignore_file(&path) {
            return Ok(path);
        }
    }

    // Check for single file with appropriate extension
    let entries: Vec<_> = fs::read_dir(".")
        .context("Failed to read current directory")?
        .filter_map(Result::ok)
        .collect();

    let code_files: Vec<_> = entries
        .iter()
        .filter(|e| {
            let path = e.path();
            if let Some(ext) = path.extension() {
                matches!(ext.to_str(), Some("rs") | Some("py") | Some("c"))
                    && !ignore.should_ignore_file(&path)
            } else {
                false
            }
        })
        .collect();

    if code_files.len() == 1 {
        return Ok(code_files[0].path());
    }

    bail!("No main file detected.\n\n{}\n  {} Create a main file: {}\n  {} Or specify a file: {}\n  {} Or run from directory: {}",
        "Solutions:".yellow(),
        "•".cyan(), "main.rs, main.py, or main.c".green(),
        "•".cyan(), "horus run myfile.rs".green(),
        "•".cyan(), "horus run src/".green()
    )
}

fn detect_language(file: &Path) -> Result<String> {
    match file.extension().and_then(|s| s.to_str()) {
        Some("rs") => Ok("rust".to_string()),
        Some("py") => Ok("python".to_string()),
        Some("c") | Some("cc") | Some("cpp") => Ok("c".to_string()),
        _ => bail!(
            "Unsupported file type: {}\n\n{}\n  {} Supported: {}\n  {} Got: {}",
            file.display(),
            "Supported file types:".yellow(),
            "•".cyan(),
            ".rs (Rust), .py (Python), .c/.cc/.cpp (C/C++)".green(),
            "•".cyan(),
            file.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("no extension")
                .red()
        ),
    }
}

fn ensure_horus_directory() -> Result<()> {
    let horus_dir = PathBuf::from(".horus");

    // Create .horus/ if it doesn't exist
    if !horus_dir.exists() {
        println!("{} Creating .horus/ environment...", "".cyan());
        fs::create_dir_all(&horus_dir)?;
    }

    // Always ensure subdirectories exist (they might not if created by `horus new`)
    fs::create_dir_all(horus_dir.join("packages"))?;
    fs::create_dir_all(horus_dir.join("bin"))?;
    fs::create_dir_all(horus_dir.join("lib"))?;
    fs::create_dir_all(horus_dir.join("include"))?;
    fs::create_dir_all(horus_dir.join("cache"))?;

    // Setup C environment if needed
    setup_c_environment()?;

    Ok(())
}

fn scan_imports(file: &Path, language: &str, ignore: &IgnorePatterns) -> Result<HashSet<String>> {
    let content = fs::read_to_string(file)?;
    let mut dependencies = HashSet::new();

    // First, check if horus.yaml exists and use it
    let from_yaml = Path::new("horus.yaml").exists();

    if from_yaml {
        eprintln!("  {} Reading dependencies from horus.yaml", "".cyan());
        let yaml_deps = parse_horus_yaml_dependencies("horus.yaml")?;
        dependencies.extend(yaml_deps);
    } else {
        // Fallback: scan imports from source code
        match language {
            "rust" => {
                // Scan for: use horus::*, use horus_library::*, extern crate
                for line in content.lines() {
                    if let Some(dep) = parse_rust_import(line) {
                        dependencies.insert(dep);
                    }
                }

                // Also check Cargo.toml if exists (legacy support)
                if Path::new("Cargo.toml").exists() {
                    let cargo_deps = parse_cargo_dependencies("Cargo.toml")?;
                    dependencies.extend(cargo_deps);
                }
            }
            "python" => {
                // Scan for ALL imports (not just HORUS)
                for line in content.lines() {
                    if let Some(dep) = parse_all_python_imports(line) {
                        dependencies.insert(dep);
                    }
                }
            }
            "c" => {
                // Scan for: #include <horus/*.h>
                for line in content.lines() {
                    if let Some(dep) = parse_c_include(line) {
                        dependencies.insert(dep);
                    }
                }
            }
            _ => {}
        }
    }

    // Filter out ignored packages
    dependencies.retain(|dep| !ignore.should_ignore_package(dep));

    // Auto-create or update horus.yaml if we scanned from source
    if !from_yaml && !dependencies.is_empty() {
        auto_update_horus_yaml(file, language, &dependencies)?;
    }

    Ok(dependencies)
}

fn parse_rust_import(line: &str) -> Option<String> {
    let line = line.trim();

    // use horus_library::*
    if line.starts_with("use ") {
        let parts: Vec<&str> = line[4..].split("::").collect();
        if !parts.is_empty() {
            let package = parts[0].trim_end_matches(';');
            if package.starts_with("horus") {
                return Some(package.to_string());
            }
        }
    }

    // extern crate horus_library
    if line.starts_with("extern crate ") {
        let package = line[13..].trim_end_matches(';').trim();
        if package.starts_with("horus") {
            return Some(package.to_string());
        }
    }

    None
}

#[allow(dead_code)]
fn parse_python_import(line: &str) -> Option<String> {
    let line = line.trim();

    // import horus
    if line.starts_with("import ") {
        let package = line[7..].split_whitespace().next()?;
        if package.starts_with("horus") {
            return Some(package.split('.').next()?.to_string());
        }
    }

    // from horus_library import
    if line.starts_with("from ") {
        let parts: Vec<&str> = line[5..].split(" import ").collect();
        if !parts.is_empty() {
            let package = parts[0].trim();
            if package.starts_with("horus") {
                return Some(package.split('.').next()?.to_string());
            }
        }
    }

    None
}

/// Parse ALL Python imports (not just HORUS)
fn parse_all_python_imports(line: &str) -> Option<String> {
    let line = line.trim();

    // Skip comments
    if line.starts_with('#') {
        return None;
    }

    // import numpy
    // import numpy as np
    if line.starts_with("import ") {
        let rest = &line[7..];
        let package = rest.split_whitespace().next()?.split('.').next()?;
        // Skip relative imports and standard library
        if !is_stdlib_package(package) && !package.starts_with('.') {
            return Some(package.to_string());
        }
    }

    // from numpy import something
    if line.starts_with("from ") {
        let parts: Vec<&str> = line[5..].split(" import ").collect();
        if !parts.is_empty() {
            let package = parts[0].trim().split('.').next()?;
            // Skip relative imports and standard library
            if !is_stdlib_package(package) && !package.starts_with('.') {
                return Some(package.to_string());
            }
        }
    }

    None
}

/// Check if package is Python standard library
fn is_stdlib_package(name: &str) -> bool {
    let stdlib = [
        "os",
        "sys",
        "re",
        "json",
        "time",
        "datetime",
        "math",
        "random",
        "collections",
        "itertools",
        "functools",
        "pathlib",
        "typing",
        "asyncio",
        "threading",
        "multiprocessing",
        "subprocess",
        "logging",
        "argparse",
        "configparser",
        "io",
        "pickle",
        "csv",
        "xml",
        "html",
        "http",
        "urllib",
        "socket",
        "email",
        "base64",
        "hashlib",
        "hmac",
        "secrets",
        "uuid",
        "dataclasses",
        "enum",
        "abc",
        "contextlib",
    ];
    stdlib.contains(&name)
}

fn parse_c_include(line: &str) -> Option<String> {
    let line = line.trim();

    // #include <horus/node.h>
    if line.starts_with("#include") {
        if let Some(start) = line.find('<') {
            if let Some(end) = line.find('>') {
                let include = &line[start + 1..end];
                if include.starts_with("horus") {
                    return Some("horus_c".to_string());
                }
            }
        }
    }

    None
}

/// Parse a single YAML dependency and convert to Cargo.toml format
/// Handles: - horus, - name: serde with version: "1" features: [derive]
fn parse_yaml_cargo_dependency(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::String(dep_str) => {
            // Simple string: - horus
            let dep = dep_str.trim();
            if dep == "horus" || dep.starts_with("horus_") {
                None // Skip, already added
            } else {
                Some(format!("{} = \"*\"", dep))
            }
        }
        serde_yaml::Value::Mapping(map) => {
            // Map format: - name: serde, version: "1", features: [derive]
            let name = map.get("name")?.as_str()?;

            if name == "horus" || name.starts_with("horus_") {
                return None; // Skip, already added
            }

            let mut cargo_dep = format!("{} = ", name);

            // Check for simple version string
            if let Some(version) = map.get("version").and_then(|v| v.as_str()) {
                // Check if features exist
                if let Some(features_val) = map.get("features") {
                    if let Some(features_list) = features_val.as_sequence() {
                        let features: Vec<&str> =
                            features_list.iter().filter_map(|f| f.as_str()).collect();

                        if !features.is_empty() {
                            cargo_dep.push_str(&format!(
                                "{{ version = \"{}\", features = [{}] }}",
                                version,
                                features
                                    .iter()
                                    .map(|f| format!("\"{}\"", f))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                            return Some(cargo_dep);
                        }
                    }
                }

                // No features, just version
                cargo_dep.push_str(&format!("\"{}\"", version));
                Some(cargo_dep)
            } else {
                // No version specified
                Some(format!("{} = \"*\"", name))
            }
        }
        _ => None,
    }
}

fn parse_horus_yaml_dependencies(path: &str) -> Result<HashSet<String>> {
    let content = fs::read_to_string(path)?;

    // Try to parse as proper YAML first (supports complex table syntax)
    match serde_yaml::from_str::<serde_yaml::Value>(&content) {
        Ok(yaml) => {
            let mut dependencies = HashSet::new();

            // Get language context
            let language = yaml
                .get("language")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Parse dependencies
            if let Some(deps_value) = yaml.get("dependencies") {
                match deps_value {
                    // Array format: dependencies: [- pkg, - pkg = "version"]
                    serde_yaml::Value::Sequence(list) => {
                        for item in list {
                            if let Some(dep_str) =
                                parse_dependency_value(item, language.as_deref())?
                            {
                                dependencies.insert(dep_str);
                            }
                        }
                    }

                    // Map format: dependencies: { pkg: version, pkg: {path: ...} }
                    serde_yaml::Value::Mapping(map) => {
                        for (key, value) in map {
                            if let serde_yaml::Value::String(pkg_name) = key {
                                if let Some(dep_str) = parse_dependency_map_entry(
                                    pkg_name,
                                    value,
                                    language.as_deref(),
                                )? {
                                    dependencies.insert(dep_str);
                                }
                            }
                        }
                    }

                    _ => {}
                }
            }

            Ok(dependencies)
        }

        // Fallback to simple line-by-line parsing for malformed YAML
        Err(_) => parse_horus_yaml_dependencies_simple(path),
    }
}

/// Parse a single dependency value from YAML (array item)
fn parse_dependency_value(
    value: &serde_yaml::Value,
    language: Option<&str>,
) -> Result<Option<String>> {
    match value {
        serde_yaml::Value::String(dep_str) => {
            let dep_str = dep_str.trim();
            if dep_str.is_empty() || dep_str.starts_with('#') {
                return Ok(None);
            }

            // Normalize "pkg = version" syntax
            if let Some(equals_pos) = dep_str.find('=') {
                let pkg_name = dep_str[..equals_pos].trim();
                let version_part = dep_str[equals_pos + 1..].trim();
                let version = version_part.trim_matches('\'').trim_matches('"').trim();

                if !pkg_name.contains(':') {
                    let prefix = match language {
                        Some("python") => "pip",
                        Some("rust") => "cargo",
                        _ => "cargo",
                    };
                    return Ok(Some(format!("{}:{}@{}", prefix, pkg_name, version)));
                } else {
                    return Ok(Some(format!("{}@{}", pkg_name, version)));
                }
            }

            Ok(Some(dep_str.to_string()))
        }
        _ => Ok(None),
    }
}

/// Parse dependency from map format: pkg: {version: "1.0", features: [...]}
fn parse_dependency_map_entry(
    pkg_name: &str,
    value: &serde_yaml::Value,
    language: Option<&str>,
) -> Result<Option<String>> {
    match value {
        // Simple string version: pkg: "1.0"
        serde_yaml::Value::String(version_str) => {
            let version = version_str.trim_matches('\'').trim_matches('"').trim();
            let prefix = if pkg_name.contains(':') {
                ""
            } else {
                match language {
                    Some("python") => "pip:",
                    Some("rust") => "cargo:",
                    _ => "cargo:",
                }
            };
            Ok(Some(format!("{}{}@{}", prefix, pkg_name, version)))
        }

        // Table format: pkg: {version: "1.0", features: ["full"], path: "...", git: "..."}
        serde_yaml::Value::Mapping(map) => {
            // Check for empty map (pkg: {}) - treat as simple dependency
            if map.is_empty() {
                return Ok(Some(pkg_name.to_string()));
            }

            // Extract version
            let version = map
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.trim_matches('\'').trim_matches('"').trim());

            // Extract features
            let features: Vec<String> = map
                .get("features")
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|f| f.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            // Check for path dependency
            if let Some(path_value) = map.get("path") {
                if let Some(path_str) = path_value.as_str() {
                    // Path dependencies are now supported! Return special format: path:pkg_name:path_str
                    return Ok(Some(format!("path:{}:{}", pkg_name, path_str)));
                }
            }

            // Check for git dependency
            if let Some(git_value) = map.get("git") {
                if let Some(git_str) = git_value.as_str() {
                    eprintln!(
                        "    Warning: Git dependency '{}' (git: {}) is not supported in horus run",
                        pkg_name, git_str
                    );
                    eprintln!("     Consider using 'cargo run' or 'horus build' for projects with git dependencies");
                    return Ok(None);
                }
            }

            // Build dependency string
            let prefix = if pkg_name.contains(':') {
                ""
            } else {
                match language {
                    Some("python") => "pip:",
                    Some("rust") => "cargo:",
                    _ => "cargo:",
                }
            };

            if let Some(ver) = version {
                if !features.is_empty() {
                    Ok(Some(format!(
                        "{}{}@{}:features={}",
                        prefix,
                        pkg_name,
                        ver,
                        features.join(",")
                    )))
                } else {
                    Ok(Some(format!("{}{}@{}", prefix, pkg_name, ver)))
                }
            } else {
                // No version specified
                if !features.is_empty() {
                    Ok(Some(format!(
                        "{}{}:features={}",
                        prefix,
                        pkg_name,
                        features.join(",")
                    )))
                } else {
                    Ok(Some(format!("{}{}", prefix, pkg_name)))
                }
            }
        }

        _ => Ok(None),
    }
}

/// Fallback simple line-by-line parser for malformed YAML
fn parse_horus_yaml_dependencies_simple(path: &str) -> Result<HashSet<String>> {
    let content = fs::read_to_string(path)?;
    let mut dependencies = HashSet::new();

    // First, detect language from horus.yaml to determine default prefix
    let mut language = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("language:") {
            language = trimmed
                .strip_prefix("language:")
                .map(|s| s.trim().to_string());
            break;
        }
    }

    // Simple YAML parsing for dependencies section
    let mut in_deps = false;
    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("dependencies:") {
            in_deps = true;
            continue;
        }

        // Exit dependencies section if we hit another top-level key
        if in_deps
            && !trimmed.is_empty()
            && !trimmed.starts_with("- ")
            && !trimmed.starts_with("#")
            && trimmed.contains(':')
        {
            in_deps = false;
        }

        if in_deps && trimmed.starts_with("- ") {
            // Extract full dependency string "package@version" or "package"
            let dep_str = trimmed[2..].trim();
            if dep_str.starts_with("#") {
                continue; // Skip comments
            }

            // Strip inline comments (handle both quoted and unquoted strings)
            let dep_str = if let Some(comment_pos) = dep_str.find('#') {
                // Check if the # is inside quotes
                let before_comment = &dep_str[..comment_pos];
                let single_quotes = before_comment.matches('\'').count();
                let double_quotes = before_comment.matches('"').count();

                // If quotes are balanced, # is a comment. Otherwise, it's part of the string
                if single_quotes % 2 == 0 && double_quotes % 2 == 0 {
                    dep_str[..comment_pos].trim()
                } else {
                    dep_str
                }
            } else {
                dep_str
            };

            // Remove surrounding quotes if present
            let dep_str = dep_str.trim_matches('\'').trim_matches('"');

            // Normalize package manager syntax: "eframe = \"0.29\"" -> "cargo:eframe@0.29" or "pip:numpy@1.24"
            let dep_str = if let Some(equals_pos) = dep_str.find('=') {
                let pkg_name = dep_str[..equals_pos].trim();
                let version_part = dep_str[equals_pos + 1..].trim();
                // Remove quotes from version
                let version = version_part.trim_matches('\'').trim_matches('"').trim();

                // If no prefix (cargo:/pip:/horus), infer from language context
                if !pkg_name.contains(':') {
                    let prefix = match language.as_deref() {
                        Some("python") => "pip",
                        Some("rust") => "cargo",
                        _ => "cargo", // Default to cargo if unknown
                    };
                    format!("{}:{}@{}", prefix, pkg_name, version)
                } else {
                    // Already has prefix, just reformat
                    format!("{}@{}", pkg_name, version)
                }
            } else {
                dep_str.to_string()
            };

            // Insert the full dependency string (including version)
            // This will be split later into HORUS vs pip packages
            dependencies.insert(dep_str);
        }
    }

    Ok(dependencies)
}

/// Parse horus.yaml dependencies with support for path, git, and registry sources
/// Returns `Vec<DependencySpec>` which includes source information
pub fn parse_horus_yaml_dependencies_v2(path: &str) -> Result<Vec<DependencySpec>> {
    let content = fs::read_to_string(path)?;

    // Try parsing as proper YAML first (supports structured format)
    match serde_yaml::from_str::<serde_yaml::Value>(&content) {
        Ok(yaml) => {
            let mut deps = Vec::new();

            if let Some(deps_value) = yaml.get("dependencies") {
                match deps_value {
                    // List format: dependencies: [- pkg@version]
                    serde_yaml::Value::Sequence(list) => {
                        for item in list {
                            if let serde_yaml::Value::String(dep_str) = item {
                                // Parse simple string format
                                deps.push(DependencySpec::parse(dep_str)?);
                            }
                        }
                    }

                    // Map format: dependencies: { pkg: version, pkg: {path: ...} }
                    serde_yaml::Value::Mapping(map) => {
                        for (key, value) in map {
                            if let serde_yaml::Value::String(name) = key {
                                deps.push(DependencySpec::from_yaml_value(name.clone(), value)?);
                            }
                        }
                    }

                    _ => {}
                }
            }

            Ok(deps)
        }

        // Fallback to simple parsing for backward compatibility
        Err(_) => {
            let old_deps = parse_horus_yaml_dependencies(path)?;
            let mut deps = Vec::new();
            for dep_str in old_deps {
                deps.push(DependencySpec::parse(&dep_str)?);
            }
            Ok(deps)
        }
    }
}

/// Ignore patterns from horus.yaml
#[derive(Debug, Clone, Default)]
pub struct IgnorePatterns {
    pub files: Vec<String>,
    pub directories: Vec<String>,
    pub packages: Vec<String>,
}

impl IgnorePatterns {
    /// Check if a file path should be ignored
    pub fn should_ignore_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check directory patterns first
        for dir_pattern in &self.directories {
            let pattern = dir_pattern.trim_end_matches('/');
            if path_str.contains(pattern) {
                return true;
            }
        }

        // Check file patterns with glob matching
        for file_pattern in &self.files {
            if glob_match(file_pattern, &path_str) {
                return true;
            }
        }

        false
    }

    /// Check if a package should be ignored
    pub fn should_ignore_package(&self, package: &str) -> bool {
        self.packages.iter().any(|p| p == package)
    }
}

/// Simple glob matching for ignore patterns
fn glob_match(pattern: &str, text: &str) -> bool {
    // Handle ** for directory recursion
    if pattern.contains("**/") {
        let parts: Vec<&str> = pattern.split("**/").collect();
        if parts.len() == 2 {
            let suffix = parts[1];
            return text.contains(suffix) || text.ends_with(suffix);
        }
    }

    // Handle * wildcard
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.is_empty() {
            return true;
        }

        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            if i == 0 && !text.starts_with(part) {
                return false;
            }

            if let Some(found_pos) = text[pos..].find(part) {
                pos += found_pos + part.len();
            } else {
                return false;
            }
        }

        // If pattern ends with *, we're good
        // Otherwise, make sure we matched to the end
        if !pattern.ends_with('*') && pos != text.len() {
            return false;
        }

        true
    } else {
        // Exact match or ends_with for simple patterns
        text == pattern || text.ends_with(pattern)
    }
}

/// Parse ignore section from horus.yaml
pub fn parse_horus_yaml_ignore(path: &str) -> Result<IgnorePatterns> {
    let content = fs::read_to_string(path)?;

    match serde_yaml::from_str::<serde_yaml::Value>(&content) {
        Ok(yaml) => {
            let mut ignore = IgnorePatterns::default();

            if let Some(ignore_section) = yaml.get("ignore") {
                if let serde_yaml::Value::Mapping(ignore_map) = ignore_section {
                    // Parse files
                    if let Some(serde_yaml::Value::Sequence(files)) =
                        ignore_map.get(&serde_yaml::Value::String("files".to_string()))
                    {
                        for file in files {
                            if let serde_yaml::Value::String(pattern) = file {
                                ignore.files.push(pattern.clone());
                            }
                        }
                    }

                    // Parse directories
                    if let Some(serde_yaml::Value::Sequence(dirs)) =
                        ignore_map.get(&serde_yaml::Value::String("directories".to_string()))
                    {
                        for dir in dirs {
                            if let serde_yaml::Value::String(pattern) = dir {
                                ignore.directories.push(pattern.clone());
                            }
                        }
                    }

                    // Parse packages
                    if let Some(serde_yaml::Value::Sequence(pkgs)) =
                        ignore_map.get(&serde_yaml::Value::String("packages".to_string()))
                    {
                        for pkg in pkgs {
                            if let serde_yaml::Value::String(package) = pkg {
                                ignore.packages.push(package.clone());
                            }
                        }
                    }
                }
            }

            Ok(ignore)
        }
        Err(_) => Ok(IgnorePatterns::default()),
    }
}

fn parse_cargo_dependencies(path: &str) -> Result<HashSet<String>> {
    let content = fs::read_to_string(path)?;
    let mut dependencies = HashSet::new();

    // Simple TOML parsing for dependencies section
    let mut in_deps = false;
    for line in content.lines() {
        if line.starts_with("[dependencies]") {
            in_deps = true;
            continue;
        }
        if line.starts_with('[') {
            in_deps = false;
        }

        if in_deps {
            if let Some(eq_pos) = line.find('=') {
                let dep = line[..eq_pos].trim();
                // Check if this is a HORUS package or resolvable package
                if dep.starts_with("horus") || is_horus_package(dep) {
                    dependencies.insert(dep.to_string());
                }
            }
        }
    }

    Ok(dependencies)
}

fn is_horus_package(package: &str) -> bool {
    // Only HORUS packages start with "horus" prefix
    // Everything else will be handled by pip integration
    package.starts_with("horus")
}

fn is_cargo_package(package_name: &str) -> bool {
    use reqwest::blocking::Client;

    // Common CLI tools from crates.io
    let common_cli_tools = [
        "bat",
        "fd-find",
        "ripgrep",
        "exa",
        "tokei",
        "hyperfine",
        "starship",
        "zoxide",
        "delta",
        "dust",
        "procs",
        "bottom",
        "tealdeer",
        "sd",
        "grex",
        "xsv",
        "bandwhich",
    ];

    if common_cli_tools.contains(&package_name) {
        return true;
    }

    // Check crates.io API for less common packages
    let client = Client::new();
    let url = format!("https://crates.io/api/v1/crates/{}", package_name);
    if let Ok(response) = client
        .get(&url)
        .header("User-Agent", "horus-pkg-manager")
        .send()
    {
        return response.status().is_success();
    }

    false
}

/// Separate HORUS packages, pip packages, and cargo packages
///
/// # Arguments
/// * `deps` - Set of dependency strings from horus.yaml
/// * `context_language` - Optional language context ("rust", "python", "cpp") to guide auto-detection
fn split_dependencies_with_context(
    deps: HashSet<String>,
    context_language: Option<&str>,
) -> (Vec<String>, Vec<PipPackage>, Vec<CargoPackage>) {
    let mut horus_packages = Vec::new();
    let mut pip_packages = Vec::new();
    let mut cargo_packages = Vec::new();

    for dep in deps {
        let dep = dep.trim();

        // Check for explicit prefixes
        if dep.starts_with("pip:") {
            let pkg_str = dep.strip_prefix("pip:").unwrap();
            match PipPackage::from_string(pkg_str) {
                Ok(pkg) => pip_packages.push(pkg),
                Err(e) => {
                    eprintln!(
                        "  {} Failed to parse pip dependency '{}': {}",
                        "".yellow(),
                        dep,
                        e
                    );
                    eprintln!("     Syntax: pip:PACKAGE@VERSION or pip:PACKAGE");
                    eprintln!("     Example: pip:numpy@1.24.0");
                }
            }
            continue;
        }

        if dep.starts_with("cargo:") {
            let pkg_str = dep.strip_prefix("cargo:").unwrap();
            match CargoPackage::from_string(pkg_str) {
                Ok(pkg) => cargo_packages.push(pkg),
                Err(e) => {
                    eprintln!(
                        "  {} Failed to parse cargo dependency '{}': {}",
                        "".yellow(),
                        dep,
                        e
                    );
                    eprintln!("     Syntax: cargo:PACKAGE@VERSION:features=FEAT1,FEAT2");
                    eprintln!("     Examples:");
                    eprintln!("       - 'cargo:serde@1.0:features=derive'");
                    eprintln!("       - 'cargo:tokio@1.35:features=full,macros'");
                    eprintln!("       - cargo:rand@0.8");
                }
            }
            continue;
        }

        // Auto-detect: if starts with "horus"  HORUS registry
        if dep.starts_with("horus") {
            horus_packages.push(dep.to_string());
            continue;
        }

        // Check if it's a known HORUS package using registry
        if is_horus_package(dep) {
            horus_packages.push(dep.to_string());
            continue;
        }

        // For unprefixed dependencies, use language context to determine type
        if let Some(lang) = context_language {
            match lang {
                "rust" => {
                    // Rust context: unprefixed deps are cargo packages
                    if let Ok(pkg) = CargoPackage::from_string(dep) {
                        cargo_packages.push(pkg);
                    }
                }
                "python" => {
                    // Python context: unprefixed deps are pip packages
                    if let Ok(pkg) = PipPackage::from_string(dep) {
                        pip_packages.push(pkg);
                    }
                }
                _ => {
                    // Unknown context: fall back to old auto-detection
                    let dep_name = if let Some(at_pos) = dep.find('@') {
                        &dep[..at_pos]
                    } else {
                        dep
                    };

                    if is_cargo_package(dep_name) {
                        if let Ok(pkg) = CargoPackage::from_string(dep) {
                            cargo_packages.push(pkg);
                        }
                    } else if let Ok(pkg) = PipPackage::from_string(dep) {
                        pip_packages.push(pkg);
                    }
                }
            }
        } else {
            // No context: use old auto-detection behavior
            let dep_name = if let Some(at_pos) = dep.find('@') {
                &dep[..at_pos]
            } else {
                dep
            };

            if is_cargo_package(dep_name) {
                if let Ok(pkg) = CargoPackage::from_string(dep) {
                    cargo_packages.push(pkg);
                }
            } else if let Ok(pkg) = PipPackage::from_string(dep) {
                pip_packages.push(pkg);
            }
        }
    }

    (horus_packages, pip_packages, cargo_packages)
}

/// Backward-compatible wrapper without language context
fn split_dependencies(deps: HashSet<String>) -> (Vec<String>, Vec<PipPackage>, Vec<CargoPackage>) {
    split_dependencies_with_context(deps, None)
}

/// Split dependencies including path dependencies
/// Returns: (horus_packages, pip_packages, cargo_packages, path_packages)
/// path_packages is Vec<(name, path)>
fn split_dependencies_with_path_context(
    deps: HashSet<String>,
    context_language: Option<&str>,
) -> (
    Vec<String>,
    Vec<PipPackage>,
    Vec<CargoPackage>,
    Vec<(String, String)>,
) {
    let mut horus_packages = Vec::new();
    let mut pip_packages = Vec::new();
    let mut cargo_packages = Vec::new();
    let mut path_packages = Vec::new();

    for dep in deps {
        let dep = dep.trim();

        // Handle path dependencies: path:pkg_name:path_str
        if dep.starts_with("path:") {
            let parts: Vec<&str> = dep.splitn(3, ':').collect();
            if parts.len() == 3 {
                let pkg_name = parts[1].to_string();
                let pkg_path = parts[2].to_string();
                path_packages.push((pkg_name, pkg_path));
                continue;
            }
        }

        // Check for explicit prefixes
        if dep.starts_with("pip:") {
            let pkg_str = dep.strip_prefix("pip:").unwrap();
            match PipPackage::from_string(pkg_str) {
                Ok(pkg) => pip_packages.push(pkg),
                Err(e) => {
                    eprintln!(
                        "  {} Failed to parse pip dependency '{}': {}",
                        "".yellow(),
                        dep,
                        e
                    );
                }
            }
            continue;
        }

        if dep.starts_with("cargo:") {
            let pkg_str = dep.strip_prefix("cargo:").unwrap();
            match CargoPackage::from_string(pkg_str) {
                Ok(pkg) => cargo_packages.push(pkg),
                Err(e) => {
                    eprintln!(
                        "  {} Failed to parse cargo dependency '{}': {}",
                        "".yellow(),
                        dep,
                        e
                    );
                }
            }
            continue;
        }

        // Auto-detect: if starts with "horus"  HORUS registry
        if dep.starts_with("horus") {
            horus_packages.push(dep.to_string());
            continue;
        }

        // Check if it's a known HORUS package using registry
        if is_horus_package(dep) {
            horus_packages.push(dep.to_string());
            continue;
        }

        // For unprefixed dependencies, use language context to determine type
        if let Some(lang) = context_language {
            match lang {
                "rust" => {
                    // Rust context: unprefixed deps are cargo packages
                    if let Ok(pkg) = CargoPackage::from_string(dep) {
                        cargo_packages.push(pkg);
                    }
                }
                "python" => {
                    // Python context: unprefixed deps are pip packages
                    if let Ok(pkg) = PipPackage::from_string(dep) {
                        pip_packages.push(pkg);
                    }
                }
                _ => {}
            }
        }
    }

    (horus_packages, pip_packages, cargo_packages, path_packages)
}

/// Install pip packages using global cache (HORUS philosophy)
/// Packages stored at: ~/.horus/cache/pypi_{name}@{version}/
fn install_pip_packages(packages: Vec<PipPackage>) -> Result<()> {
    if packages.is_empty() {
        return Ok(());
    }

    println!("{} Resolving Python packages...", "🐍".cyan());

    let global_cache = home_dir().join(".horus/cache");
    let local_packages = PathBuf::from(".horus/packages");

    fs::create_dir_all(&global_cache)?;
    fs::create_dir_all(&local_packages)?;

    // Use system Python's pip directly
    let python_cmd = detect_python_interpreter()?;

    for pkg in &packages {
        // Check if package exists in system first
        if let Ok(Some(system_version)) = detect_system_python_package(&pkg.name) {
            let local_link = local_packages.join(&pkg.name);

            // Skip if already handled
            if local_link.exists() || local_link.read_link().is_ok() {
                continue;
            }

            // Prompt user for choice
            match prompt_system_package_choice_run(&pkg.name, &system_version)? {
                SystemPackageChoiceRun::UseSystem => {
                    create_system_reference_python_run(&pkg.name, &system_version)?;
                    continue;
                }
                SystemPackageChoiceRun::InstallHORUS => {
                    println!("  {} Installing isolated copy to HORUS...", "".blue());
                    // Continue with installation below
                }
                SystemPackageChoiceRun::Cancel => {
                    println!("  {} Skipped {}", "⊘".yellow(), pkg.name);
                    continue;
                }
            }
        }

        // Get actual version by querying PyPI or using installed version
        let version_str = pkg
            .version
            .as_ref()
            .map(|v| {
                v.replace(">=", "")
                    .replace("==", "")
                    .replace("~=", "")
                    .replace(">", "")
                    .replace("<", "")
            })
            .unwrap_or_else(|| "latest".to_string());

        // Cache directory with pypi_ prefix to distinguish from HORUS packages
        let pkg_cache_dir = global_cache.join(format!("pypi_{}@{}", pkg.name, version_str));

        let local_link = local_packages.join(&pkg.name);

        // If already symlinked, skip
        if local_link.exists() || local_link.read_link().is_ok() {
            println!("  {} {} (already linked)", "".green(), pkg.name);
            continue;
        }

        // If not cached, install to global cache
        if !pkg_cache_dir.exists() {
            println!("  {} Installing {} to global cache...", "".cyan(), pkg.name);

            fs::create_dir_all(&pkg_cache_dir)?;

            // Install package with pip to cache directory using system pip
            let mut cmd = Command::new(&python_cmd);
            cmd.args(&[
                "-m",
                "pip",
                "install",
                "--target",
                pkg_cache_dir.to_str().unwrap(),
            ]);
            cmd.arg(pkg.requirement_string());

            let output = cmd.output().context("Failed to run pip install")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("pip install failed for {}: {}", pkg.name, stderr);
            }

            // Create metadata.json for package tracking
            let metadata = serde_json::json!({
                "name": pkg.name,
                "version": version_str,
                "source": "PyPI"
            });
            let metadata_path = pkg_cache_dir.join("metadata.json");
            fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?)?;

            println!("  {} Cached {}", "".green(), pkg.name);
        } else {
            println!("  {} {} -> global cache", "↗".cyan(), pkg.name);
        }

        // Symlink from local packages to global cache
        symlink(&pkg_cache_dir, &local_link)
            .context(format!("Failed to symlink {} from global cache", pkg.name))?;
        println!("  {} Linked {}", "".green(), pkg.name);
    }

    Ok(())
}

fn install_cargo_packages(packages: Vec<CargoPackage>) -> Result<()> {
    if packages.is_empty() {
        return Ok(());
    }

    println!("{} Resolving Rust binaries...", "🦀".cyan());

    let global_cache = home_dir().join(".horus/cache");
    let local_bin = PathBuf::from(".horus/bin");
    let local_packages = PathBuf::from(".horus/packages");

    fs::create_dir_all(&global_cache)?;
    fs::create_dir_all(&local_bin)?;
    fs::create_dir_all(&local_packages)?;

    // Check if cargo is available
    if Command::new("cargo").arg("--version").output().is_err() {
        bail!("cargo not found. Please install Rust toolchain from https://rustup.rs");
    }

    for pkg in &packages {
        // Check if system binary exists first
        if let Ok(Some(system_version)) = detect_system_cargo_binary(&pkg.name) {
            let local_link = local_bin.join(&pkg.name);

            // Skip if already handled
            if local_link.exists() || local_link.read_link().is_ok() {
                continue;
            }

            // Prompt user for choice
            match prompt_system_cargo_choice_run(&pkg.name, &system_version)? {
                SystemPackageChoiceRun::UseSystem => {
                    create_system_reference_cargo_run(&pkg.name, &system_version)?;
                    continue;
                }
                SystemPackageChoiceRun::InstallHORUS => {
                    println!("  {} Installing isolated copy to HORUS...", "".blue());
                    // Continue with installation below
                }
                SystemPackageChoiceRun::Cancel => {
                    println!("  {} Skipped {}", "⊘".yellow(), pkg.name);
                    continue;
                }
            }
        }

        let version_str = pkg
            .version
            .as_ref()
            .unwrap_or(&"latest".to_string())
            .clone();
        let pkg_cache_dir = global_cache.join(format!("cratesio_{}@{}", pkg.name, version_str));
        let local_link = local_bin.join(&pkg.name);

        // If already linked, skip
        if local_link.exists() || local_link.read_link().is_ok() {
            println!("  {} {} (already linked)", "".green(), pkg.name);
            continue;
        }

        // If not cached, install to global cache
        if !pkg_cache_dir.exists() {
            println!("  {} Installing {} to global cache...", "".cyan(), pkg.name);

            fs::create_dir_all(&pkg_cache_dir)?;

            // Install with cargo to cache directory
            let mut cmd = Command::new("cargo");
            cmd.arg("install");

            if let Some(version) = &pkg.version {
                cmd.arg(&format!("{}@{}", pkg.name, version));
            } else {
                cmd.arg(&pkg.name);
            }

            cmd.arg("--root").arg(&pkg_cache_dir);

            let output = cmd.output().context("Failed to run cargo install")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("cargo install failed for {}: {}", pkg.name, stderr);
            }

            // Create metadata.json for package tracking
            let metadata = serde_json::json!({
                "name": pkg.name,
                "version": version_str,
                "source": "CratesIO"
            });
            let metadata_path = pkg_cache_dir.join("metadata.json");
            fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?)?;

            println!("  {} Cached {}", "".green(), pkg.name);
        } else {
            println!("  {} {} -> global cache", "↗".cyan(), pkg.name);
        }

        // Symlink binary from cache/bin/ to .horus/bin/
        let cached_bin = pkg_cache_dir.join("bin").join(&pkg.name);
        if cached_bin.exists() {
            symlink(&cached_bin, &local_link)
                .context(format!("Failed to symlink {} from global cache", pkg.name))?;
            println!("  {} Linked {}", "".green(), pkg.name);
        } else {
            println!(
                "  {} Warning: Binary {} not found in cache",
                "⚠".yellow(),
                pkg.name
            );
        }
    }

    Ok(())
}

fn resolve_dependencies(dependencies: HashSet<String>) -> Result<()> {
    // Check version compatibility first
    if let Err(e) = version::check_version_compatibility() {
        eprintln!("\n{}", "Hint:".cyan());
        eprintln!("  If you recently updated HORUS, run ./install.sh to update libraries.");
        return Err(e);
    }

    // Split dependencies into HORUS packages, pip packages, and cargo packages
    let (horus_packages, pip_packages, cargo_packages) = split_dependencies(dependencies);

    // Resolve HORUS packages (existing logic)
    if !horus_packages.is_empty() {
        resolve_horus_packages(horus_packages.into_iter().collect())?;
    }

    // Resolve pip packages
    if !pip_packages.is_empty() {
        install_pip_packages(pip_packages)?;
    }

    // Resolve cargo packages
    if !cargo_packages.is_empty() {
        install_cargo_packages(cargo_packages)?;
    }

    Ok(())
}

fn resolve_horus_packages(dependencies: HashSet<String>) -> Result<()> {
    let global_cache = home_dir().join(".horus/cache");
    let local_packages = PathBuf::from(".horus/packages");

    // Ensure directories exist
    fs::create_dir_all(&global_cache)?;
    fs::create_dir_all(&local_packages)?;

    // Collect missing packages first
    let mut missing_packages = Vec::new();

    for package in &dependencies {
        let local_link = local_packages.join(package);

        // Skip if already linked
        if local_link.exists() {
            println!("  {} {} (already linked)", "".green(), package);
            continue;
        }

        // Check global cache
        let cached_versions = find_cached_versions(&global_cache, package)?;

        if let Some(cached) = cached_versions.first() {
            // Check if we're using a different version than requested
            let cached_name = cached.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let version_mismatch = package.contains('@') && cached_name != package;

            // Special handling for horus_py - the Python package is named "horus"
            if package.starts_with("horus_py") {
                // Check if lib/horus exists in the cached package
                let lib_horus = cached.join("lib/horus");
                if lib_horus.exists() {
                    // Create symlink named "horus" pointing to lib/horus
                    let horus_link = local_packages.join("horus");

                    // Check if symlink already exists
                    if horus_link.exists() {
                        println!("  {} {} (already linked)", "".green(), package);
                        continue;
                    }

                    if version_mismatch {
                        println!(
                            "  {} {} -> {} (using {})",
                            "↗".cyan(),
                            package,
                            "global cache".dimmed(),
                            cached_name.yellow()
                        );
                    } else {
                        println!(
                            "  {} {} -> {}",
                            "↗".cyan(),
                            package,
                            "global cache".dimmed()
                        );
                    }
                    symlink(&lib_horus, &horus_link).context("Failed to symlink horus_py")?;
                    continue;
                }
            }

            // Create symlink to global cache
            if version_mismatch {
                println!(
                    "  {} {} -> {} (using {})",
                    "↗".cyan(),
                    package,
                    "global cache".dimmed(),
                    cached_name.yellow()
                );
            } else {
                println!(
                    "  {} {} -> {}",
                    "↗".cyan(),
                    package,
                    "global cache".dimmed()
                );
            }
            symlink(cached, &local_link).context(format!("Failed to symlink {}", package))?;
        } else {
            // Package not found locally
            missing_packages.push(package.clone());
        }
    }

    // If there are missing packages, ask user if they want to install
    if !missing_packages.is_empty() {
        println!(
            "\n{} Missing {} package(s):",
            "".yellow(),
            missing_packages.len()
        );
        for pkg in &missing_packages {
            println!("  • {}", pkg.yellow());
        }

        print!(
            "\n{} Install missing packages from registry? [Y/n]: ",
            "?".cyan()
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input.is_empty() || input == "y" || input == "yes" {
            // User wants to install
            println!("\n{} Installing packages...", "".cyan());

            // Import registry client
            use crate::registry::RegistryClient;
            use crate::workspace;
            let client = RegistryClient::new();
            let target = workspace::detect_or_select_workspace(true)?;

            // Try to use structured dependencies from horus.yaml
            let horus_yaml_path = Path::new("horus.yaml");
            let use_structured_deps = horus_yaml_path.exists();

            // Get base directory for resolving relative paths (directory containing horus.yaml)
            let base_dir = horus_yaml_path.parent().or_else(|| Some(Path::new(".")));

            if use_structured_deps {
                // Parse with v2 to get DependencySpecs with source information
                match parse_horus_yaml_dependencies_v2("horus.yaml") {
                    Ok(dep_specs) => {
                        // Create a map of package name -> DependencySpec
                        let mut spec_map: std::collections::HashMap<String, DependencySpec> =
                            dep_specs
                                .into_iter()
                                .map(|spec| (spec.name.clone(), spec))
                                .collect();

                        for package in &missing_packages {
                            if let Some(spec) = spec_map.remove(package) {
                                print!("  {} Installing {}... ", "".cyan(), package.yellow());
                                io::stdout().flush()?;

                                match client.install_dependency_spec(
                                    &spec,
                                    target.clone(),
                                    base_dir,
                                ) {
                                    Ok(_) => {
                                        println!("{}", "".green());
                                    }
                                    Err(e) => {
                                        println!("{}", "".red());
                                        eprintln!(
                                            "    {} Failed to install {}: {}",
                                            "".red(),
                                            package,
                                            e
                                        );
                                        bail!("Failed to install required dependency: {}", package);
                                    }
                                }
                            } else {
                                // Fallback to registry install if spec not found
                                print!(
                                    "  {} Installing {} (from registry)... ",
                                    "".cyan(),
                                    package.yellow()
                                );
                                io::stdout().flush()?;
                                match client.install(package, None) {
                                    Ok(_) => println!("{}", "".green()),
                                    Err(e) => {
                                        println!("{}", "".red());
                                        bail!("Failed to install {}: {}", package, e);
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // Fallback to old parser
                        for package in &missing_packages {
                            print!("  {} Installing {}... ", "".cyan(), package.yellow());
                            io::stdout().flush()?;

                            match client.install(package, None) {
                                Ok(_) => {
                                    println!("{}", "".green());
                                }
                                Err(e) => {
                                    println!("{}", "".red());
                                    eprintln!(
                                        "    {} Failed to install {}: {}",
                                        "".red(),
                                        package,
                                        e
                                    );
                                    bail!("Failed to install required dependency: {}", package);
                                }
                            }
                        }
                    }
                }
            } else {
                // No horus.yaml, use old behavior
                for package in &missing_packages {
                    print!("  {} Installing {}... ", "".cyan(), package.yellow());
                    io::stdout().flush()?;

                    match client.install(package, None) {
                        Ok(_) => {
                            println!("{}", "".green());
                        }
                        Err(e) => {
                            println!("{}", "".red());
                            eprintln!("    {} Failed to install {}: {}", "".red(), package, e);
                            bail!("Failed to install required dependency: {}", package);
                        }
                    }
                }
            }

            println!("\n{} All dependencies installed successfully!", "".green());
        } else {
            // User declined
            println!(
                "\n{} Installation cancelled. Cannot proceed without dependencies.",
                "".red()
            );
            bail!(
                "Missing required dependencies: {}",
                missing_packages.join(", ")
            );
        }
    }

    Ok(())
}

fn find_cached_versions(cache_dir: &Path, package: &str) -> Result<Vec<PathBuf>> {
    let mut versions = Vec::new();

    if !cache_dir.exists() {
        return Ok(versions);
    }

    // Parse package name and version if specified (e.g., "horus_py@0.1.0" -> ("horus_py", Some("0.1.0")))
    let (base_package, requested_version) = if let Some(at_pos) = package.find('@') {
        (&package[..at_pos], Some(&package[at_pos + 1..]))
    } else {
        (package, None)
    };

    for entry in fs::read_dir(cache_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Match base package name
        if name_str == base_package || name_str.starts_with(&format!("{}@", base_package)) {
            // If a specific version was requested, prefer exact match
            if let Some(req_ver) = requested_version {
                if name_str == format!("{}@{}", base_package, req_ver) {
                    // Exact version match - prioritize this
                    versions.insert(0, entry.path());
                } else {
                    // Different version - add to list as fallback
                    versions.push(entry.path());
                }
            } else {
                // No specific version requested - add all
                versions.push(entry.path());
            }
        }
    }

    // Sort by version (newest first), but keep exact match at front if it exists
    if requested_version.is_some() && !versions.is_empty() {
        // First entry is exact match (if found), don't sort it out
        let exact_match = versions.first().cloned();
        let is_exact = exact_match.as_ref().is_some_and(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n == format!("{}@{}", base_package, requested_version.unwrap()))
        });

        if is_exact {
            // Keep exact match at front, sort the rest
            let mut rest = versions.split_off(1);
            rest.sort_by(|a, b| b.cmp(a));
            versions.extend(rest);
        } else {
            // No exact match, sort all by version (newest first)
            versions.sort_by(|a, b| b.cmp(a));
        }
    } else {
        // Sort by version (newest first)
        versions.sort_by(|a, b| b.cmp(a));
    }

    Ok(versions)
}

fn home_dir() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()))
}

fn setup_environment() -> Result<()> {
    let current_dir = env::current_dir()?;
    let horus_bin = current_dir.join(".horus/bin");
    let horus_lib = current_dir.join(".horus/lib");
    let horus_packages = current_dir.join(".horus/packages");

    // Update PATH
    if let Ok(path) = env::var("PATH") {
        let new_path = format!("{}:{}", horus_bin.display(), path);
        env::set_var("PATH", new_path);
    }

    // Build LD_LIBRARY_PATH: local + global cache libs
    let mut lib_paths = vec![horus_lib.display().to_string()];

    // Add global cache library paths if they exist
    let home = home_dir();
    let global_cache = home.join(".horus/cache");
    {
        if global_cache.exists() {
            // Scan for packages with lib/ directories
            if let Ok(entries) = fs::read_dir(&global_cache) {
                for entry in entries.flatten() {
                    let lib_dir = entry.path().join("lib");
                    if lib_dir.exists() {
                        lib_paths.push(lib_dir.display().to_string());
                    }
                    // Also check target/release for Rust packages
                    let target_lib = entry.path().join("target/release");
                    if target_lib.exists() {
                        lib_paths.push(target_lib.display().to_string());
                    }
                }
            }
        }
    }

    // Set LD_LIBRARY_PATH with all paths
    let lib_path_str = lib_paths.join(":");
    if let Ok(ld_path) = env::var("LD_LIBRARY_PATH") {
        let new_path = format!("{}:{}", lib_path_str, ld_path);
        env::set_var("LD_LIBRARY_PATH", new_path);
    } else {
        env::set_var("LD_LIBRARY_PATH", lib_path_str);
    }

    // Update PYTHONPATH for Python imports
    if let Ok(py_path) = env::var("PYTHONPATH") {
        let new_path = format!("{}:{}", horus_packages.display(), py_path);
        env::set_var("PYTHONPATH", new_path);
    } else {
        env::set_var("PYTHONPATH", horus_packages.display().to_string());
    }

    Ok(())
}

fn execute_python_node(file: PathBuf, args: Vec<String>, _release: bool) -> Result<()> {
    eprintln!("{} Setting up Python environment...", "".cyan());

    // Check for Python interpreter
    let python_cmd = detect_python_interpreter()?;

    // Setup Python path for horus_py integration
    setup_python_environment()?;

    // Detect if this is a HORUS node or plain Python script
    let uses_horus = detect_horus_usage_python(&file)?;

    if uses_horus {
        // Use scheduler wrapper for HORUS nodes
        eprintln!(
            "{} Executing Python node with HORUS scheduler...",
            "".cyan()
        );

        let wrapper_script = create_python_wrapper(&file)?;

        let mut cmd = Command::new(python_cmd);
        cmd.arg(&wrapper_script);
        cmd.args(args);

        // Spawn child process so we can handle Ctrl+C
        let mut child = cmd.spawn()?;
        let child_id = child.id();

        // Setup Ctrl+C handler
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        ctrlc::set_handler(move || {
            println!(
                "\n{} Ctrl+C received, stopping Python process...",
                "".yellow()
            );
            r.store(false, Ordering::SeqCst);
            // Send SIGINT to child process on Unix systems
            #[cfg(unix)]
            unsafe {
                libc::kill(child_id as i32, libc::SIGINT);
            }
        })
        .ok();

        // Wait for child to complete
        let status = child.wait()?;

        // Cleanup wrapper script
        fs::remove_file(wrapper_script).ok();

        // Exit with the same code as the Python script
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
    } else {
        // Direct execution for plain Python scripts
        eprintln!("{} Executing Python script directly...", "".cyan());

        let mut cmd = Command::new(python_cmd);
        cmd.arg(&file);
        cmd.args(args);

        // Spawn child process so we can handle Ctrl+C
        let mut child = cmd.spawn()?;
        let child_id = child.id();

        // Setup Ctrl+C handler
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        ctrlc::set_handler(move || {
            println!(
                "\n{} Ctrl+C received, stopping Python process...",
                "".yellow()
            );
            r.store(false, Ordering::SeqCst);
            // Send SIGINT to child process on Unix systems
            #[cfg(unix)]
            unsafe {
                libc::kill(child_id as i32, libc::SIGINT);
            }
        })
        .ok();

        // Wait for child to complete
        let status = child.wait()?;

        // Exit with the same code as the Python script
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
    }

    Ok(())
}

fn detect_python_interpreter() -> Result<String> {
    // Use system Python - packages are in PYTHONPATH via .horus/packages/
    for cmd in &["python3", "python"] {
        if Command::new(cmd).arg("--version").output().is_ok() {
            return Ok(cmd.to_string());
        }
    }
    bail!("No Python interpreter found. Install Python 3.7+ and ensure it's in PATH.");
}

fn setup_python_environment() -> Result<()> {
    let current_dir = env::current_dir()?;
    let horus_packages = current_dir.join(".horus/packages");

    // Add global cache Python packages to PYTHONPATH
    let home = dirs::home_dir().context("Could not find home directory")?;
    let global_cache = home.join(".horus/cache");

    let mut python_paths = Vec::new();

    // Collect all global cache Python package lib directories
    if global_cache.exists() {
        if let Ok(entries) = fs::read_dir(&global_cache) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Check for lib directory (Python packages)
                    let lib_dir = path.join("lib");
                    if lib_dir.exists() {
                        python_paths.push(lib_dir.display().to_string());
                    }
                }
            }
        }
    }

    // Add local packages
    python_paths.push(horus_packages.display().to_string());

    // Add existing PYTHONPATH
    if let Ok(current_path) = env::var("PYTHONPATH") {
        python_paths.push(current_path);
    }

    // Set the combined PYTHONPATH
    let new_path = python_paths.join(":");
    env::set_var("PYTHONPATH", new_path);

    Ok(())
}

fn detect_horus_usage_python(file: &Path) -> Result<bool> {
    let content = fs::read_to_string(file)?;

    // Check for HORUS imports
    let horus_patterns = [
        "import horus",
        "from horus",
        "import horus_py",
        "from horus_py",
    ];

    for pattern in &horus_patterns {
        if content.contains(pattern) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn create_python_wrapper(original_file: &Path) -> Result<PathBuf> {
    let wrapper_path = env::temp_dir().join(format!(
        "horus_wrapper_{}.py",
        original_file.file_stem().unwrap().to_string_lossy()
    ));

    let wrapper_content = format!(
        r#"#!/usr/bin/env python3
"""
HORUS Python Node Wrapper
Auto-generated wrapper for HORUS scheduler integration
"""
import sys
import os

# HORUS Python bindings are available via the 'horus' package
# Install with: pip install maturin && maturin develop (from horus_py directory)
# Or: pip install -e horus_py/

class HorusSchedulerIntegration:
    def __init__(self):
        self.running = True

    def run_node(self):
        """Run the user's node code with scheduler integration"""
        exit_code = 0
        try:
            # Execute user code in global namespace with proper scope
            # Pass globals() so imports and module-level code are accessible everywhere
            exec(compile(open(r'{}').read(), r'{}', 'exec'), globals())
        except SystemExit as e:
            # Preserve exit code from sys.exit()
            exit_code = e.code if e.code is not None else 0
        except KeyboardInterrupt:
            # Ctrl+C received - exit cleanly
            print("\n🛑 Graceful shutdown initiated...", file=sys.stderr)
            exit_code = 0
        except Exception as e:
            print(f" Node execution failed: {{e}}", file=sys.stderr)
            exit_code = 1

        sys.exit(exit_code)

# Initialize HORUS integration
if __name__ == "__main__":
    print(" HORUS Python Node Starting...", file=sys.stderr)
    scheduler = HorusSchedulerIntegration()
    scheduler.run_node()
"#,
        original_file.display(),
        original_file.display()
    );

    fs::write(&wrapper_path, wrapper_content)?;

    Ok(wrapper_path)
}

fn clean_build_cache() -> Result<()> {
    // Clean .horus/cache directory (where compiled binaries are stored)
    let cache_dir = PathBuf::from(".horus/cache");
    if cache_dir.exists() {
        for entry in fs::read_dir(&cache_dir)? {
            let entry = entry?;
            fs::remove_file(entry.path()).ok();
        }
        println!("  {} Cleaned .horus/cache/", "".green());
    }

    // Clean .horus/bin directory
    let bin_dir = PathBuf::from(".horus/bin");
    if bin_dir.exists() {
        for entry in fs::read_dir(&bin_dir)? {
            let entry = entry?;
            fs::remove_file(entry.path()).ok();
        }
        println!("  {} Cleaned .horus/bin/", "".green());
    }

    // Clean Rust target directory if exists
    let target_dir = PathBuf::from("target");
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
        println!("  {} Cleaned target/", "".green());
    }

    // Clean Python __pycache__ in current directory
    let pycache = PathBuf::from("__pycache__");
    if pycache.exists() {
        fs::remove_dir_all(&pycache)?;
        println!("  {} Cleaned __pycache__/", "".green());
    }

    Ok(())
}

fn execute_with_scheduler(
    file: PathBuf,
    language: String,
    args: Vec<String>,
    release: bool,
    clean: bool,
) -> Result<()> {
    match language.as_str() {
        "rust" => {
            // Use Cargo-based compilation (same as horus.yaml path)
            println!("{} Setting up Cargo workspace...", "".cyan());

            // Parse horus.yaml to get dependencies
            let (horus_deps, cargo_packages, path_deps) = if Path::new("horus.yaml").exists() {
                let deps = parse_horus_yaml_dependencies("horus.yaml")?;
                let (horus_pkgs, _pip_pkgs, cargo_pkgs, path_pkgs) =
                    split_dependencies_with_path_context(deps, Some("rust"));
                (horus_pkgs, cargo_pkgs, path_pkgs)
            } else {
                (Vec::new(), Vec::new(), Vec::new())
            };

            // Find HORUS source directory
            let horus_source = find_horus_source_dir()?;
            println!(
                "  {} Using HORUS source: {}",
                "".cyan(),
                horus_source.display()
            );

            // Generate Cargo.toml in .horus/ that references the source file
            let cargo_toml_path = PathBuf::from(".horus/Cargo.toml");

            // Get relative path from .horus/ to the source file
            let source_relative_path = format!("../{}", file.display());

            let mut cargo_toml = format!(
                r#"[package]
name = "horus-project"
version = "0.1.0"
edition = "2021"

# Empty workspace to prevent inheriting parent workspace
[workspace]

[[bin]]
name = "horus-project"
path = "{}"

[dependencies]
"#,
                source_relative_path
            );

            // Add HORUS dependencies from horus.yaml or defaults
            let horus_packages_to_add = if !horus_deps.is_empty() {
                horus_deps
            } else {
                // Default HORUS packages if no horus.yaml
                vec!["horus".to_string(), "horus_library".to_string()]
            };

            for dep in &horus_packages_to_add {
                // Strip version from dependency name for path lookup
                let dep_name = if let Some(at_pos) = dep.find('@') {
                    &dep[..at_pos]
                } else {
                    dep.as_str()
                };

                let dep_path = horus_source.join(dep_name);
                if dep_path.exists() && dep_path.join("Cargo.toml").exists() {
                    cargo_toml.push_str(&format!(
                        "{} = {{ path = \"{}\" }}\n",
                        dep_name,
                        dep_path.display()
                    ));
                    println!(
                        "  {} Added dependency: {} -> {}",
                        "".cyan(),
                        dep,
                        dep_path.display()
                    );
                } else {
                    eprintln!(
                        "  {} Warning: dependency {} not found at {}",
                        "".yellow(),
                        dep,
                        dep_path.display()
                    );
                }
            }

            // Add cargo dependencies from crates.io
            for pkg in &cargo_packages {
                if !pkg.features.is_empty() {
                    // With features
                    if let Some(ref version) = pkg.version {
                        cargo_toml.push_str(&format!(
                            "{} = {{ version = \"{}\", features = [{}] }}\n",
                            pkg.name,
                            version,
                            pkg.features
                                .iter()
                                .map(|f| format!("\"{}\"", f))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    } else {
                        cargo_toml.push_str(&format!(
                            "{} = {{ version = \"*\", features = [{}] }}\n",
                            pkg.name,
                            pkg.features
                                .iter()
                                .map(|f| format!("\"{}\"", f))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                        eprintln!(
                            "  {} Warning: Using wildcard version for '{}' - specify a version for reproducibility",
                            "".yellow(),
                            pkg.name
                        );
                    }
                    println!(
                        "  {} Added crates.io dependency: {} (features: {})",
                        "".cyan(),
                        pkg.name,
                        pkg.features.join(", ")
                    );
                } else if let Some(ref version) = pkg.version {
                    cargo_toml.push_str(&format!("{} = \"{}\"\n", pkg.name, version));
                    println!(
                        "  {} Added crates.io dependency: {}@{}",
                        "".cyan(),
                        pkg.name,
                        version
                    );
                } else {
                    cargo_toml.push_str(&format!("{} = \"*\"\n", pkg.name));
                    eprintln!(
                        "  {} Warning: Using wildcard version for '{}' - specify a version for reproducibility",
                        "".yellow(),
                        pkg.name
                    );
                    eprintln!("     Example: 'cargo:{}@1.0' in horus.yaml", pkg.name);
                    println!("  {} Added crates.io dependency: {}", "".cyan(), pkg.name);
                }
            }

            // Add path dependencies
            for (pkg_name, pkg_path) in &path_deps {
                // Convert relative path from current directory to relative from .horus/
                let full_path = PathBuf::from("..").join(pkg_path);
                cargo_toml.push_str(&format!(
                    "{} = {{ path = \"{}\" }}\n",
                    pkg_name,
                    full_path.display()
                ));
                println!(
                    "  {} Added path dependency: {} -> {}",
                    "".cyan(),
                    pkg_name,
                    pkg_path
                );
            }

            // Also add dependencies directly from horus.yaml (in case some weren't parsed by resolve_dependencies)
            if Path::new("horus.yaml").exists() {
                if let Ok(yaml_content) = fs::read_to_string("horus.yaml") {
                    if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&yaml_content) {
                        if let Some(deps_value) = yaml.get("dependencies") {
                            if let serde_yaml::Value::Sequence(list) = deps_value {
                                for item in list {
                                    if let Some(dep_str) = parse_yaml_cargo_dependency(item) {
                                        cargo_toml.push_str(&format!("{}\n", dep_str));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            fs::write(&cargo_toml_path, cargo_toml)?;
            println!("  {} Generated Cargo.toml", "".green());

            // Run cargo clean if requested
            if clean {
                println!("{} Cleaning build artifacts...", "".cyan());
                let mut clean_cmd = Command::new("cargo");
                clean_cmd.arg("clean");
                clean_cmd.current_dir(".horus");
                let status = clean_cmd.status()?;
                if !status.success() {
                    eprintln!("{} Warning: cargo clean failed", "[!]".yellow());
                }
            }

            // Run cargo build in .horus directory
            println!("{} Building with Cargo...", "".cyan());
            let mut cmd = Command::new("cargo");
            cmd.arg("build");
            cmd.current_dir(".horus");
            if release {
                cmd.arg("--release");
            }

            let status = cmd.status()?;
            if !status.success() {
                bail!("Cargo build failed");
            }

            // Determine binary path
            let binary_path = if release {
                ".horus/target/release/horus-project"
            } else {
                ".horus/target/debug/horus-project"
            };

            // Execute the binary
            println!("{} Executing...\n", "".cyan());
            let mut cmd = Command::new(binary_path);
            cmd.args(args);

            let status = cmd.status()?;

            // Exit with the same code as the program
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        "python" => {
            execute_python_node(file, args, release)?;
        }
        "c" => {
            execute_c_node(file, args, release)?;
        }
        _ => bail!("Unsupported language: {}", language),
    }

    Ok(())
}

fn get_project_name() -> Result<String> {
    // Try to get from Cargo.toml
    if Path::new("Cargo.toml").exists() {
        let content = fs::read_to_string("Cargo.toml")?;
        for line in content.lines() {
            if line.starts_with("name = ") {
                let name = line[7..].trim_matches('"').trim_matches('\'');
                return Ok(name.to_string());
            }
        }
    }

    // Fallback to directory name
    let current_dir = env::current_dir()?;
    Ok(current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("main")
        .to_string())
}

#[allow(dead_code)]
fn create_minimal_cargo_toml(file: &Path) -> Result<()> {
    let project_name = env::current_dir()?
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("horus_project")
        .to_string();

    let _file_name = file.file_stem().and_then(|n| n.to_str()).unwrap_or("main");

    let content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{}"
path = "{}"

[dependencies]
# HORUS dependencies will be auto-detected and added
horus = "0.1.0"
"#,
        project_name.replace("-", "_"),
        project_name.replace("-", "_"),
        file.display()
    );

    fs::write("Cargo.toml", content)?;
    println!("  {} Created Cargo.toml for {}", "".green(), project_name);

    Ok(())
}

#[allow(dead_code)]
fn setup_rust_environment(_source: &Path) -> Result<()> {
    // Rust compilation uses .horus/target/ via Cargo
    // No additional setup needed
    Ok(())
}

fn setup_c_environment() -> Result<()> {
    let horus_dir = PathBuf::from(".horus");
    let include_dir = horus_dir.join("include");
    let lib_dir = horus_dir.join("lib");

    // Copy horus.h header file to .horus/include/
    let header_path = include_dir.join("horus.h");
    if !header_path.exists() {
        // Try to copy from horus_cpp directory first
        let possible_h_paths = ["horus_cpp/include/horus.h", "../horus_cpp/include/horus.h"];

        let mut h_found = false;
        for path in &possible_h_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                fs::copy(&p, &header_path)?;
                println!("  {} Installed horus.h", "".green());
                h_found = true;
                break;
            }
        }

        // Fallback to embedded horus.h content if not found
        if !h_found {
            // Embedded horus.h content
            let header_content = r#"// HORUS C API - Hardware driver integration interface
#ifndef HORUS_H
#define HORUS_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque handle types - users never see internals
typedef uint32_t Node;
typedef uint32_t Pub;
typedef uint32_t Sub;
typedef uint32_t Scheduler;

// Message type identifiers
typedef enum {
    MSG_CUSTOM = 0,
    MSG_TWIST,
    MSG_POSE,
    MSG_LASER_SCAN,
    MSG_IMAGE,
    MSG_IMU,
    MSG_JOINT_STATE,
    MSG_POINT_CLOUD,
} MessageType;

// Core API - Simple and safe
bool init(const char* node_name);
void shutdown(void);
bool ok(void);

// Publisher/Subscriber
Pub publisher(const char* topic, MessageType type);
Pub publisher_custom(const char* topic, size_t msg_size);
Sub subscriber(const char* topic, MessageType type);
Sub subscriber_custom(const char* topic, size_t msg_size);

// Send/Receive
bool send(Pub pub, const void* data);
bool recv(Sub sub, void* data);
bool try_recv(Sub sub, void* data);

// Timing
void sleep_ms(uint32_t ms);
uint64_t time_now_ms(void);
void spin_once(void);
void spin(void);

// Logging
void log_info(const char* msg);
void log_warn(const char* msg);
void log_error(const char* msg);
void log_debug(const char* msg);

// Common message structs
typedef struct {
    float x, y, z;
} Vector3;

typedef struct {
    float x, y, z, w;
} Quaternion;

typedef struct {
    Vector3 linear;
    Vector3 angular;
} Twist;

typedef struct {
    Vector3 position;
    Quaternion orientation;
} Pose;

typedef struct {
    Vector3 linear_acceleration;
    Vector3 angular_velocity;
    Quaternion orientation;
    float covariance[9];
} IMU;

typedef struct {
    float* ranges;
    float* intensities;
    uint32_t count;
    float angle_min;
    float angle_max;
    float angle_increment;
    float range_min;
    float range_max;
    float scan_time;
} LaserScan;

typedef struct {
    uint8_t* data;
    uint32_t width;
    uint32_t height;
    uint32_t step;
    uint8_t channels;
} Image;

typedef struct {
    float* positions;
    float* velocities;
    float* efforts;
    char** names;
    uint32_t count;
} JointState;

typedef struct {
    float* points;  // x,y,z packed array
    uint32_t count;
    uint32_t stride;  // bytes between points
} PointCloud;

#ifdef __cplusplus
}
#endif

#endif // HORUS_H"#;
            fs::write(&header_path, header_content)?;
            println!("  {} Installed horus.h (embedded fallback)", "".green());
        }
    }

    // Copy horus.hpp C++ header file to .horus/include/
    let hpp_header_path = include_dir.join("horus.hpp");
    if !hpp_header_path.exists() {
        // Try to find horus.hpp in horus_cpp directory
        let possible_hpp_paths = [
            "horus_cpp/include/horus.hpp",
            "../horus_cpp/include/horus.hpp",
            "target/horus_cpp/include/horus.hpp",
        ];

        let mut hpp_found = false;
        for path in &possible_hpp_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                fs::copy(&p, &hpp_header_path)?;
                println!("  {} Installed horus.hpp", "".green());
                hpp_found = true;
                break;
            }
        }

        if !hpp_found {
            println!(
                "  {} horus.hpp not found - C++ framework API not available",
                "".yellow()
            );
        }
    }

    // Check if horus_cpp library exists in .horus/lib/
    let lib_name = if cfg!(target_os = "windows") {
        "horus_cpp.dll"
    } else if cfg!(target_os = "macos") {
        "libhorus_cpp.dylib"
    } else {
        "libhorus_cpp.so"
    };

    let lib_path = lib_dir.join(lib_name);
    if !lib_path.exists() {
        // Try to find and copy horus_cpp library
        if let Ok(horus_cpp_lib) = find_horus_cpp_library() {
            fs::copy(&horus_cpp_lib, &lib_path)?;
            println!("  {} Installed {}", "".green(), lib_name);
        } else {
            println!(
                "  {} {} not found - will attempt to build",
                "".yellow(),
                lib_name
            );
        }
    }

    Ok(())
}

fn find_horus_cpp_library() -> Result<PathBuf> {
    // Look for horus_cpp library in common locations
    let possible_paths = [
        "horus_cpp/target/release/libhorus_cpp.so",
        "horus_cpp/target/debug/libhorus_cpp.so",
        "../horus_cpp/target/release/libhorus_cpp.so",
        "../horus_cpp/target/debug/libhorus_cpp.so",
        "target/release/libhorus_cpp.so",
        "target/debug/libhorus_cpp.so",
    ];

    for path in &possible_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    bail!("horus_cpp library not found")
}

fn execute_c_node(file: PathBuf, args: Vec<String>, release: bool) -> Result<()> {
    eprintln!("{} Setting up C environment...", "".cyan());

    // Detect C compiler
    let compiler = detect_c_compiler()?;
    eprintln!("  {} Using {} compiler", "".green(), compiler);

    // Generate cache-friendly binary name
    let binary_name = generate_c_binary_name(&file, release)?;
    let cache_dir = PathBuf::from(".horus/cache");
    fs::create_dir_all(&cache_dir)?;
    let binary_path = cache_dir.join(&binary_name);

    // Check if we need to compile
    let needs_compile = should_recompile(&file, &binary_path)?;

    if needs_compile {
        eprintln!(
            "{} Compiling C program ({} mode)...",
            "".cyan(),
            if release { "release" } else { "debug" }
        );

        compile_c_file(&file, &binary_path, &compiler, release)?;
        eprintln!("  {} Compiled to {}", "".green(), binary_path.display());
    } else {
        eprintln!("  {} Using cached binary", "".green());
    }

    // Execute the binary
    eprintln!("{} Executing C program...", "".cyan());
    let mut cmd = Command::new(&binary_path);
    cmd.args(args);

    let status = cmd.status()?;

    // Exit with the same code as the program
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn detect_c_compiler() -> Result<String> {
    // Try compilers in order of preference
    let compilers = ["gcc", "clang", "cc"];

    for compiler in &compilers {
        if Command::new(compiler).arg("--version").output().is_ok() {
            return Ok(compiler.to_string());
        }
    }

    bail!("No C compiler found. Please install gcc, clang, or another C compiler and ensure it's in PATH.")
}

fn generate_c_binary_name(file: &Path, release: bool) -> Result<String> {
    let file_stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program");

    let mode = if release { "release" } else { "debug" };
    let binary_name = format!("{}_{}", file_stem, mode);

    Ok(binary_name)
}

fn should_recompile(source: &Path, binary: &Path) -> Result<bool> {
    if !binary.exists() {
        return Ok(true);
    }

    // Check if source is newer than binary
    let source_modified = fs::metadata(source)?.modified()?;
    let binary_modified = fs::metadata(binary)?.modified()?;

    Ok(source_modified > binary_modified)
}

fn compile_c_file(source: &Path, output: &Path, compiler: &str, release: bool) -> Result<()> {
    // Detect if this is a C++ file
    let is_cpp = matches!(
        source.extension().and_then(|e| e.to_str()),
        Some("cpp") | Some("cc") | Some("cxx") | Some("C")
    );

    // Use C++ compiler if needed
    let actual_compiler = if is_cpp {
        if compiler.contains("gcc") {
            "g++"
        } else if compiler.contains("clang") {
            "clang++"
        } else {
            "g++" // default
        }
    } else {
        compiler
    };

    let mut cmd = Command::new(actual_compiler);

    // Basic arguments
    cmd.arg(source);
    cmd.arg("-o");
    cmd.arg(output);

    // Add C++ standard if C++ file
    if is_cpp {
        cmd.arg("-std=c++17");
    }

    // Check if source uses HORUS headers
    let content = fs::read_to_string(source).unwrap_or_default();
    let uses_horus_h = content.contains("#include <horus.h>")
        || content.contains("#include \"horus.h\"")
        || content.contains("horus.h\""); // Catches relative paths too
    let uses_horus_hpp = content.contains("#include <horus.hpp>")
        || content.contains("#include \"horus.hpp\"")
        || content.contains("horus.hpp\""); // Catches relative paths too
    let _uses_framework =
        uses_horus_hpp || content.contains("horus::Node") || content.contains("horus::Scheduler");

    if uses_horus_h || uses_horus_hpp {
        // Include path for horus headers
        cmd.arg("-I.horus/include");

        // Library path
        cmd.arg("-L.horus/lib");

        // Link with horus_cpp (works for both C and C++)
        let horus_cpp_lib = PathBuf::from(".horus/lib/libhorus_cpp.so");
        if horus_cpp_lib.exists() {
            cmd.arg("-lhorus_cpp");
        } else {
            println!("  {} libhorus_cpp.so not found in .horus/lib/", "".yellow());
        }
    }

    // Standard libraries
    cmd.arg("-lpthread");
    cmd.arg("-ldl");
    cmd.arg("-lm");

    // Optimization flags
    if release {
        cmd.arg("-O2");
        cmd.arg("-DNDEBUG");
    } else {
        cmd.arg("-g");
        cmd.arg("-O0");
        cmd.arg("-DDEBUG");
    }

    // Warning flags
    cmd.arg("-Wall");
    cmd.arg("-Wextra");

    // Runtime library path
    #[cfg(target_os = "linux")]
    cmd.arg("-Wl,-rpath,.horus/lib");

    #[cfg(target_os = "macos")]
    cmd.arg("-Wl,-rpath,@loader_path/../lib");

    // Execute compilation
    let output_result = cmd.output()?;

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        eprintln!("{} Compilation failed:", "".red());
        eprintln!("{}", stderr);
        bail!("C compilation failed");
    }

    // Make binary executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(output)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(output, perms)?;
    }

    Ok(())
}

/// Find the HORUS source directory by checking common locations
fn find_horus_source_dir() -> Result<PathBuf> {
    // Check environment variable first
    if let Ok(horus_source) = env::var("HORUS_SOURCE") {
        let path = PathBuf::from(horus_source);
        if path.exists() && path.join("horus/Cargo.toml").exists() {
            return Ok(path);
        }
    }

    // Check common development locations
    let candidates = vec![
        PathBuf::from("/horus"),
        home_dir().join("horus/HORUS"),
        home_dir().join("horus"),
        PathBuf::from("/opt/horus"),
        PathBuf::from("/usr/local/horus"),
    ];

    for candidate in candidates {
        if candidate.exists() && candidate.join("horus/Cargo.toml").exists() {
            return Ok(candidate);
        }
    }

    // Fallback: Check for installed packages in cache
    let cache_dir = home_dir().join(".horus/cache");
    if cache_dir.join("horus@0.1.0").exists() {
        return Ok(cache_dir);
    }

    bail!("HORUS not found. Please install HORUS or set HORUS_SOURCE environment variable.")
}

#[derive(Debug, Clone, PartialEq)]
enum SystemPackageChoiceRun {
    UseSystem,
    InstallHORUS,
    Cancel,
}

fn detect_system_cargo_binary(package_name: &str) -> Result<Option<String>> {
    use std::process::Command;

    // Check ~/.cargo/bin/
    if let Some(home) = dirs::home_dir() {
        let cargo_bin = home.join(".cargo/bin").join(package_name);
        if cargo_bin.exists() {
            // Try to get version by running --version
            if let Ok(output) = Command::new(&cargo_bin).arg("--version").output() {
                if output.status.success() {
                    let version_str = String::from_utf8_lossy(&output.stdout);
                    // Parse version (usually "name version")
                    let version = version_str
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("unknown")
                        .to_string();
                    return Ok(Some(version));
                }
            }
            // Binary exists but version unknown
            return Ok(Some("unknown".to_string()));
        }
    }

    Ok(None)
}

fn prompt_system_cargo_choice_run(
    package_name: &str,
    system_version: &str,
) -> Result<SystemPackageChoiceRun> {
    use std::io::{self, Write};

    println!(
        "\n{} crates.io {} found in system (version: {})",
        "⚠".yellow(),
        package_name.green(),
        system_version.cyan()
    );
    println!("\nWhat would you like to do?");
    println!("  [1] {} Use system binary (create reference)", "".green());
    println!(
        "  [2] {} Install to HORUS (isolated environment)",
        "".blue()
    );
    println!("  [3] {} Skip this package", "⊘".yellow());

    print!("\nChoice [1-3]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    match input.trim() {
        "1" => Ok(SystemPackageChoiceRun::UseSystem),
        "2" => Ok(SystemPackageChoiceRun::InstallHORUS),
        "3" => Ok(SystemPackageChoiceRun::Cancel),
        _ => {
            println!("Invalid choice, defaulting to Install to HORUS");
            Ok(SystemPackageChoiceRun::InstallHORUS)
        }
    }
}

fn create_system_reference_cargo_run(package_name: &str, system_version: &str) -> Result<()> {
    println!("  {} Creating reference to system binary...", "".green());

    // Find actual system binary location
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    let cargo_bin = home.join(".cargo/bin").join(package_name);

    if !cargo_bin.exists() {
        bail!("System binary not found at expected location");
    }

    let packages_dir = PathBuf::from(".horus/packages");
    fs::create_dir_all(&packages_dir)?;

    let metadata_file = packages_dir.join(format!("{}.system.json", package_name));
    let metadata = serde_json::json!({
        "name": package_name,
        "version": system_version,
        "source": "System",
        "system_path": cargo_bin.display().to_string(),
        "package_type": "CratesIO"
    });

    fs::write(&metadata_file, serde_json::to_string_pretty(&metadata)?)?;

    // Create symlink in .horus/bin to system binary
    let bin_dir = PathBuf::from(".horus/bin");
    fs::create_dir_all(&bin_dir)?;

    let bin_link = bin_dir.join(package_name);
    if bin_link.exists() {
        fs::remove_file(&bin_link)?;
    }
    std::os::unix::fs::symlink(&cargo_bin, &bin_link)?;

    println!(
        "  {} Using system binary at {}",
        "".green(),
        cargo_bin.display()
    );
    println!(
        "  {} Reference created: {}",
        "".cyan(),
        metadata_file.display()
    );
    println!("  {} Binary linked: {}", "".cyan(), bin_link.display());

    Ok(())
}

fn detect_system_python_package(package_name: &str) -> Result<Option<String>> {
    use std::process::Command;

    // Check if package is installed in system Python using pip show
    let output = Command::new("python3")
        .args(&["-m", "pip", "show", package_name])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse version from pip show output
            for line in stdout.lines() {
                if line.starts_with("Version:") {
                    let version = line.trim_start_matches("Version:").trim().to_string();
                    return Ok(Some(version));
                }
            }
            // Package found but version unknown
            return Ok(Some("unknown".to_string()));
        }
    }

    Ok(None)
}

fn prompt_system_package_choice_run(
    package_name: &str,
    system_version: &str,
) -> Result<SystemPackageChoiceRun> {
    use std::io::{self, Write};

    println!(
        "\n{} PyPI package {} found in system (version: {})",
        "⚠".yellow(),
        package_name.green(),
        system_version.cyan()
    );
    println!("\nWhat would you like to do?");
    println!("  [1] {} Use system package (create reference)", "".green());
    println!(
        "  [2] {} Install to HORUS (isolated environment)",
        "".blue()
    );
    println!("  [3] {} Skip this package", "⊘".yellow());

    print!("\nChoice [1-3]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    match input.trim() {
        "1" => Ok(SystemPackageChoiceRun::UseSystem),
        "2" => Ok(SystemPackageChoiceRun::InstallHORUS),
        "3" => Ok(SystemPackageChoiceRun::Cancel),
        _ => {
            println!("Invalid choice, defaulting to Install to HORUS");
            Ok(SystemPackageChoiceRun::InstallHORUS)
        }
    }
}

fn create_system_reference_python_run(package_name: &str, system_version: &str) -> Result<()> {
    println!("  {} Creating reference to system package...", "".green());

    let packages_dir = PathBuf::from(".horus/packages");
    fs::create_dir_all(&packages_dir)?;

    let metadata_file = packages_dir.join(format!("{}.system.json", package_name));
    let metadata = serde_json::json!({
        "name": package_name,
        "version": system_version,
        "source": "System",
        "package_type": "PyPI"
    });

    fs::write(&metadata_file, serde_json::to_string_pretty(&metadata)?)?;

    println!("  {} Using system package", "".green());
    println!(
        "  {} Reference created: {}",
        "".cyan(),
        metadata_file.display()
    );

    Ok(())
}

/// Auto-create or update horus.yaml with detected dependencies
fn auto_update_horus_yaml(
    file: &Path,
    language: &str,
    dependencies: &HashSet<String>,
) -> Result<()> {
    let yaml_path = PathBuf::from("horus.yaml");

    if yaml_path.exists() {
        // Update existing horus.yaml
        update_existing_horus_yaml(&yaml_path, language, dependencies)?;
    } else {
        // Create new horus.yaml
        create_horus_yaml(&yaml_path, file, language, dependencies)?;
    }

    Ok(())
}

/// Create new horus.yaml from scratch
fn create_horus_yaml(
    yaml_path: &Path,
    file: &Path,
    language: &str,
    dependencies: &HashSet<String>,
) -> Result<()> {
    // Derive project name from directory or file
    let project_name = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| {
            file.file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("my-project")
                .to_string()
        });

    // Categorize dependencies based on language context
    let mut horus_deps = Vec::new();
    let mut pip_deps = Vec::new();
    let mut cargo_deps = Vec::new();

    for dep in dependencies {
        if dep.starts_with("horus") {
            horus_deps.push(dep.clone());
        } else {
            // Default based on language
            match language {
                "python" => pip_deps.push(format!("pip:{}", dep)),
                "rust" => cargo_deps.push(format!("cargo:{}", dep)),
                "c" => {} // C deps are handled differently
                _ => pip_deps.push(format!("pip:{}", dep)), // Default to pip
            }
        }
    }

    // Sort for consistent output
    horus_deps.sort();
    pip_deps.sort();
    cargo_deps.sort();

    // Build YAML content
    let mut content = String::new();
    content.push_str(&format!("name: {}\n", project_name));
    content.push_str("version: 0.1.0\n");
    content.push_str(&format!("language: {}\n", language));
    content.push_str("\ndependencies:\n");

    // Add HORUS packages first
    for dep in horus_deps {
        content.push_str(&format!("  - {}\n", dep));
    }

    // Add pip packages
    for dep in pip_deps {
        content.push_str(&format!("  - {}\n", dep));
    }

    // Add cargo packages
    for dep in cargo_deps {
        content.push_str(&format!("  - {}\n", dep));
    }

    // Write file
    fs::write(yaml_path, content)?;

    eprintln!(
        "  {} Created horus.yaml with {} dependencies",
        "".green(),
        dependencies.len()
    );

    Ok(())
}

/// Update existing horus.yaml with new dependencies
fn update_existing_horus_yaml(
    yaml_path: &Path,
    language: &str,
    new_dependencies: &HashSet<String>,
) -> Result<()> {
    // Parse existing yaml to get current dependencies
    let existing_content = fs::read_to_string(yaml_path)?;
    let existing_deps = parse_horus_yaml_dependencies_from_content(&existing_content)?;

    // Find new dependencies not in existing
    let mut added = Vec::new();
    for dep in new_dependencies {
        let dep_str = if dep.starts_with("horus") {
            dep.clone()
        } else {
            // Categorize based on language context
            match language {
                "python" => format!("pip:{}", dep),
                "rust" => format!("cargo:{}", dep),
                _ => format!("pip:{}", dep), // Default to pip
            }
        };

        // Check if dependency already exists (in any form)
        let base_name = dep_str
            .split(':')
            .last()
            .unwrap_or(&dep_str)
            .split('@')
            .next()
            .unwrap_or(&dep_str);

        let already_exists = existing_deps.iter().any(|e| {
            let e_base = e
                .split(':')
                .last()
                .unwrap_or(e)
                .split('@')
                .next()
                .unwrap_or(e);
            e_base == base_name
        });

        if !already_exists {
            added.push(dep_str);
        }
    }

    if added.is_empty() {
        return Ok(()); // No new dependencies
    }

    // Append new dependencies to file
    let mut content = existing_content;
    if !content.ends_with('\n') {
        content.push('\n');
    }

    for dep in &added {
        content.push_str(&format!("  - {}\n", dep));
    }

    fs::write(yaml_path, content)?;

    eprintln!(
        "  {} Added {} new dependencies to horus.yaml",
        "".green(),
        added.len()
    );
    for dep in &added {
        eprintln!("     + {}", dep);
    }

    Ok(())
}

/// Parse dependencies from YAML content string
fn parse_horus_yaml_dependencies_from_content(content: &str) -> Result<HashSet<String>> {
    let mut dependencies = HashSet::new();
    let mut in_deps = false;

    for line in content.lines() {
        if line.trim() == "dependencies:" {
            in_deps = true;
            continue;
        }

        if in_deps {
            if line.starts_with("  -") {
                let dep = line.trim_start_matches("  -").trim();
                if !dep.is_empty() && !dep.starts_with('#') {
                    dependencies.insert(dep.to_string());
                }
            } else if !line.starts_with("  ") && !line.trim().is_empty() {
                // End of dependencies section
                in_deps = false;
            }
        }
    }

    Ok(dependencies)
}

/// Parse session_id from horus.yaml
/// Returns Ok(Some(session_id)) if found, Ok(None) if not found, Err if file error
fn parse_session_id_from_horus_yaml(path: &str) -> Result<Option<String>> {
    if !Path::new(path).exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;

    // Try parsing as YAML first
    if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
        if let Some(session_id_value) = yaml.get("session_id") {
            if let Some(session_id) = session_id_value.as_str() {
                return Ok(Some(session_id.to_string()));
            }
        }
    }

    Ok(None)
}
