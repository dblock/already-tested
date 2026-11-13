use crate::{Error, Result, fail};
use std::{env, str::FromStr};

/// CI integration policy. Automatic detection falls back to GitHub Actions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ci {
    #[default]
    Auto,
    GitHubActions,
    AzurePipelines,
}

impl Ci {
    /// Resolve CI signals without inspecting remotes or repository files.
    /// Conflicting signals require an explicit choice.
    pub fn resolve(self) -> Result<Self> {
        if self != Self::Auto {
            return Ok(self);
        }
        let enabled = |name| env::var(name).is_ok_and(|value| value.eq_ignore_ascii_case("true"));
        match (enabled("GITHUB_ACTIONS"), enabled("TF_BUILD")) {
            (true, true) => {
                fail("Both GITHUB_ACTIONS and TF_BUILD are enabled; select --ci explicitly.")
            }
            (_, true) => Ok(Self::AzurePipelines),
            _ => Ok(Self::GitHubActions),
        }
    }
}

impl FromStr for Ci {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "auto" => Ok(Self::Auto),
            "github-actions" => Ok(Self::GitHubActions),
            "azure-pipelines" => Ok(Self::AzurePipelines),
            _ => fail(format!(
                "Unknown CI system: {value}. Expected auto, github-actions, or azure-pipelines."
            )),
        }
    }
}
