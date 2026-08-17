use std::env;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppPaths {
    pub knowledge_base_root: PathBuf,
    pub private_state_root: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Result<Self, PathError> {
        Self::resolve_with(|key| env::var_os(key).map(PathBuf::from))
    }

    fn resolve_with(mut variable: impl FnMut(&str) -> Option<PathBuf>) -> Result<Self, PathError> {
        let home = variable("HOME");

        let knowledge_base_root = if let Some(path) = variable("MANI_HOME") {
            path
        } else if let Some(path) = variable("XDG_CONFIG_HOME") {
            path.join("mani")
        } else {
            home.as_ref()
                .ok_or(PathError::HomeUnavailable)?
                .join(".config/mani")
        };

        let private_state_root = if let Some(path) = variable("XDG_STATE_HOME") {
            path.join("mani")
        } else {
            home.as_ref()
                .ok_or(PathError::HomeUnavailable)?
                .join(".local/state/mani")
        };

        Ok(Self {
            knowledge_base_root,
            private_state_root,
        })
    }

    pub fn config_file(&self) -> PathBuf {
        self.knowledge_base_root.join("config.toml")
    }
}

#[derive(Debug, Error)]
pub enum PathError {
    #[error("cannot resolve Mani paths because HOME is not set")]
    HomeUnavailable,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn from(values: &[(&str, &str)]) -> AppPaths {
        let values: HashMap<_, _> = values.iter().copied().collect();
        AppPaths::resolve_with(|key| values.get(key).map(|value| PathBuf::from(*value))).unwrap()
    }

    #[test]
    fn mani_home_has_priority() {
        let paths = from(&[
            ("HOME", "/home/me"),
            ("XDG_CONFIG_HOME", "/config"),
            ("MANI_HOME", "/guides"),
        ]);

        assert_eq!(paths.knowledge_base_root, PathBuf::from("/guides"));
    }

    #[test]
    fn uses_xdg_then_home_fallbacks() {
        let xdg = from(&[
            ("HOME", "/home/me"),
            ("XDG_CONFIG_HOME", "/config"),
            ("XDG_STATE_HOME", "/state"),
        ]);
        assert_eq!(xdg.knowledge_base_root, PathBuf::from("/config/mani"));
        assert_eq!(xdg.private_state_root, PathBuf::from("/state/mani"));

        let fallback = from(&[("HOME", "/home/me")]);
        assert_eq!(
            fallback.knowledge_base_root,
            PathBuf::from("/home/me/.config/mani")
        );
        assert_eq!(
            fallback.private_state_root,
            PathBuf::from("/home/me/.local/state/mani")
        );
    }

    #[test]
    fn xdg_paths_do_not_require_home() {
        let paths = from(&[("XDG_CONFIG_HOME", "/config"), ("XDG_STATE_HOME", "/state")]);

        assert_eq!(paths.knowledge_base_root, PathBuf::from("/config/mani"));
        assert_eq!(paths.private_state_root, PathBuf::from("/state/mani"));
    }

    #[test]
    fn missing_home_is_an_error_when_a_fallback_is_needed() {
        assert!(matches!(
            AppPaths::resolve_with(|_| None),
            Err(PathError::HomeUnavailable)
        ));
    }
}
