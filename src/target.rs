use log::debug;
use serde::{Deserialize, Serialize};
use std::{
    cmp::{Ord, Ordering, PartialOrd},
    fmt,
};
use thiserror::Error;

type ParseResult<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected character found.\nExpected: {expected:?}\nFound: {found:?}")]
    UnexpectedChar { expected: char, found: char },

    #[error("Unexpected end of input")]
    UnexpectedEndOfInput,

    #[error("Not a valid number: {content:?}")]
    InvalidNumber { content: String },
}

#[derive(Debug, Error)]
pub enum VersionError {
    #[error("Couldn't parse major version: {source}")]
    Major { source: ParseError },

    #[error("Couldn't parse minor version: {source}")]
    Minor { source: ParseError },

    #[error("Couldn't parse patch version: {source}")]
    Patch { source: ParseError },
}

#[derive(Debug, Error)]
pub enum TargetError {
    #[error("Couldn't parse version from the target: {source}")]
    Version {
        #[from]
        source: VersionError,
    },

    #[error("Failed to find spearator after: {after}: {source}")]
    Separator {
        source: ParseError,
        after: &'static str,
    },

    #[error("Failed to parse operating system: {source}")]
    OperatingSystem {
        #[from]
        source: OperatingSystemError,
    },
}

#[derive(Debug, Error)]
pub enum OperatingSystemError {
    #[error("Unrecognized operating system: {0}. Valid values are: linux, macos, and windows")]
    Unrecognized(String),
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub struct Target {
    os: OperatingSystem,
    version: Version,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct Version {
    pub major: usize,
    pub minor: usize,
    pub patch: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum OperatingSystem {
    Darwin,
    Linux,
    Windows,
}

impl Target {
    pub fn new(os: OperatingSystem, version: Version) -> Self {
        Target { os, version }
    }

    // content is expected to look like: node-v12.9.1-linux-x64
    pub fn parse(content: &str) -> std::result::Result<Self, TargetError> {
        debug!("Target parsing content: {}", content);
        // skip "node-"
        let rest = &content[5..];

        let end_index = rest
            .chars()
            .position(|ch| ch == '-')
            .unwrap_or_else(|| rest.len());
        let (version_string, rest) = (&rest[..end_index], &rest[end_index..]);
        let version = Version::parse(version_string)?;

        let (_, rest) = parse_dash(rest).map_err(|e| TargetError::Separator {
            after: "version",
            source: e,
        })?;

        let end_index = rest
            .chars()
            .position(|ch| ch == '-')
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

impl Version {
    pub fn parse(content: &str) -> std::result::Result<Version, VersionError> {
        debug!("Parsing Version: {}", content);
        let rest = match content.chars().next() {
            Some('v') => &content[1..],
            _ => content,
        };

        let (major, rest) = parse_number(rest).map_err(|e| VersionError::Major { source: e })?;
        let (_, rest) = parse_dot(rest).map_err(|e| VersionError::Minor { source: e })?;

        let (minor, rest) = parse_number(rest).map_err(|e| VersionError::Minor { source: e })?;
        let (_, rest) = parse_dot(rest).map_err(|e| VersionError::Patch { source: e })?;

        let (patch, _) = parse_number(rest).map_err(|e| VersionError::Patch { source: e })?;

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

impl OperatingSystem {
    pub fn parse(content: &str) -> std::result::Result<Self, OperatingSystemError> {
        match content {
            "linux" => Ok(OperatingSystem::Linux),
            "win" => Ok(OperatingSystem::Windows),
            "darwin" => Ok(OperatingSystem::Darwin),
            _ => Err(OperatingSystemError::Unrecognized(content.to_string())),
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

pub fn parse_number(content: &str) -> ParseResult<(usize, &str)> {
    let end_index = content
        .chars()
        .position(|ch| !ch.is_ascii_digit())
        .unwrap_or_else(|| content.len());

    let (major_string, rest) = (&content[..end_index], &content[end_index..]);

    let major: usize = major_string
        .parse()
        .map_err(|_| ParseError::InvalidNumber {
            content: content.to_string(),
        })?;

    Ok((major, rest))
}

pub fn parse_dot(content: &str) -> ParseResult<(char, &str)> {
    take_char('.', content)
}

pub fn parse_dash(content: &str) -> ParseResult<(char, &str)> {
    take_char('-', content)
}

pub fn take_char(expected: char, content: &str) -> ParseResult<(char, &str)> {
    match content.chars().next() {
        Some(ch) if ch == expected => Ok((ch, &content[1..])),
        Some(ch) => Err(ParseError::UnexpectedChar {
            expected,
            found: ch,
        }),
        None => Err(ParseError::UnexpectedEndOfInput),
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

    #[test]
    fn parse_target() {
        let target_string = "node-v12.15.1-linux-x64";

        let actual = Target::parse(target_string).unwrap();
        let expected = Target::new(
            OperatingSystem::Linux,
            Version {
                major: 12,
                minor: 15,
                patch: 1,
            },
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_different_target() {
        let target_string = "node-v1.1.1000-linux-x64";

        let actual = Target::parse(target_string).unwrap();
        let expected = Target::new(
            OperatingSystem::Linux,
            Version {
                major: 1,
                minor: 1,
                patch: 1000,
            },
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_another_target() {
        let target_string = "node-v1000.1000.1000-linux-x64";

        let actual = Target::parse(target_string).unwrap();
        let expected = Target::new(
            OperatingSystem::Linux,
            Version {
                major: 1000,
                minor: 1000,
                patch: 1000,
            },
        );

        assert_eq!(actual, expected);
    }

    #[test]
    #[ignore] // Comment out to see error messages
    fn error_messages() {
        let target_string = "node-v12.15.1-linux-x64";
        println!("{}\n{:?}\n", target_string, Target::parse(target_string));

        let target_string = "node-v12a.15.1-linux-x64";
        println!(
            "{}\n{}\n",
            target_string,
            Target::parse(target_string).unwrap_err()
        );

        let target_string = "node-v12.15-linux-x64";
        println!(
            "{}\n{}\n",
            target_string,
            Target::parse(target_string).unwrap_err()
        );

        let target_string = "node-v12.-linux-x64";
        println!(
            "{}\n{}\n",
            target_string,
            Target::parse(target_string).unwrap_err()
        );

        let target_string = "node-v12.15.2linux-x64";
        println!(
            "{}\n{}\n",
            target_string,
            Target::parse(target_string).unwrap_err()
        );

        let target_string = "node-v12.15.1-faker-x64";
        println!(
            "{}\n{}\n",
            target_string,
            Target::parse(target_string).unwrap_err()
        );

        assert_eq!(true, false);
    }
}
