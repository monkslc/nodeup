use anyhow::{anyhow, Context, Result};
use dirs::home_dir;
use flate2::read::GzDecoder;
use reqwest::{blocking, StatusCode};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    fs::OpenOptions,
    io::Read,
    os::unix::{fs::symlink, process::CommandExt},
    path::PathBuf,
    process::Command,
};
use tar::Archive;

mod version;

pub use version::Version;

// Full url example: https://nodejs.org/dist/v12.9.1/node-v12.9.1-linux-x64.tar.gz
const BASE_URL: &'static str = "https://nodejs.org/dist/";
const BIN_DIR: &'static str = "bin";
const BIN_NODE: &'static str = "node";
const BIN_NODEUP: &'static str = "nodeup";
const BIN_NPM: &'static str = "npm";
const BIN_NPX: &'static str = "npx";
const INSTALL_DIR: &'static str = "node";
const NODEUP_DIR: &'static str = ".nodeup";
const SETTINGS_FILE: &'static str = "settings.toml";
const UPDATED_SETTINGS_FILE_TEMP: &'static str = ".updated.settings.toml";

#[cfg(target_os = "macos")]
static OS: &str = "darwin";
#[cfg(target_os = "linux")]
static OS: &str = "linux";
#[cfg(target_os = "windows")]
static OS: &str = "win";

fn get_node_download_url(version: Version) -> String {
    let full_url = format!(
        "{}{}/{}.tar.gz",
        BASE_URL,
        version,
        get_node_arch_string(version)
    );
    full_url
}

fn get_node_arch_string(version: Version) -> String {
    format!("node-{}-{}-x64", version, OS)
}

#[derive(Debug, Deserialize, Serialize)]
struct AvailableVersion {
    version: String,
    lts: LTSVersion,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
enum LTSVersion {
    Yes(String),
    No(bool),
}

pub fn get_latest_lts() -> Result<Version> {
    let url = format!("{}index.json", BASE_URL);
    let resp = blocking::get(&url)
        .with_context(|| format!("Request to Node distribution registry: {}", url))?;
    let all_versions: Vec<AvailableVersion> = serde_json::from_reader(resp)?;
    let latest_lts = all_versions
        .into_iter()
        .filter_map(|v| match v.lts {
            LTSVersion::Yes(_) => Some(
                Version::parse(&v.version)
                    .expect(&format!("Error parsing verson from node registry: {:?}", v)),
            ),
            _ => None,
        })
        .max()
        .expect("Received no lts versions from the node distribution registry");

    Ok(latest_lts)
}

fn get_nodeup_dir() -> Result<PathBuf> {
    let nodeup_dir = home_dir()
        .ok_or(anyhow!("Error getting home directory"))?
        .join(NODEUP_DIR);

    Ok(nodeup_dir)
}

pub fn download_node(version: Version) -> Result<()> {
    let url = get_node_download_url(version);
    let tar_gzip =
        blocking::get(&url).with_context(|| format!("Failed to make request to {}", url))?;
    match tar_gzip.status() {
        StatusCode::OK => {
            let node_dir = get_nodeup_dir()?.join(INSTALL_DIR);

            let tar = GzDecoder::new(tar_gzip);
            let mut arc = Archive::new(tar);
            arc.unpack(node_dir)
                .with_context(|| format!("Failed to unpack node into directory: {}", "."))?;
            Ok(())
        }
        StatusCode::NOT_FOUND => Err(anyhow!("Version: {} does not exist", version)),
        code => Err(anyhow!("Unknown Error: {}", code)),
    }
}

// TODO: check that the version is installed before removing
pub fn remove_node(version: Version) -> Result<()> {
    let path = get_nodeup_dir()?
        .join(INSTALL_DIR)
        .join(get_node_arch_string(version));
    fs::remove_dir_all(path).with_context(|| {
        format!(
            "Error removing node version: {}. Maybe it wasn't installed?",
            version
        )
    })?;
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

#[derive(Debug, Serialize)]
struct Config {
    version_mappings: HashMap<String, String>,
}

impl From<ConfigDTO> for Config {
    fn from(dto: ConfigDTO) -> Config {
        Config {
            version_mappings: dto.version_mappings.unwrap_or_else(HashMap::new),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ConfigDTO {
    version_mappings: Option<HashMap<String, String>>,
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

    let config: ConfigDTO =
        toml::from_slice(&content[..]).context("Error deserializing config file")?;

    Ok(Config::from(config))
}

// TODO: check that the version is installed
pub fn change_default_version(version: Version) -> Result<()> {
    let arch_string = get_node_arch_string(version);

    let mut config = get_config_file()?;
    config
        .version_mappings
        .insert(String::from("default"), arch_string);

    let updated_contents = toml::to_vec(&config).context("Error serializing contents")?;

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
        println!("{} {}", dir, version);
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

pub fn execute_node<I: std::iter::Iterator<Item = String>>(args: I) -> Result<()> {
    let config = get_config_file()?;
    if let Some(version) = config.version_mappings.get("default") {
        let bin_path = get_nodeup_dir()?
            .join("node")
            .join(version)
            .join("bin")
            .join("node");

        Command::new(&bin_path).args(args).exec();
        Err(anyhow!("Failed to execute bin at path: {:?}", bin_path))
    } else {
        Err(anyhow!("No default version found"))
    }
}

pub fn execute_bin<I: std::iter::Iterator<Item = String>>(bin: &str, args: I) -> Result<()> {
    let config = get_config_file()?;
    if let Some(version) = config.version_mappings.get("default") {
        let bin_path = get_nodeup_dir()?
            .join("node")
            .join(version)
            .join("bin")
            .join(bin);

        Command::new(&bin_path).args(args).exec();
        Err(anyhow!("Failed to execute bin at path: {:?}", bin_path))
    } else {
        Err(anyhow!("No default version found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_node_url() {
        let version = Version {
            major: 12,
            minor: 9,
            patch: 1,
        };

        let actual = get_node_download_url(version);
        let expected = "https://nodejs.org/dist/v12.9.1/node-v12.9.1-linux-x64.tar.gz";
        assert_eq!(actual, expected);
    }
}
