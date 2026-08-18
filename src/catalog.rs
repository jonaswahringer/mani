use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use thiserror::Error;
use wait_timeout::ChildExt;

use crate::command_path::CommandPath;

const HELP_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceKind {
    Custom,
    Official,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Heading {
    pub level: u8,
    pub title: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    pub content: String,
    pub source_label: String,
    pub headings: Vec<Heading>,
}

impl Document {
    fn custom(content: String, source: &Path) -> Result<Self, GuideError> {
        validate_custom_guide(&content)?;
        let headings = markdown_headings(&content);
        Ok(Self {
            content,
            source_label: source.display().to_string(),
            headings,
        })
    }

    fn official(content: String, source_label: String) -> Self {
        let content = clean_terminal_text(&content);
        let headings = plain_text_headings(&content);
        Self {
            content,
            source_label,
            headings,
        }
    }

    pub fn short_custom_content(&self) -> &str {
        let end = self
            .headings
            .iter()
            .find(|heading| heading.level == 2)
            .and_then(|heading| byte_offset_for_line(&self.content, heading.line))
            .unwrap_or(self.content.len());
        self.content[..end].trim_end()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelpTopic {
    pub command_path: CommandPath,
    pub custom: Option<Document>,
    pub official: Option<Document>,
    custom_guide_path: PathBuf,
    command_help: Option<Document>,
}

impl HelpTopic {
    pub fn document(&self, source: SourceKind) -> Option<&Document> {
        match source {
            SourceKind::Custom => self.custom.as_ref(),
            SourceKind::Official => self.official.as_ref(),
        }
    }

    pub fn preferred_source(&self) -> Option<SourceKind> {
        self.custom
            .as_ref()
            .map(|_| SourceKind::Custom)
            .or_else(|| self.official.as_ref().map(|_| SourceKind::Official))
    }

    pub fn custom_guide_path(&self) -> &Path {
        &self.custom_guide_path
    }

    pub fn short_output(&self) -> Option<&str> {
        self.custom
            .as_ref()
            .map(Document::short_custom_content)
            .or_else(|| {
                self.command_help
                    .as_ref()
                    .map(|document| document.content.as_str())
            })
    }

    #[cfg(test)]
    pub(crate) fn from_documents_for_test(
        command_path: CommandPath,
        custom: Option<Document>,
        official: Option<Document>,
    ) -> Self {
        Self {
            custom_guide_path: PathBuf::from("/guides").join(command_path.guide_relative_path()),
            command_path,
            custom,
            official,
            command_help: None,
        }
    }
}

pub struct Catalog<R = SystemProcessRunner> {
    knowledge_base_root: PathBuf,
    runner: R,
}

impl Catalog<SystemProcessRunner> {
    pub fn new(knowledge_base_root: PathBuf) -> Self {
        Self {
            knowledge_base_root,
            runner: SystemProcessRunner,
        }
    }
}

impl<R: ProcessRunner> Catalog<R> {
    pub fn with_runner(knowledge_base_root: PathBuf, runner: R) -> Self {
        Self {
            knowledge_base_root,
            runner,
        }
    }

    pub fn resolve(&self, command_path: CommandPath) -> Result<HelpTopic, CatalogError> {
        let custom_guide_path = self
            .knowledge_base_root
            .join(command_path.guide_relative_path());
        let custom = self.resolve_custom(&custom_guide_path)?;
        let man = self.resolve_man(&command_path);
        let command_help = if man.is_none() || custom.is_none() {
            self.resolve_command_help(&command_path)
        } else {
            None
        };
        let official = man.or_else(|| command_help.clone());

        Ok(HelpTopic {
            command_path,
            custom,
            official,
            custom_guide_path,
            command_help,
        })
    }

    fn resolve_custom(&self, path: &Path) -> Result<Option<Document>, CatalogError> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(CatalogError::ReadGuide {
                    path: path.to_path_buf(),
                    source,
                });
            }
        };
        let content = String::from_utf8(bytes).map_err(|_| CatalogError::InvalidGuide {
            path: path.to_path_buf(),
            source: GuideError::InvalidUtf8,
        })?;
        Document::custom(content, path)
            .map(Some)
            .map_err(|source| CatalogError::InvalidGuide {
                path: path.to_path_buf(),
                source,
            })
    }

    fn resolve_man(&self, command_path: &CommandPath) -> Option<Document> {
        let topic = command_path.man_topic();
        let location = self
            .runner
            .run(ProcessRequest::new("man", ["-w", topic.as_str()]))?;
        if !location.success || location.stdout.trim().is_empty() {
            return None;
        }

        let result = self.runner.run(
            ProcessRequest::new("man", [topic.as_str()])
                .with_env("MANPAGER", "cat")
                .with_env("PAGER", "cat")
                .with_env("MANWIDTH", "88"),
        )?;
        if !result.success || result.stdout.trim().is_empty() {
            return None;
        }

        let section = man_section(location.stdout.lines().next().unwrap_or(""));
        let label = section
            .map(|section| format!("man {topic}({section})"))
            .unwrap_or_else(|| format!("man {topic}"));
        Some(Document::official(result.stdout, label))
    }

    fn resolve_command_help(&self, command_path: &CommandPath) -> Option<Document> {
        let mut request = ProcessRequest::new(command_path.program(), command_path.arguments());
        request.arguments.push("--help".into());
        let result = self.runner.run(request)?;
        let content = if !result.stdout.trim().is_empty() {
            result.stdout
        } else {
            result.stderr
        };
        if content.trim().is_empty() {
            return None;
        }
        Some(Document::official(
            content,
            format!("{command_path} --help"),
        ))
    }
}

#[derive(Clone, Debug)]
pub struct ProcessRequest {
    pub program: String,
    pub arguments: Vec<String>,
    pub environment: Vec<(String, String)>,
    pub timeout: Duration,
}

impl ProcessRequest {
    fn new<I, S>(program: impl Into<String>, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self {
            program: program.into(),
            arguments: arguments
                .into_iter()
                .map(|value| value.as_ref().to_owned())
                .collect(),
            environment: Vec::new(),
            timeout: HELP_TIMEOUT,
        }
    }

    fn with_env(mut self, key: &str, value: &str) -> Self {
        self.environment.push((key.into(), value.into()));
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProcessOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub trait ProcessRunner {
    fn run(&self, request: ProcessRequest) -> Option<ProcessOutput>;
}

pub struct SystemProcessRunner;

impl ProcessRunner for SystemProcessRunner {
    fn run(&self, request: ProcessRequest) -> Option<ProcessOutput> {
        let mut command = Command::new(&request.program);
        command
            .args(&request.arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in request.environment {
            command.env(key, value);
        }

        let mut child = command.spawn().ok()?;
        let mut stdout = child.stdout.take()?;
        let mut stderr = child.stderr.take()?;
        let stdout_reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).map(|_| bytes)
        });
        let stderr_reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).map(|_| bytes)
        });
        match child.wait_timeout(request.timeout).ok()? {
            Some(status) => {
                let stdout = stdout_reader.join().ok()?.ok()?;
                let stderr = stderr_reader.join().ok()?.ok()?;
                Some(ProcessOutput {
                    success: status.success(),
                    stdout: String::from_utf8_lossy(&stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&stderr).into_owned(),
                })
            }
            None => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                None
            }
        }
    }
}

fn validate_custom_guide(content: &str) -> Result<(), GuideError> {
    if content.contains('\u{1b}') {
        return Err(GuideError::AnsiEscape);
    }
    let options = Options::ENABLE_TABLES;
    for event in Parser::new_ext(content, options) {
        if matches!(event, Event::Html(_) | Event::InlineHtml(_)) {
            return Err(GuideError::RawHtml);
        }
    }
    Ok(())
}

fn markdown_headings(content: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    let mut current: Option<(u8, usize, String)> = None;
    let options = Options::ENABLE_TABLES;
    for (event, range) in Parser::new_ext(content, options).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some((
                    heading_level(level),
                    line_for_offset(content, range.start),
                    String::new(),
                ));
            }
            Event::Text(text) | Event::Code(text) if current.is_some() => {
                current.as_mut().expect("checked above").2.push_str(&text);
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, line, title)) = current.take() {
                    headings.push(Heading { level, title, line });
                }
            }
            _ => {}
        }
    }
    headings
}

fn plain_text_headings(content: &str) -> Vec<Heading> {
    content
        .lines()
        .enumerate()
        .filter_map(|(line, value)| {
            let trimmed = value.trim();
            let is_heading = !trimmed.is_empty()
                && trimmed.len() < 80
                && trimmed.chars().any(char::is_alphabetic)
                && trimmed
                    .chars()
                    .all(|character| !character.is_alphabetic() || character.is_uppercase());
            is_heading.then(|| Heading {
                level: 2,
                title: trimmed.to_owned(),
                line,
            })
        })
        .collect()
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn line_for_offset(content: &str, offset: usize) -> usize {
    content[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
}

fn byte_offset_for_line(content: &str, target: usize) -> Option<usize> {
    if target == 0 {
        return Some(0);
    }
    content
        .match_indices('\n')
        .nth(target - 1)
        .map(|(offset, _)| offset + 1)
}

fn clean_terminal_text(value: &str) -> String {
    let mut clean = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' {
            if characters.next_if_eq(&'[').is_some() {
                for next in characters.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            continue;
        }
        if character == '\u{8}' {
            clean.pop();
            continue;
        }
        clean.push(character);
    }
    clean.replace('\t', "    ").trim().to_owned()
}

fn man_section(location: &str) -> Option<&str> {
    let name = Path::new(location.trim()).file_name()?.to_str()?;
    let without_compression = [".gz", ".bz2", ".xz", ".zst"]
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .unwrap_or(name);
    without_compression
        .rsplit_once('.')
        .map(|(_, section)| section)
}

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("cannot read Custom Guide {path}: {source}")]
    ReadGuide {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid Custom Guide {path}: {source}")]
    InvalidGuide {
        path: PathBuf,
        #[source]
        source: GuideError,
    },
}

#[derive(Debug, Error)]
pub enum GuideError {
    #[error("the file is not valid UTF-8")]
    InvalidUtf8,
    #[error("raw ANSI escapes are not allowed")]
    AnsiEscape,
    #[error("raw HTML is not allowed")]
    RawHtml,
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use super::*;

    #[derive(Default)]
    struct FakeRunner {
        requests: RefCell<Vec<ProcessRequest>>,
        outputs: RefCell<VecDeque<Option<ProcessOutput>>>,
    }

    impl ProcessRunner for FakeRunner {
        fn run(&self, request: ProcessRequest) -> Option<ProcessOutput> {
            self.requests.borrow_mut().push(request);
            self.outputs.borrow_mut().pop_front().flatten()
        }
    }

    fn output(success: bool, stdout: &str, stderr: &str) -> Option<ProcessOutput> {
        Some(ProcessOutput {
            success,
            stdout: stdout.into(),
            stderr: stderr.into(),
        })
    }

    #[test]
    fn loads_nested_custom_guide_and_selects_its_intro() {
        let directory = tempfile::tempdir().unwrap();
        let guide = directory.path().join("git/rebase.md");
        fs::create_dir_all(guide.parent().unwrap()).unwrap();
        fs::write(
            &guide,
            "# Rebase\n\nKeep commits tidy.\n\n## Continue\n\n`git rebase --continue`\n",
        )
        .unwrap();
        let runner = FakeRunner::default();
        runner.outputs.borrow_mut().push_back(None);
        runner.outputs.borrow_mut().push_back(None);
        let catalog = Catalog::with_runner(directory.path().to_path_buf(), runner);

        let topic = catalog
            .resolve(CommandPath::new(vec!["git".into(), "rebase".into()]).unwrap())
            .unwrap();

        assert_eq!(topic.short_output(), Some("# Rebase\n\nKeep commits tidy."));
        assert!(topic.official.is_none());
    }

    #[test]
    fn prefers_man_but_keeps_command_help_for_short_output() {
        let directory = tempfile::tempdir().unwrap();
        let runner = FakeRunner::default();
        runner.outputs.borrow_mut().extend([
            output(true, "/usr/share/man/man1/git-rebase.1.gz\n", ""),
            output(true, "GIT-REBASE(1)\n\nOFFICIAL WORDING\n", ""),
            output(true, "usage: git rebase\n", ""),
        ]);
        let catalog = Catalog::with_runner(directory.path().to_path_buf(), runner);

        let topic = catalog
            .resolve(CommandPath::new(vec!["git".into(), "rebase".into()]).unwrap())
            .unwrap();

        assert_eq!(
            topic.official.as_ref().unwrap().source_label,
            "man git-rebase(1)"
        );
        assert_eq!(topic.short_output(), Some("usage: git rebase"));
        let requests = catalog.runner.requests.borrow();
        assert_eq!(requests[2].program, "git");
        assert_eq!(requests[2].arguments, ["rebase", "--help"]);
    }

    #[test]
    fn rejects_html_and_ansi_in_custom_guides() {
        assert!(matches!(
            Document::custom("<b>no</b>".into(), Path::new("guide.md")),
            Err(GuideError::RawHtml)
        ));
        assert!(matches!(
            Document::custom("\u{1b}[31mno".into(), Path::new("guide.md")),
            Err(GuideError::AnsiEscape)
        ));
    }

    #[test]
    fn strips_controls_without_rewriting_words() {
        assert_eq!(
            clean_terminal_text("A\u{8}A\u{1b}[31m WORDS\u{1b}[0m"),
            "A WORDS"
        );
    }

    #[test]
    fn command_help_uses_stderr_when_stdout_is_empty() {
        let directory = tempfile::tempdir().unwrap();
        let runner = FakeRunner::default();
        runner.outputs.borrow_mut().extend([
            output(false, "", "no man page"),
            output(false, "", "usage: tool subcommand\n"),
        ]);
        let catalog = Catalog::with_runner(directory.path().to_path_buf(), runner);

        let topic = catalog
            .resolve(CommandPath::new(vec!["tool".into(), "subcommand".into()]).unwrap())
            .unwrap();

        assert_eq!(topic.short_output(), Some("usage: tool subcommand"));
        assert_eq!(
            topic.official.as_ref().unwrap().source_label,
            "tool subcommand --help"
        );
        let requests = catalog.runner.requests.borrow();
        assert_eq!(requests[1].program, "tool");
        assert_eq!(requests[1].arguments, ["subcommand", "--help"]);
    }

    #[test]
    fn extracts_setext_headings_and_stops_short_output_at_level_two() {
        let document = Document::custom(
            "Tool\n====\n\nIntroduction.\n\nDetails\n-------\n\nBody.\n".into(),
            Path::new("tool.md"),
        )
        .unwrap();

        assert_eq!(
            document.headings,
            [
                Heading {
                    level: 1,
                    title: "Tool".into(),
                    line: 0,
                },
                Heading {
                    level: 2,
                    title: "Details".into(),
                    line: 5,
                },
            ]
        );
        assert_eq!(
            document.short_custom_content(),
            "Tool\n====\n\nIntroduction."
        );
    }

    #[test]
    fn reports_non_utf8_custom_guides() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("tool.md"), [0xff, 0xfe]).unwrap();
        let runner = FakeRunner::default();
        let catalog = Catalog::with_runner(directory.path().to_path_buf(), runner);

        assert!(matches!(
            catalog.resolve(CommandPath::new(vec!["tool".into()]).unwrap()),
            Err(CatalogError::InvalidGuide {
                source: GuideError::InvalidUtf8,
                ..
            })
        ));
    }
}
