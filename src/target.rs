use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    cmp::{Ord, Ordering, PartialOrd},
    fmt,
};

#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub struct Target {
    os: OperatingSystem,
    version: Version,
}

impl Target {
    pub fn new(os: OperatingSystem, version: Version) -> Self {
        Target { os, version }
    }

    // content is expected to look like: node-v12.9.1-linux-x64
    pub fn parse(content: &str) -> Result<Self> {
        // skip "node-"
        let rest = &content[6..];

        let end_index = rest
            .chars()
            .position(|ch| ch != '-')
            .unwrap_or_else(|| rest.len());
        let (version_string, rest) = (&rest[..end_index], &rest[end_index..]);
        let version = Version::parse(version_string)?;

        let end_index = rest
            .chars()
            .position(|ch| ch != '-')
            .unwrap_or_else(|| rest.len());
        let (os_string, _) = (&rest[..end_index], &rest[end_index..]);
        let os = OperatingSystem::parse(os_string)?;

        // TODO: add parsing arch
        Ok(Target::new(os, version))
    }

    pub fn from_version(version: Version) -> Self {
        Target::new(Default::default(), version)
    }

    pub fn version(&self) -> Version {
        self.version
    }
}

/* display is implemented to match the last part of the download url path which also matches how it
 * is stored in the file system
 * ex/ node-v12.9.1-linux-x64
 */
impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "node-{}-{}-x64",
            self.version(),
            OperatingSystem::default()
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum OperatingSystem {
    Darwin,
    Linux,
    Windows,
}

impl OperatingSystem {
    // content should match the name of the os in the download url
    pub fn parse(content: &str) -> Result<Self> {
        match content {
            "linux" => Ok(OperatingSystem::Linux),
            "win" => Ok(OperatingSystem::Windows),
            "darwin" => Ok(OperatingSystem::Darwin),
            _ => Err(anyhow!("Uncrecognized operating system: {}", content)),
        }
    }
}

impl Default for OperatingSystem {
    #[cfg(target_os = "linux")]
    fn default() -> Self {
        OperatingSystem::Linux
    }

    #[cfg(target_os = "windows")]
    fn default() -> Self {
        OperatingSystem::Windows
    }

    #[cfg(target_os = "macos")]
    fn default() -> Self {
        OperatingSystem::Darwin
    }
}

/*
 * Display is implemented so the os is formatted according to how it appears in the node download
 * url
 */
impl fmt::Display for OperatingSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use OperatingSystem::*;
        match self {
            Darwin => write!(f, "darwin"),
            Linux => write!(f, "linux"),
            Windows => write!(f, "win"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct Version {
    pub major: usize,
    pub minor: usize,
    pub patch: usize,
}

impl Version {
    pub fn parse(content: &str) -> Result<Version> {
        let rest = match content.chars().next() {
            Some('v') => &content[1..],
            _ => content,
        };

        let (major, rest) = parse_number(rest).context("Error parsing major version")?;
        let (_, rest) = parse_dot(rest).context("Error parsing dot after major version")?;

        let (minor, rest) = parse_number(rest).context("Error parsing minor versiono")?;
        let (_, rest) = parse_dot(rest).context("Error parsing dot after minor version")?;

        let (patch, _) = parse_number(rest).context("Error parsing patch version")?;

        Ok(Version {
            major,
            minor,
            patch,
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => self.patch.cmp(&other.patch),
                o => o,
            },
            o => o,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub fn parse_number(content: &str) -> Result<(usize, &str)> {
    let end_index = content
        .chars()
        .position(|ch| !ch.is_ascii_digit())
        .unwrap_or_else(|| content.len());

    let (major_string, rest) = (&content[..end_index], &content[end_index..]);

    let major: usize = major_string
        .parse()
        .with_context(|| format!("Error parsing number from content: {:?}", major_string))?;

    Ok((major, rest))
}

pub fn parse_dot(content: &str) -> Result<((), &str)> {
    match content.chars().next() {
        Some('.') => Ok(((), &content[1..])),
        _ => Err(anyhow!("Error parsing the dot from content: {}", content)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version() {
        let expected = Version {
            major: 12,
            minor: 15,
            patch: 1,
        };

        let content = "12.15.1";
        let actual = Version::parse(content).unwrap();
        assert_eq!(actual, expected);

        let content = "v12.15.1";
        let actual = Version::parse(content).unwrap();
        assert_eq!(actual, expected);
    }
}
