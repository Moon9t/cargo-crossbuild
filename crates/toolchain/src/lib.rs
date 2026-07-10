//! Toolchain configuration and resolution for cross-compilation.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Result;
use crossbuild_core::model::TargetTriple;

/// Toolchain configuration for a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainConfig {
    pub target: TargetTriple,
    pub rustc_path: Option<PathBuf>,
    pub cargo_path: Option<PathBuf>,
    pub target_spec: Option<String>,
    pub rustflags: Vec<String>,
    pub env: BTreeMap<String, String>,
}

impl ToolchainConfig {
    pub fn new(target: TargetTriple) -> Self {
        Self {
            target,
            rustc_path: None,
            cargo_path: None,
            target_spec: None,
            rustflags: Vec::new(),
            env: BTreeMap::new(),
        }
    }

    pub fn with_rustc(mut self, path: PathBuf) -> Self {
        self.rustc_path = Some(path);
        self
    }

    pub fn with_cargo(mut self, path: PathBuf) -> Self {
        self.cargo_path = Some(path);
        self
    }

    pub fn with_target_spec(mut self, spec: String) -> Self {
        self.target_spec = Some(spec);
        self
    }

    pub fn with_rustflag(mut self, flag: impl Into<String>) -> Self {
        self.rustflags.push(flag.into());
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

/// Resolution result from a toolchain provider.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolchainResolution {
    pub env: BTreeMap<String, String>,
    pub cargo_config: Option<toml::Table>,
    pub notes: Vec<String>,
    pub rustc_path: Option<PathBuf>,
    pub cargo_path: Option<PathBuf>,
    pub target_spec: Option<String>,
    pub rustflags: Vec<String>,
}

impl ToolchainResolution {
    pub fn new() -> Self {
        Self {
            env: BTreeMap::new(),
            cargo_config: None,
            notes: Vec::new(),
            rustc_path: None,
            cargo_path: None,
            target_spec: None,
            rustflags: Vec::new(),
        }
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn with_cargo_config(mut self, config: toml::Table) -> Self {
        self.cargo_config = Some(config);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_rustc(mut self, path: PathBuf) -> Self {
        self.rustc_path = Some(path);
        self
    }

    pub fn with_cargo(mut self, path: PathBuf) -> Self {
        self.cargo_path = Some(path);
        self
    }

    pub fn with_target_spec(mut self, spec: String) -> Self {
        self.target_spec = Some(spec);
        self
    }

    pub fn with_rustflags(mut self, flags: Vec<String>) -> Self {
        self.rustflags = flags;
        self
    }
}

impl Default for ToolchainResolution {
    fn default() -> Self {
        Self::new()
    }
}

pub fn find_rustup_toolchain(
    rustup_home: &str,
) -> Result<String, crossbuild_core::error::CrossBuildError> {
    let toolchains_dir = PathBuf::from(rustup_home).join("toolchains");
    if !toolchains_dir.exists() {
        return Err(crossbuild_core::error::CrossBuildError::SysrootNotFound {
            target: "rustup".to_string(),
        });
    }

    // Read the default toolchain
    let default_file = PathBuf::from(rustup_home)
        .join("settings")
        .join("default-toolchain");
    if default_file.exists() {
        let content = std::fs::read_to_string(&default_file).map_err(|_| {
            crossbuild_core::error::CrossBuildError::SysrootNotFound {
                target: "rustup".to_string(),
            }
        })?;
        let toolchain = content.trim().to_string();
        if toolchains_dir.join(&toolchain).exists() {
            return Ok(toolchain);
        }
    }

    // Fallback: find first stable toolchain
    for entry in std::fs::read_dir(&toolchains_dir).map_err(|_| {
        crossbuild_core::error::CrossBuildError::SysrootNotFound {
            target: "rustup".to_string(),
        }
    })? {
        let entry =
            entry.map_err(
                |_| crossbuild_core::error::CrossBuildError::SysrootNotFound {
                    target: "rustup".to_string(),
                },
            )?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.contains("stable") || name.contains("1.") {
            return Ok(name);
        }
    }

    Err(crossbuild_core::error::CrossBuildError::SysrootNotFound {
        target: "rustup".to_string(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolchain_config_creation() {
        let target =
            crossbuild_core::model::TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        let config = ToolchainConfig::new(target.clone());
        assert_eq!(config.target, target);
    }
}
