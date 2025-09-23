use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub user: User,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub head: Branch,
    pub base: Branch,
    pub commits: u32,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub label: String,
    pub r#ref: String,
    pub sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub sha: String,
    pub commit: CommitDetail,
    pub author: Option<User>,
    pub committer: Option<User>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDetail {
    pub message: String,
    pub author: CommitAuthor,
    pub committer: CommitAuthor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitAuthor {
    pub name: String,
    pub email: String,
    pub date: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct FileChange {
    pub filename: String,
    pub status: FileStatus,
    pub additions: u32,
    pub deletions: u32,
    pub patch: Option<String>,
    pub raw_content: Option<String>,
    pub diff_content: Option<DiffContent>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
}

#[derive(Debug, Clone)]
pub struct DiffContent {
    pub hunks: Vec<DiffHunk>,
    /// Full file content with inline diff annotations
    pub full_file_view: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub line_type: LineType,
    pub content: String,
    pub old_line_no: Option<usize>,
    pub new_line_no: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LineType {
    Addition,
    Deletion,
    Context,
    Header,
}

pub struct ParsedPrUrl {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}
