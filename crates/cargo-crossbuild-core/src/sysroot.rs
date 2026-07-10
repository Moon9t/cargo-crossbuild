//! Sysroot configuration and management.

use std::path::PathBuf;
use crate::model::TargetTriple;

/// Sysroot configuration for a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysrootConfig {
    pub target: TargetTriple,
    pub path: PathBuf,
    pub lib_dir: PathBuf,
    pub include_dir: Option<PathBuf>,
    pub is_builtin: bool,
}

impl SysrootConfig {
    pub fn new(target: TargetTriple, path: PathBuf) -> Self {
        let lib_dir = path.join("lib");
        let include_dir = path.join("include");

        Self {
            target,
            path,
            lib_dir: lib_dir.clone(),
            include_dir: if include_dir.exists() { Some(include_dir) } else { None },
            is_builtin: false,
        }
    }

    pub fn with_builtin(mut self, builtin: bool) -> Self {
        self.is_builtin = builtin;
        self
    }

    pub fn linker_search_paths(&self) -> Vec<PathBuf> {
        vec![self.lib_dir.clone()]
    }

    pub fn include_paths(&self) -> Vec<PathBuf> {
        self.include_dir.iter().cloned().collect()
    }
}