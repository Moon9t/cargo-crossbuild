//! Linker configuration and resolution.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::{LinkerFlavor, TargetFamily, TargetTriple};

/// Linker configuration for a target.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkerConfig {
    pub target: TargetTriple,
    pub linker_path: PathBuf,
    pub flavor: LinkerFlavor,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cargo_config: Option<toml::Table>,
}

impl LinkerConfig {
    pub fn new(target: TargetTriple, linker_path: PathBuf, flavor: LinkerFlavor) -> Self {
        Self {
            target,
            linker_path,
            flavor,
            args: Vec::new(),
            env: BTreeMap::new(),
            cargo_config: None,
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
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

    /// Generates the cargo config snippet for this linker.
    pub fn cargo_config_snippet(&self) -> toml::Table {
        let mut table = toml::Table::new();
        let target_key = format!("target.{}", self.target.as_str());
        let mut target_table = toml::Table::new();
        target_table.insert("linker".to_string(), toml::Value::String(self.linker_path.to_string_lossy().into_owned()));
        
        if !self.args.is_empty() {
            target_table.insert("linker-flavor".to_string(), toml::Value::String(self.flavor.cargo_name().to_string()));
            target_table.insert("linker-args".to_string(), toml::Value::Array(
                self.args.iter().map(|a| toml::Value::String(a.clone())).collect()
            ));
        } else {
            target_table.insert("linker-flavor".to_string(), toml::Value::String(self.flavor.cargo_name().to_string()));
        }

        table.insert(target_key, toml::Value::Table(target_table));
        table
    }
}

pub fn resolve_linker(target: &TargetTriple) -> LinkerConfig {
    let (linker_name, flavor) = match (target.family(), target.abi) {
        (TargetFamily::Windows, crate::model::Abi::Msvc) => ("link.exe", LinkerFlavor::Msvc),
        (TargetFamily::Windows, _) => ("lld-link", LinkerFlavor::Lld),
        (TargetFamily::Wasm, _) => ("wasm-ld", LinkerFlavor::WasmLld),
        (TargetFamily::MacOs, _) => ("ld64.lld", LinkerFlavor::Darwin),
        _ => ("ld.lld", LinkerFlavor::Lld),
    };

    // Try to find the linker
    let linker_path = which::which(linker_name)
        .unwrap_or_else(|_| PathBuf::from(linker_name));

    let mut config = LinkerConfig::new(target.clone(), linker_path, flavor);

    // Add target-specific linker args
    match (target.family(), target.abi) {
        (TargetFamily::Windows, crate::model::Abi::Gnu) => {
            config = config.with_args(vec!["-flavor".into(), "gnu".into()]);
        }
        (TargetFamily::Wasm, _) => {
            config = config.with_args(vec!["--no-entry".into(), "--import-memory".into()]);
        }
        _ => {}
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TargetTriple;

    #[test]
    fn resolves_lld_for_linux() {
        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        let config = resolve_linker(&target);
        assert_eq!(config.flavor, LinkerFlavor::Lld);
    }

    #[test]
    fn resolves_msvc_for_windows_msvc() {
        let target = TargetTriple::parse("x86_64-pc-windows-msvc").unwrap();
        let config = resolve_linker(&target);
        assert_eq!(config.flavor, LinkerFlavor::Msvc);
    }

    #[test]
    fn resolves_wasm_ld_for_wasm() {
        let target = TargetTriple::parse("wasm32-wasi").unwrap();
        let config = resolve_linker(&target);
        assert_eq!(config.flavor, LinkerFlavor::WasmLld);
    }
}