use crate::{
    diff::DiffParser,
    github::{Commit, FileChange, GitHubClient, PullRequest},
    settings::Settings,
    theme::Theme,
    ui::{DiffView, Navigation, Sidebar},
};
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusedPane {
    Sidebar,
    DiffView,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadingStepStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone)]
pub struct LoadingStep {
    pub name: String,
    pub status: LoadingStepStatus,
}

#[derive(Debug, Clone)]
pub struct LoadingStatus {
    pub steps: Vec<LoadingStep>,
    pub current_message: String,
}

impl LoadingStatus {
    pub fn new() -> Self {
        Self {
            steps: vec![
                LoadingStep {
                    name: "Initializing client".to_string(),
                    status: LoadingStepStatus::Completed,
                },
                LoadingStep {
                    name: "Fetching PR details".to_string(),
                    status: LoadingStepStatus::Pending,
                },
                LoadingStep {
                    name: "Loading commits".to_string(),
                    status: LoadingStepStatus::Pending,
                },
                LoadingStep {
                    name: "Fetching file changes".to_string(),
                    status: LoadingStepStatus::Pending,
                },
                LoadingStep {
                    name: "Processing diffs".to_string(),
                    status: LoadingStepStatus::Pending,
                },
            ],
            current_message: "Initializing...".to_string(),
        }
    }

    pub fn update_step(&mut self, step_index: usize, status: LoadingStepStatus) {
        if let Some(step) = self.steps.get_mut(step_index) {
            step.status = status;
        }
    }

    pub fn set_current_message(&mut self, message: String) {
        self.current_message = message;
    }
}

pub enum AppState {
    Loading(LoadingStatus),
    Ready,
    Error(String),
}

pub struct App {
    pub state: AppState,
    pub should_quit: bool,
    pub pr: Option<PullRequest>,
    pub sidebar: Option<Sidebar>,
    pub diff_view: DiffView,
    pub navigation: Option<Navigation>,
    pub files: Vec<FileChange>,
    pub commits: Vec<Commit>,
    pub client: GitHubClient,
    pub owner: String,
    pub repo: String,
    pub pr_number: u64,
    pub settings: Settings,
    pub theme: Theme,
    pub focused_pane: FocusedPane,
}

impl App {
    pub async fn new(pr_url: &str, token: Option<String>) -> Result<Self> {
        let client = GitHubClient::new(token).await?;
        let parsed = GitHubClient::parse_pr_url(pr_url)?;

        // Load settings and theme
        let settings = Settings::load().unwrap_or_default();
        let theme = settings.get_theme().unwrap_or_else(|_| {
            Theme::load("catppuccin-mocha").expect("Failed to load default theme")
        });

        let mut diff_view = DiffView::new();
        diff_view.set_theme(&theme.name);

        Ok(Self {
            state: AppState::Loading(LoadingStatus::new()),
            should_quit: false,
            pr: None,
            sidebar: None,
            diff_view,
            navigation: None,
            files: Vec::new(),
            commits: Vec::new(),
            client,
            owner: parsed.owner,
            repo: parsed.repo,
            pr_number: parsed.number,
            settings,
            theme,
            focused_pane: FocusedPane::Sidebar,
        })
    }

    pub async fn load_pr_data(&mut self) -> Result<()> {
        let mut loading_status = LoadingStatus::new();

        // Load PR details
        loading_status.update_step(1, LoadingStepStatus::InProgress);
        loading_status.set_current_message("Fetching pull request details...".to_string());
        self.state = AppState::Loading(loading_status.clone());

        let pr = self
            .client
            .get_pull_request(&self.owner, &self.repo, self.pr_number)
            .await?;
        self.pr = Some(pr.clone());

        loading_status.update_step(1, LoadingStepStatus::Completed);

        // Load commits
        loading_status.update_step(2, LoadingStepStatus::InProgress);
        loading_status.set_current_message("Fetching commits...".to_string());
        self.state = AppState::Loading(loading_status.clone());

        let commits = self
            .client
            .get_pr_commits(&self.owner, &self.repo, self.pr_number)
            .await?;
        let commit_count = commits.len();
        self.commits = commits.clone();
        self.navigation = Some(Navigation::new(commits));

        loading_status.update_step(2, LoadingStepStatus::Completed);
        loading_status.steps[2].name = format!("Loading commits ({commit_count} found)");

        // Load files for the first commit
        if !self.commits.is_empty() {
            loading_status.update_step(3, LoadingStepStatus::InProgress);
            loading_status
                .set_current_message("Fetching file changes for first commit...".to_string());
            self.state = AppState::Loading(loading_status.clone());

            self.load_commit_files(0).await?;

            loading_status.update_step(3, LoadingStepStatus::Completed);
            loading_status.update_step(4, LoadingStepStatus::Completed);
        }

        self.state = AppState::Ready;
        Ok(())
    }

    pub fn toggle_focus(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::Sidebar => FocusedPane::DiffView,
            FocusedPane::DiffView => FocusedPane::Sidebar,
        };
    }

    pub fn handle_navigate_up(&mut self) {
        match self.focused_pane {
            FocusedPane::Sidebar => {
                if let Some(ref mut sidebar) = self.sidebar {
                    sidebar.previous();
                    if let Some(file) = sidebar.get_selected_file() {
                        self.diff_view.set_file(Some(file.clone()));
                    }
                }
            }
            FocusedPane::DiffView => {
                self.diff_view.scroll_up(1);
            }
        }
    }

    pub fn handle_navigate_down(&mut self) {
        match self.focused_pane {
            FocusedPane::Sidebar => {
                if let Some(ref mut sidebar) = self.sidebar {
                    sidebar.next();
                    if let Some(file) = sidebar.get_selected_file() {
                        self.diff_view.set_file(Some(file.clone()));
                    }
                }
            }
            FocusedPane::DiffView => {
                self.diff_view.scroll_down(1);
            }
        }
    }

    pub async fn handle_next_commit(&mut self) -> Result<()> {
        if let Some(ref mut nav) = self.navigation {
            if nav.next_commit() {
                // In a real app, you might want to reload files for specific commit
                // For now, we'll keep showing all PR changes
                self.reload_current_view().await?;
            }
        }
        Ok(())
    }

    pub async fn handle_prev_commit(&mut self) -> Result<()> {
        if let Some(ref mut nav) = self.navigation {
            if nav.prev_commit() {
                self.reload_current_view().await?;
            }
        }
        Ok(())
    }

    async fn reload_current_view(&mut self) -> Result<()> {
        // Load files for the current commit
        if let Some(ref nav) = self.navigation {
            let current_index = nav.get_current_index();
            let mut loading_status = LoadingStatus::new();
            loading_status.update_step(0, LoadingStepStatus::Completed);
            loading_status.update_step(1, LoadingStepStatus::Completed);
            loading_status.update_step(2, LoadingStepStatus::Completed);
            loading_status.update_step(3, LoadingStepStatus::InProgress);
            let index = current_index + 1;
            let total = self.commits.len();
            loading_status.set_current_message(format!("Loading commit {index} of {total}..."));
            self.state = AppState::Loading(loading_status);
            self.load_commit_files(current_index).await?;
            self.state = AppState::Ready;
        }
        Ok(())
    }

    pub async fn load_commit_files(&mut self, commit_index: usize) -> Result<()> {
        if commit_index >= self.commits.len() {
            return Ok(());
        }

        let commit = &self.commits[commit_index];
        let mut files = self
            .client
            .get_commit_files(&self.owner, &self.repo, &commit.sha)
            .await?;

        // Enrich files with diff content for this specific commit
        if let Some(ref pr) = self.pr {
            // For the first commit, compare with base
            // For subsequent commits, compare with previous commit
            let base_sha = if commit_index == 0 {
                pr.base.sha.clone()
            } else {
                self.commits[commit_index - 1].sha.clone()
            };

            DiffParser::enrich_file_changes(
                &mut files,
                &self.client,
                &self.owner,
                &self.repo,
                &base_sha,
                &commit.sha,
            )
            .await?;
        }

        self.files = files.clone();
        self.sidebar = Some(Sidebar::new(files));

        // Select first file by default
        if let Some(ref sidebar) = self.sidebar {
            if let Some(file) = sidebar.get_selected_file() {
                self.diff_view.set_file(Some(file.clone()));
            } else {
                self.diff_view.set_file(None);
            }
        }

        Ok(())
    }

    pub fn handle_scroll_up(&mut self) {
        self.diff_view.scroll_up(1);
    }

    pub fn handle_scroll_down(&mut self) {
        self.diff_view.scroll_down(1);
    }

    pub fn handle_page_up(&mut self) {
        self.diff_view.page_up();
    }

    pub fn handle_page_down(&mut self) {
        self.diff_view.page_down();
    }

    pub fn handle_home(&mut self) {
        self.diff_view.scroll_to_top();
    }

    pub fn handle_end(&mut self) {
        self.diff_view.scroll_to_bottom();
    }

    pub fn handle_next_hunk(&mut self) {
        self.diff_view.next_hunk();
    }

    pub fn handle_prev_hunk(&mut self) {
        self.diff_view.prev_hunk();
    }

    pub async fn handle_refresh(&mut self) -> Result<()> {
        self.load_pr_data().await
    }

    pub fn cycle_theme(&mut self) -> Result<()> {
        self.settings.cycle_theme()?;
        self.theme = self.settings.get_theme()?;
        Ok(())
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}
