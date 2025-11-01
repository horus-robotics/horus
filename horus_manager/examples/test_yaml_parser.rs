use horus_manager::dependency_resolver::{DependencySpec, DependencySource};
use std::fs;

fn main() -> anyhow::Result<()> {
    let yaml_path = std::env::args().nth(1).unwrap_or_else(|| "horus.yaml".to_string());
    
    println!("Testing YAML parser on: {}", yaml_path);
    println!();
    
    let content = fs::read_to_string(&yaml_path)?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;
    
    if let Some(deps_value) = yaml.get("dependencies") {
        println!("Found dependencies section:");
        println!();
        
        match deps_value {
            serde_yaml::Value::Mapping(map) => {
                println!("  Format: Structured (Map)");
                println!();
                for (key, value) in map {
                    if let serde_yaml::Value::String(name) = key {
                        let spec = DependencySpec::from_yaml_value(name.clone(), value)?;
                        
                        println!("  Package: {}", spec.name);
                        println!("    Version requirement: {}", spec.requirement);
                        match &spec.source {
                            DependencySource::Registry => println!("    Source: Registry"),
                            DependencySource::Path(p) => println!("    Source: Path ({})", p.display()),
                            DependencySource::Git { url, rev, branch, tag } => {
                                println!("    Source: Git");
                                println!("      URL: {}", url);
                                if let Some(r) = rev { println!("      Rev: {}", r); }
                                if let Some(b) = branch { println!("      Branch: {}", b); }
                                if let Some(t) = tag { println!("      Tag: {}", t); }
                            }
                        }
                        println!();
                    }
                }
            }
            serde_yaml::Value::Sequence(list) => {
                println!("  Format: List (old format)");
                println!();
                for item in list {
                    if let serde_yaml::Value::String(dep_str) = item {
                        println!("  Dependency: {}", dep_str);
                    }
                }
            }
            _ => println!("  Unknown format"),
        }
    } else {
        println!("No dependencies section found");
    }
    
    Ok(())
}
