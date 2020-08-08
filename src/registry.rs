use flate2::read::GzDecoder;
use log::debug;
use reqwest::{blocking, StatusCode};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tar::Archive;
use thiserror::Error;

use crate::target::{Target, Version};

const BASE_URL: &str = "https://nodejs.org/dist/";

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Error making request to {:?}: {source}", source.url())]
    Request { source: reqwest::Error },

    #[error("Unexpected response from {url:?}: {source}")]
    UnexpectedResponse {
        source: serde_json::Error,
        url: String,
    },

    #[error("Error writing the download out to disk at location: {path:?}: {source}")]
    IO {
        source: std::io::Error,
        path: PathBuf,
    },

    #[error("Target {target} does not exist")]
    InvalidTarget { target: Target },

    #[error("Unexpected result from {url:?}: {code}")]
    UnexpectedResult {
        url: String,
        code: reqwest::StatusCode,
    },
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

pub fn download_node_toolchain(location: &Path, target: Target) -> Result<(), RegistryError> {
    let url = get_node_download_url(target);
    debug!("Downloading node at url: {}", target);

    let tar_gzip = blocking::get(&url).map_err(|source| RegistryError::Request { source })?;
    match tar_gzip.status() {
        StatusCode::OK => {
            let tar = GzDecoder::new(tar_gzip);
            let mut arc = Archive::new(tar);
            arc.unpack(location).map_err(|source| RegistryError::IO {
                source,
                path: location.to_path_buf(),
            })?;
            Ok(())
        }
        StatusCode::NOT_FOUND => Err(RegistryError::InvalidTarget { target }),
        code => Err(RegistryError::UnexpectedResult { url, code }),
    }
}

pub fn get_latest_lts() -> Result<Version, RegistryError> {
    let url = format!("{}index.json", BASE_URL);
    debug!("Fetching node lts from: {}", url);

    let resp = blocking::get(&url).map_err(|source| RegistryError::Request { source })?;

    let all_versions: Vec<AvailableVersion> =
        serde_json::from_reader(resp).map_err(|source| RegistryError::UnexpectedResponse {
            source,
            url: url.to_string(),
        })?;

    let latest_lts = all_versions
        .into_iter()
        .filter_map(|v| match v.lts {
            LTSVersion::Yes(_) => Some(
                Version::parse(&v.version)
                    .unwrap_or_else(|_| panic!("Error parsing verson from node registry: {:?}", v)),
            ),
            _ => None,
        })
        .max()
        .expect("Received no lts versions from the node distribution registry");

    Ok(latest_lts)
}

// Full url example: https://nodejs.org/dist/v12.9.1/node-v12.9.1-linux-x64.tar.gz
fn get_node_download_url(target: Target) -> String {
    let full_url = format!("{}{}/{}.tar.gz", BASE_URL, target.version(), target);
    full_url
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::OperatingSystem;
    use crate::target::Version;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn create_node_url() {
        let version = Version {
            major: 12,
            minor: 9,
            patch: 1,
        };

        let actual = get_node_download_url(Target::from_version(version));
        let expected = "https://nodejs.org/dist/v12.9.1/node-v12.9.1-linux-x64.tar.gz";
        assert_eq!(actual, expected);
    }

    #[test]
    #[ignore] // Take a little too long to run
    fn download_node_to_temp_dir() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path();

        let target = Target::new(
            OperatingSystem::Linux,
            Version {
                major: 12,
                minor: 0,
                patch: 0,
            },
        );

        download_node_toolchain(path, target).unwrap();

        let downloaded_path = path.join("node-v12.0.0-linux-x64");
        fs::read_dir(downloaded_path).unwrap();
    }

    #[test]
    fn latest_lts() {
        get_latest_lts().unwrap();
    }
}
