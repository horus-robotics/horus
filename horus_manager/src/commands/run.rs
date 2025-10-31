use crate::version;
use anyhow::{bail, Context, Result};
use colored::*;
use glob::glob;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
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

            // Generate Cargo.toml in .horus/ that references source files in parent directory
            let cargo_toml_path = PathBuf::from(".horus/Cargo.toml");

            // Get relative path from .horus/ to the source file
            let source_relative_path = format!("../{}", target_file.display());

            let mut cargo_toml = format!(
                r#"[package]
name = "horus-project"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "horus-project"
path = "{}"

[dependencies]
"#,
                source_relative_path
            );

            // Find HORUS source directory
            let horus_source = find_horus_source_dir()?;
            println!("  {} Using HORUS source: {}", "".cyan(), horus_source.display());

            // Add dependencies from HORUS source
            for dep in &dependencies {
                // Map dependency to source directory
                let dep_path = horus_source.join(dep);

                if dep_path.exists() && dep_path.join("Cargo.toml").exists() {
                    cargo_toml.push_str(&format!(
                        "{} = {{ path = \"{}\" }}\n",
                        dep,
                        dep_path.display()
                    ));
                    println!("  {} Added dependency: {} -> {}", "".cyan(), dep, dep_path.display());
                } else {
                    eprintln!(
                        "  {} Warning: dependency {} not found at {}",
                        "".yellow(),
                        dep,
                        dep_path.display()
                    );
                }
            }

            fs::write(&cargo_toml_path, cargo_toml)?;
            println!("  {} Generated Cargo.toml (no source copying needed)", "".green());

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

            println!(
                "{} Successfully built: {}",
                "".green(),
                binary_path.green()
            );
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
    // Generate unique session ID for this run
    let session_id = Uuid::new_v4().to_string();
    env::set_var("HORUS_SESSION_ID", &session_id);

    let language = detect_language(&file_path)?;

    eprintln!(
        "{} Detected: {} ({})",
        "".cyan(),
        file_path.display().to_string().green(),
        language.yellow()
    );
    eprintln!("  {} Session: {}", "🔒".dimmed(), session_id.dimmed());

    // Ensure .horus directory exists
    ensure_horus_directory()?;

    // Scan imports and resolve dependencies
    eprintln!("{} Scanning imports...", "".cyan());
    let dependencies = scan_imports(&file_path, &language)?;

    if !dependencies.is_empty() {
        eprintln!("{} Found {} dependencies", "".cyan(), dependencies.len());
        resolve_dependencies(dependencies)?;
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
    println!("  {} Session: {}", "🔒".dimmed(), session_id.dimmed());

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
            file_path,
            language,
            release,
            false, // Don't clean - already done if needed
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
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

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
                eprintln!(
                    "  {} Failed to start [{}]: {}",
                    "".red(),
                    node_name,
                    e
                );
            }
        }
    }

    println!("\n{} All processes running. Press Ctrl+C to stop.\n", "".green());

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
        fs::remove_dir_all(&session_dir)
            .with_context(|| format!("Failed to clean up session directory: {}", session_dir.display()))?;
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

    // Find HORUS source directory
    let horus_source = find_horus_source_dir()?;

    // Collect all dependencies from all Rust files
    let mut all_dependencies = HashSet::new();
    for file_path in &file_paths {
        let dependencies = scan_imports(file_path, "rust")?;
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

    // Scan imports and resolve dependencies
    let dependencies = scan_imports(&file_path, &language)?;
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
    let mut files = Vec::new();

    for entry in glob(pattern).context("Failed to parse glob pattern")? {
        match entry {
            Ok(path) => {
                if path.is_file() {
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
        if path.exists() {
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
            if let Some(ext) = e.path().extension() {
                matches!(ext.to_str(), Some("rs") | Some("py") | Some("c"))
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

fn scan_imports(file: &Path, language: &str) -> Result<HashSet<String>> {
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
                // Scan for: import horus, from horus_library import
                for line in content.lines() {
                    if let Some(dep) = parse_python_import(line) {
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

    // Only filter HORUS packages when scanning from source code
    // When using horus.yaml, keep all dependencies (HORUS and pip)
    if !from_yaml {
        dependencies.retain(|d| is_horus_package(d));
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

fn parse_horus_yaml_dependencies(path: &str) -> Result<HashSet<String>> {
    let content = fs::read_to_string(path)?;
    let mut dependencies = HashSet::new();

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

            // Insert the full dependency string (including version)
            // This will be split later into HORUS vs pip packages
            dependencies.insert(dep_str.to_string());
        }
    }

    Ok(dependencies)
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

/// Separate HORUS packages from pip packages
fn split_dependencies(deps: HashSet<String>) -> (Vec<String>, Vec<PipPackage>) {
    let mut horus_packages = Vec::new();
    let mut pip_packages = Vec::new();

    for dep in deps {
        let dep = dep.trim();

        // Check for explicit pip: prefix
        if dep.starts_with("pip:") {
            let pkg_str = dep.strip_prefix("pip:").unwrap();
            if let Ok(pkg) = PipPackage::from_string(pkg_str) {
                pip_packages.push(pkg);
            }
            continue;
        }

        // Auto-detect: if starts with "horus" → HORUS registry
        if dep.starts_with("horus") {
            horus_packages.push(dep.to_string());
            continue;
        }

        // Check if it's a known HORUS package using registry
        if is_horus_package(dep) {
            horus_packages.push(dep.to_string());
            continue;
        }

        // Otherwise, assume it's a pip package
        if let Ok(pkg) = PipPackage::from_string(dep) {
            pip_packages.push(pkg);
        }
    }

    (horus_packages, pip_packages)
}

/// Ensure .horus/venv exists and is set up
fn ensure_python_venv() -> Result<PathBuf> {
    let venv_path = PathBuf::from(".horus/venv");

    if venv_path.exists() {
        return Ok(venv_path);
    }

    println!("{} Creating Python virtual environment...", "🐍".cyan());

    // Find Python interpreter
    let python_cmd = if Command::new("python3")
        .arg("--version")
        .output()
        .is_ok()
    {
        "python3"
    } else if Command::new("python").arg("--version").output().is_ok() {
        "python"
    } else {
        bail!("No Python interpreter found. Please install Python 3.");
    };

    // Create venv
    let status = Command::new(python_cmd)
        .args(&["-m", "venv", venv_path.to_str().unwrap()])
        .status()
        .context("Failed to create virtual environment")?;

    if !status.success() {
        bail!("Virtual environment creation failed");
    }

    // Upgrade pip in the venv
    let pip_path = venv_path.join("bin/pip");
    if pip_path.exists() {
        println!("  {} Upgrading pip...", "↗".cyan());
        Command::new(&pip_path)
            .args(&["install", "--upgrade", "pip", "--quiet"])
            .output()
            .ok();
    }

    println!("  {} Virtual environment ready", "✓".green());
    Ok(venv_path)
}

/// Check if a pip package is installed in venv
fn is_pip_package_installed(venv_path: &PathBuf, package: &PipPackage) -> Result<bool> {
    let pip_path = venv_path.join("bin/pip");

    let output = Command::new(&pip_path)
        .args(&["show", &package.name])
        .output()
        .context("Failed to check package installation")?;

    Ok(output.status.success())
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

    // Create a temporary venv for pip operations
    let temp_venv = ensure_python_venv()?;
    let pip_path = temp_venv.join("bin/pip");

    for pkg in &packages {
        // Get actual version by querying PyPI or using installed version
        let version_str = pkg.version.as_ref()
            .map(|v| v.replace(">=", "").replace("==", "").replace("~=", "").replace(">", "").replace("<", ""))
            .unwrap_or_else(|| "latest".to_string());

        // Cache directory with pypi_ prefix to distinguish from HORUS packages
        let pkg_cache_dir = global_cache.join(format!("pypi_{}@{}", pkg.name, version_str));

        let local_link = local_packages.join(&pkg.name);

        // If already symlinked, skip
        if local_link.exists() || local_link.read_link().is_ok() {
            println!("  {} {} (already linked)", "✓".green(), pkg.name);
            continue;
        }

        // If not cached, install to global cache
        if !pkg_cache_dir.exists() {
            println!("  {} Installing {} to global cache...", "↓".cyan(), pkg.name);

            fs::create_dir_all(&pkg_cache_dir)?;

            // Install package with pip to cache directory
            let mut cmd = Command::new(&pip_path);
            cmd.args(&["install", "--target", pkg_cache_dir.to_str().unwrap()]);
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

            println!("  {} Cached {}", "✓".green(), pkg.name);
        } else {
            println!("  {} {} -> global cache", "↗".cyan(), pkg.name);
        }

        // Symlink from local packages to global cache
        symlink(&pkg_cache_dir, &local_link)
            .context(format!("Failed to symlink {} from global cache", pkg.name))?;
        println!("  {} Linked {}", "✓".green(), pkg.name);
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

    // Split dependencies into HORUS packages and pip packages
    let (horus_packages, pip_packages) = split_dependencies(dependencies);

    // Resolve HORUS packages (existing logic)
    if !horus_packages.is_empty() {
        resolve_horus_packages(horus_packages.into_iter().collect())?;
    }

    // Resolve pip packages (new logic)
    if !pip_packages.is_empty() {
        install_pip_packages(pip_packages)?;
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
            // Special handling for horus_py - the Python package is named "horus"
            if package == "horus_py" {
                // Check if lib/horus exists in the cached package
                let lib_horus = cached.join("lib/horus");
                if lib_horus.exists() {
                    // Create symlink named "horus" pointing to lib/horus
                    let horus_link = local_packages.join("horus");
                    println!("  {} horus_py -> {}", "↗".cyan(), "global cache".dimmed());
                    symlink(&lib_horus, &horus_link).context("Failed to symlink horus_py")?;
                    continue;
                }
            }

            // Create symlink to global cache
            println!(
                "  {} {} -> {}",
                "↗".cyan(),
                package,
                "global cache".dimmed()
            );
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
            println!("\n{} Installing packages from registry...", "".cyan());

            // Import registry client
            use crate::registry::RegistryClient;
            let client = RegistryClient::new();

            for package in &missing_packages {
                print!("  {} Installing {}... ", "".cyan(), package.yellow());
                io::stdout().flush()?;

                match client.install(package, None) {
                    Ok(_) => {
                        println!("{}", "".green());
                        // client.install() already handles global/local detection and symlinks
                    }
                    Err(e) => {
                        println!("{}", "".red());
                        eprintln!("    {} Failed to install {}: {}", "".red(), package, e);
                        bail!("Failed to install required dependency: {}", package);
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

    for entry in fs::read_dir(cache_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Match package@version or just package
        if name_str.starts_with(package)
            && (name_str == package || name_str.starts_with(&format!("{}@", package)))
        {
            versions.push(entry.path());
        }
    }

    // Sort by version (newest first)
    versions.sort_by(|a, b| b.cmp(a));

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

    // Check for Python interpreter (venv or system)
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

        let status = cmd.status()?;

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

        let status = cmd.status()?;

        // Exit with the same code as the Python script
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
    }

    Ok(())
}

#[allow(dead_code)]
fn create_venv_if_needed() -> Result<()> {
    let venv_path = PathBuf::from(".horus/venv");

    if venv_path.exists() {
        return Ok(());
    }

    println!("{} Python virtual environment not found", "[i]".blue());
    println!("  {} Creating venv at .horus/venv/...", "".cyan());

    // Find system Python
    let python_cmd = if Command::new("python3").arg("--version").output().is_ok() {
        "python3"
    } else if Command::new("python").arg("--version").output().is_ok() {
        "python"
    } else {
        bail!("No Python interpreter found for venv creation");
    };

    // Create venv
    let mut cmd = Command::new(python_cmd);
    cmd.arg("-m").arg("venv").arg(&venv_path);

    let status = cmd.status()?;
    if !status.success() {
        bail!("Failed to create Python virtual environment");
    }

    println!("  {} Virtual environment created", "".green());

    // Upgrade pip
    let venv_pip = if cfg!(target_os = "windows") {
        venv_path.join("Scripts/pip.exe")
    } else {
        venv_path.join("bin/pip")
    };

    if venv_pip.exists() {
        println!("  {} Upgrading pip...", "".cyan());
        Command::new(&venv_pip)
            .arg("install")
            .arg("--upgrade")
            .arg("pip")
            .output()
            .ok();
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

    // Check if venv should be used
    let venv_path = PathBuf::from(".horus/venv");
    if venv_path.exists() {
        // Use venv's site-packages
        let venv_site_packages = find_venv_site_packages(&venv_path)?;
        python_paths.push(venv_site_packages.display().to_string());
    }

    // Add existing PYTHONPATH
    if let Ok(current_path) = env::var("PYTHONPATH") {
        python_paths.push(current_path);
    }

    // Set the combined PYTHONPATH
    let new_path = python_paths.join(":");
    env::set_var("PYTHONPATH", new_path);

    Ok(())
}

fn find_venv_site_packages(venv_path: &Path) -> Result<PathBuf> {
    // Look for site-packages in venv
    let lib_path = venv_path.join("lib");
    if lib_path.exists() {
        if let Ok(entries) = fs::read_dir(&lib_path) {
            for entry in entries.flatten() {
                let site_packages = entry.path().join("site-packages");
                if site_packages.exists() {
                    return Ok(site_packages);
                }
            }
        }
    }

    // Fallback: try common patterns
    let patterns = vec![
        venv_path.join("lib/python3.*/site-packages"),
        venv_path.join("Lib/site-packages"), // Windows
    ];

    for pattern in patterns {
        let pattern_str = pattern.to_string_lossy();
        if let Ok(paths) = glob(&pattern_str) {
            if let Some(Ok(path)) = paths.into_iter().next() {
                return Ok(path);
            }
        }
    }

    bail!(
        "Could not find site-packages in venv: {}",
        venv_path.display()
    )
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
import signal
import threading
import time

# HORUS Python bindings are available via the 'horus' package
# Install with: pip install maturin && maturin develop (from horus_py directory)
# Or: pip install -e horus_py/

class HorusSchedulerIntegration:
    def __init__(self):
        self.running = True
        self.setup_signal_handlers()

    def setup_signal_handlers(self):
        """Setup graceful shutdown on Ctrl+C"""
        def signal_handler(sig, frame):
            print("\n🛑 Graceful shutdown initiated...", file=sys.stderr)
            self.running = False
            sys.exit(0)

        signal.signal(signal.SIGINT, signal_handler)
        signal.signal(signal.SIGTERM, signal_handler)

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

            // Find HORUS source directory
            let horus_source = find_horus_source_dir()?;
            println!("  {} Using HORUS source: {}", "".cyan(), horus_source.display());

            // Generate Cargo.toml in .horus/ that references the source file
            let cargo_toml_path = PathBuf::from(".horus/Cargo.toml");

            // Get relative path from .horus/ to the source file
            let source_relative_path = format!("../{}", file.display());

            let mut cargo_toml = format!(
                r#"[package]
name = "horus-project"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "horus-project"
path = "{}"

[dependencies]
"#,
                source_relative_path
            );

            // Add HORUS core dependencies
            // Check if using source (development) or cache (installed)
            if horus_source.ends_with(".horus/cache") || horus_source.ends_with(".horus\\cache") {
                // Using installed packages from cache
                cargo_toml.push_str(&format!(
                    "horus = {{ path = \"{}\" }}\n",
                    horus_source.join("horus@0.1.0/horus").display()
                ));
                cargo_toml.push_str(&format!(
                    "horus_library = {{ path = \"{}\" }}\n",
                    horus_source.join("horus@0.1.0/horus_library").display()
                ));
            } else {
                // Using source code (development)
                cargo_toml.push_str(&format!(
                    "horus = {{ path = \"{}\" }}\n",
                    horus_source.join("horus").display()
                ));
                cargo_toml.push_str(&format!(
                    "horus_library = {{ path = \"{}\" }}\n",
                    horus_source.join("horus_library").display()
                ));
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
    // Ensure .horus/build exists for compilation
    fs::create_dir_all(".horus/build")?;

    // Could add Rust-specific setup here if needed
    // Cargo handles compilation via Cargo.toml in .horus/

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
        let possible_h_paths = [
            "horus_cpp/include/horus.h",
            "../horus_cpp/include/horus.h",
        ];

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
                    || content.contains("horus.h\"");  // Catches relative paths too
    let uses_horus_hpp = content.contains("#include <horus.hpp>")
                      || content.contains("#include \"horus.hpp\"")
                      || content.contains("horus.hpp\"");  // Catches relative paths too
    let uses_framework = uses_horus_hpp
                      || content.contains("horus::Node")
                      || content.contains("horus::Scheduler");

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
            println!(
                "  {} libhorus_cpp.so not found in .horus/lib/",
                "".yellow()
            );
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

    bail!(
        "HORUS not found. Please install HORUS or set HORUS_SOURCE environment variable."
    )
}
