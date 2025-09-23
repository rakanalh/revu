use super::models::*;
use crate::cache::{FileCacheKey, FileContentCache};
use anyhow::{Context, Result};
use octocrab::Octocrab;
use regex::Regex;
use reqwest;

#[derive(Clone)]
pub struct GitHubClient {
    client: Option<Octocrab>,
    token: Option<String>,
    cache: FileContentCache,
}

impl GitHubClient {
    pub async fn new(token: Option<String>) -> Result<Self> {
        let client = if let Some(ref t) = token {
            let builder = Octocrab::builder().personal_token(t.clone());
            Some(builder.build().context("Failed to build Octocrab client")?)
        } else {
            None
        };

        Ok(Self {
            client,
            token,
            cache: FileContentCache::new(100),
        })
    }

    pub fn parse_pr_url(url: &str) -> Result<ParsedPrUrl> {
        // Handle direct PR number
        if let Ok(number) = url.parse::<u64>() {
            // Try to get from environment or use default
            let owner = std::env::var("GITHUB_OWNER").unwrap_or_else(|_| "owner".to_string());
            let repo = std::env::var("GITHUB_REPO").unwrap_or_else(|_| "repo".to_string());
            return Ok(ParsedPrUrl {
                owner,
                repo,
                number,
            });
        }

        // Parse GitHub PR URL
        let re = Regex::new(r"github\.com/([^/]+)/([^/]+)/pull/(\d+)")
            .context("Failed to create regex")?;

        let caps = re.captures(url).context("Invalid GitHub PR URL format")?;

        Ok(ParsedPrUrl {
            owner: caps[1].to_string(),
            repo: caps[2].to_string(),
            number: caps[3].parse()?,
        })
    }

    pub async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> Result<PullRequest> {
        let pr = match &self.client {
            Some(client) => client.pulls(owner, repo).get(number).await?,
            None => octocrab::instance().pulls(owner, repo).get(number).await?,
        };

        // Map octocrab types to our models
        Ok(PullRequest {
            number: pr.number,
            title: pr.title.unwrap_or_default(),
            body: pr.body,
            state: pr
                .state
                .map(|s| format!("{s:?}"))
                .unwrap_or_else(|| "unknown".to_string()),
            user: User {
                login: pr
                    .user
                    .as_ref()
                    .map(|u| u.login.clone())
                    .unwrap_or_default(),
                avatar_url: pr
                    .user
                    .as_ref()
                    .map(|u| u.avatar_url.to_string())
                    .unwrap_or_default(),
            },
            created_at: pr.created_at.unwrap_or_else(chrono::Utc::now),
            updated_at: pr.updated_at.unwrap_or_else(chrono::Utc::now),
            head: Branch {
                label: pr.head.label.unwrap_or_default(),
                r#ref: pr.head.ref_field.clone(),
                sha: pr.head.sha.clone(),
            },
            base: Branch {
                label: pr.base.label.unwrap_or_default(),
                r#ref: pr.base.ref_field.clone(),
                sha: pr.base.sha.clone(),
            },
            commits: pr.commits.unwrap_or(0) as u32,
            additions: pr.additions.unwrap_or(0) as u32,
            deletions: pr.deletions.unwrap_or(0) as u32,
            changed_files: pr.changed_files.unwrap_or(0) as u32,
        })
    }

    pub async fn get_pr_commits(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> Result<Vec<Commit>> {
        // Fetch actual commits from the PR
        let commits = match &self.client {
            Some(client) => {
                client
                    .pulls(owner, repo)
                    .pr_commits(number)
                    .per_page(250) // Get up to 250 commits
                    .send()
                    .await?
            }
            None => {
                // Use anonymous client
                let client = octocrab::instance();
                client
                    .pulls(owner, repo)
                    .pr_commits(number)
                    .per_page(250)
                    .send()
                    .await?
            }
        };

        let mut result = Vec::new();
        for commit in commits {
            result.push(Commit {
                sha: commit.sha.clone(),
                commit: CommitDetail {
                    message: commit.commit.message.clone(),
                    author: CommitAuthor {
                        name: commit
                            .commit
                            .author
                            .as_ref()
                            .map(|a| a.name.clone())
                            .unwrap_or_default(),
                        email: commit
                            .commit
                            .author
                            .as_ref()
                            .map(|a| a.email.clone())
                            .unwrap_or_default(),
                        date: commit
                            .commit
                            .author
                            .as_ref()
                            .and_then(|a| a.date)
                            .unwrap_or_else(chrono::Utc::now),
                    },
                    committer: CommitAuthor {
                        name: commit
                            .commit
                            .committer
                            .as_ref()
                            .map(|c| c.name.clone())
                            .unwrap_or_default(),
                        email: commit
                            .commit
                            .committer
                            .as_ref()
                            .map(|c| c.email.clone())
                            .unwrap_or_default(),
                        date: commit
                            .commit
                            .committer
                            .as_ref()
                            .and_then(|c| c.date)
                            .unwrap_or_else(chrono::Utc::now),
                    },
                },
                author: commit.author.as_ref().map(|a| User {
                    login: a.login.clone(),
                    avatar_url: a.avatar_url.to_string(),
                }),
                committer: commit.committer.as_ref().map(|c| User {
                    login: c.login.clone(),
                    avatar_url: c.avatar_url.to_string(),
                }),
            });
        }

        Ok(result)
    }

    pub async fn get_pr_files(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> Result<Vec<FileChange>> {
        let files = match &self.client {
            Some(client) => client.pulls(owner, repo).list_files(number).await?,
            None => {
                octocrab::instance()
                    .pulls(owner, repo)
                    .list_files(number)
                    .await?
            }
        };

        let mut result = Vec::new();
        for file in files {
            use octocrab::models::repos::DiffEntryStatus;
            let status = match file.status {
                DiffEntryStatus::Added => FileStatus::Added,
                DiffEntryStatus::Removed => FileStatus::Deleted,
                DiffEntryStatus::Modified => FileStatus::Modified,
                DiffEntryStatus::Renamed => FileStatus::Renamed,
                DiffEntryStatus::Copied => FileStatus::Copied,
                _ => FileStatus::Modified,
            };

            result.push(FileChange {
                filename: file.filename.clone(),
                status,
                additions: file.additions as u32,
                deletions: file.deletions as u32,
                patch: file.patch.clone(),
                raw_content: None,
                diff_content: None,
            });
        }

        Ok(result)
    }

    pub async fn get_file_content(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
        r#ref: &str,
    ) -> Result<String> {
        // Check cache first
        let cache_key = FileCacheKey {
            owner: owner.to_string(),
            repo: repo.to_string(),
            path: path.to_string(),
            sha: r#ref.to_string(),
        };

        if let Some(cached_content) = self.cache.get(&cache_key).await {
            return Ok(cached_content);
        }

        // Not in cache, fetch from GitHub
        let url = format!(
            "https://raw.githubusercontent.com/{owner}/{repo}/{ref}/{path}",
            r#ref = r#ref
        );

        let client = reqwest::Client::new();
        let mut request = client.get(&url);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let response = request
            .send()
            .await
            .context("Failed to fetch file content")?;

        if !response.status().is_success() {
            // File might not exist in this ref (e.g., deleted file)
            return Ok(String::new());
        }

        let content = response
            .text()
            .await
            .context("Failed to read file content")?;

        // Cache the content
        self.cache.put(cache_key, content.clone()).await;

        Ok(content)
    }

    pub async fn get_commit_files(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> Result<Vec<FileChange>> {
        // Use the GitHub API directly to fetch commit details
        let url = format!("https://api.github.com/repos/{owner}/{repo}/commits/{sha}");

        let client = reqwest::Client::new();
        let mut request = client
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "revu");

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let response = request.send().await?;
        let commit_data: serde_json::Value = response.json().await?;

        let mut result = Vec::new();
        if let Some(files) = commit_data["files"].as_array() {
            for file in files {
                let status_str = file["status"].as_str().unwrap_or("modified");
                let status = match status_str {
                    "added" => FileStatus::Added,
                    "removed" => FileStatus::Deleted,
                    "modified" => FileStatus::Modified,
                    "renamed" => FileStatus::Renamed,
                    "copied" => FileStatus::Copied,
                    _ => FileStatus::Modified,
                };

                result.push(FileChange {
                    filename: file["filename"].as_str().unwrap_or("").to_string(),
                    status,
                    additions: file["additions"].as_u64().unwrap_or(0) as u32,
                    deletions: file["deletions"].as_u64().unwrap_or(0) as u32,
                    patch: file["patch"].as_str().map(|s| s.to_string()),
                    raw_content: None,
                    diff_content: None,
                });
            }
        }

        Ok(result)
    }
}
