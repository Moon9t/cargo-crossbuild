use std::sync::Arc;

use crossbuild_core::error::CrossBuildError;
use crossbuild_core::model::{BuildRequest, HostInfo, TargetTriple};
use crossbuild_core::provider::{
    LinkerProvider, LinkerResolution, SysrootProvider, SysrootResolution, ToolchainProvider,
    ToolchainResolution,
};

use crossbuild_provider_clang::ClangToolchainProvider;
use crossbuild_provider_gcc::{
    LldLinkerProvider, MoldLinkerProvider, MsvcLinkerProvider, SystemLinkerProvider,
    ZigLinkerProvider,
};
use crossbuild_provider_sysroot::{NoSysrootProvider, RustupSysrootProvider, ZigSysrootProvider};
use crossbuild_provider_zig::{
    BuiltinToolchainProvider, RustupToolchainProvider, ZigToolchainProvider,
};

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
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_default_providers();
        registry
    }

    fn register_default_providers(&mut self) {
        self.register_toolchain(Box::new(BuiltinToolchainProvider));
        self.register_toolchain(Box::new(RustupToolchainProvider));
        self.register_toolchain(Box::new(ZigToolchainProvider));
        self.register_toolchain(Box::new(ClangToolchainProvider));

        self.register_sysroot(Box::new(NoSysrootProvider));
        self.register_sysroot(Box::new(RustupSysrootProvider));
        self.register_sysroot(Box::new(ZigSysrootProvider));

        self.register_linker(Box::new(MsvcLinkerProvider));
        self.register_linker(Box::new(MoldLinkerProvider));
        self.register_linker(Box::new(LldLinkerProvider));
        self.register_linker(Box::new(ZigLinkerProvider));
        self.register_linker(Box::new(SystemLinkerProvider));
    }

    pub fn register_toolchain(&mut self, provider: Box<dyn ToolchainProvider>) {
        self.toolchain_providers.push(Arc::from(provider));
        self.toolchain_providers.sort_by_key(|p| -p.priority());
    }

    pub fn register_sysroot(&mut self, provider: Box<dyn SysrootProvider>) {
        self.sysroot_providers.push(Arc::from(provider));
        self.sysroot_providers.sort_by_key(|p| -p.priority());
    }

    pub fn register_linker(&mut self, provider: Box<dyn LinkerProvider>) {
        self.linker_providers.push(Arc::from(provider));
        self.linker_providers.sort_by_key(|p| -p.priority());
    }

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

    pub fn resolve_sysroot(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        request: &BuildRequest,
    ) -> Result<Option<SysrootResolution>, CrossBuildError> {
        for provider in &self.sysroot_providers {
            if provider.can_provide(target, host) {
                let res = provider.resolve(target, host, request)?;
                return Ok(Some(res));
            }
        }
        Ok(None)
    }

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

    pub fn toolchain_provider_names(&self) -> Vec<&str> {
        self.toolchain_providers.iter().map(|p| p.name()).collect()
    }

    pub fn sysroot_provider_names(&self) -> Vec<&str> {
        self.sysroot_providers.iter().map(|p| p.name()).collect()
    }

    pub fn linker_provider_names(&self) -> Vec<&str> {
        self.linker_providers.iter().map(|p| p.name()).collect()
    }

    pub fn get_toolchain_provider(&self, name: &str) -> Option<&Arc<dyn ToolchainProvider>> {
        self.toolchain_providers.iter().find(|p| p.name() == name)
    }

    pub fn get_sysroot_provider(&self, name: &str) -> Option<&Arc<dyn SysrootProvider>> {
        self.sysroot_providers.iter().find(|p| p.name() == name)
    }

    pub fn get_linker_provider(&self, name: &str) -> Option<&Arc<dyn LinkerProvider>> {
        self.linker_providers.iter().find(|p| p.name() == name)
    }

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

#[derive(Debug, Clone)]
pub struct CompleteResolution {
    pub toolchain: ToolchainResolution,
    pub sysroot: Option<SysrootResolution>,
    pub linker: LinkerResolution,
}

#[cfg(test)]
mod tests {
    use super::*;

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
