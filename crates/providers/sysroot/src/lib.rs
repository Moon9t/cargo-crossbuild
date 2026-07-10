use std::path::PathBuf;

use crossbuild_core::{
    error::CrossBuildError,
    model::{BuildRequest, HostInfo, TargetTriple},
    provider::{SysrootProvider, SysrootResolution},
};

/// Zig-based sysroot provider.
pub struct ZigSysrootProvider;

impl SysrootProvider for ZigSysrootProvider {
    fn name(&self) -> &'static str {
        "zig"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn can_provide(&self, target: &TargetTriple, _host: &HostInfo) -> bool {
        !matches!(target.os, crossbuild_core::model::OperatingSystem::None)
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        _host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<SysrootResolution, CrossBuildError> {
        let _zig_path = which::which("zig").map_err(|_| CrossBuildError::ToolNotFound {
            tool: "zig".to_string(),
        })?;

        let sysroot = std::env::temp_dir()
            .join("zig-sysroot")
            .join(&target.triple);

        let resolution = SysrootResolution::new(sysroot)
            .with_note("Using zig's built-in libc/sysroot")
            .with_env("ZIG_SYSROOT", "1");

        Ok(resolution)
    }
}

/// No sysroot needed (wasm, bare metal).
pub struct NoSysrootProvider;

impl SysrootProvider for NoSysrootProvider {
    fn name(&self) -> &'static str {
        "none"
    }

    fn priority(&self) -> i32 {
        200
    }

    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool {
        target.triple == host.host_triple.triple || target.is_wasm() || target.is_bare_metal()
    }

    fn resolve(
        &self,
        _target: &TargetTriple,
        _host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<SysrootResolution, CrossBuildError> {
        Ok(SysrootResolution::new(PathBuf::new()).with_note("No sysroot required for this target"))
    }
}

/// Rustup-based sysroot provider.
pub struct RustupSysrootProvider;

impl SysrootProvider for RustupSysrootProvider {
    fn name(&self) -> &'static str {
        "rustup"
    }

    fn priority(&self) -> i32 {
        100
    }

    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool {
        if target.triple == host.host_triple.triple {
            return false;
        }
        rustup_target_available(target)
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<SysrootResolution, CrossBuildError> {
        if target.triple == host.host_triple.triple {
            return Err(CrossBuildError::SysrootNotNeeded);
        }

        let rustup_home = std::env::var("RUSTUP_HOME")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .map(|h| PathBuf::from(h).join(".rustup"))
            })
            .unwrap_or_else(|_| PathBuf::from("/rustup"));

        let toolchain = find_rustup_toolchain(&rustup_home.to_string_lossy())?;
        let sysroot = rustup_home
            .join("toolchains")
            .join(&toolchain)
            .join("lib")
            .join("rustlib")
            .join(&target.triple);

        if !sysroot.exists() {
            return Err(CrossBuildError::SysrootNotFound {
                target: target.triple.clone(),
            });
        }

        let mut resolution = SysrootResolution::new(sysroot.clone())
            .with_note(format!("Using rustup sysroot from toolchain: {toolchain}"))
            .with_env("CARGO_SYSROOT", sysroot.to_string_lossy())
            .with_builtin(true);

        let lib_dir = sysroot.join("lib");
        if lib_dir.exists() {
            resolution = resolution.with_env("LIBRARY_PATH", lib_dir.to_string_lossy());
        }

        Ok(resolution)
    }
}

fn rustup_target_available(target: &TargetTriple) -> bool {
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

fn find_rustup_toolchain(rustup_home: &str) -> Result<String, CrossBuildError> {
    let toolchains_dir = PathBuf::from(rustup_home).join("toolchains");
    if !toolchains_dir.exists() {
        return Err(CrossBuildError::SysrootNotFound {
            target: "rustup".to_string(),
        });
    }

    let default_file = PathBuf::from(rustup_home)
        .join("settings")
        .join("default-toolchain");
    if default_file.exists() {
        let content = std::fs::read_to_string(&default_file).map_err(|_| {
            CrossBuildError::SysrootNotFound {
                target: "rustup".to_string(),
            }
        })?;
        let toolchain = content.trim().to_string();
        if toolchains_dir.join(&toolchain).exists() {
            return Ok(toolchain);
        }
    }

    for entry in
        std::fs::read_dir(&toolchains_dir).map_err(|_| CrossBuildError::SysrootNotFound {
            target: "rustup".to_string(),
        })?
    {
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
