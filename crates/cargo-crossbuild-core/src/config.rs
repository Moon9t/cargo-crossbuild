use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

/// Configuration loaded from environment and workspace defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossBuildConfig {
    pub cargo_program: PathBuf,
    pub target_dir: PathBuf,
    pub extra_env: BTreeMap<String, String>,
}

impl Default for CrossBuildConfig {
    fn default() -> Self {
        Self {
            cargo_program: PathBuf::from("cargo"),
            target_dir: PathBuf::from("target").join("crossbuild"),
            extra_env: BTreeMap::new(),
        }
    }
}

impl CrossBuildConfig {
    pub fn from_environment() -> Self {
        let mut config = Self::default();

        if let Ok(value) = env::var("CARGO") {
            if !value.trim().is_empty() {
                config.cargo_program = PathBuf::from(value);
            }
        }

        if let Ok(value) = env::var("CROSSBUILD_TARGET_DIR") {
            if !value.trim().is_empty() {
                config.target_dir = PathBuf::from(value);
            }
        }

        if let Ok(value) = env::var("CROSSBUILD_COLOR") {
            if !value.trim().is_empty() {
                config
                    .extra_env
                    .insert("CARGO_TERM_COLOR".to_string(), value);
            }
        }

        config
    }

    pub fn cargo_program_str(&self) -> String {
        self.cargo_program.to_string_lossy().into_owned()
    }

    pub fn target_dir_for(&self, workspace_root: &Path) -> PathBuf {
        if self.target_dir.is_absolute() {
            self.target_dir.clone()
        } else {
            workspace_root.join(&self.target_dir)
        }
    }
}
