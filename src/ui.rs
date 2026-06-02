use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Mode, View};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(f.area());

    draw_header(f, chunks[0], app);
    draw_list(f, chunks[1], app);
    draw_detail(f, chunks[2], app);
    draw_footer(f, chunks[3], app);

    match app.mode {
        Mode::Confirming => draw_confirm(f, app),
        Mode::Deleting => draw_progress(f, app),
        _ => {}
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let title = format!(
        " GitHub Cleanup — {} · {} {}, {} selected ",
        app.view.label(),
        app.item_count(),
        match app.view {
            View::Repos => "repos",
            View::Gists => "gists",
        },
        app.selected.len()
    );
    let filter_line = if app.filter.is_empty() && app.mode != Mode::Filtering {
        Line::from(vec![Span::styled(
            "press / to filter",
            Style::default().fg(Color::DarkGray),
        )])
    } else {
        let cursor = if app.mode == Mode::Filtering { "_" } else { "" };
        Line::from(vec![
            Span::styled("filter: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.filter),
            Span::styled(cursor, Style::default().fg(Color::Yellow)),
        ])
    };
    let p = Paragraph::new(filter_line).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    );
    f.render_widget(p, area);
}

fn draw_list(f: &mut Frame, area: Rect, app: &mut App) {
    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .map(|&i| match app.view {
            View::Repos => repo_item(app, &app.repos[i]),
            View::Gists => gist_item(app, &app.gists[i]),
        })
        .collect();

    let title = format!(" {} ", app.view.label());
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    if !app.filtered.is_empty() {
        state.select(Some(app.cursor));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn checkbox(selected: bool) -> Span<'static> {
    Span::styled(
        if selected { "[x]" } else { "[ ]" },
        Style::default().fg(if selected { Color::Green } else { Color::DarkGray }),
    )
}

fn repo_item<'a>(app: &App, r: &'a crate::github::Repo) -> ListItem<'a> {
    let selected = app.selected.contains(&r.full_name);
    let mut spans = vec![
        checkbox(selected),
        Span::raw(" "),
        Span::styled(r.full_name.clone(), Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!("  ★{}", r.stargazers_count)),
    ];
    if r.private {
        spans.push(Span::styled(" priv", Style::default().fg(Color::Red)));
    }
    if r.fork {
        spans.push(Span::styled(" fork", Style::default().fg(Color::Magenta)));
    }
    if r.archived {
        spans.push(Span::styled(" arch", Style::default().fg(Color::DarkGray)));
    }
    ListItem::new(Line::from(spans))
}

fn gist_item<'a>(app: &App, g: &'a crate::github::Gist) -> ListItem<'a> {
    let selected = app.selected.contains(&g.id);
    let files = g.file_count();
    let mut spans = vec![
        checkbox(selected),
        Span::raw(" "),
        Span::styled(g.display_name(), Style::default().add_modifier(Modifier::BOLD)),
    ];
    if files > 1 {
        spans.push(Span::styled(
            format!("  +{} files", files - 1),
            Style::default().fg(Color::DarkGray),
        ));
    }
    spans.push(if g.public {
        Span::styled(" public", Style::default().fg(Color::Yellow))
    } else {
        Span::styled(" secret", Style::default().fg(Color::DarkGray))
    });
    ListItem::new(Line::from(spans))
}

fn draw_detail(f: &mut Frame, area: Rect, app: &App) {
    let text = match app.view {
        View::Repos => match app.current_repo() {
            Some(r) => {
                let desc = r.description.clone().unwrap_or_else(|| "(no description)".to_string());
                format!("{} · updated {} · {}", desc, r.updated_at, r.html_url)
            }
            None => "(no repository)".to_string(),
        },
        View::Gists => match app.current_gist() {
            Some(g) => {
                let desc = g
                    .description
                    .clone()
                    .filter(|d| !d.is_empty())
                    .unwrap_or_else(|| "(no description)".to_string());
                let files: Vec<&str> = g.files.keys().map(|s| s.as_str()).collect();
                format!(
                    "{} · {} · updated {} · {}",
                    desc,
                    files.join(", "),
                    g.updated_at,
                    g.html_url
                )
            }
            None => "(no gist)".to_string(),
        },
    };
    let p = Paragraph::new(text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(" Details "));
    f.render_widget(p, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let help = match app.mode {
        Mode::Browsing => {
            "↑/↓ j/k move · Tab repos/gists · space select · a clear · / filter · x reset · d delete · r refresh · q quit"
        }
        Mode::Filtering => "type to filter · ↑/↓ move · Backspace · Enter/Esc exit",
        Mode::Confirming => "y confirm DELETE  ·  n / esc cancel",
        Mode::Deleting => "deleting…",
    };
    let line = if app.status.is_empty() {
        Line::from(Span::styled(help, Style::default().fg(Color::DarkGray)))
    } else {
        Line::from(vec![
            Span::styled(help, Style::default().fg(Color::DarkGray)),
            Span::raw("   "),
            Span::styled(&app.status, Style::default().fg(Color::Yellow)),
        ])
    };
    f.render_widget(Paragraph::new(line), area);
}

fn draw_confirm(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 40, f.area());
    f.render_widget(Clear, area);
    let items = app.selected_labeled();
    let noun = match app.view {
        View::Repos => "repositories",
        View::Gists => "gists",
    };
    let mut lines = vec![
        Line::from(Span::styled(
            format!("Permanently delete {} {}?", items.len(), noun),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    for (i, (_, label)) in items.iter().enumerate() {
        if i >= 10 {
            lines.push(Line::from(format!("… and {} more", items.len() - 10)));
            break;
        }
        lines.push(Line::from(format!("  • {}", label)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press 'y' to delete, 'n' or Esc to cancel.",
        Style::default().fg(Color::Yellow),
    )));

    let p = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm deletion ")
                .border_style(Style::default().fg(Color::Red)),
        );
    f.render_widget(p, area);
}

fn draw_progress(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 40, f.area());
    f.render_widget(Clear, area);
    let mut lines = vec![Line::from(Span::styled(
        "Deleting…",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    ))];
    for (_, label, res) in &app.deletion_results {
        let (sym, color) = match res {
            Ok(()) => ("✓", Color::Green),
            Err(_) => ("✗", Color::Red),
        };
        lines.push(Line::from(vec![
            Span::styled(sym, Style::default().fg(color)),
            Span::raw(format!(" {}", label)),
            match res {
                Ok(()) => Span::raw(""),
                Err(e) => Span::styled(format!("  ({})", e), Style::default().fg(Color::Red)),
            },
        ]));
    }
    let p = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).title(" Progress "));
    f.render_widget(p, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}

