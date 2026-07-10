//! Lockfile management for reproducible cross-builds.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crossbuild_core::error::CrossBuildError;
use crossbuild_core::model::TargetTriple;

/// Lockfile for reproducible cross-builds.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Lockfile {
    pub version: u32,
    pub target: String,
    pub manifest_path: String,
    pub manifest_hash: String,
    pub toolchain: ToolchainLock,
    pub sysroot: Option<SysrootLock>,
    pub linker: LinkerLock,
    pub config_hash: String,
    pub created: String,
    pub metadata: BTreeMap<String, String>,
}

impl Lockfile {
    pub fn new(
        target: &TargetTriple,
        manifest_path: impl AsRef<Path>,
        manifest_hash: String,
        config_hash: String,
    ) -> Self {
        Self {
            version: 1,
            target: target.triple.clone(),
            manifest_path: manifest_path.as_ref().to_string_lossy().to_string(),
            manifest_hash,
            toolchain: ToolchainLock::default(),
            sysroot: None,
            linker: LinkerLock::default(),
            config_hash,
            created: chrono::Utc::now().to_rfc3339(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_toolchain(mut self, toolchain: ToolchainLock) -> Self {
        self.toolchain = toolchain;
        self
    }

    pub fn with_sysroot(mut self, sysroot: SysrootLock) -> Self {
        self.sysroot = Some(sysroot);
        self
    }

    pub fn with_linker(mut self, linker: LinkerLock) -> Self {
        self.linker = linker;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Saves the lockfile to disk.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), CrossBuildError> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| CrossBuildError::configuration(e.to_string()))?;
        let p = path.as_ref().to_path_buf();
        fs::write(&p, content).map_err(|source| CrossBuildError::Io {
            path: Some(p),
            source,
        })
    }

    /// Loads a lockfile from disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, CrossBuildError> {
        let content = fs::read_to_string(&path).map_err(|source| CrossBuildError::Io {
            path: Some(path.as_ref().to_path_buf()),
            source,
        })?;
        serde_json::from_str(&content).map_err(|e| CrossBuildError::LockfileCorrupted {
            reason: e.to_string(),
        })
    }

    /// Computes the cache key for this lockfile.
    pub fn cache_key(&self) -> String {
        format!("{}::{}", self.target, self.manifest_path)
    }

    /// Verifies that the lockfile matches the current configuration.
    pub fn verify(
        &self,
        target: &TargetTriple,
        manifest_path: impl AsRef<Path>,
        manifest_hash: &str,
        config_hash: &str,
    ) -> Result<(), CrossBuildError> {
        if self.target != target.triple {
            return Err(CrossBuildError::LockfileCorrupted {
                reason: format!(
                    "target mismatch: expected {}, got {}",
                    target.triple, self.target
                ),
            });
        }

        if self.manifest_path != manifest_path.as_ref().to_string_lossy() {
            return Err(CrossBuildError::LockfileCorrupted {
                reason: "manifest path mismatch".to_string(),
            });
        }

        if self.manifest_hash != manifest_hash {
            return Err(CrossBuildError::LockfileCorrupted {
                reason: "manifest hash mismatch".to_string(),
            });
        }

        if self.config_hash != config_hash {
            return Err(CrossBuildError::LockfileCorrupted {
                reason: "config hash mismatch".to_string(),
            });
        }

        Ok(())
    }
}

/// Toolchain information in lockfile.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ToolchainLock {
    pub provider: String,
    pub version: String,
    pub path: String,
    pub target_spec: Option<String>,
    pub rustc_version: String,
    pub cargo_version: String,
}

/// Sysroot information in lockfile.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SysrootLock {
    pub provider: String,
    pub path: String,
    pub hash: String,
}

/// Linker information in lockfile.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LinkerLock {
    pub provider: String,
    pub path: String,
    pub flavor: String,
    pub version: String,
}

/// Lockfile manager.
pub struct LockfileManager {
    lockfile_path: PathBuf,
}

impl LockfileManager {
    /// Creates a new lockfile manager.
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            lockfile_path: workspace_root.as_ref().join("crossbuild.lock"),
        }
    }

    /// Creates a new lockfile manager with a custom path.
    pub fn with_path(path: impl AsRef<Path>) -> Self {
        Self {
            lockfile_path: path.as_ref().to_path_buf(),
        }
    }

    /// Gets the lockfile path.
    pub fn path(&self) -> &Path {
        &self.lockfile_path
    }

    /// Creates and saves a new lockfile.
    pub fn create_lockfile(
        &self,
        target: &crossbuild_core::model::TargetTriple,
        manifest_path: impl AsRef<Path>,
        manifest_hash: String,
        config_hash: String,
    ) -> Result<Lockfile, CrossBuildError> {
        let lockfile = Lockfile::new(target, manifest_path, manifest_hash, config_hash);
        lockfile.save(&self.lockfile_path)?;
        Ok(lockfile)
    }

    /// Loads the existing lockfile.
    pub fn load_lockfile(&self) -> Result<Lockfile, CrossBuildError> {
        Lockfile::load(&self.lockfile_path)
    }

    /// Updates the lockfile with resolved provider information.
    pub fn update_lockfile(
        &self,
        mut lockfile: Lockfile,
        toolchain: ToolchainLock,
        sysroot: Option<SysrootLock>,
        linker: LinkerLock,
    ) -> Result<Lockfile, CrossBuildError> {
        lockfile.toolchain = toolchain;
        lockfile.sysroot = sysroot;
        lockfile.linker = linker;
        lockfile.save(&self.lockfile_path)?;
        Ok(lockfile)
    }

    /// Verifies the lockfile matches current state.
    pub fn verify_lockfile(
        &self,
        target: &crossbuild_core::model::TargetTriple,
        manifest_path: impl AsRef<Path>,
        manifest_hash: &str,
        config_hash: &str,
    ) -> Result<Lockfile, CrossBuildError> {
        let lockfile = self.load_lockfile()?;
        lockfile.verify(target, manifest_path, manifest_hash, config_hash)?;
        Ok(lockfile)
    }

    /// Checks if a lockfile exists.
    pub fn exists(&self) -> bool {
        self.lockfile_path.exists()
    }

    /// Removes the lockfile.
    pub fn remove(&self) -> Result<(), CrossBuildError> {
        if self.lockfile_path.exists() {
            fs::remove_file(&self.lockfile_path).map_err(|source| CrossBuildError::Io {
                path: Some(self.lockfile_path.clone()),
                source,
            })?;
        }
        Ok(())
    }
}

/// Computes a hash of the Cargo.toml manifest.
pub fn compute_manifest_hash(manifest_path: impl AsRef<Path>) -> Result<String, CrossBuildError> {
    let content = fs::read_to_string(&manifest_path).map_err(|source| CrossBuildError::Io {
        path: Some(manifest_path.as_ref().to_path_buf()),
        source,
    })?;

    // Hash the parsed manifest to normalize formatting
    let manifest: toml::Value = toml::from_str(&content)
        .map_err(|e| CrossBuildError::configuration(format!("failed to parse manifest: {e}")))?;

    let canonical =
        toml::to_string(&manifest).map_err(|e| CrossBuildError::configuration(e.to_string()))?;

    Ok(sha256_hash(&canonical))
}

/// Computes a hash of the cross-build configuration.
pub fn compute_config_hash(
    target: &crossbuild_core::model::TargetTriple,
    cargo_args: &[String],
    env_vars: &std::collections::BTreeMap<String, String>,
    profile: &str,
) -> String {
    use std::collections::BTreeMap;

    let mut config = BTreeMap::new();
    config.insert("target", target.triple.clone());
    config.insert("profile", profile.to_string());
    config.insert("cargo_args", cargo_args.join(" "));

    let mut env_vec: Vec<_> = env_vars.iter().collect();
    env_vec.sort_by_key(|(k, _)| *k);
    config.insert(
        "env",
        env_vec
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    let serialized = serde_json::to_string(&config)
        .expect("lockfile config serialization should always succeed");
    sha256_hash(&serialized)
}

/// Computes SHA256 hash of a string.
fn sha256_hash(input: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbuild_core::model::TargetTriple;
    use tempfile::tempdir;

    #[test]
    fn lockfile_creation() {
        let dir = tempdir().unwrap();
        let manager = LockfileManager::new(dir.path());

        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        let lockfile = manager
            .create_lockfile(
                &target,
                "Cargo.toml",
                "manifest-hash".to_string(),
                "config-hash".to_string(),
            )
            .unwrap();

        assert_eq!(lockfile.target, "x86_64-unknown-linux-gnu");
        assert_eq!(lockfile.manifest_hash, "manifest-hash");
        assert_eq!(lockfile.config_hash, "config-hash");
    }

    #[test]
    fn lockfile_verification() {
        let dir = tempdir().unwrap();
        let manager = LockfileManager::new(dir.path());

        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        let _lockfile = manager
            .create_lockfile(
                &target,
                "Cargo.toml",
                "manifest-hash".to_string(),
                "config-hash".to_string(),
            )
            .unwrap();

        // Should verify successfully
        let verified = manager
            .verify_lockfile(&target, "Cargo.toml", "manifest-hash", "config-hash")
            .unwrap();
        assert_eq!(verified.target, "x86_64-unknown-linux-gnu");

        // Should fail on hash mismatch
        let result = manager.verify_lockfile(&target, "Cargo.toml", "wrong-hash", "config-hash");
        assert!(result.is_err());
    }

    #[test]
    fn manifest_hash() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("Cargo.toml");
        fs::write(
            &manifest,
            r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"
"#,
        )
        .unwrap();

        let hash1 = compute_manifest_hash(&manifest).unwrap();
        let hash2 = compute_manifest_hash(&manifest).unwrap();
        assert_eq!(hash1, hash2);

        // Different content = different hash
        fs::write(
            &manifest,
            r#"
[package]
name = "test"
version = "0.2.0"
edition = "2021"
"#,
        )
        .unwrap();
        let hash3 = compute_manifest_hash(&manifest).unwrap();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn config_hash() {
        let target =
            crossbuild_core::model::TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        let cargo_args = vec!["--release".to_string()];
        let mut env = BTreeMap::new();
        env.insert("CC".to_string(), "clang".to_string());

        let hash1 = compute_config_hash(&target, &cargo_args, &env, "release");
        let hash2 = compute_config_hash(&target, &cargo_args, &env, "release");
        assert_eq!(hash1, hash2);

        let mut env2 = BTreeMap::new();
        env2.insert("CC".to_string(), "gcc".to_string());
        let hash3 = compute_config_hash(&target, &cargo_args, &env2, "release");
        assert_ne!(hash1, hash3);
    }
}
