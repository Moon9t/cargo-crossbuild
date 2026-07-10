//! Cargo configuration generation for cross-compilation.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::model::{BuildRequest, TargetTriple};
use crate::error::CrossBuildError;

/// Generates a .cargo/config.toml for cross-compilation.
pub struct CargoConfigGenerator {
    config: toml::Table,
    target: TargetTriple,
    workspace_root: PathBuf,
}

impl CargoConfigGenerator {
    /// Creates a new generator for the given target.
    pub fn new(target: TargetTriple, workspace_root: PathBuf) -> Self {
        Self {
            config: toml::Table::new(),
            target,
            workspace_root,
        }
    }

    /// Sets the linker for the target.
    pub fn with_linker(mut self, linker_path: impl Into<PathBuf>, flavor: &str, args: Vec<String>) -> Self {
        let target_key = format!("target.{}", self.target.as_str());
        let mut target_table = self.get_or_create_target_table(&target_key);

        target_table.insert("linker".to_string(), toml::Value::String(linker_path.into().to_string_lossy().into_owned()));
        target_table.insert("linker-flavor".to_string(), toml::Value::String(flavor.to_string()));

        if !args.is_empty() {
            target_table.insert("linker-args".to_string(), toml::Value::Array(
                args.into_iter().map(toml::Value::String).collect()
            ));
        }

        self
    }

    /// Sets the runner for the target (for running tests).
    pub fn with_runner(mut self, runner: impl Into<String>) -> Self {
        let target_key = format!("target.{}", self.target.as_str());
        let mut target_table = self.get_or_create_target_table(&target_key);
        target_table.insert("runner".to_string(), toml::Value::String(runner.into()));
        self
    }

    /// Sets custom rustflags for the target.
    pub fn with_rustflags(mut self, flags: Vec<String>) -> Self {
        let target_key = format!("target.{}", self.target.as_str());
        let mut target_table = self.get_or_create_target_table(&target_key);
        target_table.insert("rustflags".to_string(), toml::Value::Array(
            flags.into_iter().map(toml::Value::String).collect()
        ));
        self
    }

    /// Sets environment variables for the target.
    pub fn with_env(mut self, env: BTreeMap<String, String>) -> Self {
        if env.is_empty() {
            return self;
        }

        let target_key = format!("target.{}", self.target.as_str());
        let mut target_table = self.get_or_create_target_table(&target_key);
        let mut env_table = toml::Table::new();
        for (k, v) in env {
            env_table.insert(k, toml::Value::String(v));
        }
        target_table.insert("env".to_string(), toml::Value::Table(env_table));
        self
    }

    /// Sets the sysroot for the target.
    pub fn with_sysroot(mut self, sysroot: impl Into<PathBuf>) -> Self {
        let target_key = format!("target.{}", self.target.as_str());
        let mut target_table = self.get_or_create_target_table(&target_key);
        target_table.insert("sysroot".to_string(), toml::Value::String(sysroot.into().to_string_lossy().into_owned()));
        self
    }

    /// Adds a custom [target.xxx] section.
    pub fn with_custom_section(mut self, key: String, value: toml::Value) -> Self {
        let target_key = format!("target.{}", self.target.as_str());
        let mut target_table = self.get_or_create_target_table(&target_key);
        target_table.insert(key, value);
        self
    }

    /// Sets the build target.
    pub fn with_build_target(mut self, target: &TargetTriple) -> Self {
        let mut build_table = toml::Table::new();
        build_table.insert("target".to_string(), toml::Value::String(target.as_str().to_string()));
        self.config.insert("build".to_string(), toml::Value::Table(build_table));
        self
    }

    /// Gets or creates a target table.
    fn get_or_create_target_table(&mut self, key: &str) -> &mut toml::Table {
        self.config
            .entry(key.to_string())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .expect("target entry should be a table")
    }

    /// Builds the final config.
    pub fn build(self) -> toml::Table {
        self.config
    }

    /// Writes the config to .cargo/config.toml in the workspace root.
    pub fn write(self) -> Result<PathBuf, CrossBuildError> {
        let cargo_dir = self.workspace_root.join(".cargo");
        std::fs::create_dir_all(&cargo_dir).map_err(|e| CrossBuildError::Io {
            path: Some(cargo_dir.clone()),
            source: e,
        })?;

        let config_path = cargo_dir.join("config.toml");
        let config_str = toml::to_string_pretty(&self.config)
            .map_err(|e| CrossBuildError::configuration(e.to_string()))?;

        std::fs::write(&config_path, config_str).map_err(|e| CrossBuildError::Io {
            path: Some(config_path.clone()),
            source: e,
        })?;

        Ok(config_path)
    }

    /// Writes the config to a specific path.
    pub fn write_to(self, path: impl AsRef<Path>) -> Result<(), CrossBuildError> {
        let config_str = toml::to_string_pretty(&self.config)
            .map_err(|e| CrossBuildError::configuration(e.to_string()))?;

        let path_ref = path.as_ref();
        std::fs::write(path_ref, config_str).map_err(|e| CrossBuildError::Io {
            path: Some(path_ref.to_path_buf()),
            source: e,
        })?;

        Ok(())
    }
}

/// Merges multiple cargo config snippets.
pub fn merge_cargo_configs(configs: Vec<toml::Table>) -> toml::Table {
    let mut merged = toml::Table::new();

    for config in configs {
        for (key, value) in config {
            if let Some(existing) = merged.get_mut(&key) {
                merge_values(existing, value);
            } else {
                merged.insert(key, value);
            }
        }
    }

    merged
}

/// Merges two toml values.
fn merge_values(existing: &mut toml::Value, new: toml::Value) {
    match (existing, new) {
        (toml::Value::Table(existing_table), toml::Value::Table(new_table)) => {
            for (key, value) in new_table {
                if let Some(existing_value) = existing_table.get_mut(&key) {
                    merge_values(existing_value, value);
                } else {
                    existing_table.insert(key, value);
                }
            }
        }
        (existing, new) => {
            *existing = new;
        }
    }
}

/// Creates a standard cross-compilation cargo config.
pub fn create_cross_config(
    target: &TargetTriple,
    linker_path: Option<PathBuf>,
    sysroot: Option<PathBuf>,
    runner: Option<String>,
    rustflags: Vec<String>,
    env: BTreeMap<String, String>,
    workspace_root: &Path,
) -> Result<toml::Table, CrossBuildError> {
    let mut generator = CargoConfigGenerator::new(target.clone(), workspace_root.to_path_buf());

    if let Some(linker) = linker_path {
        let flavor = match target.family() {
            crate::model::TargetFamily::Windows => "msvc",
            crate::model::TargetFamily::Wasm => "wasm-ld",
            crate::model::TargetFamily::MacOs => "ld64",
            _ => "gcc",
        };
        generator = generator.with_linker(linker, flavor, Vec::new());
    }

    if let Some(sysroot) = sysroot {
        generator = generator.with_sysroot(sysroot);
    }

    if let Some(runner) = runner {
        generator = generator.with_runner(runner);
    }

    if !rustflags.is_empty() {
        generator = generator.with_rustflags(rustflags);
    }

    if !env.is_empty() {
        generator = generator.with_env(env);
    }

    generator = generator.with_build_target(target);

    Ok(generator.build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TargetTriple;
    use std::path::PathBuf;

    #[test]
    fn generates_basic_config() {
        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        let config = CargoConfigGenerator::new(target, PathBuf::from("/tmp"))
            .with_linker("/usr/bin/ld.lld", "gcc", vec![])
            .with_sysroot("/opt/sysroot")
            .build();

        assert!(config.contains_key("target.x86_64-unknown-linux-gnu"));
    }

    #[test]
    fn generates_runner_config() {
        let target = TargetTriple::parse("wasm32-wasi").unwrap();
        let config = CargoConfigGenerator::new(target, PathBuf::from("/tmp"))
            .with_runner("wasmtime")
            .build();

        let target_table = config.get("target.wasm32-wasi").unwrap().as_table().unwrap();
        assert_eq!(target_table.get("runner").unwrap().as_str().unwrap(), "wasmtime");
    }

    #[test]
    fn merges_configs() {
        let mut config1 = toml::Table::new();
        let mut target1 = toml::Table::new();
        target1.insert("linker".to_string(), toml::Value::String("ld.lld".to_string()));
        config1.insert("target.x86_64-unknown-linux-gnu".to_string(), toml::Value::Table(target1));

        let mut config2 = toml::Table::new();
        let mut target2 = toml::Table::new();
        target2.insert("runner".to_string(), toml::Value::String("qemu".to_string()));
        config2.insert("target.x86_64-unknown-linux-gnu".to_string(), toml::Value::Table(target2));

        let merged = merge_cargo_configs(vec![config1, config2]);
        let target = merged.get("target.x86_64-unknown-linux-gnu").unwrap().as_table().unwrap();
        assert_eq!(target.get("linker").unwrap().as_str().unwrap(), "ld.lld");
        assert_eq!(target.get("runner").unwrap().as_str().unwrap(), "qemu");
    }
}