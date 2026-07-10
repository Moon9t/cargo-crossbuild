use std::path::{Path, PathBuf};

/// Describes a planned installation destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPlan {
    pub destination: PathBuf,
}

impl InstallPlan {
    pub fn new(destination: impl Into<PathBuf>) -> Self {
        Self {
            destination: destination.into(),
        }
    }

    pub fn resolved_destination(&self, workspace_root: &Path) -> PathBuf {
        if self.destination.is_absolute() {
            self.destination.clone()
        } else {
            workspace_root.join(&self.destination)
        }
    }
}
