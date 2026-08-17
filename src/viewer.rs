use ratatui::layout::Rect;

use crate::catalog::{Document, HelpTopic, SourceKind};

const OUTLINE_MINIMUM_WIDTH: u16 = 80;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutKind {
    OutlineBrowser,
    MinimalPager,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ViewerInput {
    SwitchSource,
    ScrollDown,
    ScrollUp,
    PageDown(usize),
    PageUp(usize),
    Top,
    Bottom,
    FocusOutline,
    MoveOutline(i32),
    ActivateOutline,
    BeginSearch,
    SearchCharacter(char),
    SearchBackspace,
    SubmitSearch,
    NextMatch,
    PreviousMatch,
    Escape,
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewerEffect {
    None,
    Quit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewerFrame {
    pub layout: LayoutKind,
    pub command_path: String,
    pub active_source: SourceKind,
    pub source_label: String,
    pub lines: Vec<String>,
    pub scroll: usize,
    pub headings: Vec<String>,
    pub selected_heading: usize,
    pub outline_focused: bool,
    pub search_input: Option<String>,
    pub search_summary: Option<String>,
    pub can_switch_source: bool,
    pub page_size: usize,
}

#[derive(Clone, Debug, Default)]
struct SourceState {
    scroll: usize,
    active_heading: usize,
    selected_heading: usize,
    matches: Vec<usize>,
    current_match: usize,
}

pub struct Viewer {
    topic: HelpTopic,
    active_source: SourceKind,
    custom: SourceState,
    official: SourceState,
    outline_focused: bool,
    search_query: String,
    search_input: Option<String>,
}

impl Viewer {
    pub fn new(topic: HelpTopic, initial_source: SourceKind) -> Self {
        let active_source = if topic.document(initial_source).is_some() {
            initial_source
        } else {
            topic.preferred_source().unwrap_or(initial_source)
        };
        Self {
            topic,
            active_source,
            custom: SourceState::default(),
            official: SourceState::default(),
            outline_focused: false,
            search_query: String::new(),
            search_input: None,
        }
    }

    pub fn update(&mut self, input: ViewerInput) -> ViewerEffect {
        match input {
            ViewerInput::Quit => return ViewerEffect::Quit,
            ViewerInput::SwitchSource => self.switch_source(),
            ViewerInput::ScrollDown => self.scroll_by(1),
            ViewerInput::ScrollUp => self.scroll_by(-1),
            ViewerInput::PageDown(amount) => self.scroll_by(amount as isize),
            ViewerInput::PageUp(amount) => self.scroll_by(-(amount as isize)),
            ViewerInput::Top => self.set_scroll(0),
            ViewerInput::Bottom => self.set_scroll(usize::MAX),
            ViewerInput::FocusOutline => self.outline_focused = true,
            ViewerInput::MoveOutline(amount) => self.move_outline(amount),
            ViewerInput::ActivateOutline => self.activate_outline(),
            ViewerInput::BeginSearch => self.search_input = Some(self.search_query.clone()),
            ViewerInput::SearchCharacter(character) => {
                if let Some(input) = &mut self.search_input {
                    input.push(character);
                }
            }
            ViewerInput::SearchBackspace => {
                if let Some(input) = &mut self.search_input {
                    input.pop();
                }
            }
            ViewerInput::SubmitSearch => self.submit_search(),
            ViewerInput::NextMatch => self.move_match(1),
            ViewerInput::PreviousMatch => self.move_match(-1),
            ViewerInput::Escape => {
                if self.search_input.is_some() {
                    self.search_input = None;
                } else {
                    self.outline_focused = false;
                }
            }
        }
        ViewerEffect::None
    }

    pub fn frame(&self, area: Rect) -> ViewerFrame {
        let document = self
            .active_document()
            .expect("the viewer always has an active document");
        let state = self.state();
        let search_summary = (!self.search_query.is_empty()).then(|| {
            if state.matches.is_empty() {
                format!("0 matches for {}", self.search_query)
            } else {
                format!(
                    "{} of {} matches for {}",
                    state.current_match + 1,
                    state.matches.len(),
                    self.search_query
                )
            }
        });
        ViewerFrame {
            layout: if area.width >= OUTLINE_MINIMUM_WIDTH {
                LayoutKind::OutlineBrowser
            } else {
                LayoutKind::MinimalPager
            },
            command_path: self.topic.command_path.to_string(),
            active_source: self.active_source,
            source_label: document.source_label.clone(),
            lines: document.content.lines().map(str::to_owned).collect(),
            scroll: state.scroll,
            headings: document
                .headings
                .iter()
                .map(|heading| heading.title.clone())
                .collect(),
            selected_heading: state.selected_heading,
            outline_focused: self.outline_focused,
            search_input: self.search_input.clone(),
            search_summary,
            can_switch_source: self.topic.custom.is_some() && self.topic.official.is_some(),
            page_size: area.height.saturating_sub(3).max(1) as usize,
        }
    }

    fn active_document(&self) -> Option<&Document> {
        self.topic.document(self.active_source)
    }

    fn state(&self) -> &SourceState {
        match self.active_source {
            SourceKind::Custom => &self.custom,
            SourceKind::Official => &self.official,
        }
    }

    fn state_mut(&mut self) -> &mut SourceState {
        match self.active_source {
            SourceKind::Custom => &mut self.custom,
            SourceKind::Official => &mut self.official,
        }
    }

    fn switch_source(&mut self) {
        let next = match self.active_source {
            SourceKind::Custom => SourceKind::Official,
            SourceKind::Official => SourceKind::Custom,
        };
        if self.topic.document(next).is_some() {
            self.active_source = next;
            self.refresh_active_heading();
        }
    }

    fn scroll_by(&mut self, amount: isize) {
        let current = self.state().scroll;
        let next = current.saturating_add_signed(amount);
        self.set_scroll(next);
    }

    fn set_scroll(&mut self, target: usize) {
        let last_line = self
            .active_document()
            .map(|document| document.content.lines().count().saturating_sub(1))
            .unwrap_or(0);
        self.state_mut().scroll = target.min(last_line);
        self.refresh_active_heading();
    }

    fn refresh_active_heading(&mut self) {
        let scroll = self.state().scroll;
        let active = self
            .active_document()
            .and_then(|document| {
                document
                    .headings
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, heading)| heading.line <= scroll)
                    .map(|(index, _)| index)
            })
            .unwrap_or(0);
        let outline_focused = self.outline_focused;
        let state = self.state_mut();
        state.active_heading = active;
        if !outline_focused {
            state.selected_heading = active;
        }
    }

    fn move_outline(&mut self, amount: i32) {
        if !self.outline_focused {
            return;
        }
        let heading_count = self
            .active_document()
            .map(|document| document.headings.len())
            .unwrap_or(0);
        if heading_count == 0 {
            return;
        }
        let state = self.state_mut();
        state.selected_heading = state
            .selected_heading
            .saturating_add_signed(amount as isize)
            .min(heading_count - 1);
    }

    fn activate_outline(&mut self) {
        if !self.outline_focused {
            return;
        }
        let selected = self.state().selected_heading;
        if let Some(line) = self
            .active_document()
            .and_then(|document| document.headings.get(selected))
            .map(|heading| heading.line)
        {
            self.set_scroll(line);
        }
        self.outline_focused = false;
    }

    fn submit_search(&mut self) {
        let Some(input) = self.search_input.take() else {
            return;
        };
        self.search_query = input;
        self.recalculate_matches(SourceKind::Custom);
        self.recalculate_matches(SourceKind::Official);
        self.jump_to_current_match();
    }

    fn recalculate_matches(&mut self, source: SourceKind) {
        let query = self.search_query.clone();
        let matches = self
            .topic
            .document(source)
            .map(|document| search_lines(&document.content, &query))
            .unwrap_or_default();
        let state = match source {
            SourceKind::Custom => &mut self.custom,
            SourceKind::Official => &mut self.official,
        };
        state.matches = matches;
        state.current_match = 0;
    }

    fn move_match(&mut self, amount: isize) {
        let state = self.state_mut();
        if state.matches.is_empty() {
            return;
        }
        let len = state.matches.len() as isize;
        state.current_match = (state.current_match as isize + amount).rem_euclid(len) as usize;
        self.jump_to_current_match();
    }

    fn jump_to_current_match(&mut self) {
        let line = {
            let state = self.state();
            state.matches.get(state.current_match).copied()
        };
        if let Some(line) = line {
            self.set_scroll(line);
        }
    }
}

fn search_lines(content: &str, query: &str) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }
    let smart_case = query.chars().any(char::is_uppercase);
    let needle = if smart_case {
        query.to_owned()
    } else {
        query.to_lowercase()
    };
    content
        .lines()
        .enumerate()
        .filter_map(|(line, value)| {
            let haystack = if smart_case {
                value.to_owned()
            } else {
                value.to_lowercase()
            };
            haystack.contains(&needle).then_some(line)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::catalog::{Document, Heading};
    use crate::command_path::CommandPath;

    use super::*;

    fn topic() -> HelpTopic {
        HelpTopic::from_documents_for_test(
            CommandPath::new(vec!["git".into()]).unwrap(),
            Some(Document {
                content: "# Git\nintro\n## Add\ncustom add\n## Push\ncustom push".into(),
                source_label: "/guides/git.md".into(),
                headings: vec![
                    Heading {
                        level: 1,
                        title: "Git".into(),
                        line: 0,
                    },
                    Heading {
                        level: 2,
                        title: "Add".into(),
                        line: 2,
                    },
                    Heading {
                        level: 2,
                        title: "Push".into(),
                        line: 4,
                    },
                ],
            }),
            Some(Document {
                content: "GIT(1)\nofficial add\nofficial push".into(),
                source_label: "man git(1)".into(),
                headings: vec![Heading {
                    level: 2,
                    title: "GIT(1)".into(),
                    line: 0,
                }],
            }),
        )
    }

    #[test]
    fn switches_sources_without_sharing_scroll_positions() {
        let mut viewer = Viewer::new(topic(), SourceKind::Custom);
        viewer.update(ViewerInput::PageDown(4));
        viewer.update(ViewerInput::SwitchSource);
        viewer.update(ViewerInput::ScrollDown);
        assert_eq!(viewer.frame(Rect::new(0, 0, 100, 20)).scroll, 1);

        viewer.update(ViewerInput::SwitchSource);
        assert_eq!(viewer.frame(Rect::new(0, 0, 100, 20)).scroll, 4);
    }

    #[test]
    fn smart_case_search_runs_for_both_sources() {
        let mut viewer = Viewer::new(topic(), SourceKind::Custom);
        viewer.update(ViewerInput::BeginSearch);
        for character in "add".chars() {
            viewer.update(ViewerInput::SearchCharacter(character));
        }
        viewer.update(ViewerInput::SubmitSearch);
        assert_eq!(
            viewer
                .frame(Rect::new(0, 0, 100, 20))
                .search_summary
                .as_deref(),
            Some("1 of 2 matches for add")
        );

        viewer.update(ViewerInput::SwitchSource);
        assert_eq!(
            viewer
                .frame(Rect::new(0, 0, 100, 20))
                .search_summary
                .as_deref(),
            Some("1 of 1 matches for add")
        );
    }

    #[test]
    fn narrow_terminals_use_minimal_pager() {
        let viewer = Viewer::new(topic(), SourceKind::Custom);
        assert_eq!(
            viewer.frame(Rect::new(0, 0, 79, 20)).layout,
            LayoutKind::MinimalPager
        );
    }

    #[test]
    fn outline_selection_jumps_to_the_selected_heading() {
        let mut viewer = Viewer::new(topic(), SourceKind::Custom);

        viewer.update(ViewerInput::FocusOutline);
        viewer.update(ViewerInput::MoveOutline(2));
        viewer.update(ViewerInput::ActivateOutline);

        let frame = viewer.frame(Rect::new(0, 0, 100, 20));
        assert_eq!(frame.scroll, 4);
        assert_eq!(frame.selected_heading, 2);
        assert!(!frame.outline_focused);
    }

    #[test]
    fn uppercase_search_is_case_sensitive() {
        let mut viewer = Viewer::new(topic(), SourceKind::Custom);
        viewer.update(ViewerInput::BeginSearch);
        for character in "Add".chars() {
            viewer.update(ViewerInput::SearchCharacter(character));
        }
        viewer.update(ViewerInput::SubmitSearch);

        assert_eq!(
            viewer
                .frame(Rect::new(0, 0, 100, 20))
                .search_summary
                .as_deref(),
            Some("1 of 1 matches for Add")
        );
    }

    #[test]
    fn match_navigation_wraps_in_both_directions() {
        let mut viewer = Viewer::new(topic(), SourceKind::Custom);
        viewer.update(ViewerInput::BeginSearch);
        for character in "custom".chars() {
            viewer.update(ViewerInput::SearchCharacter(character));
        }
        viewer.update(ViewerInput::SubmitSearch);
        assert_eq!(viewer.frame(Rect::new(0, 0, 100, 20)).scroll, 3);

        viewer.update(ViewerInput::PreviousMatch);
        assert_eq!(viewer.frame(Rect::new(0, 0, 100, 20)).scroll, 5);
        viewer.update(ViewerInput::NextMatch);
        assert_eq!(viewer.frame(Rect::new(0, 0, 100, 20)).scroll, 3);
    }

    #[test]
    fn unavailable_source_cannot_be_selected() {
        let topic = HelpTopic::from_documents_for_test(
            CommandPath::new(vec!["git".into()]).unwrap(),
            topic().custom,
            None,
        );
        let mut viewer = Viewer::new(topic, SourceKind::Official);

        assert_eq!(
            viewer.frame(Rect::new(0, 0, 100, 20)).active_source,
            SourceKind::Custom
        );
        viewer.update(ViewerInput::SwitchSource);
        assert_eq!(
            viewer.frame(Rect::new(0, 0, 100, 20)).active_source,
            SourceKind::Custom
        );
    }
}
