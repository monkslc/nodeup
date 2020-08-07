use crate::target::Target;
use anyhow::{anyhow, Result};
use dirs;
use std::{env, path::PathBuf};

const CONFIG_FILE_NAME: &'static str = "settings.toml";
const NODEUP: &'static str = "nodeup";
const NODEUP_DIR: &'static str = ".nodeup";

const CONFIG_DIR_NOT_FOUND: &'static str = "Can't find an appropriate directory for config. Searched $NODEUP_CONFIG_DIR/settings.toml -> $XDG_CONFIG_HOME/nodeup/settings.toml -> $HOME/.config/nodeup/settings.toml";
const DOWNLOAD_DIR_NOT_FOUND: &'static str = "Can't find an appropriate directory for node binaries. Searched $NODEUP_DOWNLOADS -> $XDG_DATA_HOME/nodeup -> $HOME/.local/share/nodeup";

pub fn get() -> Result<PathBuf> {
    let nodeup_dir = dirs::home_dir()
        .ok_or(anyhow!("Error getting home directory"))?
        .join(NODEUP_DIR);

    Ok(nodeup_dir)
}

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
pub fn config() -> Result<PathBuf> {
    let nodeup_bin =
        env::var_os("NODEUP_CONFIG").map(|dir| PathBuf::from(&dir).join(CONFIG_FILE_NAME));
    if let Some(nodeup_bin) = nodeup_bin {
        return Ok(nodeup_bin);
    }

    dirs::config_dir()
        .map(|dir| dir.join(NODEUP).join(CONFIG_FILE_NAME))
        .ok_or(anyhow!(CONFIG_DIR_NOT_FOUND))
}

/*
 * Order of preference for download directory
 * 1. $NODEUP_LINKS
 * 2. $XDG_BIN_HOME/nodeup/links
 * 3. $HOME/.local/bin
 */
pub fn links() -> Result<PathBuf> {
    env::var_os("NODEUP_LINKS")
        .map(|path| PathBuf::from(path))
        .or(env::var_os("XDG_BIN_HOME").map(|dir| PathBuf::from(dir).join("nodeup").join("links")))
        .or(dirs::home_dir().map(|dir| dir.join(".local").join("bin")))
        .ok_or(anyhow!("Error getting executable dir"))
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
        let actual = config().unwrap();
        let expected = PathBuf::from("/tmp/config/settings.toml");
        env::remove_var("NODEUP_CONFIG");
        assert_eq!(actual, expected);

        env::set_var("XDG_CONFIG_HOME", "/tmp/xdg-config");
        let actual = config().unwrap();
        let expected = PathBuf::from("/tmp/xdg-config/nodeup/settings.toml");
        env::remove_var("XDG_CONFIG_HOME");
        assert_eq!(actual, expected);

        let actual = config().unwrap();
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
