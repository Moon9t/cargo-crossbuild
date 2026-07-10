//! Cache management for cross-build artifacts, downloads, and sysroots.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::model::TargetTriple;
use crate::error::CrossBuildError;

/// Cache policy configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachePolicy {
    pub root: PathBuf,
    pub max_size_bytes: Option<u64>,
    pub max_age: Option<Duration>,
    pub compress: bool,
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            root: PathBuf::from("target").join("crossbuild-cache"),
            max_size_bytes: Some(10 * 1024 * 1024 * 1024), // 10 GB
            max_age: Some(Duration::from_secs(30 * 24 * 60 * 60)), // 30 days
            compress: true,
        }
    }
}

impl CachePolicy {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            ..Default::default()
        }
    }

    pub fn with_max_size(mut self, bytes: u64) -> Self {
        self.max_size_bytes = Some(bytes);
        self
    }

    pub fn with_max_age(mut self, age: Duration) -> Self {
        self.max_age = Some(age);
        self
    }

    pub fn with_compression(mut self, compress: bool) -> Self {
        self.compress = compress;
        self
    }

    pub fn absolute_root(&self, workspace_root: &Path) -> PathBuf {
        if self.root.is_absolute() {
            self.root.clone()
        } else {
            workspace_root.join(&self.root)
        }
    }

    pub fn cache_key(&self, workspace_root: &Path, target: &TargetTriple) -> String {
        let workspace_label = workspace_root
            .to_string_lossy()
            .replace(['\\', '/', ':'], "_");
        format!("{}::{}", workspace_label, target.triple)
    }

    pub fn download_dir(&self, workspace_root: &Path) -> PathBuf {
        self.absolute_root(workspace_root).join("downloads")
    }

    pub fn sysroot_dir(&self, workspace_root: &Path) -> PathBuf {
        self.absolute_root(workspace_root).join("sysroots")
    }

    pub fn toolchain_dir(&self, workspace_root: &Path) -> PathBuf {
        self.absolute_root(workspace_root).join("toolchains")
    }

    pub fn build_dir(&self, workspace_root: &Path, target: &TargetTriple) -> PathBuf {
        self.absolute_root(workspace_root)
            .join("builds")
            .join(&target.triple)
    }

    pub fn metadata_path(&self, workspace_root: &Path) -> PathBuf {
        self.absolute_root(workspace_root).join("metadata.json")
    }
}

/// Cache metadata for tracking entries.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheMetadata {
    pub entries: BTreeMap<String, CacheEntry>,
    pub total_size_bytes: u64,
    pub last_cleanup: u64,
}

impl Default for CacheMetadata {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            total_size_bytes: 0,
            last_cleanup: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

/// Individual cache entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub created: u64,
    pub last_accessed: u64,
    pub access_count: u64,
    pub entry_type: CacheEntryType,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum CacheEntryType {
    Download,
    Sysroot,
    Toolchain,
    BuildArtifact,
    Other,
}

/// Cache manager for handling all cache operations.
pub struct CacheManager {
    policy: CachePolicy,
    workspace_root: PathBuf,
    metadata: CacheMetadata,
}

impl CacheManager {
    /// Returns the cache policy.
    pub fn policy(&self) -> &CachePolicy {
        &self.policy
    }

    /// Creates a new cache manager.
    pub fn new(policy: CachePolicy, workspace_root: impl AsRef<Path>) -> Result<Self, CrossBuildError> {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        let root = policy.absolute_root(&workspace_root);
        fs::create_dir_all(&root).map_err(|source| CrossBuildError::Io {
            path: Some(root),
            source,
        })?;

        fs::create_dir_all(policy.download_dir(&workspace_root)).map_err(|source| CrossBuildError::Io {
            path: Some(policy.download_dir(&workspace_root)),
            source,
        })?;
        fs::create_dir_all(policy.sysroot_dir(&workspace_root)).map_err(|source| CrossBuildError::Io {
            path: Some(policy.sysroot_dir(&workspace_root)),
            source,
        })?;
        fs::create_dir_all(policy.toolchain_dir(&workspace_root)).map_err(|source| CrossBuildError::Io {
            path: Some(policy.toolchain_dir(&workspace_root)),
            source,
        })?;

        let metadata_path = policy.metadata_path(&workspace_root);
        let metadata = if metadata_path.exists() {
            let content = fs::read_to_string(&metadata_path).map_err(|source| CrossBuildError::Io {
                path: Some(metadata_path),
                source,
            })?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            CacheMetadata::default()
        };

        Ok(Self {
            policy,
            workspace_root,
            metadata,
        })
    }

    /// Gets the path for a cached download.
    pub fn get_download(&self, url: &str, checksum: Option<&str>) -> Option<PathBuf> {
        let key = self.download_key(url, checksum);
        self.metadata.entries.get(&key).and_then(|entry| {
            if entry.path.exists() {
                Some(entry.path.clone())
            } else {
                None
            }
        })
    }

    /// Stores a downloaded file in the cache.
    pub fn store_download(
        &mut self,
        url: &str,
        checksum: Option<&str>,
        source_path: &Path,
    ) -> Result<PathBuf, CrossBuildError> {
        let key = self.download_key(url, checksum);
        let dest = self.policy.download_dir(&self.workspace_root).join(&key);

        fs::copy(source_path, &dest).map_err(|source| CrossBuildError::Io {
            path: Some(dest.clone()),
            source,
        })?;

        let size = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
        let now = current_timestamp();

        self.metadata.entries.insert(
            key.clone(),
            CacheEntry {
                key: key.clone(),
                path: dest.clone(),
                size_bytes: size,
                created: now,
                last_accessed: now,
                access_count: 1,
                entry_type: CacheEntryType::Download,
                metadata: {
                    let mut m = BTreeMap::new();
                    m.insert("url".to_string(), url.to_string());
                    if let Some(cs) = checksum {
                        m.insert("checksum".to_string(), cs.to_string());
                    }
                    m
                },
            },
        );
        self.metadata.total_size_bytes += size;
        self.save_metadata()?;

        Ok(dest)
    }

    /// Gets or creates a sysroot cache entry.
    pub fn get_sysroot(&self, target: &TargetTriple, provider: &str) -> Option<PathBuf> {
        let key = self.sysroot_key(target, provider);
        self.metadata.entries.get(&key).and_then(|entry| {
            if entry.path.exists() {
                Some(entry.path.clone())
            } else {
                None
            }
        })
    }

    /// Stores a sysroot in the cache.
    pub fn store_sysroot(
        &mut self,
        target: &TargetTriple,
        provider: &str,
        source_path: &Path,
    ) -> Result<PathBuf, CrossBuildError> {
        let key = self.sysroot_key(target, provider);
        let dest = self.policy.sysroot_dir(&self.workspace_root).join(&key);

        if source_path.is_dir() {
            copy_dir(source_path, &dest)?;
        } else {
            fs::copy(source_path, &dest).map_err(|source| CrossBuildError::Io {
                path: Some(dest.clone()),
                source,
            })?;
        }

        let size = dir_size(&dest).unwrap_or(0);
        let now = current_timestamp();

        self.metadata.entries.insert(
            key.clone(),
            CacheEntry {
                key: key.clone(),
                path: dest.clone(),
                size_bytes: size,
                created: now,
                last_accessed: now,
                access_count: 1,
                entry_type: CacheEntryType::Sysroot,
                metadata: {
                    let mut m = BTreeMap::new();
                    m.insert("target".to_string(), target.triple.clone());
                    m.insert("provider".to_string(), provider.to_string());
                    m
                },
            },
        );
        self.metadata.total_size_bytes += size;
        self.save_metadata()?;

        Ok(dest)
    }

    /// Gets the build directory for a target.
    pub fn build_dir(&self, target: &TargetTriple) -> PathBuf {
        self.policy.build_dir(&self.workspace_root, target)
    }

    /// Cleans up old or excess cache entries.
    pub fn cleanup(&mut self) -> Result<CleanupReport, CrossBuildError> {
        let mut report = CleanupReport::default();
        let now = current_timestamp();

        // Remove expired entries
        if let Some(max_age) = self.policy.max_age {
            let cutoff = now - max_age.as_secs();
            let expired: Vec<_> = self.metadata.entries
                .iter()
                .filter(|(_, entry)| entry.last_accessed < cutoff)
                .map(|(k, _)| k.clone())
                .collect();

            for key in expired {
                if let Some(entry) = self.metadata.entries.remove(&key) {
                    if entry.path.exists() {
                        remove_entry(&entry.path)?;
                        report.removed_entries += 1;
                        report.freed_bytes += entry.size_bytes;
                        self.metadata.total_size_bytes = self.metadata.total_size_bytes.saturating_sub(entry.size_bytes);
                    }
                }
            }
        }

        // Enforce size limit
        if let Some(max_size) = self.policy.max_size_bytes {
            if self.metadata.total_size_bytes > max_size {
                // Sort by last accessed (LRU)
                let mut entries: Vec<_> = self.metadata.entries
                    .iter()
                    .map(|(k, v)| (k.clone(), v.last_accessed, v.size_bytes, v.path.clone()))
                    .collect();
                entries.sort_by_key(|(_, last_accessed, _, _)| *last_accessed);

                for (key, _, size, path) in entries {
                    if self.metadata.total_size_bytes <= max_size {
                        break;
                    }
                    if let Some(entry) = self.metadata.entries.remove(&key) {
                        if path.exists() {
                            remove_entry(&path)?;
                            report.removed_entries += 1;
                            report.freed_bytes += entry.size_bytes;
                            self.metadata.total_size_bytes = self.metadata.total_size_bytes.saturating_sub(entry.size_bytes);
                        }
                    }
                }
            }
        }

        self.metadata.last_cleanup = now;
        self.save_metadata()?;

        Ok(report)
    }

    /// Gets cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_entries: self.metadata.entries.len(),
            total_size_bytes: self.metadata.total_size_bytes,
            by_type: {
                let mut by_type = BTreeMap::new();
                for entry in self.metadata.entries.values() {
                    *by_type.entry(entry.entry_type).or_insert(0) += 1;
                }
                by_type
            },
            root: self.policy.absolute_root(&self.workspace_root),
        }
    }

    fn download_key(&self, url: &str, checksum: Option<&str>) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        if let Some(cs) = checksum {
            hasher.update(cs.as_bytes());
        }
        hex::encode(hasher.finalize())[..16].to_string()
    }

    fn sysroot_key(&self, target: &TargetTriple, provider: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(target.triple.as_bytes());
        hasher.update(provider.as_bytes());
        format!("sysroot-{}", &hex::encode(hasher.finalize())[..16])
    }

    fn save_metadata(&self) -> Result<(), CrossBuildError> {
        let path = self.policy.metadata_path(&self.workspace_root);
        let content = serde_json::to_string_pretty(&self.metadata)
            .map_err(|e| CrossBuildError::configuration(e.to_string()))?;
        fs::write(&path, content).map_err(|source| CrossBuildError::Io {
            path: Some(path),
            source,
        })
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: u64,
    pub by_type: BTreeMap<CacheEntryType, usize>,
    pub root: PathBuf,
}

/// Cleanup report.
#[derive(Debug, Default, Clone)]
pub struct CleanupReport {
    pub removed_entries: usize,
    pub freed_bytes: u64,
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn copy_dir(src: &Path, dest: &Path) -> Result<(), CrossBuildError> {
    fs::create_dir_all(dest).map_err(|source| CrossBuildError::Io {
        path: Some(dest.to_path_buf()),
        source,
    })?;

    for entry in fs::read_dir(src).map_err(|source| CrossBuildError::Io {
        path: Some(src.to_path_buf()),
        source,
    })? {
        let entry = entry.map_err(|source| CrossBuildError::Io {
            path: Some(src.to_path_buf()),
            source,
        })?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path).map_err(|source| CrossBuildError::Io {
                path: Some(dest_path),
                source,
            })?;
        }
    }
    Ok(())
}

fn dir_size(path: &Path) -> Result<u64, CrossBuildError> {
    let mut size = 0;
    for entry in fs::read_dir(path).map_err(|source| CrossBuildError::Io {
        path: Some(path.to_path_buf()),
        source,
    })? {
        let entry = entry.map_err(|source| CrossBuildError::Io {
            path: Some(path.to_path_buf()),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            size += dir_size(&path)?;
        } else {
            size += fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        }
    }
    Ok(size)
}

fn remove_entry(path: &Path) -> Result<(), CrossBuildError> {
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|source| CrossBuildError::Io {
            path: Some(path.to_path_buf()),
            source,
        })
    } else {
        fs::remove_file(path).map_err(|source| CrossBuildError::Io {
            path: Some(path.to_path_buf()),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TargetTriple;
    use tempfile::tempdir;

    #[test]
    fn cache_policy_default() {
        let policy = CachePolicy::default();
        assert_eq!(policy.root, PathBuf::from("target").join("crossbuild-cache"));
        assert_eq!(policy.max_size_bytes, Some(10 * 1024 * 1024 * 1024));
    }

    #[test]
    fn cache_key_generation() {
        let policy = CachePolicy::default();
        let workspace = PathBuf::from("/home/user/project");
        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();

        let key = policy.cache_key(&workspace, &target);
        assert!(key.contains("home_user_project"));
        assert!(key.contains("x86_64-unknown-linux-gnu"));
    }

    #[test]
    fn cache_manager_creation() {
        let dir = tempdir().unwrap();
        let policy = CachePolicy::new(dir.path().join("cache"));
        let manager = CacheManager::new(policy, dir.path()).unwrap();

        let stats = manager.stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.total_size_bytes, 0);
    }

    #[test]
    fn download_caching() {
        let dir = tempdir().unwrap();
        let policy = CachePolicy::new(dir.path().join("cache"));
        let mut manager = CacheManager::new(policy, dir.path()).unwrap();

        // Create a test file
        let source = dir.path().join("test-download");
        fs::write(&source, b"test content").unwrap();

        // Store in cache
        let cached = manager.store_download(
            "https://example.com/file",
            Some("sha256:abc123"),
            &source,
        ).unwrap();

        assert!(cached.exists());

        // Retrieve from cache
        let retrieved = manager.get_download("https://example.com/file", Some("sha256:abc123"));
        assert_eq!(retrieved, Some(cached));

        // Different checksum should not match
        let retrieved2 = manager.get_download("https://example.com/file", Some("sha256:different"));
        assert_eq!(retrieved2, None);
    }

    #[test]
    fn sysroot_caching() {
        let dir = tempdir().unwrap();
        let policy = CachePolicy::new(dir.path().join("cache"));
        let mut manager = CacheManager::new(policy, dir.path()).unwrap();

        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();

        // Create a test sysroot
        let sysroot = dir.path().join("test-sysroot");
        fs::create_dir_all(sysroot.join("lib")).unwrap();
        fs::write(sysroot.join("lib").join("libc.so"), b"fake").unwrap();

        // Store in cache
        let cached = manager.store_sysroot(&target, "rustup", &sysroot).unwrap();

        assert!(cached.exists());
        assert!(cached.join("lib").join("libc.so").exists());

        // Retrieve from cache
        let retrieved = manager.get_sysroot(&target, "rustup");
        assert_eq!(retrieved, Some(cached));
    }

    #[test]
    fn cleanup_removes_old_entries() {
        let dir = tempdir().unwrap();
        let policy = CachePolicy::new(dir.path().join("cache"))
            .with_max_age(Duration::from_secs(60)); // 1 minute
        let mut manager = CacheManager::new(policy, dir.path()).unwrap();

        // Create entries with old timestamps
        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        let sysroot = dir.path().join("sysroot");
        fs::create_dir_all(sysroot.join("lib")).unwrap();
        fs::write(sysroot.join("lib").join("libc.so"), b"fake").unwrap();

        let cached = manager.store_sysroot(&target, "rustup", &sysroot).unwrap();

        // Manually set old timestamp
        if let Some(entry) = manager.metadata.entries.get_mut(&manager.sysroot_key(&target, "rustup")) {
            entry.last_accessed = current_timestamp() - 120; // 2 minutes ago
        }
        manager.save_metadata().unwrap();

        // Run cleanup
        let report = manager.cleanup().unwrap();
        assert_eq!(report.removed_entries, 1);
        assert!(!cached.exists());
    }
}