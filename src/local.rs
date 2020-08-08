use crate::target::Target;
use anyhow::{anyhow, Context, Result};
use std::{
    env,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;
use uuid::Uuid;

const CONFIG_FILE_NAME: &str = "settings.toml";
const NODEUP: &str = "nodeup";
const TRANSITORY_UPDATE_FILE: &str = ".updated.settings.toml";

const CONFIG_DIR_NOT_FOUND: &str = "Can't find an appropriate directory for config. Searched $NODEUP_CONFIG_DIR/settings.toml -> $XDG_CONFIG_HOME/nodeup/settings.toml -> $HOME/.config/nodeup/settings.toml";
const DOWNLOAD_DIR_NOT_FOUND: &str = "Can't find an appropriate directory for node binaries. Searched $NODEUP_DOWNLOADS -> $XDG_DATA_HOME/nodeup -> $HOME/.local/share/nodeup";

/*
 * Order of preference for download directory
 * 1. $NODEUP_DOWNLOADS
 * 2. $XDG_DATA_HOME/nodeup
 * 3. $HOME/.local/share/nodeup
 */
pub fn download_dir() -> Result<PathBuf> {
    let nodeup_bin = env::var_os("NODEUP_DOWNLOADS").map(|dir| PathBuf::from(&dir));
    if let Some(nodeup_bin) = nodeup_bin {
        return Ok(nodeup_bin);
    }

    dirs::data_dir()
        .map(|dir| PathBuf::from(&dir).join(NODEUP))
        .ok_or(anyhow!(DOWNLOAD_DIR_NOT_FOUND))
}

pub fn target_path(target: &Target) -> Result<PathBuf> {
    download_dir().map(|dir| dir.join(target.to_string()))
}

/*
 * Order of preference for binary directory
 * 1. $NODEUP_CONFIG/settings.toml
 * 2. $XDG_CONFIG_HOME/nodeup/settings.toml
 * 3. $HOME/.config/nodeup/settings.toml
 */
pub fn config_file() -> Result<PathBuf> {
    env::var_os("NODEUP_CONFIG")
        .map(|dir| PathBuf::from(dir).join(CONFIG_FILE_NAME))
        .or_else(|| dirs::config_dir().map(|dir| dir.join(NODEUP).join(CONFIG_FILE_NAME)))
        .ok_or(anyhow!(CONFIG_DIR_NOT_FOUND))
}

/*
 * Transitory config file. Used for writing updates before overwriting the original file. The file
 * will have a randomly generated file name
 */
pub fn transitory_config_file() -> Result<NamedTempFile> {
    let transitory_file_name = Path::new(TRANSITORY_UPDATE_FILE).join(Uuid::new_v4().to_string());
    let transitory_file_path = env::var_os("NODEUP_CONFIG")
        .map(|dir| PathBuf::from(dir).join(&transitory_file_name))
        .or_else(|| dirs::config_dir().map(|dir| dir.join(NODEUP).join(&transitory_file_name)))
        .ok_or(anyhow!(CONFIG_DIR_NOT_FOUND))?;
    NamedTempFile::new_in(transitory_file_path).context("Error creating transitory file")
}

/*
 * Order of preference for download directory
 * 1. $NODEUP_LINKS
 * 2. $XDG_BIN_HOME/nodeup/links
 * 3. $HOME/.local/bin
 */
pub fn links() -> Result<PathBuf> {
    env::var_os("NODEUP_LINKS")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("XDG_BIN_HOME").map(|dir| PathBuf::from(dir).join("nodeup").join("links"))
        })
        .or_else(|| dirs::home_dir().map(|dir| dir.join(".local").join("bin")))
        .ok_or_else(|| anyhow!("Error getting executable dir"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_download_dir() {
        env::set_var("NODEUP_DOWNLOADS", "/tmp/nodeup");
        let actual = download_dir().unwrap();
        let expected = PathBuf::from("/tmp/nodeup");
        env::remove_var("NODEUP_DOWNLOADS");
        assert_eq!(actual, expected);

        env::set_var("XDG_DATA_HOME", "/tmp/other-nodeup");
        let actual = download_dir().unwrap();
        let expected = PathBuf::from("/tmp/other-nodeup/nodeup");
        env::remove_var("XDG_DATA_HOME");
        assert_eq!(actual, expected);

        let actual = download_dir().unwrap();
        let expected = dirs::home_dir()
            .map(|dir| dir.join(".local").join("share").join("nodeup"))
            .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn find_config_dir() {
        env::set_var("NODEUP_CONFIG", "/tmp/config");
        let actual = config_file().unwrap();
        let expected = PathBuf::from("/tmp/config/settings.toml");
        env::remove_var("NODEUP_CONFIG");
        assert_eq!(actual, expected);

        env::set_var("XDG_CONFIG_HOME", "/tmp/xdg-config");
        let actual = config_file().unwrap();
        let expected = PathBuf::from("/tmp/xdg-config/nodeup/settings.toml");
        env::remove_var("XDG_CONFIG_HOME");
        assert_eq!(actual, expected);

        let actual = config_file().unwrap();
        let expected = dirs::home_dir()
            .map(|dir| dir.join(".config").join("nodeup").join("settings.toml"))
            .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn linking() {
        env::set_var("NODEUP_LINKS", "/tmp/links");
        let actual = links().unwrap();
        let expected = PathBuf::from("/tmp/links");
        env::remove_var("NODEUP_LINKS");
        assert_eq!(actual, expected);

        env::set_var("XDG_BIN_HOME", "/tmp/xdg-links");
        let actual = links().unwrap();
        let expected = PathBuf::from("/tmp/xdg-links/nodeup/links");
        env::remove_var("XDG_BIN_HOME");
        assert_eq!(actual, expected);

        let home = dirs::home_dir().unwrap();
        env::set_var("HOME", "/tmp/home");
        let actual = links().unwrap();
        let expected = PathBuf::from("/tmp/home/.local/bin/");
        env::set_var("HOME", home);
        assert_eq!(actual, expected);
    }
}
