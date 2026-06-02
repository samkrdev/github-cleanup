mod app;
mod github;
mod ui;

use std::{
    collections::HashSet,
    io,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

use crate::app::{App, Mode, View};
use crate::github::{Gist, GithubClient, Repo};

enum Msg {
    ReposLoaded(Result<Vec<Repo>>),
    GistsLoaded(Result<Vec<Gist>>),
    DeleteResult(String, Result<()>),
    DeletionsDone,
}

#[tokio::main]
async fn main() -> Result<()> {
    let token = std::env::var("GITHUB_TOKEN").context(
        "GITHUB_TOKEN env var not set. Create a Personal Access Token with `delete_repo` scope and export it.",
    )?;
    let client = Arc::new(GithubClient::new(token)?);

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run(&mut terminal, client).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    res
}

async fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    client: Arc<GithubClient>,
) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<Msg>();

    // Kick off initial repo + gist load
    spawn_load(client.clone(), tx.clone());
    spawn_load_gists(client.clone(), tx.clone());

    let mut app = App::new(Vec::new());
    app.status = "loading repositories & gists…".into();

    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Drain async messages
        while let Ok(msg) = rx.try_recv() {
            handle_msg(&mut app, msg);
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                handle_key(&mut app, key.code, key.modifiers, &client, &tx);
                if app.should_quit {
                    break;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    Ok(())
}

fn handle_msg(app: &mut App, msg: Msg) {
    match msg {
        Msg::ReposLoaded(Ok(repos)) => {
            let total = repos.len();
            let private = repos.iter().filter(|r| r.private).count();
            app.repos = repos;
            if app.view == View::Repos {
                app.apply_filter();
                app.cursor = 0;
                app.status = if private == 0 {
                    format!("loaded {} repos (0 private — token may lack `repo` scope)", total)
                } else {
                    format!("loaded {} repos ({} private)", total, private)
                };
            }
        }
        Msg::ReposLoaded(Err(e)) => {
            app.status = format!("repo load failed: {}", e);
        }
        Msg::GistsLoaded(Ok(gists)) => {
            let total = gists.len();
            let public = gists.iter().filter(|g| g.public).count();
            app.gists = gists;
            if app.view == View::Gists {
                app.apply_filter();
                app.cursor = 0;
                app.status = format!("loaded {} gists ({} public, {} secret)", total, public, total - public);
            }
        }
        Msg::GistsLoaded(Err(e)) => {
            app.status = format!("gist load failed: {}", e);
        }
        Msg::DeleteResult(key, res) => {
            let label = app.label_for(&key);
            app.deletion_results
                .push((key, label, res.map_err(|e| e.to_string())));
        }
        Msg::DeletionsDone => {
            let succeeded: HashSet<String> = app
                .deletion_results
                .iter()
                .filter(|(_, _, r)| r.is_ok())
                .map(|(k, _, _)| k.clone())
                .collect();
            let ok = succeeded.len();
            let fail = app.deletion_results.len() - ok;
            let what = app.view.label().to_lowercase();
            app.remove_selected(&succeeded);
            app.status = format!("deleted {} {} · failed {}", ok, what, fail);
            app.mode = Mode::Browsing;
            app.deletion_results.clear();
        }
    }
}

fn handle_key(
    app: &mut App,
    code: KeyCode,
    mods: KeyModifiers,
    client: &Arc<GithubClient>,
    tx: &mpsc::UnboundedSender<Msg>,
) {
    match app.mode {
        Mode::Deleting => {} // ignore input during deletion
        Mode::Confirming => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let keys = app.selected_keys();
                if keys.is_empty() {
                    app.mode = Mode::Browsing;
                } else {
                    app.mode = Mode::Deleting;
                    app.deletion_results.clear();
                    spawn_delete(client.clone(), tx.clone(), app.view, keys);
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.mode = Mode::Browsing;
            }
            _ => {}
        },
        Mode::Browsing => match code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('c') if mods.contains(KeyModifiers::CONTROL) => app.should_quit = true,
            KeyCode::Down | KeyCode::Char('j') => app.move_cursor(1),
            KeyCode::Up | KeyCode::Char('k') => app.move_cursor(-1),
            KeyCode::PageDown => app.move_cursor(10),
            KeyCode::PageUp => app.move_cursor(-10),
            KeyCode::Home | KeyCode::Char('g') => app.jump_top(),
            KeyCode::End | KeyCode::Char('G') => app.jump_bottom(),
            KeyCode::Char(' ') => app.toggle_selected(),
            KeyCode::Char('a') => app.clear_selection(),
            KeyCode::Tab | KeyCode::BackTab => {
                app.toggle_view();
                app.status = format!("viewing {}", app.view.label().to_lowercase());
            }
            KeyCode::Char('r') => {
                app.status = "refreshing…".into();
                spawn_load(client.clone(), tx.clone());
                spawn_load_gists(client.clone(), tx.clone());
            }
            KeyCode::Char('d') => {
                if !app.selected.is_empty() {
                    app.mode = Mode::Confirming;
                } else {
                    app.status = "no repos selected".into();
                }
            }
            KeyCode::Char('/') => {
                app.mode = Mode::Filtering;
                app.status = "filtering (Esc to exit)".into();
            }
            KeyCode::Char('x') => {
                app.filter.clear();
                app.apply_filter();
                app.status = "filter cleared".into();
            }
            _ => {}
        },
        Mode::Filtering => match code {
            KeyCode::Esc | KeyCode::Enter => {
                app.mode = Mode::Browsing;
                app.status.clear();
            }
            KeyCode::Backspace => {
                app.filter.pop();
                app.apply_filter();
            }
            KeyCode::Down => app.move_cursor(1),
            KeyCode::Up => app.move_cursor(-1),
            KeyCode::Char(c) => {
                app.filter.push(c);
                app.apply_filter();
            }
            _ => {}
        },
    }
}

fn spawn_load(client: Arc<GithubClient>, tx: mpsc::UnboundedSender<Msg>) {
    tokio::spawn(async move {
        let res = client.list_repos().await;
        let _ = tx.send(Msg::ReposLoaded(res));
    });
}

fn spawn_load_gists(client: Arc<GithubClient>, tx: mpsc::UnboundedSender<Msg>) {
    tokio::spawn(async move {
        let res = client.list_gists().await;
        let _ = tx.send(Msg::GistsLoaded(res));
    });
}

fn spawn_delete(
    client: Arc<GithubClient>,
    tx: mpsc::UnboundedSender<Msg>,
    view: View,
    keys: Vec<String>,
) {
    tokio::spawn(async move {
        for key in keys {
            let res = match view {
                View::Repos => client.delete_repo(&key).await,
                View::Gists => client.delete_gist(&key).await,
            };
            let _ = tx.send(Msg::DeleteResult(key, res));
        }
        let _ = tx.send(Msg::DeletionsDone);
    });
}
