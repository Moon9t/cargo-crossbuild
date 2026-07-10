//! Secure downloader with checksum verification.

use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use crossbuild_core::{cache::CachePolicy, error::CrossBuildError};

/// Download request with optional verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadRequest {
    pub url: String,
    pub destination: PathBuf,
    pub expected_checksum: Option<String>,
    pub checksum_algorithm: ChecksumAlgorithm,
    pub timeout: Duration,
    pub headers: HashMap<String, String>,
}

impl DownloadRequest {
    /// Creates a new download request.
    pub fn new(url: impl Into<String>, destination: impl Into<PathBuf>) -> Self {
        Self {
            url: url.into(),
            destination: destination.into(),
            expected_checksum: None,
            checksum_algorithm: ChecksumAlgorithm::Sha256,
            timeout: Duration::from_secs(300),
            headers: HashMap::new(),
        }
    }

    /// Creates a new download request with checksum verification.
    pub fn with_checksum(
        url: impl Into<String>,
        destination: impl Into<PathBuf>,
        checksum: impl Into<String>,
        algorithm: ChecksumAlgorithm,
    ) -> Self {
        Self {
            url: url.into(),
            destination: destination.into(),
            expected_checksum: Some(checksum.into()),
            checksum_algorithm: algorithm,
            timeout: Duration::from_secs(300),
            headers: HashMap::new(),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Returns the provenance label for logging.
    pub fn provenance_label(&self) -> String {
        if let Some(checksum) = &self.expected_checksum {
            format!("{} ({}:{})", self.url, self.checksum_algorithm, checksum)
        } else {
            self.url.clone()
        }
    }

    /// Checks if the request has checksum verification.
    pub fn is_verified(&self) -> bool {
        self.expected_checksum.is_some()
    }
}

/// Supported checksum algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
    Sha256,
    Sha512,
    Blake3,
}

impl Display for ChecksumAlgorithm {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ChecksumAlgorithm::Sha256 => f.write_str("sha256"),
            ChecksumAlgorithm::Sha512 => f.write_str("sha512"),
            ChecksumAlgorithm::Blake3 => f.write_str("blake3"),
        }
    }
}

/// Download progress callback.
pub type ProgressCallback = Box<dyn FnMut(DownloadProgress) + Send>;

/// Download progress information.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub url: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub status: DownloadStatus,
}

/// Download status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadStatus {
    Starting,
    Downloading,
    Verifying,
    Complete,
    Failed,
}

/// Download result.
#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub checksum: String,
    pub algorithm: ChecksumAlgorithm,
}

/// Secure downloader with checksum verification.
pub struct Downloader {
    client: reqwest::blocking::Client,
    cache_policy: CachePolicy,
    progress_callback: Option<ProgressCallback>,
}

impl Downloader {
    /// Creates a new downloader.
    pub fn new(cache_policy: CachePolicy) -> Result<Self, CrossBuildError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| {
                CrossBuildError::configuration(format!("Failed to create HTTP client: {e}"))
            })?;

        Ok(Self {
            client,
            cache_policy,
            progress_callback: None,
        })
    }

    /// Sets a progress callback.
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Downloads a file with verification.
    pub fn download(
        &mut self,
        request: DownloadRequest,
    ) -> Result<DownloadResult, CrossBuildError> {
        // Create destination directory
        if let Some(parent) = request.destination.parent() {
            fs::create_dir_all(parent).map_err(|source| CrossBuildError::Io {
                path: Some(parent.to_path_buf()),
                source,
            })?;
        }

        // Check if already cached and verified
        if request.destination.exists() && request.is_verified() {
            if let Ok(checksum) =
                self.compute_checksum(&request.destination, request.checksum_algorithm)
            {
                if checksum == request.expected_checksum.as_deref().unwrap_or("") {
                    return Ok(DownloadResult {
                        path: request.destination.clone(),
                        size_bytes: fs::metadata(&request.destination)?.len(),
                        checksum: checksum.clone(),
                        algorithm: request.checksum_algorithm,
                    });
                }
            }
        }

        // Download
        let mut response = self
            .client
            .get(&request.url)
            .timeout(request.timeout)
            .send()
            .map_err(|e| CrossBuildError::DownloadFailed {
                url: request.url.clone(),
                reason: e.to_string(),
            })?;

        if !response.status().is_success() {
            return Err(CrossBuildError::DownloadFailed {
                url: request.url.clone(),
                reason: format!("HTTP {}", response.status()),
            });
        }

        let total_size = response.content_length();

        // Create temp file for atomic write
        let temp_path = request.destination.with_extension("tmp.part");
        let mut file = fs::File::create(&temp_path).map_err(|source| CrossBuildError::Io {
            path: Some(temp_path.clone()),
            source,
        })?;

        let mut downloaded = 0u64;
        let mut buffer = vec![0u8; 8192];
        let mut hasher = self.create_hasher(request.checksum_algorithm);

        loop {
            let bytes_read =
                response
                    .read(&mut buffer)
                    .map_err(|e| CrossBuildError::DownloadFailed {
                        url: request.url.clone(),
                        reason: e.to_string(),
                    })?;

            if bytes_read == 0 {
                break;
            }

            file.write_all(&buffer[..bytes_read])
                .map_err(|source| CrossBuildError::Io {
                    path: Some(temp_path.clone()),
                    source,
                })?;

            hasher.update(&buffer[..bytes_read]);
            downloaded += bytes_read as u64;

            if let Some(ref mut callback) = self.progress_callback {
                callback(DownloadProgress {
                    url: request.url.clone(),
                    downloaded_bytes: downloaded,
                    total_bytes: total_size,
                    status: DownloadStatus::Downloading,
                });
            }
        }

        file.flush().map_err(|source| CrossBuildError::Io {
            path: Some(temp_path.clone()),
            source,
        })?;

        // Verify checksum
        if let Some(expected) = &request.expected_checksum {
            if let Some(ref mut callback) = self.progress_callback {
                callback(DownloadProgress {
                    url: request.url.clone(),
                    downloaded_bytes: downloaded,
                    total_bytes: total_size,
                    status: DownloadStatus::Verifying,
                });
            }

            let actual = self.finalize_checksum(hasher, request.checksum_algorithm);
            if actual != *expected {
                let _ = fs::remove_file(&temp_path);
                return Err(CrossBuildError::ChecksumMismatch {
                    url: request.url,
                    expected: expected.clone(),
                    actual,
                });
            }
        }

        // Atomic move
        fs::rename(&temp_path, &request.destination).map_err(|source| CrossBuildError::Io {
            path: Some(request.destination.clone()),
            source,
        })?;

        let final_checksum =
            self.compute_checksum(&request.destination, request.checksum_algorithm)?;

        if let Some(ref mut callback) = self.progress_callback {
            callback(DownloadProgress {
                url: request.url.clone(),
                downloaded_bytes: downloaded,
                total_bytes: total_size,
                status: DownloadStatus::Complete,
            });
        }

        Ok(DownloadResult {
            path: request.destination,
            size_bytes: downloaded,
            checksum: final_checksum,
            algorithm: request.checksum_algorithm,
        })
    }

    /// Downloads to cache directory.
    pub fn download_to_cache(
        &mut self,
        url: &str,
        cache_key: &str,
        expected_checksum: Option<&str>,
        algorithm: ChecksumAlgorithm,
    ) -> Result<DownloadResult, CrossBuildError> {
        let dest = self
            .cache_policy
            .download_dir(&PathBuf::from("."))
            .join(cache_key);
        self.download(DownloadRequest::with_checksum(
            url,
            dest,
            expected_checksum.unwrap_or(""),
            algorithm,
        ))
    }

    fn create_hasher(&self, algorithm: ChecksumAlgorithm) -> Box<dyn ChecksumHasher> {
        match algorithm {
            ChecksumAlgorithm::Sha256 => Box::new(Sha256Hasher::new()),
            ChecksumAlgorithm::Sha512 => Box::new(Sha512Hasher::new()),
            ChecksumAlgorithm::Blake3 => Box::new(Blake3Hasher::new()),
        }
    }

    fn finalize_checksum(
        &self,
        hasher: Box<dyn ChecksumHasher>,
        _algorithm: ChecksumAlgorithm,
    ) -> String {
        hasher.finalize()
    }

    fn compute_checksum(
        &self,
        path: &Path,
        algorithm: ChecksumAlgorithm,
    ) -> Result<String, CrossBuildError> {
        let mut file = fs::File::open(path).map_err(|source| CrossBuildError::Io {
            path: Some(path.to_path_buf()),
            source,
        })?;

        let mut hasher = self.create_hasher(algorithm);
        let mut buffer = vec![0u8; 8192];

        loop {
            let bytes_read = file
                .read(&mut buffer)
                .map_err(|source| CrossBuildError::Io {
                    path: Some(path.to_path_buf()),
                    source,
                })?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(self.finalize_checksum(hasher, algorithm))
    }
}

trait ChecksumHasher: Send {
    fn update(&mut self, data: &[u8]);
    fn finalize(self: Box<Self>) -> String;
}

struct Sha256Hasher(sha2::Sha256);

impl Sha256Hasher {
    fn new() -> Self {
        use sha2::Digest;
        Self(sha2::Sha256::new())
    }
}

impl ChecksumHasher for Sha256Hasher {
    fn update(&mut self, data: &[u8]) {
        use sha2::Digest;
        self.0.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        use sha2::Digest;
        hex::encode(self.0.finalize())
    }
}

struct Sha512Hasher(sha2::Sha512);

impl Sha512Hasher {
    fn new() -> Self {
        use sha2::Digest;
        Self(sha2::Sha512::new())
    }
}

impl ChecksumHasher for Sha512Hasher {
    fn update(&mut self, data: &[u8]) {
        use sha2::Digest;
        self.0.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        use sha2::Digest;
        hex::encode(self.0.finalize())
    }
}

struct Blake3Hasher(blake3::Hasher);

impl Blake3Hasher {
    fn new() -> Self {
        Self(blake3::Hasher::new())
    }
}

impl ChecksumHasher for Blake3Hasher {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        self.0.finalize().to_hex().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_request_creation() {
        let req = DownloadRequest::new("https://example.com/file", "/tmp/file");
        assert_eq!(req.url, "https://example.com/file");
        assert!(!req.is_verified());
    }

    #[test]
    fn download_request_with_checksum() {
        let req = DownloadRequest::with_checksum(
            "https://example.com/file",
            "/tmp/file",
            "abc123",
            ChecksumAlgorithm::Sha256,
        );
        assert!(req.is_verified());
        assert_eq!(req.expected_checksum, Some("abc123".to_string()));
    }

    #[test]
    fn provenance_label() {
        let req = DownloadRequest::with_checksum(
            "https://example.com/file",
            "/tmp/file",
            "sha256:abc123",
            ChecksumAlgorithm::Sha256,
        );
        assert!(req.provenance_label().contains("sha256:abc123"));
    }

    #[test]
    fn checksum_algorithms() {
        assert_eq!(ChecksumAlgorithm::Sha256.to_string(), "sha256");
        assert_eq!(ChecksumAlgorithm::Sha512.to_string(), "sha512");
        assert_eq!(ChecksumAlgorithm::Blake3.to_string(), "blake3");
    }
}
