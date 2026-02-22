use ratatui::Frame;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};
use std::time::Duration;

use crate::challenge::{Category, Medal, Topic, medal_display};
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
                KeyCode::Char('q' | 'h') | KeyCode::Esc | KeyCode::Left => {
                    return Ok(());
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if let Some(mut i) = list_state.selected() {
                        for _ in 0..n {
                            i = (i + 1) % len;
                        }
                        list_state.select(Some(i));
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
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
                KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                    if let Some(i) = list_state.selected() {
                        let challenge = &topic.challenges[i];
                        let number = challenge_offset + i + 1;
                        play_challenge_loop(terminal, state, challenge, number)?;
                    }
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
    let mut first = true;
    loop {
        if first {
            if !show_challenge_intro(terminal, challenge)? {
                return Ok(());
            }
            first = false;
        }

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
            let medal = if result.buffer_matches {
                let m = challenge.score(result.keystrokes);
                if let Some(medal) = m {
                    state.record_result(
                        &challenge.id,
                        medal,
                        result.keystrokes,
                        result.elapsed_secs,
                        &result.keys,
                        &challenge.version,
                    );
                }
                m
            } else {
                None
            };

            // Show result
            let retry = show_result_screen(
                terminal,
                challenge,
                medal,
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
            format!(" {} ", topic.name),
            Style::new()
                .fg(Color::Black)
                .bg(cat_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(format!(" {} ", cat.name()), Style::new().fg(cat_color)),
    ]))
    .block(Block::bordered());
    frame.render_widget(title, header);

    // Stats line
    let attempted = topic
        .challenges
        .iter()
        .filter(|c| state.best_medal(&c.id).is_some())
        .count();
    let total = topic.challenges.len();
    frame.render_widget(
        Paragraph::new(format!(" Progress: {attempted}/{total}"))
            .style(Style::new().fg(Color::Gray)),
        stats_area,
    );

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
                let (s, st) = medal_display(state.best_medal(&c.id));
                (format!("[{s}]"), st)
            };
            let title_style = if state.best_medal(&c.id).is_some() {
                Style::new().fg(cat_color)
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
        render_challenge_detail(frame, detail_area, challenge, state, topic.id);
    }

    // Footer
    frame.render_widget(
        Paragraph::new(" j/k: navigate | l/Enter: play | h/q: back")
            .style(Style::new().fg(Color::DarkGray)),
        footer,
    );
}

fn render_challenge_detail(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    challenge: &crate::challenge::Challenge,
    state: &GameState,
    topic_id: u8,
) {
    let cat = Category::for_topic(topic_id);
    let mut lines = vec![
        Line::from(Span::styled(
            &challenge.title,
            Style::new().fg(cat.color()).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Hint: {}", challenge.hint),
            Style::new().fg(Color::Gray),
        )),
    ];

    // Show focused actions if available
    if let Some(actions) = &challenge.focused_actions {
        lines.push(Line::from(Span::styled(
            format!("Skills: {}", actions.join(", ")),
            Style::new().fg(Color::Green),
        )));
    }

    lines.push(Line::from(""));
    if challenge.is_freestyle() {
        if let Some(best) = state.best_keystrokes(&challenge.id) {
            lines.push(Line::from(Span::styled(
                format!("Personal best: {best} keystrokes"),
                Style::new().fg(Color::Cyan),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "Not yet attempted",
                Style::new().fg(Color::Gray),
            )));
        }
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
            let (label, style) = medal_display(Some(attempt.medal));
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
    // Reserve 2 lines for border, count lines used so far
    let used = lines.len();
    let available = area.height.saturating_sub(2) as usize; // 2 for border
    let remaining = available.saturating_sub(used + 2); // 2 for header + blank line
    let target_lines: Vec<&str> = challenge.target.content.lines().collect();
    if remaining > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Target:",
            Style::new().fg(Color::Yellow),
        )));
        let show = remaining.min(target_lines.len());
        for line in &target_lines[..show] {
            lines.push(Line::from(Span::styled(
                format!("  {line}"),
                Style::new().fg(Color::Gray),
            )));
        }
        if target_lines.len() > show {
            lines.push(Line::from(Span::styled(
                format!("  ... ({} more lines)", target_lines.len() - show),
                Style::new().fg(Color::Gray),
            )));
        }
    }

    let detail = Paragraph::new(lines)
        .block(Block::bordered().title(" Details "))
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, area);
}

/// Show the challenge intro screen. Returns Ok(true) to start, Ok(false) to go back.
fn show_challenge_intro(
    terminal: &mut ratatui::DefaultTerminal,
    challenge: &crate::challenge::Challenge,
) -> std::io::Result<bool> {
    loop {
        terminal.draw(|frame| {
            let [main, footer] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    &challenge.title,
                    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
            ];

            if challenge.is_freestyle() {
                lines.push(Line::from(Span::styled(
                    "Freestyle \u{2014} minimize keystrokes!",
                    Style::new().fg(Color::Cyan),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    format!("Par: {} keystrokes", challenge.par_keystrokes),
                    Style::new().fg(Color::Yellow),
                )));
            }

            lines.extend([
                Line::from(""),
                Line::from(Span::styled(
                    format!("Hint: {}", challenge.hint),
                    Style::new().fg(Color::Gray),
                )),
            ]);

            if let Some(actions) = &challenge.focused_actions {
                lines.push(Line::from(Span::styled(
                    format!("Skills: {}", actions.join(", ")),
                    Style::new().fg(Color::Green),
                )));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "How to play:",
                Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "  Edit the bottom buffer to match the top target",
                Style::new().fg(Color::Gray),
            )));
            lines.push(Line::from(Span::styled(
                "  Differences are highlighted with diff colors",
                Style::new().fg(Color::Gray),
            )));
            lines.push(Line::from(Span::styled(
                "  F1: cycle hints | :w to finish early",
                Style::new().fg(Color::Gray),
            )));
            lines.push(Line::from(Span::styled(
                "  Auto-completes when buffer matches target",
                Style::new().fg(Color::Gray),
            )));

            let intro = Paragraph::new(lines)
                .block(Block::bordered().title(" Challenge "))
                .wrap(Wrap { trim: false });
            frame.render_widget(intro, main);

            frame.render_widget(
                Paragraph::new(" ENTER: start | q: back").style(Style::new().fg(Color::DarkGray)),
                footer,
            );
        })?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Enter => return Ok(true),
                KeyCode::Char('q') | KeyCode::Esc => return Ok(false),
                _ => {}
            }
        }
    }
}

/// Show the result screen. Returns true if the user wants to retry.
/// `personal_best` is the previous best keystroke count for freestyle challenges.
fn show_result_screen(
    terminal: &mut ratatui::DefaultTerminal,
    challenge: &crate::challenge::Challenge,
    medal: Option<Medal>,
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
                    (
                        "FAILED - Buffer doesn't match target".to_string(),
                        Color::Red,
                    )
                }
            } else if let Some(m) = medal {
                let medal_name = match m {
                    Medal::Perfect => "PERFECT",
                    Medal::Gold => "GOLD",
                    Medal::Silver => "SILVER",
                    Medal::Bronze => "BRONZE",
                };
                (format!("PASSED - {medal_name}"), m.color())
            } else if !buffer_matched {
                (
                    "FAILED - Buffer doesn't match target".to_string(),
                    Color::Red,
                )
            } else {
                ("FAILED - Too many keystrokes".to_string(), Color::Red)
            };

            let keystroke_line = if freestyle {
                format!(
                    "Keystrokes: {} | Time: {:02}:{:02}",
                    keystrokes,
                    elapsed_secs / 60,
                    elapsed_secs % 60
                )
            } else {
                format!(
                    "Keystrokes: {} (par: {}) | Time: {:02}:{:02}",
                    keystrokes,
                    challenge.par_keystrokes,
                    elapsed_secs / 60,
                    elapsed_secs % 60
                )
            };

            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    &challenge.title,
                    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    &status,
                    Style::new().fg(status_color).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(keystroke_line),
            ];

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
    let medals = [Medal::Perfect, Medal::Gold, Medal::Silver, Medal::Bronze];
    let mut spans = vec![Span::raw("  ")];
    for (i, &m) in medals.iter().enumerate() {
        if i > 0 {
            spans.push(sep.clone());
        }
        spans.push(Span::styled(m.display_char(), m.style()));
        spans.push(Span::styled(format!(": <={}", challenge.threshold(m)), dim));
    }
    Line::from(spans)
}
