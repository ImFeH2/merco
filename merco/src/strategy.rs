pub mod context;

use crate::errors::{AppError, AppResult};
use cargo_metadata::MetadataCommand;
pub use context::StrategyContext;
use include_dir::{Dir, DirEntry, include_dir};
use libloading::{Library, Symbol};
use std::{
    collections::HashMap,
    env, fs,
    ops::{Deref, DerefMut},
    os,
    path::PathBuf,
    process::Command,
};
use toml_edit::{Document, DocumentMut, array, table, value};

const PLUGIN_CREATE_FUNCTION_NAME: &'static str = "_plugin_create";

pub trait Strategy {
    fn tick(&mut self, context: StrategyContext) -> AppResult<()>;
}

pub struct StrategyHandle {
    strategy: Box<dyn Strategy>,
    _lib: Library, // Keep the library loaded
}

impl StrategyHandle {
    pub fn try_from_path(path: &PathBuf) -> AppResult<Self> {
        unsafe {
            let lib = Library::new(path)?;
            let constructor: Symbol<fn() -> *mut dyn Strategy> =
                lib.get(PLUGIN_CREATE_FUNCTION_NAME.as_bytes())?;
            let strategy = Box::from_raw(constructor());
            Ok(Self {
                strategy,
                _lib: lib,
            })
        }
    }
}

impl Deref for StrategyHandle {
    type Target = Box<dyn Strategy>;
    fn deref(&self) -> &Self::Target {
        &self.strategy
    }
}

impl DerefMut for StrategyHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.strategy
    }
}

const WORKSPACE_CARGO_TOML: &str = include_str!("../templates/strategy/Cargo.toml.template");
const MEMBER_CARGO_TOML: &str = include_str!("../templates/strategy/member/Cargo.toml.template");
const MEMBER_LIB_RS: &str = include_str!("../templates/strategy/member/src/lib.rs.template");

#[derive(Debug, Clone)]
pub struct StrategyManager {
    workspace_dir: PathBuf,
}

pub const STRATEGY_WORKDIR_NAME: &str = "strategies";

impl StrategyManager {
    pub fn new() -> AppResult<Self> {
        let current_dir = std::env::current_dir()?;
        let strategies_dir = current_dir.join(STRATEGY_WORKDIR_NAME);

        if !strategies_dir.is_dir() {
            if strategies_dir.exists() {
                return Err(format!(
                    "Create strategies dir failed: {}",
                    strategies_dir.to_string_lossy()
                )
                .into());
            }
            fs::create_dir_all(&strategies_dir)?;
        }

        let workspace_toml = strategies_dir.join("Cargo.toml");
        fs::write(workspace_toml, WORKSPACE_CARGO_TOML)?;

        Ok(Self {
            workspace_dir: strategies_dir,
        })
    }

    pub fn add_strategy(&mut self, strategy_name: &str) -> AppResult<()> {
        let workspace_toml_path = self.workspace_dir.join("Cargo.toml");
        let mut workspace_toml: DocumentMut = fs::read_to_string(&workspace_toml_path)?.parse()?;

        let members = workspace_toml["workspace"].or_insert(table())["members"]
            .or_insert(array())
            .as_array_mut()
            .unwrap();

        if members.iter().any(|m| m.as_str().unwrap() == strategy_name) {
            return Err("Strategy exist".into());
        }

        members.push(strategy_name);
        fs::write(workspace_toml_path, workspace_toml.to_string())?;

        let strategy_dir = self.workspace_dir.join(strategy_name);
        if strategy_dir.exists() {
            return Err("Strategy directory path not empty".into());
        }

        fs::create_dir_all(&strategy_dir)?;

        let mut cargo_toml: DocumentMut = MEMBER_CARGO_TOML.parse()?;
        cargo_toml["package"]["name"] = value(strategy_name);

        let cargo_path = strategy_dir.join("Cargo.toml");
        fs::write(cargo_path, cargo_toml.to_string())?;

        let src_dir = strategy_dir.join("src");
        fs::create_dir_all(&src_dir)?;

        let lib_path = src_dir.join("lib.rs");
        fs::write(lib_path, MEMBER_LIB_RS)?;

        Ok(())
    }

    pub fn backtest(&self, strategy_name: &str, context: StrategyContext) -> AppResult<()> {
        let metadata = MetadataCommand::new()
            .current_dir(&self.workspace_dir)
            .exec()?;

        let _ = metadata
            .packages
            .iter()
            .find(|p| p.name == strategy_name)
            .ok_or(format!("Package '{}' not found", strategy_name))?;

        let status = Command::new("cargo")
            .args(["build", "--release", "--package", strategy_name])
            .current_dir(&self.workspace_dir)
            .status()?;

        if !status.success() {
            return Err("Build failed".into());
        }

        let target_dir = metadata.target_directory.as_std_path();

        #[cfg(target_os = "linux")]
        let lib_name = format!("lib{}.so", strategy_name.replace("-", "_"));

        #[cfg(target_os = "macos")]
        let lib_name = format!("lib{}.dylib", strategy_name.replace("-", "_"));

        #[cfg(target_os = "windows")]
        let lib_name = format!("{}.dll", strategy_name.replace("-", "_"));

        let lib_path = target_dir.join("release").join(&lib_name);

        if !lib_path.exists() {
            return Err(format!("Library not found: {:?}", lib_path).into());
        }

        let mut handle = StrategyHandle::try_from_path(&lib_path)?;

        handle.tick(context)?;
        Ok(())
    }
}
