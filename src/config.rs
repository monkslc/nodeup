use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    fs::OpenOptions,
    io,
    io::Read,
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::{
    local::{self, LocalError},
    target::Target,
};

pub type ConfigResult<T> = Result<T, ConfigError>;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(transparent)]
    Local(#[from] LocalError),

    #[error("An IO error occured while trying to access {path:?}: {source}")]
    IO { source: io::Error, path: PathBuf },

    #[error("An error occured trying to deserialize the config file. This may be indicative of a malformatted file. Check the file at path: {path:?}: {source}")]
    Corruption {
        source: toml::de::Error,
        path: PathBuf,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    version_mappings: HashMap<PathBuf, Target>,
}

pub type VersionIterator = std::collections::hash_map::IntoIter<std::path::PathBuf, Target>;

impl Config {
    pub fn fetch() -> ConfigResult<Self> {
        let config_file = local::config_file()?;

        if let Some(config_dir) = config_file.parent() {
            fs::create_dir_all(config_dir).map_err(|source| ConfigError::IO {
                source,
                path: config_dir.to_path_buf(),
            })?;
        }
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&config_file)
            .map_err(|source| ConfigError::IO {
                source,
                path: config_file.clone(),
            })?;

        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .map_err(|source| ConfigError::IO {
                source,
                path: config_file.clone(),
            })?;

        let config: Config =
            toml::from_slice(&content[..]).map_err(|source| ConfigError::Corruption {
                source,
                path: config_file,
            })?;

        Ok(config)
    }

    pub fn update(&self) -> ConfigResult<()> {
        let updated_contents = toml::to_vec(&self)
            .expect("Failed to serialize updated config file. This shouldn't fail");

        let updated_config_file = local::transitory_config_file()?;

        fs::write(&updated_config_file, updated_contents).map_err(|source| ConfigError::IO {
            source,
            path: updated_config_file.path().to_path_buf(),
        })?;

        let config_file = local::config_file()?;
        fs::rename(&updated_config_file, &config_file).map_err(|source| ConfigError::IO {
            source,
            path: updated_config_file.path().to_path_buf(),
        })?;

        Ok(())
    }

    pub fn active_versions(self) -> VersionIterator {
        self.version_mappings.into_iter()
    }

    pub fn get_active_target(&self, _path: &Path) -> Option<&Target> {
        self.version_mappings.get(&PathBuf::from("default"))
    }

    pub fn set_override(&mut self, target: Target, dir: PathBuf) -> ConfigResult<()> {
        self.version_mappings.insert(dir, target);
        self.update()
    }

    pub fn remove_override(&mut self, dir: PathBuf) -> ConfigResult<()> {
        self.version_mappings.remove(&dir);
        self.update()
    }
}
