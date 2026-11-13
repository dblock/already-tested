use crate::{Error, Result, fail};
use std::str::FromStr;

/// CI integration policy. Automatic detection falls back to GitHub Actions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ci {
    #[default]
    Auto,
    GitHubActions,
}

impl Ci {
    /// Resolve automatic selection to the supported GitHub Actions integration.
    pub fn resolve(self) -> Result<Self> {
        Ok(match self {
            Self::Auto => Self::GitHubActions,
            system => system,
        })
    }
}

impl FromStr for Ci {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "auto" => Ok(Self::Auto),
            "github-actions" => Ok(Self::GitHubActions),
            _ => fail(format!(
                "Unknown CI system: {value}. Expected auto or github-actions."
            )),
        }
    }
}
