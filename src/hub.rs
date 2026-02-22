use ratatui::Frame;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};
use std::time::Duration;

use crate::challenge::{Category, Grade, Topic, grade_display};
use crate::game;
use crate::state::GameState;

pub enum HubAction {
    SelectTopic(u8),
    Quit,
}

/// A visual entry in the hub list. Headers are non-selectable.
enum HubListItem {
    Spacer,
    Header(Category),
    Entry {
        topic_id: u8,
        topic_name: String,
        total: usize,
    },
}

pub struct Hub {
    topics: Vec<Topic>,
    list_items: Vec<HubListItem>,
    list_state: ListState,
    pending_g: bool,
    count: Option<u32>,
    list_height: u16,
    unlock_all: bool,
}

impl Hub {
    pub fn new(topics: Vec<Topic>, unlock_all: bool) -> Self {
        let mut list_items = Vec::new();

        for cat in Category::ALL {
            let cat_topics: Vec<&Topic> = topics
                .iter()
                .filter(|t| Category::for_topic(t.id) == cat && !t.challenges.is_empty())
                .collect();

            if cat_topics.is_empty() {
                continue;
            }

            list_items.push(HubListItem::Spacer);
            list_items.push(HubListItem::Header(cat));
            for topic in cat_topics {
                list_items.push(HubListItem::Entry {
                    topic_id: topic.id,
                    topic_name: topic.name.clone(),
                    total: topic.challenges.len(),
                });
            }
        }

        let mut list_state = ListState::default();
        // Select first selectable entry
        if let Some(idx) = list_items
            .iter()
            .position(|item| matches!(item, HubListItem::Entry { .. }))
        {
            list_state.select(Some(idx));
        }

        Self {
            topics,
            list_items,
            list_state,
            pending_g: false,
            count: None,
            list_height: 0,
            unlock_all,
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
        state: &GameState,
    ) -> std::io::Result<HubAction> {
        loop {
            terminal.draw(|frame| self.render(frame, state))?;

            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
            {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                // Handle pending gg
                if self.pending_g {
                    self.pending_g = false;
                    self.count = None;
                    if key.code == KeyCode::Char('g') {
                        self.jump_first(state);
                        continue;
                    }
                }

                // Count prefix (applied to j/k)
                match key.code {
                    KeyCode::Char(c @ '1'..='9') => {
                        self.count = Some(self.count.unwrap_or(0) * 10 + (c as u32 - '0' as u32));
                        continue;
                    }
                    KeyCode::Char('0') if self.count.is_some() => {
                        self.count = self.count.map(|c| c * 10);
                        continue;
                    }
                    _ => {}
                }

                let n = self.count.unwrap_or(1) as usize;
                self.count = None;

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(HubAction::Quit),
                    KeyCode::Char('j') => {
                        for _ in 0..n {
                            self.next(state);
                        }
                    }
                    KeyCode::Char('k') => {
                        for _ in 0..n {
                            self.previous(state);
                        }
                    }
                    KeyCode::Char('g') => self.pending_g = true,
                    KeyCode::Char('G') => self.jump_last(state),
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let half = (self.list_height / 2).max(1) as usize;
                        for _ in 0..half {
                            self.next(state);
                        }
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let half = (self.list_height / 2).max(1) as usize;
                        for _ in 0..half {
                            self.previous(state);
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Enter => {
                        if let Some(i) = self.list_state.selected()
                            && let HubListItem::Entry { topic_id, .. } = &self.list_items[i]
                            && is_category_unlocked(
                                Category::for_topic(*topic_id),
                                &self.topics,
                                state,
                                self.unlock_all,
                            )
                        {
                            return Ok(HubAction::SelectTopic(*topic_id));
                        }
                    }
                    KeyCode::Char('?') => {
                        game::show_help(terminal)?;
                    }
                    _ => {}
                }
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, state: &GameState) {
        let [header, body, footer] = Layout::vertical([
            Constraint::Length(5),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        Self::render_header(frame, header, state, &self.topics);
        self.render_topics(frame, body, state);
        frame.render_widget(
            Paragraph::new(" j/k: navigate | l/Enter: select | ?: help | q: quit")
                .style(Style::new().fg(Color::DarkGray)),
            footer,
        );
    }

    fn render_header(frame: &mut Frame, area: Rect, state: &GameState, topics: &[Topic]) {
        let [title_area, stats_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Length(2)]).areas(area);

        let title = Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                " NVIMKATA ",
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .block(Block::bordered());
        frame.render_widget(title, title_area);

        // Exclude freestyle topics from completion/perfect stats
        let curriculum_topics: Vec<&Topic> = topics
            .iter()
            .filter(|t| Category::for_topic(t.id) != Category::Freestyle)
            .collect();
        let curriculum_ids: std::collections::HashSet<&str> = curriculum_topics
            .iter()
            .flat_map(|t| t.challenges.iter().map(|c| c.id.as_str()))
            .collect();
        let completed = state
            .challenges
            .keys()
            .filter(|id| curriculum_ids.contains(id.as_str()))
            .count();
        let total: usize = curriculum_topics.iter().map(|t| t.challenges.len()).sum();
        let perfects = state
            .challenges
            .iter()
            .filter(|(id, r)| curriculum_ids.contains(id.as_str()) && r.grade == Grade::A)
            .count();
        let outdated = state.stale_count();
        let mut stats_spans = vec![Span::styled(
            format!(
                " Completed: {completed}/{total} | Grade A: {perfects} | Attempts: {}",
                state.stats.challenges_attempted
            ),
            Style::new().fg(Color::Gray),
        )];
        if outdated > 0 {
            stats_spans.push(Span::styled(" | ", Style::new().fg(Color::Gray)));
            stats_spans.push(Span::styled(
                format!("Warning: {outdated} score(s) outdated"),
                Style::new().fg(Color::Yellow),
            ));
        }
        frame.render_widget(Paragraph::new(Line::from(stats_spans)), stats_area);
    }

    fn render_topics(&mut self, frame: &mut Frame, area: Rect, state: &GameState) {
        let [list_area, detail_area] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(area);

        self.list_height = list_area.height.saturating_sub(2);

        // Build selectable index mapping for relative line numbers
        let mut sel_counter = 0usize;
        let selectable_map: Vec<Option<usize>> = self
            .list_items
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if self.is_item_selectable(i, state) {
                    let idx = sel_counter;
                    sel_counter += 1;
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();
        let selected_sel_idx = self.list_state.selected().and_then(|i| selectable_map[i]);

        let num_style = Style::new().fg(Color::DarkGray);
        let items: Vec<ListItem> = self
            .list_items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let num_span = match (selectable_map[i], selected_sel_idx) {
                    (Some(si), Some(ssi)) => {
                        Span::styled(format!("{:>2} ", si.abs_diff(ssi)), num_style)
                    }
                    _ => Span::styled("   ", num_style),
                };
                self.render_list_item(item, num_span, state)
            })
            .collect();

        let list = List::new(items)
            .block(Block::bordered().title(" Topics "))
            .highlight_style(
                Style::new()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, list_area, &mut self.list_state);

        // Detail panel
        if let Some(i) = self.list_state.selected()
            && let HubListItem::Entry { topic_id, .. } = &self.list_items[i]
            && let Some(topic) = self.topics.iter().find(|t| t.id == *topic_id)
        {
            Self::render_topic_detail(frame, detail_area, topic, state);
        }
    }

    fn render_list_item<'a>(
        &self,
        item: &HubListItem,
        num_span: Span<'a>,
        state: &GameState,
    ) -> ListItem<'a> {
        match item {
            HubListItem::Spacer => ListItem::new(Line::from("")),
            HubListItem::Header(cat) => {
                let locked = !is_category_unlocked(*cat, &self.topics, state, self.unlock_all);
                let suffix = if locked { " [LOCKED]" } else { "" };
                let style = if locked {
                    Style::new().fg(Color::DarkGray)
                } else {
                    Style::new().fg(cat.color()).add_modifier(Modifier::BOLD)
                };
                ListItem::new(Line::from(vec![
                    num_span,
                    Span::styled(format!("── {}{} ──", cat.name(), suffix), style),
                ]))
            }
            HubListItem::Entry {
                topic_id,
                topic_name,
                total,
            } => {
                let cat = Category::for_topic(*topic_id);
                let locked = !is_category_unlocked(cat, &self.topics, state, self.unlock_all);

                if locked {
                    return ListItem::new(Line::from(vec![
                        num_span,
                        Span::styled(
                            format!("x {topic_name} ({total})"),
                            Style::new().fg(Color::DarkGray),
                        ),
                    ]));
                }

                let attempted = self
                    .topics
                    .iter()
                    .find(|t| t.id == *topic_id)
                    .map_or(0, |t| {
                        t.challenges
                            .iter()
                            .filter(|c| state.best_grade(&c.id).is_some())
                            .count()
                    });

                let has_stale = self
                    .topics
                    .iter()
                    .find(|t| t.id == *topic_id)
                    .is_some_and(|t| t.challenges.iter().any(|c| state.is_stale(&c.id)));
                let stale_suffix: Vec<Span> = if has_stale {
                    vec![Span::styled(" *", Style::new().fg(Color::Yellow))]
                } else {
                    vec![]
                };

                if cat == Category::Freestyle {
                    let mut spans = vec![
                        num_span,
                        Span::styled(
                            format!("> {topic_name} ({attempted}/{total})"),
                            Style::new().fg(Color::White),
                        ),
                    ];
                    spans.extend(stale_suffix);
                    return ListItem::new(Line::from(spans));
                }

                let all_done = attempted == *total && *total > 0;
                let all_perfect = all_done
                    && self
                        .topics
                        .iter()
                        .find(|t| t.id == *topic_id)
                        .is_some_and(|t| {
                            t.challenges
                                .iter()
                                .all(|c| state.best_grade(&c.id) == Some(Grade::A))
                        });

                let prefix = if all_perfect {
                    "* "
                } else if all_done {
                    "+ "
                } else {
                    "> "
                };
                let style = if all_perfect {
                    Style::new().fg(Color::Magenta).add_modifier(Modifier::BOLD)
                } else if all_done {
                    Style::new().fg(Color::Green)
                } else {
                    Style::new().fg(Color::White)
                };

                let mut spans = vec![
                    num_span,
                    Span::styled(format!("{prefix}{topic_name} ({attempted}/{total})"), style),
                ];
                spans.extend(stale_suffix);
                ListItem::new(Line::from(spans))
            }
        }
    }

    fn render_topic_detail(frame: &mut Frame, area: Rect, topic: &Topic, state: &GameState) {
        let cat = Category::for_topic(topic.id);

        let mut lines = vec![];

        let mut spans = vec![Span::styled("Description: ", Style::new().fg(Color::Gray))];
        let tag_style = Style::new().fg(Color::White).bg(Color::DarkGray);
        if cat == Category::Freestyle {
            spans.push(Span::styled(format!(" {} ", topic.description), tag_style));
        } else {
            for (i, skill) in topic.description.split(", ").enumerate() {
                if i > 0 {
                    spans.push(Span::raw(" "));
                }
                spans.push(Span::styled(format!(" {skill} "), tag_style));
            }
        }
        lines.push(Line::from(spans));
        lines.push(Line::from(""));

        let is_freestyle = cat == Category::Freestyle;
        let stale_span = Span::styled(" *", Style::new().fg(Color::Yellow));
        for challenge in &topic.challenges {
            let is_stale = state.is_stale(&challenge.id);
            if is_freestyle {
                let (badge, badge_style) = if let Some(best) = state.best_keystrokes(&challenge.id)
                {
                    (format!("[{best}]"), Style::new().fg(Color::Cyan))
                } else {
                    ("[-]".to_string(), Style::new().fg(Color::Gray))
                };
                let title_style = if state.best_keystrokes(&challenge.id).is_some() {
                    Style::new()
                } else {
                    Style::new().fg(Color::Gray)
                };
                let mut spans = vec![
                    Span::styled(format!("{badge} "), badge_style),
                    Span::styled(challenge.title.as_str(), title_style),
                ];
                if is_stale {
                    spans.push(stale_span.clone());
                }
                lines.push(Line::from(spans));
            } else {
                let (grade_str, grade_style) = grade_display(state.best_grade(&challenge.id));
                let title_style = if state.best_grade(&challenge.id).is_some() {
                    Style::new()
                } else {
                    Style::new().fg(Color::Gray)
                };
                let mut spans = vec![
                    Span::styled(format!("[{grade_str}] "), grade_style),
                    Span::styled(challenge.title.as_str(), title_style),
                ];
                if is_stale {
                    spans.push(stale_span.clone());
                }
                lines.push(Line::from(spans));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press ENTER to browse challenges",
            Style::new().fg(Color::Green),
        )));

        let detail = Paragraph::new(lines).block(Block::bordered().title(" Details "));
        frame.render_widget(detail, area);
    }

    fn is_item_selectable(&self, idx: usize, state: &GameState) -> bool {
        match &self.list_items[idx] {
            HubListItem::Spacer | HubListItem::Header(_) => false,
            HubListItem::Entry { topic_id, .. } => is_category_unlocked(
                Category::for_topic(*topic_id),
                &self.topics,
                state,
                self.unlock_all,
            ),
        }
    }

    fn next(&mut self, state: &GameState) {
        if self.list_items.is_empty() {
            return;
        }
        if let Some(i) = self.list_state.selected() {
            let len = self.list_items.len();
            let mut next = (i + 1) % len;
            // Skip headers and locked entries
            let start = next;
            while !self.is_item_selectable(next, state) {
                next = (next + 1) % len;
                if next == start {
                    return; // No selectable items
                }
            }
            self.list_state.select(Some(next));
        }
    }

    fn previous(&mut self, state: &GameState) {
        if self.list_items.is_empty() {
            return;
        }
        if let Some(i) = self.list_state.selected() {
            let len = self.list_items.len();
            let mut prev = if i == 0 { len - 1 } else { i - 1 };
            // Skip headers and locked entries
            let start = prev;
            while !self.is_item_selectable(prev, state) {
                prev = if prev == 0 { len - 1 } else { prev - 1 };
                if prev == start {
                    return; // No selectable items
                }
            }
            self.list_state.select(Some(prev));
        }
    }

    fn jump_first(&mut self, state: &GameState) {
        for i in 0..self.list_items.len() {
            if self.is_item_selectable(i, state) {
                self.list_state.select(Some(i));
                return;
            }
        }
    }

    fn jump_last(&mut self, state: &GameState) {
        for i in (0..self.list_items.len()).rev() {
            if self.is_item_selectable(i, state) {
                self.list_state.select(Some(i));
                return;
            }
        }
    }
}

/// A category is unlocked if all challenges in the previous category have been completed.
fn is_category_unlocked(
    cat: Category,
    topics: &[Topic],
    state: &GameState,
    unlock_all: bool,
) -> bool {
    if unlock_all {
        return true;
    }
    let prev = match cat {
        Category::Beginner | Category::Freestyle => return true,
        Category::Intermediate => Category::Beginner,
        Category::Advanced => Category::Intermediate,
        Category::Legendary => Category::Advanced,
    };
    topics
        .iter()
        .filter(|t| Category::for_topic(t.id) == prev && !t.challenges.is_empty())
        .all(|t| {
            t.challenges
                .iter()
                .all(|c| state.best_grade(&c.id).is_some())
        })
}
