// Dependency resolution with version conflict detection
// Solves dependency hell by finding compatible versions

use anyhow::{anyhow, bail, Result};
use colored::*;
use semver::{Version, VersionReq};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

pub type PackageName = String;

#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    pub name: String,
    pub version: Version,
}

#[derive(Debug, Clone)]
pub struct DependencySpec {
    pub name: String,
    pub requirement: VersionReq, // Semver requirement like "^1.2.3"
}

impl fmt::Display for DependencySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.name, self.requirement)
    }
}

impl DependencySpec {
    pub fn parse(spec: &str) -> Result<Self> {
        // Parse "name@constraint" or just "name"
        if let Some(pos) = spec.find('@') {
            let name = spec[..pos].to_string();
            let constraint = &spec[pos + 1..];

            // Parse version requirement
            let requirement = if constraint == "latest" || constraint == "*" {
                VersionReq::STAR
            } else {
                VersionReq::parse(constraint)
                    .map_err(|e| anyhow!("Invalid version constraint '{}': {}", constraint, e))?
            };

            Ok(Self { name, requirement })
        } else {
            Ok(Self {
                name: spec.to_string(),
                requirement: VersionReq::STAR, // Any version
            })
        }
    }

    pub fn matches(&self, version: &Version) -> bool {
        self.requirement.matches(version)
    }
}

/// Package metadata provider
pub trait PackageProvider {
    /// Get all available versions for a package
    fn get_available_versions(&self, package: &str) -> Result<Vec<Version>>;

    /// Get dependencies for a specific package version
    fn get_dependencies(&self, package: &str, version: &Version) -> Result<Vec<DependencySpec>>;
}

/// Dependency resolver with conflict detection
pub struct DependencyResolver<'a> {
    provider: &'a dyn PackageProvider,
    resolved: HashMap<PackageName, Version>,
    requirements: HashMap<PackageName, Vec<VersionReq>>,
}

impl<'a> DependencyResolver<'a> {
    pub fn new(provider: &'a dyn PackageProvider) -> Self {
        Self {
            provider,
            resolved: HashMap::new(),
            requirements: HashMap::new(),
        }
    }

    /// Resolve dependencies starting from root requirements
    pub fn resolve(&mut self, root_deps: Vec<DependencySpec>) -> Result<Vec<ResolvedDependency>> {
        println!("{} Resolving dependencies...", "→".cyan());

        // Collect all requirements via BFS
        let mut queue: VecDeque<(String, DependencySpec)> = VecDeque::new();

        for dep in root_deps {
            println!(
                "  {} Root dependency: {} {}",
                "→".cyan(),
                dep.name,
                dep.requirement
            );
            queue.push_back(("root".to_string(), dep));
        }

        // BFS to collect all requirements
        let mut visited = HashSet::new();

        while let Some((parent, dep)) = queue.pop_front() {
            let key = format!("{}@{}", dep.name, dep.requirement);
            if visited.contains(&key) {
                continue;
            }
            visited.insert(key);

            // Add requirement
            self.requirements
                .entry(dep.name.clone())
                .or_default()
                .push(dep.requirement.clone());

            // Temporarily resolve to latest matching version to fetch its dependencies
            if let Ok(versions) = self.provider.get_available_versions(&dep.name) {
                if let Some(version) = self.find_best_version(&dep.name, &versions) {
                    // Get transitive dependencies
                    if let Ok(transitive_deps) = self.provider.get_dependencies(&dep.name, &version)
                    {
                        for trans_dep in transitive_deps {
                            queue.push_back((dep.name.clone(), trans_dep));
                        }
                    }
                }
            }
        }

        // Now resolve all packages
        println!(
            "  {} Found {} unique packages",
            "→".cyan(),
            self.requirements.len()
        );

        // Clone requirements to avoid borrow checker issues
        let requirements_snapshot: Vec<(String, Vec<_>)> = self
            .requirements
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (package, reqs) in requirements_snapshot {
            self.resolve_package(&package, &reqs)?;
        }

        // Convert to result format
        let mut result: Vec<ResolvedDependency> = self
            .resolved
            .iter()
            .map(|(name, version)| ResolvedDependency {
                name: name.clone(),
                version: version.clone(),
            })
            .collect();

        result.sort_by(|a, b| a.name.cmp(&b.name));

        println!(
            "  {} Successfully resolved {} packages!",
            "✓".green(),
            result.len()
        );
        for dep in &result {
            println!("    • {} v{}", dep.name.cyan(), dep.version);
        }

        Ok(result)
    }

    fn resolve_package(&mut self, package: &str, requirements: &[VersionReq]) -> Result<Version> {
        // Check if already resolved
        if let Some(version) = self.resolved.get(package) {
            // Verify it satisfies all requirements
            for req in requirements {
                if !req.matches(version) {
                    bail!(
                        "Version conflict for {}: resolved v{} doesn't satisfy requirement {}",
                        package,
                        version,
                        req
                    );
                }
            }
            return Ok(version.clone());
        }

        // Get available versions
        let versions = self
            .provider
            .get_available_versions(package)
            .map_err(|e| anyhow!("Cannot fetch versions for {}: {}", package, e))?;

        if versions.is_empty() {
            bail!("No versions available for package: {}", package);
        }

        // Find compatible version
        let version = self.find_best_version_with_requirements(package, &versions, requirements)
            .ok_or_else(|| {
                let req_strs: Vec<String> = requirements.iter().map(|r| r.to_string()).collect();
                anyhow!("Cannot find compatible version for {}: requirements are {:?}, available versions: {:?}",
                    package, req_strs, versions)
            })?;

        println!(
            "    {} {} v{} (satisfies {} constraints)",
            "✓".green(),
            package,
            version,
            requirements.len()
        );

        self.resolved.insert(package.to_string(), version.clone());
        Ok(version)
    }

    fn find_best_version(&self, package: &str, versions: &[Version]) -> Option<Version> {
        let requirements = self.requirements.get(package)?;
        self.find_best_version_with_requirements(package, versions, requirements)
    }

    fn find_best_version_with_requirements(
        &self,
        _package: &str,
        versions: &[Version],
        requirements: &[VersionReq],
    ) -> Option<Version> {
        // Find the highest version that satisfies all requirements
        let mut candidates: Vec<Version> = versions
            .iter()
            .filter(|v| requirements.iter().all(|req| req.matches(v)))
            .cloned()
            .collect();

        candidates.sort();
        candidates.pop() // Return highest version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider {
        versions: HashMap<String, Vec<Version>>,
        dependencies: HashMap<(String, Version), Vec<DependencySpec>>,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                versions: HashMap::new(),
                dependencies: HashMap::new(),
            }
        }

        fn add_versions(&mut self, package: &str, versions: Vec<Version>) {
            self.versions.insert(package.to_string(), versions);
        }

        fn add_deps(&mut self, package: &str, version: Version, deps: Vec<DependencySpec>) {
            self.dependencies
                .insert((package.to_string(), version), deps);
        }
    }

    impl PackageProvider for MockProvider {
        fn get_available_versions(&self, package: &str) -> Result<Vec<Version>> {
            self.versions
                .get(package)
                .cloned()
                .ok_or_else(|| anyhow!("Package not found: {}", package))
        }

        fn get_dependencies(
            &self,
            package: &str,
            version: &Version,
        ) -> Result<Vec<DependencySpec>> {
            Ok(self
                .dependencies
                .get(&(package.to_string(), version.clone()))
                .cloned()
                .unwrap_or_default())
        }
    }

    #[test]
    fn test_dependency_spec_parse() {
        let spec = DependencySpec::parse("horus_core@^1.2.3").unwrap();
        assert_eq!(spec.name, "horus_core");
        assert!(spec.matches(&Version::new(1, 2, 3)));
        assert!(spec.matches(&Version::new(1, 9, 9)));
        assert!(!spec.matches(&Version::new(2, 0, 0)));

        let spec = DependencySpec::parse("horus_library").unwrap();
        assert_eq!(spec.name, "horus_library");
        assert!(spec.matches(&Version::new(0, 1, 0)));
    }

    #[test]
    fn test_simple_resolution() {
        let mut provider = MockProvider::new();

        provider.add_versions("pkg_a", vec![Version::new(1, 0, 0), Version::new(1, 1, 0)]);
        provider.add_versions("pkg_b", vec![Version::new(2, 0, 0), Version::new(2, 1, 0)]);

        provider.add_deps(
            "pkg_a",
            Version::new(1, 1, 0),
            vec![DependencySpec {
                name: "pkg_b".to_string(),
                requirement: VersionReq::parse("^2.0.0").unwrap(),
            }],
        );

        let mut resolver = DependencyResolver::new(&provider);
        let root_deps = vec![DependencySpec {
            name: "pkg_a".to_string(),
            requirement: VersionReq::parse("^1.0.0").unwrap(),
        }];

        let resolved = resolver.resolve(root_deps).unwrap();
        assert_eq!(resolved.len(), 2);

        let pkg_a = resolved.iter().find(|r| r.name == "pkg_a").unwrap();
        assert_eq!(pkg_a.version, Version::new(1, 1, 0));

        let pkg_b = resolved.iter().find(|r| r.name == "pkg_b").unwrap();
        assert_eq!(pkg_b.version, Version::new(2, 1, 0));
    }
}
