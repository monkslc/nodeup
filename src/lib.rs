use log::debug;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env, fmt, fs,
    fs::OpenOptions,
    io,
    io::{ErrorKind, Read},
    os::unix::{fs::symlink, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};
use thiserror::Error;

pub mod local;
pub mod registry;
mod target;

use local::LocalError;
pub use registry::get_latest_lts;
pub use target::{Target, Version};

const NODE_EXECUTABLE: &str = "nodeup";
const NPM_EXECUTABLE: &str = "npm";
const NPX_EXECUTABLE: &str = "npx";

pub type NodeupResult<T> = std::result::Result<T, NodeupError>;

#[derive(Debug, Error)]
pub enum NodeupError {
    #[error("An error occured accessing local files when trying to {task}.: {source}")]
    Local { source: LocalError, task: ErrorTask },

    #[error("An io error occured trying to {task} at {path}: {source}")]
    IO {
        source: io::Error,
        task: ErrorTask,
        path: PathBuf,
    },

    #[error("An error occured accessing the config while trying to {task}: {source}")]
    Config {
        source: ConfigFileError,
        task: ErrorTask,
    },

    #[error("Couldn't create symlinks required to {task}: {source}")]
    Linking {
        source: LinkingError,
        task: ErrorTask,
    },

    #[error(
        "Not sure which version to run. Try setting a default by running nodeup default x.x.x"
    )]
    NoVersionFound,
}

#[derive(Debug, Error)]
pub enum ConfigFileError {
    #[error(transparent)]
    Local(#[from] LocalError),

    #[error("An IO error occurect while trying to access {path:?}: {source}")]
    IO { source: io::Error, path: PathBuf },

    #[error("An error occured trying to deserialize the config file. This may be indicative of a malformatted file. Check the file at path: {path:?}: {source}")]
    Corruption {
        source: toml::de::Error,
        path: PathBuf,
    },
}

#[derive(Debug, Error)]
pub enum LinkingError {
    #[error("An IO error occurect while trying to access {path:?}: {source}")]
    IO { source: io::Error, path: PathBuf },

    #[error("It looks like something already exists at {path}. Try removing and linking again. The link directory can also be controlled by setting the $NODEUP_LINKS environment variable")]
    AlreadyExists { path: PathBuf },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorTask {
    ActiveVersions,
    ChangingDefault,
    Executing,
    Installing,
    Linking,
    Override,
    Removing,
}

// Display should be implemented to fit into the NodeupError above
// Should be written in the imperative mood
impl fmt::Display for ErrorTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorTask::ActiveVersions => write!(f, "list active versions"),
            ErrorTask::ChangingDefault => write!(f, "change default"),
            ErrorTask::Executing => write!(f, "execute command"),
            ErrorTask::Installing => write!(f, "install node"),
            ErrorTask::Linking => write!(f, "create sym links"),
            ErrorTask::Override => write!(f, "create override"),
            ErrorTask::Removing => write!(f, "remove node"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    #[serde(default)]
    version_mappings: HashMap<PathBuf, Target>,
}

// TODO: check that the version is installed before removing
pub fn remove_node(target: Target) -> NodeupResult<()> {
    use ErrorTask::Removing as task;

    let path = local::target_path(&target).map_err(|source| NodeupError::Local { source, task })?;
    fs::remove_dir_all(&path).map_err(|source| NodeupError::IO { source, task, path })?;
    Ok(())
}

pub fn installed_versions(path: &Path) -> NodeupResult<Vec<Target>> {
    use ErrorTask::Installing as task;

    let entries = fs::read_dir(path).map_err(|source| NodeupError::IO {
        source,
        task,
        path: path.to_path_buf(),
    })?;

    let target_paths = entries.filter_map(|entry| match entry {
        Ok(entry) => Some(entry),
        Err(e) => {
            debug!(
                "IO Error while trying to read targets in: {}\n{}",
                path.display(),
                e
            );
            None
        }
    });

    let target_names = target_paths.map(|target_path| target_path.file_name());

    let targets = target_names.filter_map(|target| match target.to_str() {
        Some(target_name) => match Target::parse(target_name) {
            Ok(target) => Some(target),
            Err(e) => {
                debug!(
                    "Error parsing target: {}\n{}",
                    target.to_str().unwrap_or("[unknown]"),
                    e
                );
                None
            }
        },
        None => {
            debug!(
                "Error trying to convert: {} to a str",
                target.to_str().unwrap_or("[error]")
            );
            None
        }
    });

    Ok(targets.collect())
}

// TODO: check that the version is installed
pub fn change_default_target(target: Target) -> NodeupResult<()> {
    use ErrorTask::ChangingDefault as task;

    let mut config = open_config_file().map_err(|source| NodeupError::Config { source, task })?;
    config
        .version_mappings
        .insert(PathBuf::from_str("default").unwrap(), target);

    let updated_contents = toml::to_vec(&config).expect(
        "Error occured when trying to serialize and updated config file. This shouldn't happen",
    );

    let updated_config_file =
        local::transitory_config_file().map_err(|source| NodeupError::Local { source, task })?;

    fs::write(&updated_config_file, updated_contents).map_err(|source| NodeupError::IO {
        source,
        task,
        path: updated_config_file.path().to_path_buf(),
    })?;

    let config_file = local::config_file().map_err(|source| NodeupError::Local { source, task })?;

    fs::rename(&updated_config_file, config_file).map_err(|source| NodeupError::IO {
        source,
        task,
        path: updated_config_file.path().to_path_buf(),
    })?;

    Ok(())
}

pub fn active_versions() -> NodeupResult<Vec<(PathBuf, Target)>> {
    use ErrorTask::ActiveVersions as task;

    let config = open_config_file().map_err(|source| NodeupError::Config { source, task })?;
    Ok(config.version_mappings.into_iter().collect())
}

pub fn link_node_bins(links_path: &Path) -> NodeupResult<PathBuf> {
    use ErrorTask::Linking as task;

    let nodeup_path = std::env::current_exe().map_err(|source| NodeupError::IO {
        source,
        task,
        path: PathBuf::from("Looking for current executable"),
    })?;

    link_bin(&nodeup_path, links_path, Path::new(NODE_EXECUTABLE))
        .map_err(|source| NodeupError::Linking { source, task })?;

    link_bin(&nodeup_path, links_path, Path::new(NPM_EXECUTABLE))
        .map_err(|source| NodeupError::Linking { source, task })?;

    link_bin(&nodeup_path, links_path, Path::new(NPX_EXECUTABLE))
        .map_err(|source| NodeupError::Linking { source, task })?;

    Ok(links_path.to_path_buf())
}

pub fn execute_bin<I: std::iter::Iterator<Item = String>>(bin: &str, args: I) -> NodeupResult<()> {
    use ErrorTask::Executing as task;

    let config = open_config_file().map_err(|source| NodeupError::Config { source, task })?;
    if let Some(target) = config.version_mappings.get(Path::new("default")) {
        let target_path =
            local::target_path(target).map_err(|source| NodeupError::Local { source, task })?;
        let bin_path = target_path.join("bin").join(bin);

        Command::new(&bin_path).args(args).exec();
        Ok(())
    } else {
        Err(NodeupError::NoVersionFound)
    }
}

pub fn override_cwd(target: Target) -> NodeupResult<()> {
    use ErrorTask::Override as task;

    let cwd = env::current_dir().map_err(|source| NodeupError::IO {
        source,
        task,
        path: PathBuf::from("cwd"),
    })?;
    set_override(target, cwd)
}

pub fn set_override(target: Target, dir: PathBuf) -> NodeupResult<()> {
    use ErrorTask::Override as task;

    let mut config = open_config_file().map_err(|source| NodeupError::Config { source, task })?;
    config.version_mappings.insert(dir, target);

    let updated_contents = toml::to_vec(&config)
        .expect("Failed to serialize updated config file. This shouldn't fail");

    let updated_config_file =
        local::transitory_config_file().map_err(|source| NodeupError::Local { source, task })?;

    fs::write(&updated_config_file, updated_contents).map_err(|source| NodeupError::IO {
        source,
        path: updated_config_file.path().to_path_buf(),
        task,
    })?;

    let config_file = local::config_file().map_err(|source| NodeupError::Local { source, task })?;
    fs::rename(&updated_config_file, &config_file).map_err(|source| NodeupError::IO {
        source,
        task,
        path: updated_config_file.path().to_path_buf(),
    })?;

    Ok(())
}

fn open_config_file() -> Result<Config, ConfigFileError> {
    let config_file = local::config_file()?;

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&config_file)
        .map_err(|source| ConfigFileError::IO {
            source,
            path: config_file.clone(),
        })?;

    let mut content = Vec::new();
    file.read_to_end(&mut content)
        .map_err(|source| ConfigFileError::IO {
            source,
            path: config_file.clone(),
        })?;

    let config: Config =
        toml::from_slice(&content[..]).map_err(|source| ConfigFileError::Corruption {
            source,
            path: config_file,
        })?;

    Ok(config)
}

fn link_bin(actual: &Path, link_dir: &Path, link_name: &Path) -> Result<(), LinkingError> {
    let full_link_path = link_dir.join(link_name);
    match symlink(actual, &full_link_path) {
        Ok(_) => Ok(()),
        Err(e) => match e.kind() {
            ErrorKind::AlreadyExists => {
                let metadata =
                    fs::symlink_metadata(&full_link_path).map_err(|source| LinkingError::IO {
                        source,
                        path: full_link_path.to_path_buf(),
                    })?;
                match metadata.file_type().is_symlink() {
                    true => Ok(()),
                    false => Err(LinkingError::AlreadyExists {
                        path: full_link_path.to_path_buf(),
                    }),
                }
            }
            ErrorKind::NotFound => {
                fs::create_dir_all(link_dir).map_err(|source| LinkingError::IO {
                    source,
                    path: link_dir.to_path_buf(),
                })?;
                symlink(actual, &full_link_path).map_err(|source| LinkingError::IO {
                    source,
                    path: full_link_path,
                })?;
                Ok(())
            }
            _ => Err(LinkingError::IO {
                source: e,
                path: full_link_path,
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use target::OperatingSystem;
    use tempfile::tempdir;

    #[test]
    fn linking() {
        let fake_dir = tempdir().unwrap();
        let linked_path = link_node_bins(fake_dir.path()).unwrap();
        assert_eq!(linked_path, fake_dir.path());

        let link_entries: Vec<_> = fs::read_dir(fake_dir.path())
            .unwrap()
            .map(|e| e.unwrap())
            .collect();
        assert_eq!(link_entries.len(), 3);

        let are_links: Vec<bool> = link_entries
            .iter()
            .map(|e| e.metadata().unwrap().file_type().is_symlink())
            .collect();
        let expected = vec![true, true, true];
        assert_eq!(are_links, expected);
    }

    #[test]
    fn already_linked() {
        let fake_dir = tempdir().unwrap();

        let node_path = fake_dir.path().join(NODE_EXECUTABLE);
        let nodeup_path = std::env::current_exe().unwrap();

        symlink(&nodeup_path, node_path).unwrap();

        let linked_path = link_node_bins(fake_dir.path()).unwrap();
        assert_eq!(linked_path, fake_dir.path());
    }

    #[test]
    fn node_already_installed() {
        let fake_dir = tempdir().unwrap();
        let already_installed_node = fake_dir.path().join(NODE_EXECUTABLE);
        File::create(already_installed_node).unwrap();

        let result = link_node_bins(fake_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn link_nonexistent_dir() {
        let fake_dir = tempdir().unwrap();
        let nonexistent_dir = fake_dir.path().join("fake-dir");

        let path = link_node_bins(&nonexistent_dir).unwrap();
        assert_eq!(path, nonexistent_dir);

        let link_entries: Vec<_> = fs::read_dir(nonexistent_dir)
            .unwrap()
            .map(|e| e.unwrap())
            .collect();
        assert_eq!(link_entries.len(), 3);

        let are_links: Vec<bool> = link_entries
            .iter()
            .map(|e| e.metadata().unwrap().file_type().is_symlink())
            .collect();
        let expected = vec![true, true, true];
        assert_eq!(are_links, expected);
    }

    #[test]
    fn get_installed_targets() {
        let fake_dir = tempdir().unwrap();
        let fake_target = Target::new(
            OperatingSystem::Linux,
            Version {
                major: 10,
                minor: 2,
                patch: 3,
            },
        );
        let fake_target_path = fake_dir.path().join(format!("{}", fake_target));
        File::create(&fake_target_path).unwrap();

        let targets = installed_versions(&fake_dir.path()).unwrap();
        assert_eq!(targets, vec![fake_target]);
    }
}
