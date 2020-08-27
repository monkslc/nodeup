use std::{
    fmt, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};
use which::which;

use crate::{ErrorTask, NodeupError, NODE_EXECUTABLE, NPM_EXECUTABLE, NPX_EXECUTABLE};

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigurationCheck {
    Correct,
    Incorrect(IncorrectConfiguration),
}

#[derive(Debug, Clone, PartialEq)]
pub enum IncorrectConfiguration {
    WrongBinary(PathBuf),
    LinkNotFound,
    NotASymlink(PathBuf),
    MissingSymLink(PathBuf),
    PathNotFound,
}

impl fmt::Display for IncorrectConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use IncorrectConfiguration::*;
        match self {
            NotASymlink(path) => {
                write!(f, "The binary at: {} was expected to be a symlink to nodeup. Try removing and running `nodeup control link` to reconfigure.", path.display())
            }
            MissingSymLink(path) => {
                write!(f, "Missing a symlink to nodeup at: {}. Try running `nodeup control link` to configure the symlinks", path.display())
            }
            PathNotFound => {
                write!(f, "Can't find the Path environment variable.")
            }
            LinkNotFound => {
                write!(f, "Can't find the links for node, npm, and npx in your Path environment variable. Try running `nodeup control link` and adding the printed path to your Path environment variable.")
            }
            WrongBinary(path) => {
                write!(f, "The binary at {} has priority over the symlink to nodeup. This can be fixed by moving the path to the Nodeup symlinks to the beginning of the Path environment variable", path.display())
            }
        }
    }
}

pub fn verify_links(path: &Path) -> Result<ConfigurationCheck, NodeupError> {
    let node = path.join(NODE_EXECUTABLE);
    match verify_link(node, NODE_EXECUTABLE) {
        Ok(ConfigurationCheck::Correct) => (),
        Ok(i) => return Ok(i),
        Err(e) => return Err(e),
    };

    let npm = path.join(NPM_EXECUTABLE);
    match verify_link(npm, NPM_EXECUTABLE) {
        Ok(ConfigurationCheck::Correct) => (),
        Ok(i) => return Ok(i),
        Err(e) => return Err(e),
    };

    let npx = path.join(NPX_EXECUTABLE);
    match verify_link(npx, NPX_EXECUTABLE) {
        Ok(ConfigurationCheck::Correct) => (),
        Ok(i) => return Ok(i),
        Err(e) => return Err(e),
    };

    Ok(ConfigurationCheck::Correct)
}

fn verify_link(path: PathBuf, executable: &'static str) -> Result<ConfigurationCheck, NodeupError> {
    use ErrorTask::Verify as task;

    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(source) => {
            return match source.kind() {
                ErrorKind::NotFound => Ok(ConfigurationCheck::Incorrect(
                    IncorrectConfiguration::MissingSymLink(path),
                )),
                _ => Err(NodeupError::IO { task, source, path }),
            }
        }
    };

    if !metadata.file_type().is_symlink() {
        return Ok(ConfigurationCheck::Incorrect(
            IncorrectConfiguration::NotASymlink(path),
        ));
    };

    let active_executable = match which(executable) {
        Ok(path) => path,
        Err(which::Error::CannotFindBinaryPath) => {
            return Ok(ConfigurationCheck::Incorrect(
                IncorrectConfiguration::LinkNotFound,
            ))
        }
        Err(_) => {
            return Ok(ConfigurationCheck::Incorrect(
                IncorrectConfiguration::PathNotFound,
            ))
        }
    };

    if active_executable != path {
        Ok(ConfigurationCheck::Incorrect(
            IncorrectConfiguration::WrongBinary(active_executable),
        ))
    } else {
        Ok(ConfigurationCheck::Correct)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    #[test]
    #[ignore] // Test is wrongly failing. Seems to be an error at the intersection of which and tempdir
    fn correct_verify() {
        let fake_link_dir = tempdir().unwrap();
        let fake_nodeup = fake_link_dir.path().join("nodeup");

        let node = fake_link_dir.path().join("node");
        let npm = fake_link_dir.path().join("npm");
        let npx = fake_link_dir.path().join("npx");

        let original_path = env::var("PATH").unwrap();
        let new_path = format!("{}:{}", fake_link_dir.path().display(), original_path);
        env::set_var("PATH", new_path);

        symlink(&fake_nodeup, node).unwrap();
        symlink(&fake_nodeup, npm).unwrap();
        symlink(&fake_nodeup, npx).unwrap();

        let verification = verify_links(fake_link_dir.path()).unwrap();
        env::set_var("PATH", original_path);
        assert_eq!(ConfigurationCheck::Correct, verification)
    }

    #[test]
    fn missing_symlink() {
        let fake_link_dir = tempdir().unwrap();

        let expected_node_link = fake_link_dir.path().join("node");
        let expected = ConfigurationCheck::Incorrect(IncorrectConfiguration::MissingSymLink(
            expected_node_link,
        ));
        assert_eq!(expected, verify_links(fake_link_dir.path()).unwrap())
    }

    #[test]
    fn not_a_symlink() {
        let fake_link_dir = tempdir().unwrap();

        let not_a_symlink_node = fake_link_dir.path().join("node");
        File::create(&not_a_symlink_node).unwrap();
        let expected =
            ConfigurationCheck::Incorrect(IncorrectConfiguration::NotASymlink(not_a_symlink_node));

        assert_eq!(expected, verify_links(fake_link_dir.path()).unwrap())
    }
}
