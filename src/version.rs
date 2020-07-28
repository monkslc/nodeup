use anyhow::{anyhow, Context, Result};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: usize,
    pub minor: usize,
    pub patch: usize,
}

impl Version {
    pub fn parse(content: &str) -> Result<Version> {
        let rest = content;

        let (major, rest) = parse_number(rest)
            .with_context(|| format!("Error parsing major version from content: {}", content))?;
        let (_, rest) = parse_dot(rest).with_context(|| {
            format!(
                "Error parsing '.' between major and minor for content: {}",
                content
            )
        })?;

        let (minor, rest) = parse_number(rest)
            .with_context(|| format!("Error parsing minor version from content: {}", content))?;
        let (_, rest) = parse_dot(rest).with_context(|| {
            format!(
                "Error parsing '.' between minor and patch for content: {}",
                content
            )
        })?;
        let (patch, _) = parse_number(rest)
            .with_context(|| format!("Error parsing patch version from content: {}", content))?;

        Ok(Version {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub fn parse_number(content: &str) -> Result<(usize, &str)> {
    let mut number_digits = 0;
    for ch in content.chars() {
        if ch.is_ascii_digit() {
            number_digits += 1;
        } else {
            break;
        }
    }

    if number_digits == 0 {
        Err(anyhow!("Error parsing number from: {}", content))
    } else {
        Ok((
            content[..number_digits].parse().unwrap(),
            &content[number_digits..],
        ))
    }
}

pub fn parse_dot(content: &str) -> Result<((), &str)> {
    if let Some(".") = content.get(0..1) {
        Ok(((), &content[1..]))
    } else {
        Err(anyhow!("Error parsing the dot from content: {}", content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version() {
        let content = "12.15.1";
        let actual = Version::parse(content).unwrap();
        let expected = Version {
            major: 12,
            minor: 15,
            patch: 1,
        };
        assert_eq!(actual, expected);
    }
}