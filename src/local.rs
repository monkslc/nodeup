use crate::target::Target;
use std::{env, fs, io, path::PathBuf};
use tempfile::NamedTempFile;
use thiserror::Error;

const CONFIG_FILE_NAME: &str = "settings.toml";
const NODEUP: &str = "nodeup";

const CONFIG_DIR_NOT_FOUND: &str = "Can't find an appropriate directory for config. Searched $NODEUP_CONFIG_DIR/settings.toml -> $XDG_CONFIG_HOME/nodeup/settings.toml -> $HOME/.config/nodeup/settings.toml";
const DOWNLOAD_DIR_NOT_FOUND: &str = "Can't find an appropriate directory for node binaries. Searched $NODEUP_DOWNLOADS -> $XDG_DATA_HOME/nodeup -> $HOME/.local/share/nodeup";
const LINKS_DIR_NOT_FOUND: &str = "Can't find an appropriate directory for nodeup symlinks. Searched $NODEUP_LINKS -> $XDG_BIN_HOME/nodeup/links -> $HOME/.local/bin";

type LocalResult<T> = Result<T, LocalError>;

#[derive(Debug, Error)]
pub enum LocalError {
    #[error("{0}")]
    NotFound(&'static str),

    #[error("IO Error when trying to access: {path:?}: {source}")]
    IO { source: io::Error, path: PathBuf },
}

/// Returns the location of the directory where node downloads are stored
///
/// ### Order of preference for download directory
///
/// | |Linux                    |Mac                                     |Windows                         |
/// |-|-------------------------|----------------------------------------|--------------------------------|
/// |1|$NODEUP_DOWNLOADS        |$NODEUP_DOWNLOADS                       |$NODEUP_DOWNLOADS               |
/// |2|$XDG_DATA_HOME/nodeup    |$HOME/library/Application Support/nodeup|{FOLDERID_RoamingAppData}\nodeup|
/// |3|$Home/.local/share/nodeup|
pub fn download_dir() -> LocalResult<PathBuf> {
    let nodeup_bin = env::var_os("NODEUP_DOWNLOADS").map(|dir| PathBuf::from(&dir));
    if let Some(nodeup_bin) = nodeup_bin {
        return Ok(nodeup_bin);
    }

    dirs::data_dir()
        .map(|dir| PathBuf::from(&dir).join(NODEUP))
        .ok_or(LocalError::NotFound(DOWNLOAD_DIR_NOT_FOUND))
}

pub fn target_path(target: &Target) -> LocalResult<PathBuf> {
    download_dir().map(|dir| dir.join(target.to_string()))
}

/// Returns the location of the nodeup config directory
///
/// ### Order of preference for config directory
///
/// | |Linux                  |Mac                                     |Windows                         |
/// |-|-----------------------|----------------------------------------|--------------------------------|
/// |1|$NODEUP_CONFIG         |$NODEUP_CONFIG                          |$NODEUP_CONFIG                  |
/// |2|$XDG_CONFIG_HOME/nodeup|$HOME/Library/Application Support/nodeup|{FOLDERID_RoamingAppData}\nodeup|
/// |3|$HOME/.config          |
pub fn config_dir() -> LocalResult<PathBuf> {
    let config_dir = env::var_os("NODEUP_CONFIG")
        .map(PathBuf::from)
        .or_else(|| dirs::config_dir().map(|dir| dir.join(NODEUP)));

    match config_dir {
        Some(config_dir) => {
            // Create config dir in case it doesn't already exist
            fs::create_dir_all(&config_dir).map_err(|source| LocalError::IO {
                source,
                path: config_dir.to_path_buf(),
            })?;
            Ok(config_dir)
        }
        None => Err(LocalError::NotFound(CONFIG_DIR_NOT_FOUND)),
    }
}

/// The location to the settings.toml for nodeup
pub fn config_file() -> LocalResult<PathBuf> {
    config_dir().map(|dir| dir.join(CONFIG_FILE_NAME))
}

/// Transitory config file. Used for writing updates before overwriting the original file. The file
/// will have a randomly generated file name
pub fn transitory_config_file() -> LocalResult<NamedTempFile> {
    let config_dir = config_dir()?;
    NamedTempFile::new_in(&config_dir).map_err(|source| LocalError::IO {
        source,
        path: config_dir,
    })
}

/// Returns the location of the node, npm, and npx symlinks to nodeup
///
/// ### Order of preference for links directory
///
/// | |Linux           |Mac             |Windows      |
/// |-|----------------|----------------|-------------|
/// |1|$NODEUP_LINKS   |$NODEUP_LINKS   |$NODEUP_LINKS|
/// |2|$HOME/.local/bin|$HOME/.local/bin|TODO         |
pub fn links() -> LocalResult<PathBuf> {
    #[cfg(unix)]
    env::var_os("NODEUP_LINKS")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|dir| dir.join(".local").join("bin")))
        .ok_or(LocalError::NotFound(LINKS_DIR_NOT_FOUND))
}
