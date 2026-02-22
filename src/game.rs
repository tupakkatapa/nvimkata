use ratatui::Frame;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};
use std::time::Duration;

use crate::challenge::{Category, Grade, Topic, grade_display};
use crate::nvim;
use crate::state::GameState;

/// Run the challenge picker for a topic. Lets user select and play individual challenges.
/// `challenge_offset` is the number of challenges in all preceding topics, used for
/// globally unique display numbers.
pub fn run_challenge_picker(
    terminal: &mut ratatui::DefaultTerminal,
    state: &mut GameState,
    topic: &Topic,
    challenge_offset: usize,
) -> std::io::Result<()> {
    if topic.challenges.is_empty() {
        return Ok(());
    }

    let mut list_state = ListState::default();
    list_state.select(Some(0));
    let mut pending_g = false;
    let mut count: Option<u32> = None;
    let mut list_height: u16 = 0;

    loop {
        terminal.draw(|frame| {
            render_picker(frame, topic, state, &mut list_state, &mut list_height);
        })?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            let len = topic.challenges.len();

            // Handle pending gg
            if pending_g {
                pending_g = false;
                count = None;
                if key.code == KeyCode::Char('g') {
                    list_state.select(Some(0));
                    continue;
                }
            }

            // Count prefix (applied to j/k)
            match key.code {
                KeyCode::Char(c @ '1'..='9') => {
                    count = Some(count.unwrap_or(0) * 10 + (c as u32 - '0' as u32));
                    continue;
                }
                KeyCode::Char('0') if count.is_some() => {
                    count = count.map(|c| c * 10);
                    continue;
                }
                _ => {}
            }

            let n = count.unwrap_or(1) as usize;
            count = None;

            match key.code {
                KeyCode::Char('q' | 'h') | KeyCode::Esc => {
                    return Ok(());
                }
                KeyCode::Char('j') => {
                    if let Some(mut i) = list_state.selected() {
                        for _ in 0..n {
                            i = (i + 1) % len;
                        }
                        list_state.select(Some(i));
                    }
                }
                KeyCode::Char('k') => {
                    if let Some(mut i) = list_state.selected() {
                        for _ in 0..n {
                            i = if i == 0 { len - 1 } else { i - 1 };
                        }
                        list_state.select(Some(i));
                    }
                }
                KeyCode::Char('g') => pending_g = true,
                KeyCode::Char('G') => list_state.select(Some(len - 1)),
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if let Some(mut i) = list_state.selected() {
                        let half = (list_height / 2).max(1) as usize;
                        for _ in 0..half {
                            i = (i + 1) % len;
                        }
                        list_state.select(Some(i));
                    }
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if let Some(mut i) = list_state.selected() {
                        let half = (list_height / 2).max(1) as usize;
                        for _ in 0..half {
                            i = if i == 0 { len - 1 } else { i - 1 };
                        }
                        list_state.select(Some(i));
                    }
                }
                KeyCode::Char('l') | KeyCode::Enter => {
                    if let Some(i) = list_state.selected() {
                        let challenge = &topic.challenges[i];
                        let number = challenge_offset + i + 1;
                        play_challenge_loop(terminal, state, challenge, number)?;
                    }
                }
                KeyCode::Char('?') => {
                    show_help(terminal)?;
                }
                _ => {}
            }
        }
    }
}

/// Play a single challenge with retry support.
fn play_challenge_loop(
    terminal: &mut ratatui::DefaultTerminal,
    state: &mut GameState,
    challenge: &crate::challenge::Challenge,
    number: usize,
) -> std::io::Result<()> {
    let freestyle = challenge.is_freestyle();
    loop {
        ratatui::restore();
        let result = nvim::run_challenge(challenge, number)?;
        *terminal = ratatui::init();

        if freestyle {
            let personal_best = state.best_keystrokes(&challenge.id);
            if result.buffer_matches {
                state.record_freestyle_result(
                    &challenge.id,
                    result.keystrokes,
                    result.elapsed_secs,
                    &result.keys,
                    &challenge.version,
                );
            }

            let retry = show_result_screen(
                terminal,
                challenge,
                number,
                None,
                result.keystrokes,
                result.elapsed_secs,
                result.buffer_matches,
                personal_best,
            )?;

            state.save().ok();
            if !retry {
                return Ok(());
            }
        } else {
            // Score
            let grade = if result.buffer_matches {
                let grade = challenge.score(result.keystrokes);
                state.record_result(
                    &challenge.id,
                    grade,
                    result.keystrokes,
                    result.elapsed_secs,
                    &result.keys,
                    &challenge.version,
                );
                Some(grade)
            } else {
                None
            };

            // Show result
            let retry = show_result_screen(
                terminal,
                challenge,
                number,
                grade,
                result.keystrokes,
                result.elapsed_secs,
                result.buffer_matches,
                None,
            )?;

            state.save().ok();

            if !retry {
                return Ok(());
            }
        }
    }
}

fn render_picker(
    frame: &mut Frame,
    topic: &Topic,
    state: &GameState,
    list_state: &mut ListState,
    list_height: &mut u16,
) {
    let cat = Category::for_topic(topic.id);
    let cat_color = cat.color();

    let [header, stats_area, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    // Header
    let title = Paragraph::new(Line::from(vec![
        Span::raw(" "),
        Span::styled(
            format!(" {} ", cat.name()),
            Style::new()
                .fg(Color::Black)
                .bg(cat_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(&topic.name, Style::new().add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::bordered());
    frame.render_widget(title, header);

    frame.render_widget(Paragraph::new(topic_stats_line(topic, state)), stats_area);

    // Challenge list
    let [list_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(body);

    *list_height = list_area.height.saturating_sub(2);

    let selected = list_state.selected().unwrap_or(0);
    let num_style = Style::new().fg(Color::DarkGray);
    let is_freestyle = cat == Category::Freestyle;
    let items: Vec<ListItem> = topic
        .challenges
        .iter()
        .enumerate()
        .map(|(n, c)| {
            let num_span = Span::styled(format!("{:>2} ", n.abs_diff(selected)), num_style);
            let (badge, badge_style) = if is_freestyle {
                if let Some(best) = state.best_keystrokes(&c.id) {
                    (format!("[{best}]"), Style::new().fg(Color::Cyan))
                } else {
                    ("[-]".to_string(), Style::new().fg(Color::Gray))
                }
            } else {
                let (s, st) = grade_display(state.best_grade(&c.id));
                (format!("[{s}]"), st)
            };
            let title_style = if state.best_grade(&c.id).is_some() {
                Style::new()
            } else {
                Style::new().fg(Color::Gray)
            };
            let mut spans = vec![
                num_span,
                Span::styled(format!("{badge} "), badge_style),
                Span::styled(c.title.as_str(), title_style),
            ];
            if state.is_stale(&c.id) {
                spans.push(Span::styled(" *", Style::new().fg(Color::Yellow)));
            }
            let text = Line::from(spans);
            ListItem::new(text)
        })
        .collect();

    let list = List::new(items)
        .block(Block::bordered().title(" Challenges "))
        .highlight_style(
            Style::new()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, list_area, list_state);

    // Detail panel for selected challenge
    if let Some(i) = list_state.selected() {
        let challenge = &topic.challenges[i];
        render_challenge_detail(frame, detail_area, challenge, state);
    }

    // Footer
    frame.render_widget(
        Paragraph::new(" j/k: navigate | l/Enter: play | ?: help | h/q: back")
            .style(Style::new().fg(Color::DarkGray)),
        footer,
    );
}

fn topic_stats_line<'a>(topic: &Topic, state: &GameState) -> Line<'a> {
    let attempted = topic
        .challenges
        .iter()
        .filter(|c| state.best_grade(&c.id).is_some())
        .count();
    let total = topic.challenges.len();
    let perfects = topic
        .challenges
        .iter()
        .filter(|c| state.best_grade(&c.id) == Some(Grade::A))
        .count();
    let outdated = topic
        .challenges
        .iter()
        .filter(|c| state.is_stale(&c.id))
        .count();
    let attempts: usize = topic
        .challenges
        .iter()
        .filter_map(|c| state.history.get(&c.id))
        .map(Vec::len)
        .sum();
    let mut spans = vec![Span::styled(
        format!(" Completed: {attempted}/{total} | Grade A: {perfects} | Attempts: {attempts}"),
        Style::new().fg(Color::Gray),
    )];
    if outdated > 0 {
        spans.push(Span::styled(" | ", Style::new().fg(Color::Gray)));
        spans.push(Span::styled(
            format!("Warning: {outdated} score(s) outdated"),
            Style::new().fg(Color::Yellow),
        ));
    }
    Line::from(spans)
}

fn render_challenge_detail(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    challenge: &crate::challenge::Challenge,
    state: &GameState,
) {
    let mut lines = vec![];

    // Show focused actions if available
    if let Some(actions) = &challenge.focused_actions {
        let mut spans = vec![Span::styled("Skills: ", Style::new().fg(Color::Gray))];
        for (i, action) in actions.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                format!(" {action} "),
                Style::new().fg(Color::White).bg(Color::DarkGray),
            ));
        }
        lines.push(Line::from(spans));
        lines.push(Line::from(""));
    }

    if challenge.is_freestyle() {
        let best_str = state
            .best_keystrokes(&challenge.id)
            .map_or("N/A".to_string(), |b| format!("{b} keystrokes"));
        lines.push(Line::from(format!("Personal best: {best_str}")));
    } else {
        lines.push(Line::from(format!(
            "Par: {} keystrokes",
            challenge.par_keystrokes
        )));
        lines.push(threshold_line(challenge));
    }

    // Top 3 attempts with key presses
    if let Some(history) = state.history.get(&challenge.id)
        && !history.is_empty()
    {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Top attempts:",
            Style::new().fg(Color::Yellow),
        )));
        for (i, attempt) in history.iter().take(3).enumerate() {
            let (label, style) = grade_display(Some(attempt.grade));
            lines.push(Line::from(vec![
                Span::raw(format!("  {}. ", i + 1)),
                Span::styled(format!("[{label}]"), style),
                Span::raw(format!(
                    " {} | {} keys | {:02}:{:02}",
                    attempt.keys,
                    attempt.keystrokes,
                    attempt.time_secs / 60,
                    attempt.time_secs % 60
                )),
            ]));
        }
    }

    // Show target content (truncated to fit remaining space)
    // Reserve lines for: border(2) + header/blank(2) + "Press ENTER" footer(2)
    let used = lines.len();
    let available = area.height.saturating_sub(2) as usize;
    let remaining = available.saturating_sub(used + 4);
    let target_lines: Vec<&str> = challenge.target.content.lines().collect();
    if remaining > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Preview:",
            Style::new().add_modifier(Modifier::BOLD),
        )));
        let show = remaining.min(target_lines.len()).min(20);
        for (i, line) in target_lines[..show].iter().enumerate() {
            lines.push(Line::from(vec![
                Span::styled(format!("{:>3} ", i + 1), Style::new().fg(Color::DarkGray)),
                Span::styled(*line, Style::new().fg(Color::Gray)),
            ]));
        }
        if target_lines.len() > show {
            lines.push(Line::from(Span::styled(
                format!("  ... ({} more lines)", target_lines.len() - show),
                Style::new().fg(Color::DarkGray),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press ENTER to start challenge",
        Style::new().fg(Color::Green),
    )));

    let detail = Paragraph::new(lines)
        .block(Block::bordered().title(" Details "))
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, area);
}

/// Show the how-to-play help screen. Blocks until any key is pressed.
pub fn show_help(terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
    loop {
        terminal.draw(|frame| {
            let [main, footer] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

            let dim = Style::new().fg(Color::Gray);
            let bold = Style::new().fg(Color::White).add_modifier(Modifier::BOLD);
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(" How to play", bold)),
                Line::from(""),
                Line::from(Span::styled(
                    "   The screen splits into a read-only target (top) and",
                    dim,
                )),
                Line::from(Span::styled(
                    "   your editable buffer (bottom). Edit until the diff",
                    dim,
                )),
                Line::from(Span::styled(
                    "   disappears — the challenge auto-completes when your",
                    dim,
                )),
                Line::from(Span::styled("   buffer matches the target.", dim)),
                Line::from(""),
                Line::from(Span::styled(" Modes", bold)),
                Line::from(""),
                Line::from(Span::styled(
                    "   Graded     Beat the par keystroke count for Grade A.",
                    dim,
                )),
                Line::from(Span::styled(
                    "              Grades A-F based on how close you get.",
                    dim,
                )),
                Line::from(Span::styled(
                    "   Freestyle  No par. Minimize keystrokes, track your",
                    dim,
                )),
                Line::from(Span::styled("              personal best.", dim)),
                Line::from(""),
                Line::from(Span::styled(" Controls", bold)),
                Line::from(""),
                Line::from(Span::styled(
                    "   F1     Show hint (again for detailed hint)",
                    dim,
                )),
                Line::from(Span::styled("   :w     Finish early and submit", dim)),
            ];

            let help = Paragraph::new(lines)
                .block(Block::bordered().title(" Help "))
                .wrap(Wrap { trim: false });
            frame.render_widget(help, main);

            frame.render_widget(
                Paragraph::new(" any key: back").style(Style::new().fg(Color::DarkGray)),
                footer,
            );
        })?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            return Ok(());
        }
    }
}

/// Show the result screen. Returns true if the user wants to retry.
/// `personal_best` is the previous best keystroke count for freestyle challenges.
#[allow(clippy::too_many_arguments)]
fn show_result_screen(
    terminal: &mut ratatui::DefaultTerminal,
    challenge: &crate::challenge::Challenge,
    number: usize,
    grade: Option<Grade>,
    keystrokes: u32,
    elapsed_secs: u32,
    buffer_matched: bool,
    personal_best: Option<u32>,
) -> std::io::Result<bool> {
    let freestyle = challenge.is_freestyle();
    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let (status, status_color) = if freestyle {
                if buffer_matched {
                    let is_new_best = personal_best.is_none_or(|prev| keystrokes < prev);
                    if is_new_best {
                        ("COMPLETED (NEW BEST!)".to_string(), Color::Cyan)
                    } else {
                        ("COMPLETED".to_string(), Color::Green)
                    }
                } else {
                    ("FAILED".to_string(), Color::Red)
                }
            } else if let Some(g) = grade {
                let grade_name = match g {
                    Grade::A => "GRADE A",
                    Grade::B => "GRADE B",
                    Grade::C => "GRADE C",
                    Grade::D => "GRADE D",
                    Grade::E => "GRADE E",
                    Grade::F => "GRADE F",
                };
                (grade_name.to_string(), g.color())
            } else {
                ("FAILED".to_string(), Color::Red)
            };

            let time_str = format!("{:02}:{:02}", elapsed_secs / 60, elapsed_secs % 60);

            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!(" #{number:03} - {}", challenge.title),
                    Style::new().add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
            ];

            let dim = Style::new().fg(Color::Gray);
            lines.push(Line::from(Span::styled(
                format!(" {status}"),
                Style::new().fg(status_color).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            if freestyle {
                lines.push(Line::from(vec![
                    Span::styled(" Keystrokes: ", dim),
                    Span::raw(format!("{keystrokes}")),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled(" Keystrokes: ", dim),
                    Span::raw(format!("{keystrokes} (par: {})", challenge.par_keystrokes)),
                ]));
            }
            lines.push(Line::from(vec![
                Span::styled(" Time: ", dim),
                Span::raw(time_str),
            ]));

            let [main, footer] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);

            let result = Paragraph::new(lines).block(Block::bordered().title(" Result "));
            frame.render_widget(result, main);

            frame.render_widget(
                Paragraph::new(" r: retry | any key: back").style(Style::new().fg(Color::DarkGray)),
                footer,
            );
        })?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            return Ok(key.code == KeyCode::Char('r'));
        }
    }
}

fn threshold_line(challenge: &crate::challenge::Challenge) -> Line<'static> {
    let dim = Style::new().fg(Color::Gray);
    let sep = Span::styled(" | ", dim);
    let grades = [Grade::A, Grade::B, Grade::C, Grade::D, Grade::E, Grade::F];
    let mut spans = vec![Span::raw("  ")];
    for (i, &g) in grades.iter().enumerate() {
        if i > 0 {
            spans.push(sep.clone());
        }
        spans.push(Span::styled(g.display_char(), g.style()));
        spans.push(Span::styled(format!(": <={}", challenge.threshold(g)), dim));
    }
    Line::from(spans)
}
