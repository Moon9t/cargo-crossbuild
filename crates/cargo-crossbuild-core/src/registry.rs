//! Provider registry for toolchain, sysroot, and linker providers.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::model::{
    Abi, Architecture, BuildRequest, HostInfo, LinkerFlavor, LinkerHint, OperatingSystem,
    TargetFamily, TargetTriple, ToolchainHint,
};
use crate::error::CrossBuildError;

use crate::provider::{LinkerProvider, LinkerResolution, SysrootProvider, SysrootResolution, ToolchainProvider, ToolchainResolution};

/// Registry of all providers.
pub struct ProviderRegistry {
    toolchain_providers: Vec<Arc<dyn ToolchainProvider>>,
    sysroot_providers: Vec<Arc<dyn SysrootProvider>>,
    linker_providers: Vec<Arc<dyn LinkerProvider>>,
}

impl ProviderRegistry {
    /// Creates a new registry with default providers.
    pub fn new() -> Self {
        let mut registry = Self {
            toolchain_providers: Vec::new(),
            sysroot_providers: Vec::new(),
            linker_providers: Vec::new(),
        };

        registry.register_default_providers();
        registry
    }

    /// Registers the default set of providers.
    fn register_default_providers(&mut self) {
        // Toolchain providers (highest priority first)
        self.register_toolchain(Arc::new(crate::toolchain::BuiltinToolchainProvider));
        self.register_toolchain(Arc::new(crate::toolchain::RustupToolchainProvider));
        self.register_toolchain(Arc::new(crate::toolchain::ZigToolchainProvider));

        // Sysroot providers
        self.register_sysroot(Arc::new(crate::toolchain::NoSysrootProvider));
        self.register_sysroot(Arc::new(crate::toolchain::RustupSysrootProvider));
        self.register_sysroot(Arc::new(crate::toolchain::ZigSysrootProvider));

        // Linker providers
        self.register_linker(Arc::new(crate::toolchain::MsvcLinkerProvider));
        self.register_linker(Arc::new(crate::toolchain::MoldLinkerProvider));
        self.register_linker(Arc::new(crate::toolchain::LldLinkerProvider));
        self.register_linker(Arc::new(crate::toolchain::ZigLinkerProvider));
        self.register_linker(Arc::new(crate::toolchain::SystemLinkerProvider));
    }

    /// Registers a toolchain provider.
    pub fn register_toolchain(&mut self, provider: Arc<dyn ToolchainProvider>) {
        self.toolchain_providers.push(provider);
        self.toolchain_providers.sort_by_key(|p| -p.priority());
    }

    /// Registers a sysroot provider.
    pub fn register_sysroot(&mut self, provider: Arc<dyn SysrootProvider>) {
        self.sysroot_providers.push(provider);
        self.sysroot_providers.sort_by_key(|p| -p.priority());
    }

    /// Registers a linker provider.
    pub fn register_linker(&mut self, provider: Arc<dyn LinkerProvider>) {
        self.linker_providers.push(provider);
        self.linker_providers.sort_by_key(|p| -p.priority());
    }

    /// Resolves the best toolchain provider for a target.
    pub fn resolve_toolchain(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        request: &BuildRequest,
    ) -> Result<ToolchainResolution, CrossBuildError> {
        for provider in &self.toolchain_providers {
            if provider.can_provide(target, host) {
                return provider.resolve(target, host, request);
            }
        }

        Err(CrossBuildError::ProviderNotFound {
            provider_type: "toolchain".to_string(),
            target: target.triple.clone(),
        })
    }

    /// Resolves the best sysroot provider for a target.
    pub fn resolve_sysroot(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        request: &BuildRequest,
    ) -> Result<SysrootResolution, CrossBuildError> {
        for provider in &self.sysroot_providers {
            if provider.can_provide(target, host) {
                return provider.resolve(target, host, request);
            }
        }

        Err(CrossBuildError::ProviderNotFound {
            provider_type: "sysroot".to_string(),
            target: target.triple.clone(),
        })
    }

    /// Resolves the best linker provider for a target.
    pub fn resolve_linker(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        request: &BuildRequest,
    ) -> Result<LinkerResolution, CrossBuildError> {
        for provider in &self.linker_providers {
            if provider.can_provide(target, host) {
                return provider.resolve(target, host, request);
            }
        }

        Err(CrossBuildError::ProviderNotFound {
            provider_type: "linker".to_string(),
            target: target.triple.clone(),
        })
    }

    /// Returns names of registered toolchain providers.
    pub fn toolchain_provider_names(&self) -> Vec<&'static str> {
        self.toolchain_providers.iter().map(|p| p.name()).collect()
    }

    /// Returns names of registered sysroot providers.
    pub fn sysroot_provider_names(&self) -> Vec<&'static str> {
        self.sysroot_providers.iter().map(|p| p.name()).collect()
    }

    /// Returns names of registered linker providers.
    pub fn linker_provider_names(&self) -> Vec<&'static str> {
        self.linker_providers.iter().map(|p| p.name()).collect()
    }

    /// Gets a toolchain provider by name.
    pub fn get_toolchain_provider(&self, name: &str) -> Option<&Arc<dyn ToolchainProvider>> {
        self.toolchain_providers.iter().find(|p| p.name() == name)
    }

    /// Gets a sysroot provider by name.
    pub fn get_sysroot_provider(&self, name: &str) -> Option<&Arc<dyn SysrootProvider>> {
        self.sysroot_providers.iter().find(|p| p.name() == name)
    }

    /// Gets a linker provider by name.
    pub fn get_linker_provider(&self, name: &str) -> Option<&Arc<dyn LinkerProvider>> {
        self.linker_providers.iter().find(|p| p.name() == name)
    }

    /// Resolves all providers for a target and returns a complete resolution.
    pub fn resolve_all(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        request: &BuildRequest,
    ) -> Result<CompleteResolution, CrossBuildError> {
        let toolchain = self.resolve_toolchain(target, host, request)?;
        let sysroot = self.resolve_sysroot(target, host, request).ok();
        let linker = self.resolve_linker(target, host, request)?;

        Ok(CompleteResolution {
            toolchain,
            sysroot,
            linker,
        })
    }
}

/// Complete resolution from all providers.
#[derive(Debug, Clone)]
pub struct CompleteResolution {
    pub toolchain: ToolchainResolution,
    pub sysroot: Option<SysrootResolution>,
    pub linker: LinkerResolution,
}

impl CompleteResolution {
    /// Merges all environment variables from all resolutions.
    pub fn merged_env(&self) -> BTreeMap<String, String> {
        let mut env = BTreeMap::new();
        env.extend(self.toolchain.env.clone());
        if let Some(sysroot) = &self.sysroot {
            env.extend(sysroot.env.clone());
        }
        env.extend(self.linker.env.clone());
        env
    }

    /// Merges all cargo config snippets.
    pub fn merged_cargo_config(&self) -> Option<toml::Table> {
        let mut merged = toml::Table::new();

        if let Some(config) = &self.toolchain.cargo_config {
            merged.extend(config.clone());
        }
        if let Some(sysroot) = &self.sysroot {
            if let Some(config) = &sysroot.cargo_config {
                merged.extend(config.clone());
            }
        }
        if let Some(config) = &self.linker.cargo_config {
            merged.extend(config.clone());
        }

        if merged.is_empty() {
            None
        } else {
            Some(merged)
        }
    }

    /// Collects all notes from all resolutions.
    pub fn all_notes(&self) -> Vec<String> {
        let mut notes = Vec::new();
        notes.extend(self.toolchain.notes.clone());
        if let Some(sysroot) = &self.sysroot {
            notes.extend(sysroot.notes.clone());
        }
        notes.extend(self.linker.notes.clone());
        notes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TargetTriple;

    #[test]
    fn registry_creation() {
        let registry = ProviderRegistry::new();
        assert!(!registry.toolchain_provider_names().is_empty());
        assert!(!registry.sysroot_provider_names().is_empty());
        assert!(!registry.linker_provider_names().is_empty());
    }

    #[test]
    fn registry_has_expected_providers() {
        let registry = ProviderRegistry::new();
        let toolchains = registry.toolchain_provider_names();
        assert!(toolchains.contains(&"builtin"));
        assert!(toolchains.contains(&"rustup"));
        assert!(toolchains.contains(&"zig"));

        let linkers = registry.linker_provider_names();
        assert!(linkers.contains(&"system"));
        assert!(linkers.contains(&"lld"));
        assert!(linkers.contains(&"mold"));
    }
}