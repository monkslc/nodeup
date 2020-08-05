use anyhow::{anyhow, Context, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env, fs,
    fs::OpenOptions,
    io::Read,
    os::unix::{fs::symlink, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

mod registry;
mod target;

pub use registry::get_latest_lts;
pub use target::{Target, Version};

const BIN_DIR: &'static str = "bin";
const BIN_NODE: &'static str = "node";
const BIN_NODEUP: &'static str = "nodeup";
const BIN_NPM: &'static str = "npm";
const BIN_NPX: &'static str = "npx";
const INSTALL_DIR: &'static str = "node";
const NODEUP_DIR: &'static str = ".nodeup";
const SETTINGS_FILE: &'static str = "settings.toml";
const UPDATED_SETTINGS_FILE_TEMP: &'static str = ".updated.settings.toml";

pub fn download_node(target: Target) -> Result<()> {
    let nodeup_dir = get_nodeup_dir()?;
    registry::download_node_to(target, &nodeup_dir)
}

pub fn get_nodeup_dir() -> Result<PathBuf> {
    let nodeup_dir = home_dir()
        .ok_or(anyhow!("Error getting home directory"))?
        .join(NODEUP_DIR);

    Ok(nodeup_dir)
}

// TODO: check that the version is installed before removing
pub fn remove_node(target: Target) -> Result<()> {
    let path = get_nodeup_dir()?.join(INSTALL_DIR).join(target.to_string());
    fs::remove_dir_all(path)
        .with_context(|| format!("Error removing {}. Maybe it wasn't installed?", target))?;
    Ok(())
}

pub fn list_versions() -> Result<()> {
    let node_dir = get_nodeup_dir()?.join(INSTALL_DIR);
    let entries =
        fs::read_dir(node_dir).context("Error reading entries in directory: ~/.nodeup/node")?;
    entries.for_each(|entry| {
        if let Ok(entry) = entry {
            if let Some(installed_version) = entry.file_name().to_str() {
                println!("{}", installed_version)
            }
        }
    });

    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    #[serde(default)]
    version_mappings: HashMap<PathBuf, Target>,
}

fn get_config_file() -> Result<Config> {
    let config_file = get_nodeup_dir()?.join(SETTINGS_FILE);

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(config_file)
        .context("Failed to open config file")?;

    let mut content = Vec::new();
    file.read_to_end(&mut content)
        .context("Error reading file")?;

    let config: Config =
        toml::from_slice(&content[..]).context("Error deserializing config file")?;

    Ok(Config::from(config))
}

// TODO: check that the version is installed
pub fn change_default_target(target: Target) -> Result<()> {
    let mut config = get_config_file()?;
    config
        .version_mappings
        .insert(PathBuf::from_str("default").unwrap(), target);

    let updated_contents = toml::to_vec(&config).context("Error deserializing settings.toml")?;

    let updated_config_file = get_nodeup_dir()?.join(UPDATED_SETTINGS_FILE_TEMP);

    fs::write(&updated_config_file, updated_contents)
        .context("Error writing updated config file .updated.settings.toml")?;

    let config_file = get_nodeup_dir()?.join(SETTINGS_FILE);
    fs::rename(&updated_config_file, config_file)
        .context("Error writing updates to settings.toml")?;

    Ok(())
}

pub fn active_versions() -> Result<()> {
    let config = get_config_file()?;
    config.version_mappings.iter().for_each(|(dir, version)| {
        println!("{:?} {}", dir, version);
    });

    Ok(())
}

pub fn link() -> Result<()> {
    let bin_dir = get_nodeup_dir()?.join(BIN_DIR);
    fs::create_dir_all(&bin_dir).context("Error creating bin dir")?;

    let nodeup_path = bin_dir.as_path().join(BIN_NODEUP);

    let node_path = bin_dir.as_path().join(BIN_NODE);
    symlink(&nodeup_path, node_path).context("Error symlinking node")?;

    let npm_path = bin_dir.as_path().join(BIN_NPM);
    symlink(&nodeup_path, npm_path).context("Error symlinking npm")?;

    let npx_path = bin_dir.as_path().join(BIN_NPX);
    symlink(&nodeup_path, npx_path).context("Error symlinking npx")?;

    Ok(())
}

pub fn execute_bin<I: std::iter::Iterator<Item = String>>(bin: &str, args: I) -> Result<()> {
    let config = get_config_file()?;
    if let Some(target) = config.version_mappings.get(Path::new("default")) {
        let bin_path = get_nodeup_dir()?
            .join("node")
            .join(target.to_string())
            .join("bin")
            .join(bin);

        Command::new(&bin_path).args(args).exec();
        Err(anyhow!("Failed to execute bin at path: {:?}", bin_path))
    } else {
        Err(anyhow!("No default version found"))
    }
}

pub fn override_cwd(target: Target) -> Result<()> {
    let cwd = env::current_dir()?;
    set_override(target, cwd)
}

pub fn set_override(target: Target, dir: PathBuf) -> Result<()> {
    let mut config = get_config_file()?;
    config.version_mappings.insert(dir, target);

    let updated_contents = toml::to_vec(&config).context("Error deserializing settings.toml")?;

    let updated_config_file = get_nodeup_dir()?.join(UPDATED_SETTINGS_FILE_TEMP);

    fs::write(&updated_config_file, updated_contents)
        .context("Error writing updated config file .updated.settings.toml")?;

    let config_file = get_nodeup_dir()?.join(SETTINGS_FILE);
    fs::rename(&updated_config_file, config_file)
        .context("Error writing updates to settings.toml")?;

    Ok(())
}
