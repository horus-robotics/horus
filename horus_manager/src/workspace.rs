// Workspace tracking and detection for HORUS projects

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use colored::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceRegistry {
    pub workspaces: Vec<Workspace>,
}

impl WorkspaceRegistry {
    pub fn load() -> Result<Self> {
        let registry_path = Self::registry_path()?;

        if !registry_path.exists() {
            return Ok(Self {
                workspaces: Vec::new(),
            });
        }

        let content = fs::read_to_string(&registry_path)?;
        let registry: Self =
            serde_json::from_str(&content).context("Failed to parse workspace registry")?;

        Ok(registry)
    }

    pub fn save(&self) -> Result<()> {
        let registry_path = Self::registry_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&registry_path, content)?;
        Ok(())
    }

    fn registry_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let horus_dir = home.join(".horus");
        fs::create_dir_all(&horus_dir)?;
        Ok(horus_dir.join("workspaces.json"))
    }

    pub fn add(&mut self, name: String, path: PathBuf) -> Result<()> {
        // Remove existing entry with same path
        self.workspaces.retain(|w| w.path != path);

        let workspace = Workspace {
            name,
            path,
            created_at: Utc::now(),
            last_used: Utc::now(),
        };

        self.workspaces.push(workspace);
        self.save()?;
        Ok(())
    }

    pub fn update_last_used(&mut self, path: &Path) -> Result<()> {
        if let Some(ws) = self.workspaces.iter_mut().find(|w| w.path == path) {
            ws.last_used = Utc::now();
            self.save()?;
        }
        Ok(())
    }

    pub fn find_by_name(&self, name: &str) -> Option<&Workspace> {
        self.workspaces.iter().find(|w| w.name == name)
    }

    pub fn find_by_path(&self, path: &Path) -> Option<&Workspace> {
        self.workspaces.iter().find(|w| w.path == path)
    }
}

/// Find workspace root by searching upward for markers
pub fn find_workspace_root() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;

    // Search upwards for workspace markers (limit to 10 levels)
    for _ in 0..10 {
        // Priority 1: .horus/ directory (explicit HORUS workspace)
        if current.join(".horus").exists() {
            return Some(current);
        }

        // Priority 2: horus.yaml (workspace config)
        if current.join("horus.yaml").exists() {
            return Some(current);
        }

        // Go up one level
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    None
}

/// Detect current workspace or prompt user interactively
pub fn detect_or_select_workspace(allow_global: bool) -> Result<InstallTarget> {
    // Try to find current workspace
    if let Some(root) = find_workspace_root() {
        // Update last used time
        let mut registry = WorkspaceRegistry::load()?;
        registry.update_last_used(&root).ok();

        let name = root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        println!("  {} Detected workspace: {}", "✓".green(), name.yellow());
        return Ok(InstallTarget::Local(root));
    }

    // Not in a workspace - show interactive selector
    let registry = WorkspaceRegistry::load()?;

    if registry.workspaces.is_empty() && !allow_global {
        bail!("No HORUS workspaces found. Run 'horus new <name>' or 'horus init' first.");
    }

    interactive_workspace_selector(&registry, allow_global)
}

#[derive(Debug)]
pub enum InstallTarget {
    Local(PathBuf),
    Global,
}

fn interactive_workspace_selector(
    registry: &WorkspaceRegistry,
    allow_global: bool,
) -> Result<InstallTarget> {
    println!("\n{} Not in a HORUS workspace", "⚠".yellow());
    println!("Where should we install the package?\n");

    let mut options = Vec::new();
    let mut idx = 1;

    // List known workspaces
    for ws in &registry.workspaces {
        println!(
            "  [{}] {}  ({})",
            idx.to_string().cyan(),
            ws.name.yellow(),
            ws.path.display().to_string().dimmed()
        );
        options.push(InstallTarget::Local(ws.path.clone()));
        idx += 1;
    }

    // Global option
    if allow_global {
        println!(
            "  [{}] 🌐 Global  (~/.horus/cache) - shared across projects",
            idx.to_string().cyan()
        );
        options.push(InstallTarget::Global);
        idx += 1;
    }

    // Create new workspace option
    let current = std::env::current_dir()?;
    let new_workspace_idx = idx;
    println!(
        "  [{}] ➕ Create new workspace here  ({})",
        idx.to_string().cyan(),
        current.display().to_string().dimmed()
    );
    idx += 1;

    // Cancel option
    println!("  [{}] ❌ Cancel\n", idx.to_string().cyan());

    // Get user selection
    print!("→ Select target [1-{}]: ", idx);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let selection: usize = input.trim().parse().context("Invalid selection")?;

    if selection < 1 || selection > idx {
        bail!("Invalid selection");
    }

    if selection == idx {
        bail!("Installation cancelled");
    }

    // Handle "create new workspace" option
    if selection == new_workspace_idx {
        println!(
            "\n{} Creating new workspace in current directory...",
            "→".cyan()
        );

        // Ask for workspace name
        let default_name = current
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("workspace");

        print!("Workspace name [{}]: ", default_name.yellow());
        io::stdout().flush()?;

        let mut name_input = String::new();
        io::stdin().read_line(&mut name_input)?;
        let workspace_name = name_input.trim();
        let workspace_name = if workspace_name.is_empty() {
            default_name.to_string()
        } else {
            workspace_name.to_string()
        };

        // Create .horus/ directory
        let horus_dir = current.join(".horus");
        fs::create_dir_all(&horus_dir).context("Failed to create .horus directory")?;

        // Create minimal horus.yaml
        let horus_yaml = current.join("horus.yaml");
        let yaml_content = format!("name: {}\nversion: 0.1.0\n", workspace_name);
        fs::write(&horus_yaml, yaml_content).context("Failed to create horus.yaml")?;

        // Register in workspace registry
        let mut registry = WorkspaceRegistry::load()?;
        registry.add(workspace_name.clone(), current.clone())?;

        println!("✅ Created workspace: {}", workspace_name.yellow());
        println!("   Location: {}", current.display());

        return Ok(InstallTarget::Local(current));
    }

    Ok(options[selection - 1].clone())
}

impl Clone for InstallTarget {
    fn clone(&self) -> Self {
        match self {
            InstallTarget::Local(path) => InstallTarget::Local(path.clone()),
            InstallTarget::Global => InstallTarget::Global,
        }
    }
}

/// Register current directory as a workspace
pub fn register_current_workspace(name: Option<String>) -> Result<()> {
    let current = std::env::current_dir()?;

    let workspace_name = if let Some(n) = name {
        n
    } else {
        current
            .file_name()
            .and_then(|s| s.to_str())
            .context("Invalid directory name")?
            .to_string()
    };

    // Create .horus/ directory
    let horus_dir = current.join(".horus");
    if !horus_dir.exists() {
        fs::create_dir_all(&horus_dir)?;
        println!("  {} Created .horus/ directory", "✓".green());
    }

    // Create minimal horus.yaml if it doesn't exist
    let horus_yaml = current.join("horus.yaml");
    if !horus_yaml.exists() {
        let yaml_content = format!("name: {}\nversion: 0.1.0\n", workspace_name);
        fs::write(&horus_yaml, yaml_content)?;
        println!("  {} Created horus.yaml", "✓".green());
    }

    // Register in workspace registry
    let mut registry = WorkspaceRegistry::load()?;
    registry.add(workspace_name.clone(), current.clone())?;

    println!(
        "✅ Initialized HORUS workspace: {}",
        workspace_name.yellow()
    );
    println!("   Location: {}", current.display());
    Ok(())
}
