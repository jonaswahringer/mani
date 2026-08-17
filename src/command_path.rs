use std::fmt;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CommandPath(Vec<String>);

impl CommandPath {
    pub fn new(parts: Vec<String>) -> Result<Self, CommandPathError> {
        if parts.is_empty() {
            return Err(CommandPathError::Empty);
        }

        for part in &parts {
            if part.is_empty() {
                return Err(CommandPathError::EmptyPart);
            }
            if part.starts_with('-') {
                return Err(CommandPathError::Option(part.clone()));
            }
            if part == "." || part == ".." || part.contains('/') || part.contains('\\') {
                return Err(CommandPathError::UnsafePart(part.clone()));
            }
        }

        Ok(Self(parts))
    }

    pub fn parts(&self) -> &[String] {
        &self.0
    }

    pub fn program(&self) -> &str {
        &self.0[0]
    }

    pub fn arguments(&self) -> &[String] {
        &self.0[1..]
    }

    pub fn man_topic(&self) -> String {
        self.0.join("-")
    }

    pub fn guide_relative_path(&self) -> PathBuf {
        let mut path = PathBuf::new();
        for part in &self.0[..self.0.len() - 1] {
            path.push(part);
        }
        path.push(format!(
            "{}.md",
            self.0.last().expect("non-empty command path")
        ));
        path
    }
}

impl fmt::Display for CommandPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0.join(" "))
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum CommandPathError {
    #[error("a Command Path is required")]
    Empty,
    #[error("a Command Path cannot contain an empty part")]
    EmptyPart,
    #[error("lookup does not accept command options or operands: {0}")]
    Option(String),
    #[error("unsafe Command Path part: {0}")]
    UnsafePart(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_command_maps_to_nested_markdown() {
        let command =
            CommandPath::new(vec!["docker".into(), "compose".into(), "up".into()]).unwrap();

        assert_eq!(
            command.guide_relative_path(),
            PathBuf::from("docker/compose/up.md")
        );
        assert_eq!(command.man_topic(), "docker-compose-up");
    }

    #[test]
    fn rejects_options_and_path_traversal() {
        assert_eq!(
            CommandPath::new(vec!["git".into(), "--version".into()]),
            Err(CommandPathError::Option("--version".into()))
        );
        assert_eq!(
            CommandPath::new(vec!["git".into(), "..".into()]),
            Err(CommandPathError::UnsafePart("..".into()))
        );
        assert!(matches!(
            CommandPath::new(vec!["git/rebase".into()]),
            Err(CommandPathError::UnsafePart(_))
        ));
        assert!(matches!(
            CommandPath::new(vec!["git\\rebase".into()]),
            Err(CommandPathError::UnsafePart(_))
        ));
    }

    #[test]
    fn exposes_program_arguments_and_display_form() {
        let command = CommandPath::new(vec!["git".into(), "rebase".into()]).unwrap();

        assert_eq!(command.program(), "git");
        assert_eq!(command.arguments(), ["rebase"]);
        assert_eq!(command.to_string(), "git rebase");
    }
}
