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

#[derive(Debug, Clone)]
enum ExecutionTarget {
    File(PathBuf),
    Directory(PathBuf),
    Manifest(PathBuf),
    Multiple(Vec<PathBuf>),
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
        "🔨".cyan(),
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
        "→".cyan(),
        target_file.display().to_string().green(),
        language.yellow()
    );

    // Ensure .horus directory exists
    ensure_horus_directory()?;

    // Build based on language
    match language.as_str() {
        "python" => {
            println!("{} Python is interpreted, no build needed", "ℹ".blue());
            println!(
                "  {} File is ready to run: {}",
                "→".cyan(),
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

            println!("{} Compiling with {}...", "→".cyan(), compiler);
            compile_c_file(&target_file, &output_path, compiler, release)?;
            println!(
                "{} Successfully built: {}",
                "✓".green(),
                output_path.display().to_string().green()
            );
        }
        "rust" => {
            // Setup Rust build in .horus environment
            setup_rust_environment(&target_file)?;

            // Determine output binary name
            let file_stem = target_file
                .file_stem()
                .context("Invalid file name")?
                .to_string_lossy();
            let suffix = if release { "_release" } else { "_debug" };
            let output_binary = format!(".horus/cache/rust_{}{}", file_stem, suffix);

            // Check if already cached
            if Path::new(&output_binary).exists() {
                println!("{} Using cached Rust binary", "→".cyan());
            } else {
                println!("{} Building Rust project...", "→".cyan());

                // Copy source to .horus/build directory
                let build_dir = PathBuf::from(".horus/build");
                fs::create_dir_all(&build_dir)?;
                let build_source = build_dir.join("main.rs");
                fs::copy(&target_file, &build_source)?;

                // Use rustc directly for single files (MUCH faster than cargo)
                let mut cmd = Command::new("rustc");
                cmd.arg(&build_source);
                cmd.arg("-o").arg(&output_binary);

                if release {
                    cmd.arg("-O"); // Optimization
                }

                // Add HORUS lib paths (local + global)
                cmd.arg("-L").arg(".horus/lib");

                // Add global cache library paths
                let home = home_dir();
                let global_cache = home.join(".horus/cache");
                {
                    if global_cache.exists() {
                        if let Ok(entries) = fs::read_dir(&global_cache) {
                            for entry in entries.flatten() {
                                let lib_dir = entry.path().join("lib");
                                if lib_dir.exists() {
                                    cmd.arg("-L").arg(&lib_dir);
                                }
                                // Also add target/release for Rust packages
                                let target_dir = entry.path().join("target/release");
                                if target_dir.exists() {
                                    cmd.arg("-L").arg(&target_dir);
                                }
                            }
                        }
                    }
                }

                let status = cmd.status()?;
                if !status.success() {
                    bail!("Rust compilation failed");
                }
            }

            println!(
                "{} Successfully built: {}",
                "✓".green(),
                output_binary.green()
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
        "🚀".cyan(),
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
    let language = detect_language(&file_path)?;

    eprintln!(
        "{} Detected: {} ({})",
        "→".cyan(),
        file_path.display().to_string().green(),
        language.yellow()
    );

    // Ensure .horus directory exists
    ensure_horus_directory()?;

    // Scan imports and resolve dependencies
    eprintln!("{} Scanning imports...", "→".cyan());
    let dependencies = scan_imports(&file_path, &language)?;

    if !dependencies.is_empty() {
        eprintln!("{} Found {} dependencies", "→".cyan(), dependencies.len());
        resolve_dependencies(dependencies)?;
    }

    // Setup environment
    setup_environment()?;

    // Execute
    eprintln!("{} Executing...\n", "→".cyan());
    execute_with_scheduler(file_path, language, args, release)?;

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
        "→".cyan(),
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
        "→".cyan(),
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
        println!("{} Scanning Cargo.toml dependencies...", "→".cyan());
        let horus_deps = parse_cargo_dependencies("Cargo.toml")?;

        if !horus_deps.is_empty() {
            println!(
                "{} Found {} HORUS dependencies",
                "→".cyan(),
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
                "→".cyan(),
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
        println!("{} Executing Cargo project...\n", "→".cyan());
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
    println!("{} Detected Makefile project", "→".cyan());

    // Ensure .horus directory exists
    ensure_horus_directory()?;

    // Setup environment with .horus libraries
    setup_environment()?;

    // Clean if requested
    if clean {
        println!("{} Cleaning Makefile project...", "→".cyan());
        Command::new("make").arg("clean").status().ok();
    }

    // Build the project
    let build_target = if release { "release" } else { "all" };
    println!(
        "{} Building Makefile project (target: {})...",
        "→".cyan(),
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
            println!("{} Running executable: {}\n", "→".cyan(), exe.green());
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
        "⚠".yellow()
    );
    println!("  {} Looked for: {:?}", "→".dimmed(), possible_executables);
    Ok(())
}

fn execute_cmake_project(args: Vec<String>, release: bool, clean: bool) -> Result<()> {
    println!("{} Detected CMake project", "→".cyan());

    // Ensure .horus directory exists
    ensure_horus_directory()?;

    // Setup environment with .horus libraries
    setup_environment()?;

    let build_dir = PathBuf::from("build");

    // Clean if requested
    if clean && build_dir.exists() {
        println!("{} Cleaning CMake build directory...", "→".cyan());
        fs::remove_dir_all(&build_dir)?;
    }

    // Create build directory
    fs::create_dir_all(&build_dir)?;

    // Configure with CMake
    let build_type = if release { "Release" } else { "Debug" };
    println!("{} Configuring CMake ({} mode)...", "→".cyan(), build_type);

    let mut cmd = Command::new("cmake");
    cmd.arg("..")
        .arg(format!("-DCMAKE_BUILD_TYPE={}", build_type))
        .current_dir(&build_dir);

    let status = cmd.status()?;
    if !status.success() {
        bail!("CMake configuration failed");
    }

    // Build
    println!("{} Building CMake project...", "→".cyan());
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
            println!("{} Running executable: {}\n", "→".cyan(), exe.green());
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
        "⚠".yellow()
    );
    println!("  {} Looked for: {:?}", "→".dimmed(), possible_executables);
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
    println!(
        "{} Executing {} files concurrently:",
        "→".cyan(),
        file_paths.len()
    );

    for (i, file_path) in file_paths.iter().enumerate() {
        let language = detect_language(file_path)?;
        println!(
            "  {} {} ({})",
            format!("{}.", i + 1).dimmed(),
            file_path.display().to_string().green(),
            language.yellow()
        );
    }

    // For now, execute them sequentially
    // TODO: Implement concurrent execution with scheduler
    for file_path in file_paths {
        println!(
            "\n{} Running {}...",
            "→".cyan(),
            file_path.display().to_string().green()
        );
        execute_single_file(file_path, args.clone(), release, clean)?;
    }

    Ok(())
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

    if !horus_dir.exists() {
        println!("{} Creating .horus/ environment...", "→".cyan());
        fs::create_dir_all(&horus_dir)?;
        fs::create_dir_all(horus_dir.join("packages"))?;
        fs::create_dir_all(horus_dir.join("bin"))?;
        fs::create_dir_all(horus_dir.join("lib"))?;
        fs::create_dir_all(horus_dir.join("include"))?;
        fs::create_dir_all(horus_dir.join("cache"))?;

        // Create env.toml
        let env_content = r#"# Auto-generated environment
[environment]
created_at = "auto"
"#;
        fs::write(horus_dir.join("env.toml"), env_content)?;
    }

    // Setup C environment if needed
    setup_c_environment()?;

    Ok(())
}

fn scan_imports(file: &Path, language: &str) -> Result<HashSet<String>> {
    let content = fs::read_to_string(file)?;
    let mut dependencies = HashSet::new();

    // First, check if horus.yaml exists and use it
    if Path::new("horus.yaml").exists() {
        eprintln!("  {} Reading dependencies from horus.yaml", "→".cyan());
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

    // Filter out standard library and built-in packages
    dependencies.retain(|d| is_horus_package(d));

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
            // Extract package name from "- package@version" or "- package"
            let dep_str = trimmed[2..].trim();
            if dep_str.starts_with("#") {
                continue; // Skip comments
            }

            // Extract package name (before @)
            let package = if let Some(at_pos) = dep_str.find('@') {
                &dep_str[..at_pos]
            } else {
                dep_str
            };

            if package.starts_with("horus") || is_horus_package(package) {
                dependencies.insert(package.to_string());
            }
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
    // Always include HORUS packages
    if package.starts_with("horus") {
        return true;
    }

    // Check registry for import mapping
    // Use registry client to resolve package
    use crate::registry::RegistryClient;
    let client = RegistryClient::new();

    // Try to resolve for common languages (Python most common for robotics)
    if let Ok(Some(_)) = client.resolve_import(package, "python") {
        return true;
    }

    // Fallback for known packages
    matches!(
        package,
        "numpy" | "cv2" | "matplotlib" | "pandas" | "torch" | "tensorflow"
    )
}

fn resolve_dependencies(dependencies: HashSet<String>) -> Result<()> {
    // Check version compatibility first
    if let Err(e) = version::check_version_compatibility() {
        eprintln!("\n{}", "Hint:".cyan());
        eprintln!("  If you recently updated HORUS, run ./install.sh to update libraries.");
        return Err(e);
    }

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
            println!("  {} {} (already linked)", "✓".green(), package);
            continue;
        }

        // Check global cache
        let cached_versions = find_cached_versions(&global_cache, package)?;

        if let Some(cached) = cached_versions.first() {
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
            "⚠".yellow(),
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
            println!("\n{} Installing packages from registry...", "→".cyan());

            // Import registry client
            use crate::registry::RegistryClient;
            let client = RegistryClient::new();

            for package in &missing_packages {
                print!("  {} Installing {}... ", "→".cyan(), package.yellow());
                io::stdout().flush()?;

                match client.install(package, None) {
                    Ok(_) => {
                        println!("{}", "✓".green());
                        // client.install() already handles global/local detection and symlinks
                    }
                    Err(e) => {
                        println!("{}", "✗".red());
                        eprintln!("    {} Failed to install {}: {}", "✗".red(), package, e);
                        bail!("Failed to install required dependency: {}", package);
                    }
                }
            }

            println!("\n{} All dependencies installed successfully!", "✓".green());
        } else {
            // User declined
            println!(
                "\n{} Installation cancelled. Cannot proceed without dependencies.",
                "✗".red()
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
    eprintln!("{} Setting up Python environment...", "→".cyan());

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
            "→".cyan()
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
        eprintln!("{} Executing Python script directly...", "→".cyan());

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

fn create_venv_if_needed() -> Result<()> {
    let venv_path = PathBuf::from(".horus/venv");

    if venv_path.exists() {
        return Ok(());
    }

    println!("{} Python virtual environment not found", "ℹ".blue());
    println!("  {} Creating venv at .horus/venv/...", "→".cyan());

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

    println!("  {} Virtual environment created", "✓".green());

    // Upgrade pip
    let venv_pip = if cfg!(target_os = "windows") {
        venv_path.join("Scripts/pip.exe")
    } else {
        venv_path.join("bin/pip")
    };

    if venv_pip.exists() {
        println!("  {} Upgrading pip...", "→".cyan());
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
    // Check if venv exists and use its Python
    let venv_python = PathBuf::from(".horus/venv/bin/python");
    let venv_python3 = PathBuf::from(".horus/venv/bin/python3");
    let venv_python_win = PathBuf::from(".horus/venv/Scripts/python.exe");

    if venv_python.exists() {
        return Ok(venv_python.display().to_string());
    }
    if venv_python3.exists() {
        return Ok(venv_python3.display().to_string());
    }
    if venv_python_win.exists() {
        return Ok(venv_python_win.display().to_string());
    }

    // Try python3 first, then python (system-wide)
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

    // Check if venv should be used
    let venv_path = PathBuf::from(".horus/venv");
    if venv_path.exists() {
        // Use venv's site-packages
        let venv_site_packages = find_venv_site_packages(&venv_path)?;
        if let Ok(current_path) = env::var("PYTHONPATH") {
            let new_path = format!(
                "{}:{}:{}",
                horus_packages.display(),
                venv_site_packages.display(),
                current_path
            );
            env::set_var("PYTHONPATH", new_path);
        } else {
            let new_path = format!(
                "{}:{}",
                horus_packages.display(),
                venv_site_packages.display()
            );
            env::set_var("PYTHONPATH", new_path);
        }
    } else {
        // No venv, use system Python with .horus/packages
        if let Ok(current_path) = env::var("PYTHONPATH") {
            let new_path = format!("{}:{}", horus_packages.display(), current_path);
            env::set_var("PYTHONPATH", new_path);
        } else {
            env::set_var("PYTHONPATH", horus_packages.display().to_string());
        }
    }

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
            # Execute user code in global namespace
            exec(compile(open(r'{}').read(), r'{}', 'exec'))
        except SystemExit as e:
            # Preserve exit code from sys.exit()
            exit_code = e.code if e.code is not None else 0
        except Exception as e:
            print(f"❌ Node execution failed: {{e}}", file=sys.stderr)
            exit_code = 1

        sys.exit(exit_code)

# Initialize HORUS integration
if __name__ == "__main__":
    print("🚀 HORUS Python Node Starting...", file=sys.stderr)
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
        println!("  {} Cleaned .horus/cache/", "✓".green());
    }

    // Clean .horus/bin directory
    let bin_dir = PathBuf::from(".horus/bin");
    if bin_dir.exists() {
        for entry in fs::read_dir(&bin_dir)? {
            let entry = entry?;
            fs::remove_file(entry.path()).ok();
        }
        println!("  {} Cleaned .horus/bin/", "✓".green());
    }

    // Clean Rust target directory if exists
    let target_dir = PathBuf::from("target");
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
        println!("  {} Cleaned target/", "✓".green());
    }

    // Clean Python __pycache__ in current directory
    let pycache = PathBuf::from("__pycache__");
    if pycache.exists() {
        fs::remove_dir_all(&pycache)?;
        println!("  {} Cleaned __pycache__/", "✓".green());
    }

    Ok(())
}

fn execute_with_scheduler(
    file: PathBuf,
    language: String,
    args: Vec<String>,
    release: bool,
) -> Result<()> {
    match language.as_str() {
        "rust" => {
            // Use the same approach as build-only: rustc in .horus environment
            setup_rust_environment(&file)?;

            let file_stem = file
                .file_stem()
                .context("Invalid file name")?
                .to_string_lossy();
            let suffix = if release { "_release" } else { "_debug" };
            let binary_path = format!(".horus/cache/rust_{}{}", file_stem, suffix);

            // Build if not cached
            if !Path::new(&binary_path).exists() {
                eprintln!(
                    "{} Compiling Rust program ({} mode)...",
                    "→".cyan(),
                    if release { "release" } else { "debug" }
                );

                // Copy source to .horus/build
                let build_dir = PathBuf::from(".horus/build");
                fs::create_dir_all(&build_dir)?;
                let build_source = build_dir.join("main.rs");
                fs::copy(&file, &build_source)?;

                // Find horus library files
                let horus_pkg = PathBuf::from(".horus/packages/horus");
                if !horus_pkg.exists() {
                    bail!("HORUS package not found in .horus/packages/horus");
                }
                // Convert to absolute path so rustc can find it
                let horus_pkg = horus_pkg.canonicalize()?;

                eprintln!(
                    "  {} Searching for horus libraries in {:?}",
                    "→".cyan(),
                    horus_pkg
                );

                // Search for .rlib files in the horus package
                let mut lib_dirs = Vec::new();
                let mut extern_crates: std::collections::HashMap<String, PathBuf> =
                    std::collections::HashMap::new();

                // Check common locations for compiled libraries
                for subdir in &[
                    "lib",
                    "target/release",
                    "target/debug",
                    "target/release/deps",
                    "target/debug/deps",
                ] {
                    let lib_path = horus_pkg.join(subdir);
                    if lib_path.exists() {
                        eprintln!("  {} Checking {:?}", "✓".green(), lib_path);
                        if let Ok(entries) = fs::read_dir(&lib_path) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                    if name.ends_with(".rlib") {
                                        eprintln!("  {} Found {}", "→".cyan(), name);
                                        if name.starts_with("libhorus-") || name == "libhorus.rlib"
                                        {
                                            extern_crates.insert("horus".to_string(), path.clone());
                                            eprintln!("  {} Added horus extern", "✓".green());
                                        } else if name.starts_with("libhorus_core-")
                                            || name == "libhorus_core.rlib"
                                        {
                                            extern_crates.insert("horus_core".to_string(), path.clone());
                                            eprintln!("  {} Added horus_core extern", "✓".green());
                                        } else if name.starts_with("libhorus_macros-")
                                            || name == "libhorus_macros.rlib"
                                        {
                                            extern_crates.insert("horus_macros".to_string(), path.clone());
                                            eprintln!(
                                                "  {} Added horus_macros extern",
                                                "✓".green()
                                            );
                                        }
                                        lib_dirs.push(lib_path.clone());
                                    }
                                }
                            }
                        }
                    }
                }

                // Use rustc directly
                let mut cmd = Command::new("rustc");
                cmd.arg(&build_source);
                cmd.arg("-o").arg(&binary_path);
                if release {
                    cmd.arg("-O");
                }

                // Add library search paths
                cmd.arg("-L").arg(".horus/lib");
                for lib_dir in lib_dirs.iter().collect::<std::collections::HashSet<_>>() {
                    cmd.arg("-L")
                        .arg(format!("dependency={}", lib_dir.display()));
                }

                // Add extern declarations
                for (name, path) in &extern_crates {
                    cmd.arg("--extern")
                        .arg(format!("{}={}", name, path.display()));
                }

                eprintln!(
                    "  {} Compiling with {} extern crates, {} lib dirs",
                    "→".cyan(),
                    extern_crates.len(),
                    lib_dirs.len()
                );

                // Debug: show the actual extern declarations
                eprintln!("  {} Extern crates:", "→".cyan());
                for (name, path) in &extern_crates {
                    let exists = if path.exists() { "✓" } else { "✗" };
                    eprintln!("    {} --extern {}={}", exists, name, path.display());
                }

                let status = cmd.status()?;
                if !status.success() {
                    bail!("Rust compilation failed");
                }

                eprintln!("  {} Compiled to {}", "✓".green(), binary_path);
            } else {
                eprintln!("  {} Using cached binary", "✓".green());
            }

            // Execute the binary
            eprintln!("{} Executing Rust program...", "→".cyan());
            let mut cmd = Command::new(&binary_path);
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

fn create_minimal_cargo_toml(file: &Path) -> Result<()> {
    let project_name = env::current_dir()?
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("horus_project")
        .to_string();

    let file_name = file.file_stem().and_then(|n| n.to_str()).unwrap_or("main");

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
    println!("  {} Created Cargo.toml for {}", "✓".green(), project_name);

    Ok(())
}

fn setup_rust_environment(_source: &Path) -> Result<()> {
    // Ensure .horus/build exists for compilation
    fs::create_dir_all(".horus/build")?;

    // Could add Rust-specific setup here if needed
    // For now, rustc will handle everything

    Ok(())
}

fn setup_c_environment() -> Result<()> {
    let horus_dir = PathBuf::from(".horus");
    let include_dir = horus_dir.join("include");
    let lib_dir = horus_dir.join("lib");

    // Copy horus.h header file to .horus/include/
    let header_path = include_dir.join("horus.h");
    if !header_path.exists() {
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
        println!("  {} Installed horus.h", "✓".green());
    }

    // Check if horus_c library exists in .horus/lib/
    let lib_name = if cfg!(target_os = "windows") {
        "horus_c.dll"
    } else if cfg!(target_os = "macos") {
        "libhorus_c.dylib"
    } else {
        "libhorus_c.so"
    };

    let lib_path = lib_dir.join(lib_name);
    if !lib_path.exists() {
        // Try to find and copy horus_c library
        if let Ok(horus_c_lib) = find_horus_c_library() {
            fs::copy(&horus_c_lib, &lib_path)?;
            println!("  {} Installed {}", "✓".green(), lib_name);
        } else {
            println!(
                "  {} {} not found - will attempt to build",
                "⚠".yellow(),
                lib_name
            );
        }
    }

    Ok(())
}

fn find_horus_c_library() -> Result<PathBuf> {
    // Look for horus_c library in common locations
    let possible_paths = [
        "horus_c/target/release/libhorus_c.so",
        "horus_c/target/debug/libhorus_c.so",
        "../horus_c/target/release/libhorus_c.so",
        "../horus_c/target/debug/libhorus_c.so",
        "target/release/libhorus_c.so",
        "target/debug/libhorus_c.so",
    ];

    for path in &possible_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    bail!("horus_c library not found")
}

fn execute_c_node(file: PathBuf, args: Vec<String>, release: bool) -> Result<()> {
    eprintln!("{} Setting up C environment...", "→".cyan());

    // Detect C compiler
    let compiler = detect_c_compiler()?;
    eprintln!("  {} Using {} compiler", "✓".green(), compiler);

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
            "→".cyan(),
            if release { "release" } else { "debug" }
        );

        compile_c_file(&file, &binary_path, &compiler, release)?;
        eprintln!("  {} Compiled to {}", "✓".green(), binary_path.display());
    } else {
        eprintln!("  {} Using cached binary", "✓".green());
    }

    // Execute the binary
    eprintln!("{} Executing C program...", "→".cyan());
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
    let mut cmd = Command::new(compiler);

    // Basic arguments
    cmd.arg(source);
    cmd.arg("-o");
    cmd.arg(output);

    // Check if source uses HORUS headers
    let uses_horus = if let Ok(content) = fs::read_to_string(source) {
        content.contains("#include <horus/") || content.contains("#include \"horus/")
    } else {
        false
    };

    if uses_horus {
        // Include path for horus headers
        cmd.arg("-I.horus/include");

        // Library path
        cmd.arg("-L.horus/lib");

        // Link with horus_c only if library exists
        let horus_c_lib = PathBuf::from(".horus/lib/libhorus_c.so");
        if horus_c_lib.exists() {
            cmd.arg("-lhorus_c");
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
        eprintln!("{} Compilation failed:", "❌".red());
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
