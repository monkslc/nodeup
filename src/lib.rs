use anyhow::{anyhow, Context, Result};
use dirs::home_dir;
use flate2::read::GzDecoder;
use reqwest::{blocking, StatusCode};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, fs::OpenOptions, io::Read, path::PathBuf};
use tar::Archive;

mod version;

pub use version::Version;

// Full url example: https://nodejs.org/dist/v12.9.1/node-v12.9.1-linux-x64.tar.gz
const BASE_URL: &'static str = "https://nodejs.org/dist/";

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

fn get_nodeup_dir() -> Result<PathBuf> {
    let mut home_dir = home_dir().ok_or(anyhow!("Error getting home directory"))?;
    home_dir.push(".nodeup");
    Ok(home_dir)
}

pub fn download_node(version: Version) -> Result<()> {
    let url = get_node_download_url(version);
    let tar_gzip =
        blocking::get(&url).with_context(|| format!("Failed to make request to {}", url))?;
    match tar_gzip.status() {
        StatusCode::OK => {
            let mut node_dir = get_nodeup_dir()?;
            node_dir.push("node");

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

pub fn list_versions() -> Result<()> {
    let mut node_dir = get_nodeup_dir()?;
    node_dir.push("node");
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
    let mut config_file = get_nodeup_dir()?;
    config_file.push("settings.toml");

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

    let mut updated_config_file = get_nodeup_dir()?;
    updated_config_file.push(".updated.settings.toml");

    fs::write(&updated_config_file, updated_contents)
        .context("Error writing updated config file .updated.settings.toml")?;

    let mut config_file = get_nodeup_dir()?;
    config_file.push("settings.toml");
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
