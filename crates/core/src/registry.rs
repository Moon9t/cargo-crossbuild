use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::CrossBuildError;
use crate::model::{BuildRequest, HostInfo, TargetTriple};
use crate::provider::{
    LinkerProvider, LinkerResolution, SysrootProvider, SysrootResolution, ToolchainProvider,
    ToolchainResolution,
};

/// Registry of all providers.
#[derive(Default)]
pub struct ProviderRegistry {
    toolchain_providers: Vec<Arc<dyn ToolchainProvider>>,
    sysroot_providers: Vec<Arc<dyn SysrootProvider>>,
    linker_providers: Vec<Arc<dyn LinkerProvider>>,
}

impl std::fmt::Debug for ProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderRegistry")
            .field("toolchain_providers", &self.toolchain_provider_names())
            .field("sysroot_providers", &self.sysroot_provider_names())
            .field("linker_providers", &self.linker_provider_names())
            .finish()
    }
}

impl ProviderRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            toolchain_providers: Vec::new(),
            sysroot_providers: Vec::new(),
            linker_providers: Vec::new(),
        }
    }

    /// Registers a toolchain provider.
    pub fn register_toolchain(&mut self, provider: Box<dyn ToolchainProvider>) {
        self.toolchain_providers.push(Arc::from(provider));
        self.toolchain_providers.sort_by_key(|p| -p.priority());
    }

    /// Registers a sysroot provider.
    pub fn register_sysroot(&mut self, provider: Box<dyn SysrootProvider>) {
        self.sysroot_providers.push(Arc::from(provider));
        self.sysroot_providers.sort_by_key(|p| -p.priority());
    }

    /// Registers a linker provider.
    pub fn register_linker(&mut self, provider: Box<dyn LinkerProvider>) {
        self.linker_providers.push(Arc::from(provider));
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
    ) -> Result<Option<SysrootResolution>, CrossBuildError> {
        for provider in &self.sysroot_providers {
            if provider.can_provide(target, host) {
                let res = provider.resolve(target, host, request);
                match res {
                    Ok(resolution) => return Ok(Some(resolution)),
                    Err(CrossBuildError::SysrootNotNeeded) => return Ok(None),
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(None)
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
    pub fn toolchain_provider_names(&self) -> Vec<&str> {
        self.toolchain_providers.iter().map(|p| p.name()).collect()
    }

    /// Returns names of registered sysroot providers.
    pub fn sysroot_provider_names(&self) -> Vec<&str> {
        self.sysroot_providers.iter().map(|p| p.name()).collect()
    }

    /// Returns names of registered linker providers.
    pub fn linker_provider_names(&self) -> Vec<&str> {
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
        let sysroot = self.resolve_sysroot(target, host, request)?;
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
    /// Returns the merged environment variables from all providers.
    pub fn merged_env(&self) -> BTreeMap<String, String> {
        let mut env = BTreeMap::new();
        env.extend(self.toolchain.env.clone());
        env.extend(self.linker.env.clone());
        if let Some(ref sysroot) = self.sysroot {
            env.extend(sysroot.env.clone());
        }
        env
    }
}
