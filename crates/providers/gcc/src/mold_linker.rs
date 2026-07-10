//! Mold linker provider (fast linker).

use std::path::PathBuf;

use crossbuild_core::{
    model::{LinkerFlavor, TargetTriple},
    error::CrossBuildError,
};

/// Mold linker provider (fast linker).
pub struct MoldLinkerProvider;

impl super::LinkerProvider for MoldLinkerProvider {
    fn name(&self) -> &'static str {
        "mold"
    }

    fn priority(&self) -> i32 {
        180
    }

    fn can_provide(&self, target: &TargetTriple, _host: &crate::model::HostInfo) -> bool {
        // Mold supports ELF targets
        matches!(target.family(), crate::model::TargetFamily::Linux)
            && target.abi != crate::model::Abi::Msvc
            && target.arch.pointer_width() == 64
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        _host: &crate::model::HostInfo,
        _request: &crate::model::BuildRequest,
    ) -> Result<super::LinkerResolution, CrossBuildError> {
        let linker_path = which::which("mold").map_err(|_| CrossBuildError::ToolNotFound {
            tool: "mold".to_string(),
        })?;

        Ok(super::LinkerResolution::new(linker_path, LinkerFlavor::Mold)
            .with_note("Using mold linker (fast)"))
    }
}