//! LLD linker provider.

use std::path::PathBuf;

use crossbuild_core::{
    model::{Abi, LinkerFlavor, TargetFamily, TargetTriple},
    error::CrossBuildError,
};

/// LLD linker provider.
pub struct LldLinkerProvider;

impl super::LinkerProvider for LldLinkerProvider {
    fn name(&self) -> &'static str {
        "lld"
    }

    fn priority(&self) -> i32 {
        150
    }

    fn can_provide(&self, target: &TargetTriple, _host: &crate::model::HostInfo) -> bool {
        // LLD supports most ELF targets and WASM
        matches!(
            target.family(),
            TargetFamily::Linux
                | TargetFamily::Wasm
                | TargetFamily::MacOs
                | TargetFamily::BareMetal
                | TargetFamily::Other
        ) && target.abi != crate::model::Abi::Msvc
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        _host: &crate::model::HostInfo,
        _request: &crate::model::BuildRequest,
    ) -> Result<super::LinkerResolution, CrossBuildError> {
        let (linker_name, flavor) = match target.family() {
            TargetFamily::Wasm => ("wasm-ld", LinkerFlavor::WasmLld),
            TargetFamily::MacOs => ("ld64.lld", LinkerFlavor::Darwin),
            _ => ("ld.lld", LinkerFlavor::Lld),
        };

        let linker_path = which::which(linker_name).map_err(|_| CrossBuildError::ToolNotFound {
            tool: linker_name.to_string(),
        })?;

        Ok(super::LinkerResolution::new(linker_path, flavor)
            .with_note(format!("Using LLD linker: {}", linker_name)))
    }
}