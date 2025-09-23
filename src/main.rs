mod app;
mod auth;
mod cache;
mod diff;
mod events;
mod github;
mod keybindings;
mod settings;
mod syntax_highlight;
mod theme;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

use crate::{
    app::{App, AppState, LoadingStatus, LoadingStepStatus},
    diff::DiffParser,
    events::{Action, Event, EventHandler},
    github::{Commit, FileChange, GitHubClient, PullRequest},
    ui::{AppLayout, Navigation, Sidebar},
};

type PRData = (PullRequest, Vec<Commit>, Vec<FileChange>);

enum LoadingUpdate {
    Status(LoadingStatus),
    Complete(Box<Result<PRData>>),
}

#[derive(Parser, Debug)]
#[command(name = "revu")]
#[command(about = "TUI application for reviewing GitHub PRs", long_about = None)]
struct Cli {
    /// GitHub PR URL or PR number (e.g., https://github.com/owner/repo/pull/123 or 123)
    pr: String,

    /// GitHub personal access token (can also be set via GITHUB_TOKEN env var)
    #[arg(short, long)]
    token: Option<String>,

    /// Repository owner (required if using PR number instead of URL)
    #[arg(short, long)]
    owner: Option<String>,

    /// Repository name (required if using PR number instead of URL)
    #[arg(short, long)]
    repo: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Get token using priority ordering: CLI -> authinfo -> env var
    let token = auth::get_github_token(cli.token).context("Failed to get GitHub token")?;

    if token.is_none() {
        eprintln!("Warning: No GitHub token found. You may encounter rate limits.");
        eprintln!("Please provide authentication using one of these methods:");
        eprintln!("  1. Command line: --token YOUR_TOKEN");
        eprintln!(
            "  2. ~/.authinfo file: machine api.github.com login USERNAME^revu password TOKEN"
        );
        eprintln!("  3. Environment variable: export GITHUB_TOKEN=YOUR_TOKEN");
    }

    // Set owner/repo env vars if provided via CLI
    if let Some(owner) = cli.owner {
        std::env::set_var("GITHUB_OWNER", owner);
    }
    if let Some(repo) = cli.repo {
        std::env::set_var("GITHUB_REPO", repo);
    }

    // Create application
    let mut app = App::new(&cli.pr, token)
        .await
        .context("Failed to initialize application")?;

    // Check if we're in a TTY environment
    if !crossterm::tty::IsTty::is_tty(&io::stdout()) {
        eprintln!("Error: This application requires a terminal environment to run.");
        eprintln!(
            "Please run this command directly in a terminal, not through a pipe or redirect."
        );
        eprintln!("\nNote: PR data would be fetched for: {}", cli.pr);
        eprintln!("To test, run: cargo run -- {}", cli.pr);
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode()
        .context("Failed to enable raw mode - make sure you're running in a terminal")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to setup terminal")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to create terminal")?;

    // Clear terminal
    terminal.clear()?;

    // Load PR data in background
    let app_result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Return result
    app_result
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    let event_handler = EventHandler::new();

    // Create key mapping from settings
    let key_mapping = app
        .settings
        .keybindings
        .create_mapping()
        .context("Failed to create key bindings mapping")?;

    // Start loading data in background
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);
    let owner = app.owner.clone();
    let repo = app.repo.clone();
    let pr_number = app.pr_number;
    let client = app.client.clone();

    tokio::spawn(async move {
        let result = load_pr_data_async(client, owner, repo, pr_number, tx.clone()).await;
        let _ = tx.send(LoadingUpdate::Complete(Box::new(result))).await;
    });

    let mut data_loaded = false;

    loop {
        // Check for loading updates
        if !data_loaded {
            while let Ok(update) = rx.try_recv() {
                match update {
                    LoadingUpdate::Status(status) => {
                        app.state = AppState::Loading(status);
                    }
                    LoadingUpdate::Complete(result) => {
                        match *result {
                            Ok((pr, commits, files)) => {
                                app.pr = Some(pr);
                                app.commits = commits.clone();
                                app.navigation = Some(Navigation::new(commits));

                                // Load files for first commit
                                if !app.commits.is_empty() {
                                    if let Ok(()) = app.load_commit_files(0).await {
                                        // Load the first file's diff content for immediate display
                                        if !app.files.is_empty() {
                                            let _ = app.load_file_diff(0).await;
                                            // Update diff view with the loaded content
                                            if let Some(ref sidebar) = app.sidebar {
                                                if let Some(file) = sidebar.get_selected_file() {
                                                    app.diff_view.set_file(Some(file.clone()));
                                                }
                                            }
                                        }
                                        app.state = AppState::Ready;
                                    } else {
                                        app.state = AppState::Error(
                                            "Failed to load commit files".to_string(),
                                        );
                                    }
                                } else {
                                    app.files = files.clone();
                                    app.sidebar = Some(Sidebar::new(files));
                                    app.state = AppState::Ready;
                                }

                                data_loaded = true;
                            }
                            Err(e) => {
                                app.state = AppState::Error(format!("Failed to load PR data: {e}"));
                                data_loaded = true;
                            }
                        }
                    }
                }
            }
        }

        // Draw UI
        terminal.draw(|f| {
            let size = f.area();

            match &app.state {
                AppState::Loading(status) => {
                    AppLayout::render_loading_checklist(f, size, status, &app.theme);
                }
                AppState::Error(error) => {
                    AppLayout::render_error(f, size, error, &app.theme);
                }
                AppState::Ready => {
                    let (sidebar_area, diff_area, nav_area) = AppLayout::split_main(size);

                    // Render sidebar with focus state
                    if let Some(ref mut sidebar) = app.sidebar {
                        let is_focused = matches!(app.focused_pane, app::FocusedPane::Sidebar);
                        sidebar.render(f, sidebar_area, &app.theme, is_focused);
                    }

                    // Render diff view with focus state
                    let is_diff_focused = matches!(app.focused_pane, app::FocusedPane::DiffView);
                    app.diff_view
                        .render(f, diff_area, &app.theme, is_diff_focused);

                    // Render navigation with current focus
                    if let Some(ref navigation) = app.navigation {
                        navigation.render(
                            f,
                            nav_area,
                            &app.theme,
                            app.focused_pane,
                            &app.settings.keybindings,
                        );
                    }
                }
            }
        })?;

        // No longer need loading animation frame

        // Handle events
        if let Some(event) = event_handler.poll(Duration::from_millis(100))? {
            match event {
                Event::Key(key) => {
                    if let Some(action) = Action::from_key_event(key, &key_mapping) {
                        match action {
                            Action::Quit => {
                                app.quit();
                            }
                            Action::ToggleFocus => {
                                app.toggle_focus();
                            }
                            Action::NavigateUp => {
                                app.handle_navigate_up().await?;
                            }
                            Action::NavigateDown => {
                                app.handle_navigate_down().await?;
                            }
                            Action::NextCommit => {
                                app.handle_next_commit().await?;
                            }
                            Action::PrevCommit => {
                                app.handle_prev_commit().await?;
                            }
                            Action::ScrollUp => {
                                app.handle_scroll_up();
                            }
                            Action::ScrollDown => {
                                app.handle_scroll_down();
                            }
                            Action::PageUp => {
                                app.handle_page_up();
                            }
                            Action::PageDown => {
                                app.handle_page_down();
                            }
                            Action::Home => {
                                app.handle_home();
                            }
                            Action::End => {
                                app.handle_end();
                            }
                            Action::Refresh => {
                                app.handle_refresh().await?;
                            }
                            Action::CycleTheme => {
                                app.cycle_theme()?;
                            }
                            Action::NextHunk => {
                                app.handle_next_hunk();
                            }
                            Action::PrevHunk => {
                                app.handle_prev_hunk();
                            }
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    // Handle mouse events
                    use crossterm::event::MouseEventKind;
                    match mouse.kind {
                        MouseEventKind::ScrollDown => {
                            if matches!(app.state, AppState::Ready) {
                                app.diff_view.scroll_down(3);
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            if matches!(app.state, AppState::Ready) {
                                app.diff_view.scroll_up(3);
                            }
                        }
                        MouseEventKind::Down(_button) => {
                            // Could implement click handling for sidebar items in future
                        }
                        _ => {}
                    }
                }
                Event::Resize => {
                    // Terminal will automatically redraw on next iteration
                }
                Event::Tick => {
                    // Regular tick, nothing to do
                }
            }
        }

        // Check if we should quit
        if app.should_quit {
            break;
        }
    }

    Ok(())
}

async fn load_pr_data_async(
    client: GitHubClient,
    owner: String,
    repo: String,
    pr_number: u64,
    tx: tokio::sync::mpsc::Sender<LoadingUpdate>,
) -> Result<PRData> {
    let mut loading_status = LoadingStatus::new();

    // Load PR details
    loading_status.update_step(1, LoadingStepStatus::InProgress);
    loading_status.set_current_message("Fetching pull request details...".to_string());
    let _ = tx.send(LoadingUpdate::Status(loading_status.clone())).await;

    let pr = client.get_pull_request(&owner, &repo, pr_number).await?;

    loading_status.update_step(1, LoadingStepStatus::Completed);
    let _ = tx.send(LoadingUpdate::Status(loading_status.clone())).await;

    // Load commits
    loading_status.update_step(2, LoadingStepStatus::InProgress);
    loading_status.set_current_message("Fetching commits...".to_string());
    let _ = tx.send(LoadingUpdate::Status(loading_status.clone())).await;

    let commits = client.get_pr_commits(&owner, &repo, pr_number).await?;
    let commit_count = commits.len();

    loading_status.update_step(2, LoadingStepStatus::Completed);
    loading_status.steps[2].name = format!("Loading commits ({commit_count} found)");
    let _ = tx.send(LoadingUpdate::Status(loading_status.clone())).await;

    // Load files
    loading_status.update_step(3, LoadingStepStatus::InProgress);
    loading_status.set_current_message("Fetching file changes...".to_string());
    let _ = tx.send(LoadingUpdate::Status(loading_status.clone())).await;

    let mut files = client.get_pr_files(&owner, &repo, pr_number).await?;

    loading_status.update_step(3, LoadingStepStatus::Completed);
    let _ = tx.send(LoadingUpdate::Status(loading_status.clone())).await;

    // Enrich files with diff content
    loading_status.update_step(4, LoadingStepStatus::InProgress);
    loading_status.set_current_message("Processing diffs...".to_string());
    let _ = tx.send(LoadingUpdate::Status(loading_status.clone())).await;

    DiffParser::enrich_file_changes(
        &mut files,
        &client,
        &owner,
        &repo,
        &pr.base.sha,
        &pr.head.sha,
    )
    .await?;

    loading_status.update_step(4, LoadingStepStatus::Completed);
    let _ = tx.send(LoadingUpdate::Status(loading_status.clone())).await;

    Ok((pr, commits, files))
}
