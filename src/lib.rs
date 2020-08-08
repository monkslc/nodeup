use anyhow::{anyhow, Context, Result};
use log::debug;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env, fs,
    fs::OpenOptions,
    io::{ErrorKind, Read},
    os::unix::{fs::symlink, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

pub mod local;
pub mod registry;
mod target;

pub use registry::get_latest_lts;
pub use target::{Target, Version};

const NODE_EXECUTABLE: &str = "nodeup";
const NPM_EXECUTABLE: &str = "npm";
const NPX_EXECUTABLE: &str = "npx";

// TODO: check that the version is installed before removing
pub fn remove_node(target: Target) -> Result<()> {
    let path = local::target_path(&target)?;
    fs::remove_dir_all(path)
        .with_context(|| format!("Error removing {}. Maybe it wasn't installed?", target))?;
    Ok(())
}

pub fn installed_versions(path: &Path) -> Result<Vec<Target>> {
    let entries = fs::read_dir(path)
        .with_context(|| format!("Error reading entries in directory: {}", path.display()))?;

    let target_paths = entries.filter_map(|dir| match dir {
        Ok(dir) => Some(dir),
        Err(e) => {
            debug!(
                "IO Error while trying to read targets in: {}\n{}",
                path.display(),
                e
            );
            None
        }
    });

    let target_filenames = target_paths.map(|dir| dir.file_name());

    let targets = target_filenames.filter_map(|dir| match dir.to_str() {
        Some(target_name) => match Target::parse(target_name) {
            Ok(target) => Some(target),
            Err(e) => {
                debug!(
                    "Error parsing target: {}\n{}",
                    dir.to_str().unwrap_or("[unknown]"),
                    e
                );
                None
            }
        },
        None => {
            debug!(
                "Error trying to convert: {} to a str",
                dir.to_str().unwrap_or("[error]")
            );
            None
        }
    });

    Ok(targets.collect())
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    #[serde(default)]
    version_mappings: HashMap<PathBuf, Target>,
}

fn get_config_file() -> Result<Config> {
    let config_file = local::config_file()?;

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&config_file)
        .with_context(|| {
            format!(
                "Failed to open config file at path: {}",
                &config_file.to_str().unwrap_or("unknown")
            )
        })?;

    let mut content = Vec::new();
    file.read_to_end(&mut content)
        .context("Error reading file")?;

    let config: Config =
        toml::from_slice(&content[..]).context("Error deserializing config file")?;

    Ok(config)
}

// TODO: check that the version is installed
pub fn change_default_target(target: Target) -> Result<()> {
    let mut config = get_config_file()?;
    config
        .version_mappings
        .insert(PathBuf::from_str("default").unwrap(), target);

    let updated_contents = toml::to_vec(&config).context("Error deserializing settings.toml")?;

    let updated_config_file = local::transitory_config_file()?;

    fs::write(&updated_config_file, updated_contents)
        .context("Error writing updated config file .updated.settings.toml")?;

    let config_file = local::config_file()?;
    fs::rename(&updated_config_file, config_file)
        .context("Error writing updates to settings.toml")?;

    Ok(())
}

pub fn active_versions() -> Result<Vec<(PathBuf, Target)>> {
    let config = get_config_file()?;
    Ok(config.version_mappings.into_iter().collect())
}

pub fn link_node_bins(links_path: &Path) -> Result<PathBuf> {
    let nodeup_path = std::env::current_exe()?;

    let node_path = links_path.join(NODE_EXECUTABLE);
    link_bin(&nodeup_path, &node_path)?;

    let npm_path = links_path.join(NPM_EXECUTABLE);
    link_bin(&nodeup_path, &npm_path)?;

    let npx_path = links_path.join(NPX_EXECUTABLE);
    link_bin(&nodeup_path, &npx_path)?;

    Ok(links_path.to_path_buf())
}

fn link_bin(actual: &Path, facade: &Path) -> Result<()> {
    match symlink(actual, facade) {
        Ok(_) => Ok(()),
        Err(e) => match e.kind() {
            ErrorKind::AlreadyExists => {
                let metadata = std::fs::symlink_metadata(facade)?;
                match metadata.file_type().is_symlink() {
                    true => Ok(()),
                    false => Err(anyhow!("It appears like something already exists at: {}. Try deleting and linking again.", facade.to_str().unwrap_or("[unknown]")))
                }
            }
            ErrorKind::NotFound => {
                let links_dir = facade.parent().ok_or_else(|| {
                    anyhow!(
                        "Error creating the symlink dir at parent of: {}",
                        facade.to_str().unwrap_or("[error]")
                    )
                })?;
                fs::create_dir_all(links_dir)?;
                symlink(actual, facade)?;
                Ok(())
            }
            _ => Err(anyhow!("{}", e)),
        },
    }
}

pub fn execute_bin<I: std::iter::Iterator<Item = String>>(bin: &str, args: I) -> Result<()> {
    let config = get_config_file()?;
    if let Some(target) = config.version_mappings.get(Path::new("default")) {
        let bin_path = local::target_path(target)?.join("bin").join(bin);

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

    let updated_config_file = local::transitory_config_file()?;

    fs::write(&updated_config_file, updated_contents).with_context(|| {
        anyhow!(
            "Error writing to transitory update file: {}",
            updated_config_file.path().display(),
        )
    })?;

    let config_file = local::config_file()?;
    fs::rename(&updated_config_file, &config_file).with_context(|| {
        format!(
            "Error writing updates to {}",
            &config_file.to_str().unwrap_or("unknown")
        )
    })?;

    Ok(())
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
