//! Provider implementations for toolchain, sysroot, and linker resolution.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::model::{
    Abi, Architecture, BuildRequest, HostInfo, LinkerFlavor, LinkerHint, OperatingSystem,
    SysrootHint, TargetFamily, TargetTriple, ToolchainHint,
};
use crate::error::CrossBuildError;

/// A toolchain provider supplies the compiler toolchain for a target.
pub trait ToolchainProvider: Send + Sync {
    /// Returns the unique name of this provider.
    fn name(&self) -> &'static str;

    /// Returns the priority of this provider (higher = preferred).
    fn priority(&self) -> i32 {
        0
    }

    /// Checks if this provider can handle the given target on this host.
    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool;

    /// Resolves the toolchain for the target, returning environment variables
    /// and configuration needed to use it.
    fn resolve(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        request: &BuildRequest,
    ) -> Result<ToolchainResolution, CrossBuildError>;

    /// Returns the toolchain hint this provider satisfies.
    fn hint(&self) -> ToolchainHint;
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
}

impl Default for ToolchainResolution {
    fn default() -> Self {
        Self::new()
    }
}

/// A sysroot provider supplies the target sysroot (libc, libstd, crt objects).
pub trait SysrootProvider: Send + Sync {
    /// Returns the unique name of this provider.
    fn name(&self) -> &'static str;

    /// Returns the priority of this provider (higher = preferred).
    fn priority(&self) -> i32 {
        0
    }

    /// Checks if this provider can handle the given target on this host.
    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool;

    /// Resolves the sysroot for the target.
    fn resolve(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        request: &BuildRequest,
    ) -> Result<SysrootResolution, CrossBuildError>;
}

/// Resolution result from a sysroot provider.
#[derive(Debug, Clone, PartialEq)]
pub struct SysrootResolution {
    pub sysroot_path: PathBuf,
    pub env: BTreeMap<String, String>,
    pub cargo_config: Option<toml::Table>,
    pub notes: Vec<String>,
    pub is_builtin: bool,
}

impl SysrootResolution {
    pub fn new(sysroot_path: PathBuf) -> Self {
        Self {
            sysroot_path,
            env: BTreeMap::new(),
            cargo_config: None,
            notes: Vec::new(),
            is_builtin: false,
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

    pub fn with_builtin(mut self, builtin: bool) -> Self {
        self.is_builtin = builtin;
        self
    }
}

/// A linker provider supplies the appropriate linker for a target.
pub trait LinkerProvider: Send + Sync {
    /// Returns the unique name of this provider.
    fn name(&self) -> &'static str;

    /// Returns the priority of this provider (higher = preferred).
    fn priority(&self) -> i32 {
        0
    }

    /// Checks if this provider can handle the given target on this host.
    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool;

    /// Resolves the linker for the target.
    fn resolve(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        request: &BuildRequest,
    ) -> Result<LinkerResolution, CrossBuildError>;
}

/// Resolution result from a linker provider.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkerResolution {
    pub linker_path: PathBuf,
    pub linker_args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cargo_config: Option<toml::Table>,
    pub notes: Vec<String>,
    pub flavor: LinkerFlavor,
}

impl LinkerResolution {
    pub fn new(linker_path: PathBuf, flavor: LinkerFlavor) -> Self {
        Self {
            linker_path,
            linker_args: Vec::new(),
            env: BTreeMap::new(),
            cargo_config: None,
            notes: Vec::new(),
            flavor,
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.linker_args = args;
        self
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
}

/// Provider registry that manages all providers.
pub struct ProviderRegistry {
    toolchain_providers: Vec<Box<dyn ToolchainProvider>>,
    sysroot_providers: Vec<Box<dyn SysrootProvider>>,
    linker_providers: Vec<Box<dyn LinkerProvider>>,
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
        self.register_toolchain(Box::new(crate::toolchain::BuiltinToolchainProvider));
        self.register_toolchain(Box::new(crate::toolchain::RustupToolchainProvider));
        self.register_toolchain(Box::new(crate::toolchain::ZigToolchainProvider));

        // Sysroot providers
        self.register_sysroot(Box::new(crate::toolchain::NoSysrootProvider));
        self.register_sysroot(Box::new(crate::toolchain::RustupSysrootProvider));
        self.register_sysroot(Box::new(crate::toolchain::ZigSysrootProvider));

        // Linker providers
        self.register_linker(Box::new(crate::toolchain::MsvcLinkerProvider));
        self.register_linker(Box::new(crate::toolchain::MoldLinkerProvider));
        self.register_linker(Box::new(crate::toolchain::LldLinkerProvider));
        self.register_linker(Box::new(crate::toolchain::ZigLinkerProvider));
        self.register_linker(Box::new(crate::toolchain::SystemLinkerProvider));
    }

    /// Registers a toolchain provider.
    pub fn register_toolchain(&mut self, provider: Box<dyn ToolchainProvider>) {
        self.toolchain_providers.push(provider);
        self.toolchain_providers.sort_by_key(|p| -p.priority());
    }

    /// Registers a sysroot provider.
    pub fn register_sysroot(&mut self, provider: Box<dyn SysrootProvider>) {
        self.sysroot_providers.push(provider);
        self.sysroot_providers.sort_by_key(|p| -p.priority());
    }

    /// Registers a linker provider.
    pub fn register_linker(&mut self, provider: Box<dyn LinkerProvider>) {
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
    pub fn get_toolchain_provider(&self, name: &str) -> Option<&Box<dyn ToolchainProvider>> {
        self.toolchain_providers.iter().find(|p| p.name() == name)
    }

    /// Gets a sysroot provider by name.
    pub fn get_sysroot_provider(&self, name: &str) -> Option<&Box<dyn SysrootProvider>> {
        self.sysroot_providers.iter().find(|p| p.name() == name)
    }

    /// Gets a linker provider by name.
    pub fn get_linker_provider(&self, name: &str) -> Option<&Box<dyn LinkerProvider>> {
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

// Re-export the provider types for the registry
pub use crate::toolchain::{
    BuiltinToolchainProvider, LldLinkerProvider, MoldLinkerProvider, MsvcLinkerProvider,
    NoSysrootProvider, RustupSysrootProvider, RustupToolchainProvider, SystemLinkerProvider,
    ZigLinkerProvider, ZigSysrootProvider, ZigToolchainProvider,
};

pub fn rustup_target_available(target: &TargetTriple) -> bool {
    // Check common rustup targets
    const RUSTUP_TARGETS: &[&str] = &[
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-gnu",
        "aarch64-unknown-linux-musl",
        "x86_64-pc-windows-msvc",
        "x86_64-pc-windows-gnu",
        "aarch64-pc-windows-msvc",
        "i686-pc-windows-msvc",
        "i686-pc-windows-gnu",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "wasm32-wasi",
        "wasm32-unknown-unknown",
        "wasm32-unknown-emscripten",
        "x86_64-unknown-freebsd",
        "aarch64-unknown-freebsd",
        "powerpc64le-unknown-linux-gnu",
        "s390x-unknown-linux-gnu",
        "riscv64gc-unknown-linux-gnu",
    ];

    RUSTUP_TARGETS.contains(&target.triple.as_str())
}

pub fn find_rustup_toolchain(rustup_home: &str) -> Result<String, CrossBuildError> {
    let toolchains_dir = PathBuf::from(rustup_home).join("toolchains");
    if !toolchains_dir.exists() {
        return Err(CrossBuildError::SysrootNotFound {
            target: "rustup".to_string(),
        });
    }

    // Read the default toolchain
    let default_file = PathBuf::from(rustup_home).join("settings").join("default-toolchain");
    if default_file.exists() {
        let content = std::fs::read_to_string(&default_file)
            .map_err(|_| CrossBuildError::SysrootNotFound {
                target: "rustup".to_string(),
            })?;
        let toolchain = content.trim().to_string();
        if toolchains_dir.join(&toolchain).exists() {
            return Ok(toolchain);
        }
    }

    // Fallback: find first stable toolchain
    for entry in std::fs::read_dir(&toolchains_dir).map_err(|_| CrossBuildError::SysrootNotFound {
        target: "rustup".to_string(),
    })? {
        let entry = entry.map_err(|_| CrossBuildError::SysrootNotFound {
            target: "rustup".to_string(),
        })?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.contains("stable") || name.contains("1.") {
            return Ok(name);
        }
    }

    Err(CrossBuildError::SysrootNotFound {
        target: "rustup".to_string(),
    })
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