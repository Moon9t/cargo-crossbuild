use std::path::PathBuf;

use cargo_crossbuild_core::{
    CachePolicy, DownloadRequest, ValidationPlan, WrapperPlan,
    ReleasePlan, PackageManagerPlan,
    model::LockfileSnapshot,
    model::ChecksumAlgorithm,
    TargetTriple,
};

#[test]
fn cache_policy_derives_stable_keys() {
    let policy = CachePolicy::new("target/crossbuild-cache");
    let workspace_root = PathBuf::from("C:/workspace");

    assert_eq!(
        policy.absolute_root(&workspace_root),
        workspace_root.join("target/crossbuild-cache")
    );
    let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
    assert_eq!(policy.cache_key(&workspace_root, &target), "C:_workspace::x86_64-unknown-linux-gnu");
}

#[test]
fn download_request_records_provenance() {
    let dest = PathBuf::from("/tmp/test");
    let request = DownloadRequest::with_checksum(
        "https://example.com/toolchain.tar.gz",
        dest,
        Some("sha256:abc123".to_string()),
        ChecksumAlgorithm::Sha256,
    );

    assert!(request.is_verified());
    assert_eq!(
        request.provenance_label(),
        "https://example.com/toolchain.tar.gz (sha256:abc123)"
    );
}

#[test]
fn release_and_install_helpers_are_deterministic() {
    let release = ReleasePlan::new("1.2.3-alpha.1");
    let install = cargo_crossbuild_core::InstallPlan::new("bin");
    let wrapper = WrapperPlan::new("cargo", "build");
    let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
    let validation = ValidationPlan::new("release-validation", target);
    let package_manager = PackageManagerPlan::new("cargo");
    let lockfile = LockfileSnapshot::new("x86_64-apple-darwin", "Cargo.toml");

    assert!(release.is_prerelease());
    assert_eq!(release.tag_name(), "v1.2.3-alpha.1");
    assert_eq!(
        install.resolved_destination(&PathBuf::from("C:/workspace")),
        PathBuf::from("C:/workspace/bin")
    );
    assert_eq!(wrapper.invocation(), "cargo build");
    assert!(validation.requires_release_mode());
    assert_eq!(package_manager.command_name(), "cargo");
    assert_eq!(lockfile.cache_key(), "x86_64-apple-darwin::Cargo.toml");
}