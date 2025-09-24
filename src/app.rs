use crate::{
    cache::DiffCache,
    diff::DiffParser,
    github::{Commit, FileChange, GitHubClient, PullRequest},
    settings::Settings,
    theme::Theme,
    ui::{DiffView, Navigation, Sidebar},
};
use anyhow::Result;
use futures::future::join_all;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusedPane {
    Sidebar,
    DiffView,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
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
    pub input_mode: InputMode,
    /// Cache of commit files indexed by commit SHA
    commit_files_cache: HashMap<String, Vec<FileChange>>,
    /// All files changed in the PR (fetched once)
    pr_files: Option<Vec<FileChange>>,
    /// Cache for diff contents
    diff_cache: DiffCache,
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
            input_mode: InputMode::Normal,
            commit_files_cache: HashMap::new(),
            pr_files: None,
            diff_cache: DiffCache::new(50),
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

        // Fetch all PR files once (for optimization)
        loading_status.update_step(3, LoadingStepStatus::InProgress);
        loading_status.set_current_message("Fetching all PR file changes...".to_string());
        self.state = AppState::Loading(loading_status.clone());

        let pr_files = self
            .client
            .get_pr_files(&self.owner, &self.repo, self.pr_number)
            .await?;
        self.pr_files = Some(pr_files);

        loading_status.update_step(3, LoadingStepStatus::Completed);

        // Pre-fetch commit files for better performance
        if !self.commits.is_empty() {
            loading_status.update_step(4, LoadingStepStatus::InProgress);
            loading_status.set_current_message("Pre-fetching commit files...".to_string());
            self.state = AppState::Loading(loading_status.clone());

            // Pre-fetch first few commits in parallel for faster initial experience
            // We'll fetch the rest in the background
            self.prefetch_commit_files_parallel(5).await?;

            // Now load the first commit's files for display
            loading_status.set_current_message("Loading first commit...".to_string());
            self.state = AppState::Loading(loading_status.clone());
            self.load_commit_files(0).await?;

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

    pub async fn handle_navigate_up(&mut self) -> Result<()> {
        match self.focused_pane {
            FocusedPane::Sidebar => {
                let selected_index = if let Some(ref mut sidebar) = self.sidebar {
                    sidebar.previous();
                    sidebar.get_selected_index()
                } else {
                    None
                };

                if let Some(index) = selected_index {
                    // Always load the file's diff content when navigating
                    self.load_file_diff(index).await?;
                    if let Some(ref sidebar) = self.sidebar {
                        if let Some(file) = sidebar.get_selected_file() {
                            self.diff_view.set_file(Some(file.clone()));
                        }
                    }
                }
            }
            FocusedPane::DiffView => {
                self.diff_view.scroll_up(1);
            }
        }
        Ok(())
    }

    pub async fn handle_navigate_down(&mut self) -> Result<()> {
        match self.focused_pane {
            FocusedPane::Sidebar => {
                let selected_index = if let Some(ref mut sidebar) = self.sidebar {
                    sidebar.next();
                    sidebar.get_selected_index()
                } else {
                    None
                };

                if let Some(index) = selected_index {
                    // Always load the file's diff content when navigating
                    self.load_file_diff(index).await?;
                    if let Some(ref sidebar) = self.sidebar {
                        if let Some(file) = sidebar.get_selected_file() {
                            self.diff_view.set_file(Some(file.clone()));
                        }
                    }
                }
            }
            FocusedPane::DiffView => {
                self.diff_view.scroll_down(1);
            }
        }
        Ok(())
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

            // Check if we have this commit cached
            let commit_sha = &self.commits[current_index].sha;
            let is_cached = self.commit_files_cache.contains_key(commit_sha);

            // Only show loading status if we need to fetch from API
            if !is_cached {
                let mut loading_status = LoadingStatus::new();
                loading_status.update_step(0, LoadingStepStatus::Completed);
                loading_status.update_step(1, LoadingStepStatus::Completed);
                loading_status.update_step(2, LoadingStepStatus::Completed);
                loading_status.update_step(3, LoadingStepStatus::InProgress);
                let index = current_index + 1;
                let total = self.commits.len();
                loading_status.set_current_message(format!("Loading commit {index} of {total}..."));
                self.state = AppState::Loading(loading_status);
            }

            self.load_commit_files(current_index).await?;

            // Only reset state if we showed loading
            if !is_cached {
                self.state = AppState::Ready;
            }
        }
        Ok(())
    }

    pub async fn load_commit_files(&mut self, commit_index: usize) -> Result<()> {
        if commit_index >= self.commits.len() {
            return Ok(());
        }

        let commit = &self.commits[commit_index];

        // First, check if we have cached files for this commit
        let files = if let Some(cached_files) = self.commit_files_cache.get(&commit.sha) {
            // Use cached files (instant!)
            cached_files.clone()
        } else if commit_index == self.commits.len() - 1 && self.pr_files.is_some() {
            // For the last commit (all changes in PR), use PR files directly
            let pr_files = self.pr_files.as_ref().unwrap().clone();
            self.commit_files_cache
                .insert(commit.sha.clone(), pr_files.clone());
            pr_files
        } else {
            // Need to fetch from API (only as last resort)
            let fetched_files = self
                .client
                .get_commit_files(&self.owner, &self.repo, &commit.sha)
                .await?;

            // Cache for future use
            self.commit_files_cache
                .insert(commit.sha.clone(), fetched_files.clone());
            fetched_files
        };

        // Store files without enriching them yet (lazy loading)
        self.files = files.clone();

        // Reuse existing sidebar if possible, otherwise create new one
        if let Some(ref mut sidebar) = self.sidebar {
            sidebar.update_files(files);
        } else {
            self.sidebar = Some(Sidebar::new(files));
        }

        // Select the first file in the diff view (for UI display)
        // The file should already have a patch from the API, so it will show something
        // The full diff_content will be loaded lazily when user navigates
        if let Some(ref sidebar) = self.sidebar {
            if let Some(file) = sidebar.get_selected_file() {
                self.diff_view.set_file(Some(file.clone()));
            } else {
                self.diff_view.set_file(None);
            }
        }

        Ok(())
    }

    /// Load diff content for a specific file on demand
    pub async fn load_file_diff(&mut self, file_index: usize) -> Result<()> {
        if file_index >= self.files.len() {
            return Ok(());
        }

        // Check if already loaded
        if self.files[file_index].diff_content.is_some() {
            return Ok(());
        }

        if let Some(ref pr) = self.pr {
            if let Some(ref nav) = self.navigation {
                let commit_index = nav.get_current_index();
                let commit = &self.commits[commit_index];

                let base_sha = if commit_index == 0 {
                    pr.base.sha.clone()
                } else {
                    self.commits[commit_index - 1].sha.clone()
                };

                // Check diff cache first
                let cache_key = crate::cache::DiffCacheKey {
                    owner: self.owner.clone(),
                    repo: self.repo.clone(),
                    path: self.files[file_index].filename.clone(),
                    base_sha: base_sha.clone(),
                    head_sha: commit.sha.clone(),
                };

                if let Some(cached_diff) = self.diff_cache.get(&cache_key).await {
                    // Use cached diff
                    self.files[file_index].diff_content = Some(cached_diff);
                } else {
                    // Calculate diff and cache it
                    DiffParser::enrich_single_file(
                        &mut self.files[file_index],
                        &self.client,
                        &self.owner,
                        &self.repo,
                        &base_sha,
                        &commit.sha,
                    )
                    .await?;

                    // Cache the diff for future use
                    if let Some(ref diff) = self.files[file_index].diff_content {
                        self.diff_cache.put(cache_key, diff.clone()).await;
                    }
                }

                // Update sidebar with the enriched file
                if let Some(ref mut sidebar) = self.sidebar {
                    sidebar.update_file(file_index, self.files[file_index].clone());
                }
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

    /// Pre-fetch commit files in parallel for faster navigation
    async fn prefetch_commit_files_parallel(&mut self, max_parallel: usize) -> Result<()> {
        let commits_to_fetch: Vec<_> = self
            .commits
            .iter()
            .enumerate()
            .filter(|(_, commit)| !self.commit_files_cache.contains_key(&commit.sha))
            .take(max_parallel)
            .collect();

        if commits_to_fetch.is_empty() {
            return Ok(());
        }

        // Prepare data for parallel fetching
        let client = self.client.clone();
        let owner = self.owner.clone();
        let repo = self.repo.clone();
        let pr_files = self.pr_files.clone();
        let total_commits = self.commits.len();

        let futures: Vec<_> = commits_to_fetch
            .into_iter()
            .map(|(idx, commit)| {
                let client = client.clone();
                let owner = owner.clone();
                let repo = repo.clone();
                let sha = commit.sha.clone();
                let pr_files = pr_files.clone();
                let is_last = idx == total_commits - 1;

                async move {
                    // For the last commit, use PR files if available
                    if is_last {
                        if let Some(files) = pr_files {
                            return Ok((sha, files));
                        }
                    }

                    match client.get_commit_files(&owner, &repo, &sha).await {
                        Ok(files) => Ok((sha, files)),
                        Err(e) => {
                            eprintln!("Failed to pre-fetch commit {}: {}", &sha, e);
                            Err(e)
                        }
                    }
                }
            })
            .collect();

        // Execute all futures in parallel
        let results = join_all(futures).await;

        // Store successful results in cache
        for (sha, files) in results.into_iter().flatten() {
            self.commit_files_cache.insert(sha, files);
        }

        Ok(())
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
