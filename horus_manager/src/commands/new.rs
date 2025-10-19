use crate::version;
use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn create_new_project(
    name: String,
    path: Option<PathBuf>,
    language: String,
    use_macro: bool,
) -> Result<()> {
    // Check version compatibility before creating project
    version::check_and_prompt_update()?;

    println!(
        "{} Creating new HORUS project '{}'",
        "✨".cyan(),
        name.green().bold()
    );

    // Determine project path
    let project_path = if let Some(p) = path {
        p.join(&name)
    } else {
        PathBuf::from(&name)
    };

    // Track if we're in interactive mode
    let is_interactive = language.is_empty();

    // Get language - use flag or prompt
    let language = if is_interactive {
        prompt_language()?
    } else {
        language
    };

    // Ask about macros if Rust was selected interactively (and not already set via flag)
    let use_macro = if language == "rust" && is_interactive {
        prompt_use_macro()?
    } else {
        use_macro
    };

    let description = prompt_description()?;
    let author = get_author()?;

    // Create project directory
    fs::create_dir_all(&project_path).context("Failed to create project directory")?;

    // Create .horus/ directory structure
    create_horus_directory(&project_path)?;

    // Generate horus.yaml with dependencies
    create_horus_yaml(
        &project_path,
        &name,
        &description,
        &author,
        &language,
        use_macro,
    )?;

    // Generate main file based on language
    match language.as_str() {
        "rust" => {
            create_main_rs(&project_path, use_macro)?;
        }
        "python" => create_main_py(&project_path)?,
        "c" => create_main_c(&project_path)?,
        _ => unreachable!(),
    }

    println!("\n{} Project created successfully!", "✅".green().bold());
    println!("\nTo get started:");
    println!("  {} {}", "cd".cyan(), name);
    println!("  {} (auto-installs dependencies)", "horus run".cyan());

    Ok(())
}

fn prompt_language() -> Result<String> {
    println!("\n{} Select language:", "?".yellow().bold());
    println!("  {} Python", "1.".cyan());
    println!("  {} Rust", "2.".cyan());
    println!("  {} C", "3.".cyan());

    print!("{} [1-3] (default: 2): ", ">".cyan().bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    let choice = if input.is_empty() { "2" } else { input };

    let language = match choice {
        "1" => "python",
        "2" => "rust",
        "3" => "c",
        _ => {
            println!("{} Invalid choice, defaulting to Rust", "⚠".yellow());
            "rust"
        }
    };

    Ok(language.to_string())
}

fn prompt_use_macro() -> Result<bool> {
    print!(
        "\n{} Use HORUS macros for simpler syntax? [y/N]: ",
        "?".yellow().bold()
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    Ok(input == "y" || input == "yes")
}

fn prompt_description() -> Result<String> {
    print!("\n{} Project description: ", "?".yellow().bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let description = input.trim();

    Ok(if description.is_empty() {
        "A HORUS robotics project".to_string()
    } else {
        description.to_string()
    })
}

fn get_author() -> Result<String> {
    // Try to get from git config
    if let Ok(output) = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
    {
        if output.status.success() {
            if let Ok(name) = String::from_utf8(output.stdout) {
                let name = name.trim();
                if !name.is_empty() {
                    return Ok(name.to_string());
                }
            }
        }
    }

    // Fallback to username
    Ok(std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()))
}

fn create_horus_directory(project_path: &Path) -> Result<()> {
    let horus_dir = project_path.join(".horus");

    // Create main .horus directory
    fs::create_dir_all(&horus_dir)?;

    // Create subdirectories
    fs::create_dir_all(horus_dir.join("bin"))?;
    fs::create_dir_all(horus_dir.join("lib"))?;
    fs::create_dir_all(horus_dir.join("include"))?;

    // Create env.toml with initial content
    let env_content = r#"# Auto-generated environment snapshot
# This file is managed by HORUS - do not edit manually

[environment]
created = "auto-generated on first run"
"#;

    fs::write(horus_dir.join("env.toml"), env_content)?;

    Ok(())
}

fn create_horus_yaml(
    project_path: &Path,
    name: &str,
    description: &str,
    author: &str,
    language: &str,
    use_macro: bool,
) -> Result<()> {
    // Determine dependencies based on language
    let dependencies = match language {
        "rust" => {
            if use_macro {
                r#"dependencies:
  - horus@0.1.0
  - horus_macros@0.1.0
  # - horus_library@0.1.0  # Uncomment for standard robotics messages
"#
            } else {
                r#"dependencies:
  - horus@0.1.0
  # - horus_library@0.1.0  # Uncomment for standard robotics messages
"#
            }
        }
        "python" => {
            r#"dependencies:
  - horus_py@0.1.0
  # Add Python packages as needed
"#
        }
        "c" => {
            r#"dependencies:
  - horus_c@0.1.0
  # Add C libraries as needed
"#
        }
        _ => "",
    };

    let content = format!(
        r#"name: {}
version: 0.1.0
description: {}
author: {}
license: MIT
language: {}
horus_id: null  # Auto-generated on first dependency resolution

{}
"#,
        name, description, author, language, dependencies
    );

    fs::write(project_path.join("horus.yaml"), content)?;

    Ok(())
}

fn create_main_rs(project_path: &Path, use_macro: bool) -> Result<()> {
    let content = if use_macro {
        // Macro version - clean and concise
        r#"// Mobile robot controller

use horus::prelude::*;
use horus_macros::node;

node! {
    Controller {
        pub {
            cmd_vel: CmdVel -> "motors/cmd_vel",
        }

        tick(ctx) {
            // Your control logic here
            // ctx provides node state, timing info, and monitoring data
            let msg = CmdVel::new(1.0, 0.0);
            self.cmd_vel.send(msg, ctx).ok();
        }
    }
}

fn main() -> Result<()> {
    let mut scheduler = Scheduler::new();

    // Register the controller node with priority 0 (highest)
    scheduler.register(
        Box::new(Controller::new()),
        0,     // priority (0 = highest)
        Some(true)    // logging config
    );

    // Run the scheduler
    scheduler.tick_all()
}
"#
    } else {
        // Non-macro version
        r#"// Mobile robot controller

use horus::prelude::*;

struct Controller {
    cmd_vel: Hub<CmdVel>,
}

impl Controller {
    fn new() -> Result<Self> {
        Ok(Self {
            cmd_vel: Hub::new("motors/cmd_vel")?,
        })
    }
}

impl Node for Controller {
    fn name(&self) -> &'static str {
        "controller"
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Your control logic here
        // ctx provides node state, timing info, and monitoring data
        let msg = CmdVel::new(1.0, 0.0);
        self.cmd_vel.send(msg, ctx).ok();
    }
}

fn main() -> Result<()> {
    let mut scheduler = Scheduler::new();

    // Register the controller node with priority 10
    scheduler.register(
        Box::new(Controller::new()?),
        0,     // priority (0 = highest)
        Some(true)    // logging config
    );

    // Run the scheduler
    scheduler.tick_all()
}
"#
    };

    fs::write(project_path.join("main.rs"), content)?;
    Ok(())
}

fn create_main_py(project_path: &Path) -> Result<()> {
    let content = r#"# Mobile robot controller

import horus

def controller(node):
    """Main control logic - called repeatedly at the specified rate."""
    # Your control logic here
    # Check for incoming messages
    if node.has_msg("sensors/data"):
        sensor_data = node.get("sensors/data")
        # Process sensor data...

    # Send control commands
    cmd_vel = {"linear": 1.0, "angular": 0.0}
    node.send("motors/cmd_vel", cmd_vel)

# Create the node
node = horus.Node(
    name="controller",
    pubs="motors/cmd_vel",    # Topics to publish to
    subs="sensors/data",      # Topics to subscribe from
    tick=controller,          # Function to call repeatedly
    rate=30                   # Hz (30 times per second)
)

if __name__ == "__main__":
    # Run the node
    horus.run(node)
"#;
    fs::write(project_path.join("main.py"), content)?;
    Ok(())
}

fn create_main_c(project_path: &Path) -> Result<()> {
    // Placeholder - blank for now
    let content = "";
    fs::write(project_path.join("main.c"), content)?;
    Ok(())
}
