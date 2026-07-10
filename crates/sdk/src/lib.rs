//! Cross-compilation SDK and package manager for sysroots and dependencies.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use crossbuild_core::{
    model::TargetTriple,
    config::CrossBuildConfig,
};

/// SDK manager for acquiring and managing cross-compilation dependencies.
pub struct SdkManager {
    config: CrossBuildConfig,
    cache_dir: PathBuf,
    installed_packages: BTreeMap<String, InstalledPackage>,
}

/// Installed package metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub target: String,
    pub install_path: PathBuf,
    pub dependencies: Vec<String>,
    pub files: Vec<PathBuf>,
}

/// Package metadata from registry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub dependencies: Vec<Dependency>,
    pub targets: Vec<String>,
    pub files: Vec<PackageFile>,
}

/// Dependency specification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version_req: String,
    pub targets: Vec<String>,
}

/// Package file entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackageFile {
    pub path: PathBuf,
    pub hash: String,
    pub size: u64,
}

impl SdkManager {
    /// Creates a new SDK manager.
    pub fn new(config: CrossBuildConfig) -> Result<Self> {
        let cache_dir = config
            .target_dir
            .join("crossbuild-cache")
            .join("sdk");

        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            config,
            cache_dir,
            installed_packages: BTreeMap::new(),
        })
    }

    /// Installs a package for the given target.
    pub fn install_package(
        &mut self,
        name: &str,
        version: &str,
        target: &crossbuild_core::model::TargetTriple,
    ) -> Result<InstalledPackage> {
        // Check if already installed
        let key = format!("{}@{}::{}", name, version, target.triple);
        if let Some(pkg) = self.installed_packages.get(&key) {
            return Ok(pkg.clone());
        }

        // Download package metadata
        let metadata = self.fetch_metadata(name, version, target)?;

        // Download and verify files
        let mut files = Vec::new();
        for file in &metadata.files {
            let path = self.cache_dir.join("packages").join(&file.path);
            std::fs::create_dir_all(path.parent().unwrap())?;
            self.download_file(file.path.to_str().unwrap_or_default(), &path, &file.hash)?;
            files.push(path);
        }

        // Extract if needed (for archives)
        let install_path = self.extract_package(&metadata, &files)?;

        let installed = InstalledPackage {
            name: metadata.name.clone(),
            version: metadata.version.clone(),
            target: target.triple.clone(),
            install_path: install_path.clone(),
            dependencies: metadata.dependencies.iter().map(|d| d.name.clone()).collect(),
            files: metadata.files.iter().map(|f| f.path.clone()).collect(),
        };

        self.installed_packages.insert(key, installed.clone());

        // Save to cache
        self.save_cache()?;

        Ok(installed)
    }

    /// Fetches package metadata from registry.
    fn fetch_metadata(
        &self,
        name: &str,
        version: &str,
        target: &crossbuild_core::model::TargetTriple,
    ) -> Result<PackageMetadata> {
        // In a real implementation, this would query a package registry
        // For now, return mock metadata
        Ok(PackageMetadata {
            name: name.to_string(),
            version: version.to_string(),
            description: format!("{} for {}", name, target.triple),
            dependencies: Vec::new(),
            targets: vec![target.triple.clone()],
            files: Vec::new(),
        })
    }

    /// Downloads a file with checksum verification.
    fn download_file(&self, url: &str, dest: &Path, expected_hash: &str) -> Result<()> {
        let response = reqwest::blocking::get(url)?;
        let bytes = response.bytes()?;

        // Verify hash
        use sha2::{Digest, Sha256};
        let mut hasher = sha2::Sha256::new();
        hasher.update(&bytes);
        let hash = hex::encode(hasher.finalize());

        if hash != expected_hash {
            anyhow::bail!(
                "Checksum mismatch for {}: expected {}, got {}",
                url, expected_hash, hash
            );
        }

        std::fs::write(dest, bytes)?;
        Ok(())
    }

    /// Extracts a package archive.
    fn extract_package(&self, metadata: &PackageMetadata, files: &[PathBuf]) -> Result<PathBuf> {
        let install_dir = self.cache_dir.join("installed").join(&metadata.name).join(&metadata.version);
        std::fs::create_dir_all(&install_dir)?;

        // In a real implementation, extract archives
        // For now, just return the install directory
        Ok(install_dir)
    }

    /// Saves the installed packages cache.
    fn save_cache(&self) -> Result<()> {
        let cache_file = self.cache_dir.join("installed.json");
        let content = serde_json::to_string_pretty(&self.installed_packages)?;
        std::fs::write(cache_file, content)?;
        Ok(())
    }

    /// Gets an installed package.
    pub fn get_package(&self, name: &str, version: &str, target: &crossbuild_core::model::TargetTriple) -> Option<&InstalledPackage> {
        let key = format!("{}@{}::{}", name, version, target.triple);
        self.installed_packages.get(&key)
    }

    /// Lists all installed packages for a target.
    pub fn list_packages(&self, target: &crossbuild_core::model::TargetTriple) -> Vec<&InstalledPackage> {
        self.installed_packages
            .values()
            .filter(|p| p.target == target.triple)
            .collect()
    }

    /// Removes a package.
    pub fn remove_package(&mut self, name: &str, version: &str, target: &crossbuild_core::model::TargetTriple) -> Result<()> {
        let key = format!("{}@{}::{}", name, version, target.triple);
        if self.installed_packages.remove(&key).is_some() {
            self.save_cache()?;
        }
        Ok(())
    }
}

/// Registry for managing package sources.
pub struct PackageRegistry {
    sources: Vec<PackageSource>,
}

/// Package source configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackageSource {
    pub name: String,
    pub url: String,
    pub priority: i32,
}

impl PackageRegistry {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    pub fn add_source(&mut self, source: PackageSource) {
        self.sources.push(source);
        self.sources.sort_by_key(|s| -s.priority);
    }

    pub fn find_package(&self, name: &str, version: &str, target: &crossbuild_core::model::TargetTriple) -> Option<PackageMetadata> {
        for source in &self.sources {
            // In real implementation, query the source
            // For now return None
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbuild_core::config::CrossBuildConfig;
    use crossbuild_core::model::TargetTriple;
    use tempfile::tempdir;

    #[test]
    fn sdk_manager_creation() {
        let config = CrossBuildConfig::default();
        let manager = SdkManager::new(config).unwrap();
        assert!(manager.installed_packages.is_empty());
    }

    #[test]
    fn package_installation() {
        let config = CrossBuildConfig::default();
        let mut manager = SdkManager::new(config).unwrap();

        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();

        // Test installing a package (will use mock metadata)
        let result = manager.install_package("openssl", "1.1.1", &TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap());

        // Should succeed with mock metadata
        assert!(result.is_ok());
    }
}