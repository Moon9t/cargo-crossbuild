//! Host and target platform detection, capability assessment, and toolchain routing.

use std::collections::BTreeMap;

use crate::model::{
    Abi, Architecture, HostDetectError, HostInfo, LinkerHint, OperatingSystem, SysrootHint,
    TargetFamily, TargetInfo, TargetSupport, TargetTriple, ToolchainHint,
};

/// Detects the host platform information.
pub fn detect_host() -> Result<HostInfo, HostDetectError> {
    HostInfo::detect()
}

/// Assesses target support tier and provides toolchain hints.
pub fn assess_target(target: &TargetTriple, host: &HostInfo) -> TargetInfo {
    let is_native = target.triple == host.host_triple.triple;
    let requires_cross = !is_native;

    let supported = determine_support_tier(target);
    let toolchain_hint = suggest_toolchain_provider(target, host, &supported);
    let sysroot_hint = suggest_sysroot_provider(target, host, &supported);
    let linker_hint = suggest_linker(target, host);

    TargetInfo {
        triple: target.clone(),
        is_native,
        requires_cross,
        supported,
        toolchain_hint,
        sysroot_hint,
        linker_hint,
    }
}

/// Determines the Rust support tier for a target.
fn determine_support_tier(target: &TargetTriple) -> TargetSupport {
    // Tier 1 targets (guaranteed to build and pass tests)
    const TIER1: &[&str] = &[
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
        "aarch64-unknown-linux-gnu",
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
    ];

    // Tier 2 targets (guaranteed to build, tests may not run)
    const TIER2: &[&str] = &[
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-musl",
        "x86_64-unknown-freebsd",
        "aarch64-unknown-freebsd",
        "x86_64-unknown-netbsd",
        "aarch64-unknown-netbsd",
        "x86_64-unknown-openbsd",
        "x86_64-unknown-illumos",
        "powerpc64le-unknown-linux-gnu",
        "s390x-unknown-linux-gnu",
        "riscv64gc-unknown-linux-gnu",
        "x86_64-pc-windows-gnu",
        "i686-pc-windows-msvc",
        "i686-pc-windows-gnu",
        "aarch64-pc-windows-msvc",
        "wasm32-wasi",
        "wasm32-unknown-unknown",
        "wasm32-unknown-emscripten",
    ];

    if TIER1.contains(&target.triple.as_str()) {
        TargetSupport::Tier1
    } else if TIER2.contains(&target.triple.as_str()) {
        TargetSupport::Tier2
    } else if is_known_tier3(target) {
        TargetSupport::Tier3
    } else {
        TargetSupport::Unsupported
    }
}

/// Checks if a target is a known tier 3 target.
fn is_known_tier3(target: &TargetTriple) -> bool {
    let triple = target.triple.as_str();

    // Known tier 3 patterns
    const TIER3_PATTERNS: &[&str] = &[
        "mips",
        "mips64",
        "sparc",
        "sparc64",
        "hexagon",
        "avr",
        "xtensa",
        "csky",
        "loongarch",
        "wasm64",
        "nvptx",
        "spirv",
        "bpf",
    ];

    TIER3_PATTERNS.iter().any(|p| triple.contains(p))
}

/// Suggests the best toolchain provider for a target.
fn suggest_toolchain_provider(
    target: &TargetTriple,
    host: &HostInfo,
    support: &TargetSupport,
) -> ToolchainHint {
    // Native builds use host toolchain
    if target.triple == host.host_triple.triple {
        return ToolchainHint::Rustup;
    }

    match support {
        TargetSupport::Tier1 | TargetSupport::Tier2 => {
            // For tier 1/2, prefer rustup if available
            if rustup_target_available(target) {
                ToolchainHint::Rustup
            } else if target.is_wasm() {
                ToolchainHint::Rustup
            } else if target.is_bare_metal() {
                ToolchainHint::Rustup
            } else {
                ToolchainHint::Zig
            }
        }
        TargetSupport::Tier3 => ToolchainHint::Zig,
        TargetSupport::Unsupported => ToolchainHint::Custom,
    }
}

/// Checks if a target is available via rustup.
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

/// Suggests the best sysroot provider for a target.
fn suggest_sysroot_provider(
    target: &TargetTriple,
    host: &HostInfo,
    support: &TargetSupport,
) -> SysrootHint {
    if target.triple == host.host_triple.triple {
        return SysrootHint::None;
    }

    match support {
        TargetSupport::Tier1 | TargetSupport::Tier2 => {
            if target.is_wasm() {
                SysrootHint::None // wasm doesn't need sysroot
            } else if rustup_target_available(target) {
                SysrootHint::Rustup
            } else if target.is_bare_metal() {
                SysrootHint::None
            } else {
                SysrootHint::Zig
            }
        }
        TargetSupport::Tier3 => SysrootHint::Zig,
        TargetSupport::Unsupported => SysrootHint::Custom,
    }
}

/// Suggests the best linker for a target.
fn suggest_linker(target: &TargetTriple, host: &HostInfo) -> LinkerHint {
    if target.triple == host.host_triple.triple {
        return LinkerHint::SystemDefault;
    }

    match (target.family(), target.os, target.abi) {
        // Windows targets
        (TargetFamily::Windows, _, Abi::Msvc) => LinkerHint::Msvc,
        (TargetFamily::Windows, _, Abi::Gnu) => LinkerHint::Lld,

        // Linux targets
        (TargetFamily::Linux, OperatingSystem::Linux, Abi::Gnu) => {
            if host.os == OperatingSystem::Linux {
                LinkerHint::SystemDefault
            } else {
                LinkerHint::Lld
            }
        }
        (TargetFamily::Linux, OperatingSystem::Linux, Abi::Musl) => LinkerHint::Lld,
        (TargetFamily::Linux, OperatingSystem::Android, _) => LinkerHint::Lld,

        // macOS targets
        (TargetFamily::MacOs, OperatingSystem::MacOs, _) => LinkerHint::Lld,
        (TargetFamily::MacOs, OperatingSystem::Ios | OperatingSystem::TvOs | OperatingSystem::WatchOs, _) => {
            LinkerHint::Lld
        }

        // WASM targets
        (TargetFamily::Wasm, _, _) => LinkerHint::Lld,

        // BSD targets
        (TargetFamily::Other, OperatingSystem::FreeBsd, _) => LinkerHint::Lld,
        (TargetFamily::Other, OperatingSystem::NetBsd, _) => LinkerHint::Lld,
        (TargetFamily::Other, OperatingSystem::OpenBsd, _) => LinkerHint::Lld,

        // Bare metal
        (TargetFamily::BareMetal, _, _) => LinkerHint::Lld,

        // Default to LLD for cross-compilation
        _ => LinkerHint::Lld,
    }
}

/// Capability matrix for host/target combinations.
#[derive(Debug, Clone)]
pub struct CapabilityMatrix {
    pub host: HostInfo,
    pub targets: BTreeMap<TargetTriple, TargetInfo>,
}

impl CapabilityMatrix {
    /// Creates a new capability matrix for the current host.
    pub fn new() -> Result<Self, HostDetectError> {
        let host = detect_host()?;
        Ok(Self {
            host,
            targets: BTreeMap::new(),
        })
    }

    /// Assesses a target and adds it to the matrix.
    pub fn assess(&mut self, target: &TargetTriple) -> &TargetInfo {
        let info = assess_target(target, &self.host);
        self.targets.insert(target.clone(), info);
        self.targets.get(target).unwrap()
    }

    /// Gets the assessment for a target, computing it if necessary.
    pub fn get(&mut self, target: &TargetTriple) -> &TargetInfo {
        if !self.targets.contains_key(target) {
            self.assess(target);
        }
        self.targets.get(target).unwrap()
    }

    /// Returns all assessed targets.
    pub fn all_targets(&self) -> &BTreeMap<TargetTriple, TargetInfo> {
        &self.targets
    }

    /// Checks if a target can be built natively.
    pub fn is_native(&self, target: &TargetTriple) -> bool {
        target.triple == self.host.host_triple.triple
    }

    /// Checks if cross-compilation is required.
    pub fn requires_cross(&self, target: &TargetTriple) -> bool {
        !self.is_native(target)
    }

    /// Gets the recommended toolchain provider for a target.
    pub fn toolchain_hint(&mut self, target: &TargetTriple) -> ToolchainHint {
        self.get(target).toolchain_hint
    }

    /// Gets the recommended sysroot provider for a target.
    pub fn sysroot_hint(&mut self, target: &TargetTriple) -> SysrootHint {
        self.get(target).sysroot_hint
    }

    /// Gets the recommended linker for a target.
    pub fn linker_hint(&mut self, target: &TargetTriple) -> LinkerHint {
        self.get(target).linker_hint
    }
}

impl Default for CapabilityMatrix {
    fn default() -> Self {
        Self::new().expect("failed to detect host")
    }
}

/// Known target triples organized by tier.
pub struct KnownTargets;

impl KnownTargets {
    /// Returns all tier 1 target triples.
    pub fn tier1() -> &'static [&'static str] {
        &[
            "x86_64-unknown-linux-gnu",
            "x86_64-pc-windows-msvc",
            "aarch64-unknown-linux-gnu",
            "aarch64-apple-darwin",
            "x86_64-apple-darwin",
        ]
    }

    /// Returns all tier 2 target triples.
    pub fn tier2() -> &'static [&'static str] {
        &[
            "x86_64-unknown-linux-musl",
            "aarch64-unknown-linux-musl",
            "x86_64-unknown-freebsd",
            "aarch64-unknown-freebsd",
            "x86_64-unknown-netbsd",
            "aarch64-unknown-netbsd",
            "x86_64-unknown-openbsd",
            "x86_64-unknown-illumos",
            "powerpc64le-unknown-linux-gnu",
            "s390x-unknown-linux-gnu",
            "riscv64gc-unknown-linux-gnu",
            "x86_64-pc-windows-gnu",
            "i686-pc-windows-msvc",
            "i686-pc-windows-gnu",
            "aarch64-pc-windows-msvc",
            "wasm32-wasi",
            "wasm32-unknown-unknown",
            "wasm32-unknown-emscripten",
        ]
    }

    /// Checks if a target is a known tier 1 target.
    pub fn is_tier1(target: &str) -> bool {
        Self::tier1().contains(&target)
    }

    /// Checks if a target is a known tier 2 target.
    pub fn is_tier2(target: &str) -> bool {
        Self::tier2().contains(&target)
    }

    /// Returns all known targets (tier 1 + tier 2).
    pub fn all_known() -> Vec<&'static str> {
        let mut targets = Vec::new();
        targets.extend_from_slice(Self::tier1());
        targets.extend_from_slice(Self::tier2());
        targets
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_host() {
        let host = detect_host().expect("host detection should work");
        assert!(!host.host_triple.triple.is_empty());
        assert!(host.rustc_version.is_some());
        assert!(host.cargo_version.is_some());
    }

    #[test]
    fn assesses_tier1_targets() {
        let host = detect_host().unwrap();
        let targets = KnownTargets::tier1();
        for t in targets {
            let target = TargetTriple::parse(t).unwrap();
            let info = assess_target(&target, &host);
            assert_eq!(info.supported, TargetSupport::Tier1);
        }
    }

    #[test]
    fn assesses_tier2_targets() {
        let host = detect_host().unwrap();
        let targets = KnownTargets::tier2();
        for t in targets {
            let target = TargetTriple::parse(t).unwrap();
            let info = assess_target(&target, &host);
            assert_eq!(info.supported, TargetSupport::Tier2);
        }
    }

    #[test]
    fn suggests_rustup_for_native() {
        let host = detect_host().unwrap();
        let target = TargetTriple::parse(&host.host_triple.triple).unwrap();
        let info = assess_target(&target, &host);
        assert!(info.is_native);
        assert!(!info.requires_cross);
        assert_eq!(info.toolchain_hint, ToolchainHint::Rustup);
    }

    #[test]
    fn suggests_linker_for_windows_msvc() {
        // Test suggest_linker directly with a non-Windows host
        let host = HostInfo {
            host_triple: TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap(),
            os: OperatingSystem::Linux,
            arch: Architecture::X86_64,
            rustc_version: None,
            cargo_version: None,
            target_dir: PathBuf::new(),
        };
        let target = TargetTriple::parse("x86_64-pc-windows-msvc").unwrap();
        let hint = suggest_linker(&target, &host);
        assert_eq!(hint, LinkerHint::Msvc);
    }

    #[test]
    fn suggests_lld_for_cross_linux() {
        let host = detect_host().unwrap();
        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        let info = assess_target(&target, &host);
        assert_eq!(info.linker_hint, LinkerHint::Lld);
    }

    #[test]
    fn capability_matrix_works() {
        let mut matrix = CapabilityMatrix::new().unwrap();
        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        let info = matrix.assess(&target);
        assert_eq!(info.supported, TargetSupport::Tier1);
    }
}