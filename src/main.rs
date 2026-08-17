use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use clap::{ArgAction, Parser, ValueEnum};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use mani::catalog::{Catalog, SourceKind};
use mani::command_path::CommandPath;
use mani::config::{Config, Theme};
use mani::paths::AppPaths;
use mani::viewer::{LayoutKind, Viewer, ViewerEffect, ViewerFrame, ViewerInput};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use thiserror::Error;

#[derive(Debug, Parser)]
#[command(
    name = "mani",
    version,
    about = "Browse Custom Guides and authoritative local command documentation"
)]
struct Cli {
    /// Print concise help and do not open the Interactive View.
    #[arg(long, conflicts_with = "print")]
    short: bool,
    /// Print a complete document and do not open the Interactive View.
    #[arg(long)]
    print: bool,
    /// Select the Custom Guide.
    #[arg(long, conflicts_with_all = ["official", "short"])]
    custom: bool,
    /// Select Official Documentation.
    #[arg(long, conflicts_with_all = ["custom", "short"])]
    official: bool,
    /// Control color in stdout modes.
    #[arg(long, value_enum, default_value = "auto")]
    color: ColorChoice,
    /// Use built-in defaults even when config.toml is broken.
    #[arg(long)]
    ignore_config: bool,
    /// Command and subcommands to look up.
    #[arg(
        required = true,
        num_args = 1..,
        action = ArgAction::Append,
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    command_path: Vec<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ColorChoice {
    Auto,
    Always,
    Never,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mani: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), AppError> {
    let paths = AppPaths::resolve()?;
    let config = Config::load(&paths.config_file(), cli.ignore_config)?;

    if cli.command_path == ["config", "check"] {
        if paths.config_file().exists() {
            println!("{} is valid", paths.config_file().display());
        } else {
            println!(
                "no configuration at {}; built-in defaults are valid",
                paths.config_file().display()
            );
        }
        return Ok(());
    }
    if matches!(
        cli.command_path.first().map(String::as_str),
        Some("explain" | "setup" | "shell" | "state" | "history")
    ) {
        return Err(AppError::LaterSlice(cli.command_path.join(" ")));
    }

    let command_path = CommandPath::new(cli.command_path)?;
    let catalog = Catalog::new(paths.knowledge_base_root.clone());
    let topic = catalog.resolve(command_path.clone())?;

    if cli.short {
        let output = topic
            .short_output()
            .ok_or_else(|| AppError::MissingShort(command_path.clone()))?;
        write_document(output, &config.theme, cli.color)?;
        return Ok(());
    }

    let selected = if cli.custom {
        SourceKind::Custom
    } else if cli.official {
        SourceKind::Official
    } else {
        topic
            .preferred_source()
            .ok_or_else(|| AppError::MissingAll {
                command_path: command_path.clone(),
                custom_path: paths
                    .knowledge_base_root
                    .join(command_path.guide_relative_path()),
            })?
    };
    let document = topic
        .document(selected)
        .ok_or_else(|| AppError::MissingSelected {
            requested_source: selected,
            command_path: command_path.clone(),
            custom_path: paths
                .knowledge_base_root
                .join(command_path.guide_relative_path()),
        })?;

    if cli.print {
        write_document(&document.content, &config.theme, cli.color)?;
        return Ok(());
    }

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(AppError::InteractiveTerminalRequired);
    }
    run_viewer(Viewer::new(topic, selected), &config.theme)
}

fn write_document(content: &str, theme: &Theme, color: ColorChoice) -> Result<(), io::Error> {
    let use_color = match color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none(),
    };
    let rendered = if use_color {
        colorize_document(content, theme)
    } else {
        content.to_owned()
    };
    let mut stdout = io::stdout().lock();
    stdout.write_all(rendered.as_bytes())?;
    if !rendered.ends_with('\n') {
        stdout.write_all(b"\n")?;
    }
    Ok(())
}

fn colorize_document(content: &str, theme: &Theme) -> String {
    let heading = ansi_prefix(&theme.heading);
    let code = ansi_prefix(&theme.code);
    let mut in_code = false;
    let mut output = String::new();
    for line in content.split_inclusive('\n') {
        let plain = line.trim_end_matches('\n');
        if plain.trim_start().starts_with("```") {
            in_code = !in_code;
        }
        let style = if plain.starts_with('#') {
            &heading
        } else if in_code || plain.trim_start().starts_with("```") {
            &code
        } else {
            ""
        };
        if style.is_empty() {
            output.push_str(line);
        } else {
            output.push_str(style);
            output.push_str(plain);
            output.push_str("\u{1b}[0m");
            if line.ends_with('\n') {
                output.push('\n');
            }
        }
    }
    output
}

fn ansi_prefix(specification: &str) -> String {
    let codes: Vec<&str> = specification
        .split_whitespace()
        .filter_map(|part| match part {
            "black" => Some("30"),
            "red" => Some("31"),
            "green" => Some("32"),
            "yellow" => Some("33"),
            "blue" => Some("34"),
            "magenta" => Some("35"),
            "cyan" => Some("36"),
            "white" => Some("37"),
            "bold" => Some("1"),
            "dim" => Some("2"),
            "reverse" => Some("7"),
            _ => None,
        })
        .collect();
    format!("\u{1b}[{}m", codes.join(";"))
}

fn run_viewer(mut viewer: Viewer, theme: &Theme) -> Result<(), AppError> {
    let mut terminal = TerminalSession::start()?;
    loop {
        terminal
            .terminal
            .draw(|frame| draw_viewer(frame, &viewer, theme))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let size = terminal.terminal.size()?;
        let frame = viewer.frame(Rect::new(0, 0, size.width, size.height));
        let input = key_to_input(key.code, &frame);
        if let Some(input) = input
            && viewer.update(input) == ViewerEffect::Quit
        {
            break;
        }
    }
    Ok(())
}

fn key_to_input(code: KeyCode, frame: &ViewerFrame) -> Option<ViewerInput> {
    if frame.search_input.is_some() {
        return match code {
            KeyCode::Enter => Some(ViewerInput::SubmitSearch),
            KeyCode::Esc => Some(ViewerInput::Escape),
            KeyCode::Backspace => Some(ViewerInput::SearchBackspace),
            KeyCode::Char(character) => Some(ViewerInput::SearchCharacter(character)),
            _ => None,
        };
    }
    match code {
        KeyCode::Char('q') => Some(ViewerInput::Quit),
        KeyCode::Tab => Some(ViewerInput::SwitchSource),
        KeyCode::Char('/') => Some(ViewerInput::BeginSearch),
        KeyCode::Char('n') => Some(ViewerInput::NextMatch),
        KeyCode::Char('N') => Some(ViewerInput::PreviousMatch),
        KeyCode::Char('g') => Some(ViewerInput::Top),
        KeyCode::Char('G') => Some(ViewerInput::Bottom),
        KeyCode::Char('o') => Some(ViewerInput::FocusOutline),
        KeyCode::Enter if frame.outline_focused => Some(ViewerInput::ActivateOutline),
        KeyCode::Esc => Some(ViewerInput::Escape),
        KeyCode::Down | KeyCode::Char('j') if frame.outline_focused => {
            Some(ViewerInput::MoveOutline(1))
        }
        KeyCode::Up | KeyCode::Char('k') if frame.outline_focused => {
            Some(ViewerInput::MoveOutline(-1))
        }
        KeyCode::Down | KeyCode::Char('j') => Some(ViewerInput::ScrollDown),
        KeyCode::Up | KeyCode::Char('k') => Some(ViewerInput::ScrollUp),
        KeyCode::PageDown => Some(ViewerInput::PageDown(frame.page_size)),
        KeyCode::PageUp => Some(ViewerInput::PageUp(frame.page_size)),
        _ => None,
    }
}

fn draw_viewer(frame: &mut ratatui::Frame<'_>, viewer: &Viewer, theme: &Theme) {
    let area = frame.area();
    let view = viewer.frame(area);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    let source_name = match view.active_source {
        SourceKind::Custom => "CUSTOM",
        SourceKind::Official => "OFFICIAL",
    };
    let header = Text::from(vec![
        Line::from(vec![
            Span::styled(" mani ", style_from_spec(&theme.active_source)),
            Span::styled(
                view.command_path.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("  {source_name}")),
        ]),
        Line::styled(view.source_label.clone(), style_from_spec(&theme.muted)),
    ]);
    frame.render_widget(Paragraph::new(header), vertical[0]);

    match view.layout {
        LayoutKind::OutlineBrowser => {
            let horizontal = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(26), Constraint::Min(30)])
                .split(vertical[1]);
            draw_outline(frame, horizontal[0], &view, theme);
            draw_document(frame, horizontal[1], &view, theme);
        }
        LayoutKind::MinimalPager => draw_document(frame, vertical[1], &view, theme),
    }

    let footer = if let Some(input) = &view.search_input {
        format!(" /{input}")
    } else if let Some(summary) = &view.search_summary {
        format!(" {summary}   n/N match   / search   q quit")
    } else if view.can_switch_source {
        " Tab source   j/k scroll   / search   o outline   q quit".into()
    } else {
        " j/k scroll   / search   o outline   q quit".into()
    };
    frame.render_widget(
        Paragraph::new(footer).style(Style::default().add_modifier(Modifier::REVERSED)),
        vertical[2],
    );
}

fn draw_outline(frame: &mut ratatui::Frame<'_>, area: Rect, view: &ViewerFrame, theme: &Theme) {
    let items: Vec<ListItem<'_>> = view
        .headings
        .iter()
        .enumerate()
        .map(|(index, heading)| {
            let style = if index == view.selected_heading {
                style_from_spec(&theme.active_source)
            } else {
                style_from_spec(&theme.muted)
            };
            ListItem::new(heading.as_str()).style(style)
        })
        .collect();
    let title = if view.outline_focused {
        " ON THIS PAGE • focused "
    } else {
        " ON THIS PAGE "
    };
    frame.render_widget(
        List::new(items).block(Block::default().title(title).borders(Borders::RIGHT)),
        area,
    );
}

fn draw_document(frame: &mut ratatui::Frame<'_>, area: Rect, view: &ViewerFrame, theme: &Theme) {
    let lines = styled_document_lines(&view.lines, theme);
    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((view.scroll.min(u16::MAX as usize) as u16, 0));
    frame.render_widget(paragraph, area);
}

fn styled_document_lines(lines: &[String], theme: &Theme) -> Text<'static> {
    let mut in_code = false;
    let rendered = lines
        .iter()
        .filter_map(|line| {
            if line.trim_start().starts_with("```") {
                in_code = !in_code;
                return None;
            }
            if in_code {
                return Some(Line::styled(
                    format!("  {line}"),
                    style_from_spec(&theme.code),
                ));
            }
            if let Some(heading) = line.strip_prefix("# ") {
                return Some(Line::styled(
                    heading.to_uppercase(),
                    style_from_spec(&theme.heading).add_modifier(Modifier::BOLD),
                ));
            }
            if let Some(heading) = line.strip_prefix("## ") {
                return Some(Line::styled(
                    heading.to_uppercase(),
                    style_from_spec(&theme.heading).add_modifier(Modifier::BOLD),
                ));
            }
            Some(Line::raw(line.clone()))
        })
        .collect::<Vec<_>>();
    Text::from(rendered)
}

fn style_from_spec(specification: &str) -> Style {
    specification
        .split_whitespace()
        .fold(Style::default(), |style, part| match part {
            "black" => style.fg(Color::Black),
            "red" => style.fg(Color::Red),
            "green" => style.fg(Color::Green),
            "yellow" => style.fg(Color::Yellow),
            "blue" => style.fg(Color::Blue),
            "magenta" => style.fg(Color::Magenta),
            "cyan" => style.fg(Color::Cyan),
            "white" => style.fg(Color::White),
            "bold" => style.add_modifier(Modifier::BOLD),
            "dim" => style.add_modifier(Modifier::DIM),
            "reverse" => style.add_modifier(Modifier::REVERSED),
            _ => style,
        })
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalSession {
    fn start() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        let backend = CrosstermBackend::new(stdout);
        match Terminal::new(backend) {
            Ok(terminal) => Ok(Self { terminal }),
            Err(error) => {
                let _ = disable_raw_mode();
                let _ = execute!(io::stdout(), LeaveAlternateScreen);
                Err(error)
            }
        }
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

#[derive(Debug, Error)]
enum AppError {
    #[error(transparent)]
    Paths(#[from] mani::paths::PathError),
    #[error(transparent)]
    Config(#[from] mani::config::ConfigError),
    #[error(transparent)]
    CommandPath(#[from] mani::command_path::CommandPathError),
    #[error(transparent)]
    Catalog(#[from] mani::catalog::CatalogError),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("no Custom Guide or command-generated --help output was found for {0}")]
    MissingShort(CommandPath),
    #[error(
        "no help source was found for {command_path}; create {custom_path} to add a Custom Guide"
    )]
    MissingAll {
        command_path: CommandPath,
        custom_path: std::path::PathBuf,
    },
    #[error(
        "the requested {requested_source:?} source is unavailable for {command_path}; Custom Guide path: {custom_path}"
    )]
    MissingSelected {
        requested_source: SourceKind,
        command_path: CommandPath,
        custom_path: std::path::PathBuf,
    },
    #[error(
        "the Interactive View requires terminal stdin and stdout; use --print or --short when piping"
    )]
    InteractiveTerminalRequired,
    #[error("`mani {0}` belongs to a later delivery slice and is not implemented yet")]
    LaterSlice(String),
}
