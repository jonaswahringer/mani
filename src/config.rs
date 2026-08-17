use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub generator: GeneratorConfig,
    pub history: HistoryConfig,
    pub issues: IssuesConfig,
    pub theme: Theme,
}

impl Config {
    pub fn load(path: &Path, ignore: bool) -> Result<Self, ConfigError> {
        if ignore || !path.exists() {
            return Ok(Self::default());
        }

        let input = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let config: Self = toml::from_str(&input).map_err(|source| {
            let (line, column) = source
                .span()
                .map(|span| line_and_column(&input[..span.start]))
                .unwrap_or((1, 1));
            ConfigError::Parse {
                path: path.to_path_buf(),
                line,
                column,
                detail: source.message().to_owned(),
            }
        })?;
        config.validate(path)?;
        Ok(config)
    }

    fn validate(&self, path: &Path) -> Result<(), ConfigError> {
        if self.history.limit > 100 {
            return Err(ConfigError::Invalid {
                path: path.to_path_buf(),
                key: "history.limit",
                value: self.history.limit.to_string(),
                suggestion: "use a value from 0 through 100",
            });
        }
        if self.generator.inactivity_warning_seconds == 0 {
            return Err(ConfigError::Invalid {
                path: path.to_path_buf(),
                key: "generator.inactivity_warning_seconds",
                value: "0".into(),
                suggestion: "use a positive number of seconds",
            });
        }
        if self.generator.timeout_seconds == 0 {
            return Err(ConfigError::Invalid {
                path: path.to_path_buf(),
                key: "generator.timeout_seconds",
                value: "0".into(),
                suggestion: "use a positive number of seconds",
            });
        }
        if self.issues.refine_after == 0
            || self.issues.window_days == 0
            || self.issues.reprompt_multiplier < 2
        {
            return Err(ConfigError::Invalid {
                path: path.to_path_buf(),
                key: "issues",
                value: format!(
                    "refine_after={}, window_days={}, reprompt_multiplier={}",
                    self.issues.refine_after,
                    self.issues.window_days,
                    self.issues.reprompt_multiplier
                ),
                suggestion: "use positive thresholds and a reprompt_multiplier of at least 2",
            });
        }
        for (key, value) in [
            ("theme.heading", self.theme.heading.as_str()),
            ("theme.code", self.theme.code.as_str()),
            ("theme.warning", self.theme.warning.as_str()),
            ("theme.active_source", self.theme.active_source.as_str()),
            ("theme.muted", self.theme.muted.as_str()),
        ] {
            if !valid_style(value) {
                return Err(ConfigError::Invalid {
                    path: path.to_path_buf(),
                    key,
                    value: value.to_owned(),
                    suggestion: "use color names and the modifiers bold, dim, or reverse",
                });
            }
        }
        Ok(())
    }
}

fn valid_style(value: &str) -> bool {
    !value.trim().is_empty()
        && value.split_whitespace().all(|part| {
            matches!(
                part,
                "black"
                    | "red"
                    | "green"
                    | "yellow"
                    | "blue"
                    | "magenta"
                    | "cyan"
                    | "white"
                    | "bold"
                    | "dim"
                    | "reverse"
            )
        })
}

fn line_and_column(before: &str) -> (usize, usize) {
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = before.rsplit('\n').next().map(str::len).unwrap_or(0) + 1;
    (line, column)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct GeneratorConfig {
    pub command: Vec<String>,
    pub inactivity_warning_seconds: u64,
    pub timeout_seconds: u64,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            command: Vec::new(),
            inactivity_warning_seconds: 35,
            timeout_seconds: 270,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct HistoryConfig {
    pub limit: usize,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self { limit: 20 }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct IssuesConfig {
    pub refine_after: u64,
    pub window_days: u64,
    pub reprompt_multiplier: u64,
}

impl Default for IssuesConfig {
    fn default() -> Self {
        Self {
            refine_after: 3,
            window_days: 30,
            reprompt_multiplier: 2,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Theme {
    pub heading: String,
    pub code: String,
    pub warning: String,
    pub active_source: String,
    pub muted: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            heading: "cyan".into(),
            code: "yellow".into(),
            warning: "yellow bold".into(),
            active_source: "green reverse".into(),
            muted: "dim".into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "invalid configuration in {path}:{line}:{column}: {detail}; correct the value or use --ignore-config"
    )]
    Parse {
        path: PathBuf,
        line: usize,
        column: usize,
        detail: String,
    },
    #[error(
        "invalid configuration in {path}: {key} = {value}; {suggestion}, or use --ignore-config"
    )]
    Invalid {
        path: PathBuf,
        key: &'static str,
        value: String,
        suggestion: &'static str,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_uses_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let config = Config::load(&directory.path().join("config.toml"), false).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn rejects_unknown_keys() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, "mystery = true\n").unwrap();

        assert!(matches!(
            Config::load(&path, false),
            Err(ConfigError::Parse { .. })
        ));
    }

    #[test]
    fn validates_history_limit() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, "[history]\nlimit = 101\n").unwrap();

        assert!(matches!(
            Config::load(&path, false),
            Err(ConfigError::Invalid {
                key: "history.limit",
                ..
            })
        ));
    }

    #[test]
    fn loads_complete_valid_configuration() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(
            &path,
            "[generator]\ncommand = [\"model\", \"-\"]\ninactivity_warning_seconds = 10\ntimeout_seconds = 60\n\n[history]\nlimit = 0\n\n[issues]\nrefine_after = 4\nwindow_days = 14\nreprompt_multiplier = 3\n\n[theme]\nheading = \"blue bold\"\ncode = \"green\"\nwarning = \"red bold\"\nactive_source = \"cyan reverse\"\nmuted = \"dim\"\n",
        )
        .unwrap();

        let config = Config::load(&path, false).unwrap();

        assert_eq!(config.generator.command, ["model", "-"]);
        assert_eq!(config.history.limit, 0);
        assert_eq!(config.theme.heading, "blue bold");
    }

    #[test]
    fn ignore_config_uses_defaults_for_a_broken_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, "unknown = true\n").unwrap();

        assert_eq!(Config::load(&path, true).unwrap(), Config::default());
    }

    #[test]
    fn rejects_unknown_theme_tokens_with_the_failing_value() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, "[theme]\nheading = \"ultraviolet\"\n").unwrap();

        assert!(matches!(
            Config::load(&path, false),
            Err(ConfigError::Invalid {
                key: "theme.heading",
                value,
                ..
            }) if value == "ultraviolet"
        ));
    }
}
