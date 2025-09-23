use crate::github::models::{DiffContent, DiffHunk, DiffLine, FileChange, FileStatus, LineType};
use anyhow::Result;
use regex::Regex;

pub struct DiffParser;

impl DiffParser {
    pub fn parse_unified_diff(diff_content: &str) -> Result<DiffContent> {
        let lines: Vec<&str> = diff_content.lines().collect();
        let mut hunks = Vec::new();
        let mut all_lines = Vec::new();
        let mut current_hunk: Option<DiffHunk> = None;
        let mut old_line_no = 0;
        let mut new_line_no = 0;

        let hunk_header_re = Regex::new(r"^@@\s+-(\d+),?(\d*)\s+\+(\d+),?(\d*)\s+@@(.*)$")?;

        for line in lines {
            if let Some(caps) = hunk_header_re.captures(line) {
                // Save previous hunk if exists
                if let Some(hunk) = current_hunk.take() {
                    hunks.push(hunk);
                }

                // Update line numbers for hunk tracking
                old_line_no = caps[1].parse::<usize>()?.saturating_sub(1);
                new_line_no = caps[3].parse::<usize>()?.saturating_sub(1);

                current_hunk = Some(DiffHunk {
                    lines: vec![DiffLine {
                        line_type: LineType::Header,
                        content: line.to_string(),
                        old_line_no: None,
                        new_line_no: None,
                    }],
                });
            } else if let Some(ref mut hunk) = current_hunk {
                let (line_type, content, old_no, new_no) = if line.starts_with('+') {
                    new_line_no += 1;
                    (
                        LineType::Addition,
                        line.strip_prefix('+').unwrap_or("").to_string(),
                        None,
                        Some(new_line_no),
                    )
                } else if line.starts_with('-') {
                    old_line_no += 1;
                    (
                        LineType::Deletion,
                        line.strip_prefix('-').unwrap_or("").to_string(),
                        Some(old_line_no),
                        None,
                    )
                } else if line.starts_with(' ') {
                    old_line_no += 1;
                    new_line_no += 1;
                    (
                        LineType::Context,
                        line.strip_prefix(' ').unwrap_or("").to_string(),
                        Some(old_line_no),
                        Some(new_line_no),
                    )
                } else {
                    // Context line without prefix
                    old_line_no += 1;
                    new_line_no += 1;
                    (
                        LineType::Context,
                        line.to_string(),
                        Some(old_line_no),
                        Some(new_line_no),
                    )
                };

                let diff_line = DiffLine {
                    line_type: line_type.clone(),
                    content: content.clone(),
                    old_line_no: old_no,
                    new_line_no: new_no,
                };

                hunk.lines.push(diff_line.clone());
                all_lines.push(diff_line);
            }
        }

        // Save last hunk if exists
        if let Some(hunk) = current_hunk {
            hunks.push(hunk);
        }

        Ok(DiffContent {
            hunks,
            full_file_view: Vec::new(), // Will be populated by create_full_file_diff
        })
    }

    /// Creates a full file view with inline diff annotations
    pub fn create_full_file_diff(
        old_content: &str,
        new_content: &str,
        patch: &str,
    ) -> Result<DiffContent> {
        use similar::{ChangeTag, TextDiff};

        let _old_lines: Vec<&str> = old_content.lines().collect();
        let _new_lines: Vec<&str> = new_content.lines().collect();

        let diff = TextDiff::from_lines(old_content, new_content);
        let mut full_file_view = Vec::new();
        let mut hunks = Vec::new();

        // Build the full file view with inline diff
        let mut current_new_line = 0;
        let mut current_old_line = 0;

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal => {
                    // Unchanged line - show it as context
                    current_old_line += 1;
                    current_new_line += 1;
                    full_file_view.push(DiffLine {
                        line_type: LineType::Context,
                        content: change.value().trim_end().to_string(),
                        old_line_no: Some(current_old_line),
                        new_line_no: Some(current_new_line),
                    });
                }
                ChangeTag::Delete => {
                    // Line was removed - show it in red
                    current_old_line += 1;
                    full_file_view.push(DiffLine {
                        line_type: LineType::Deletion,
                        content: change.value().trim_end().to_string(),
                        old_line_no: Some(current_old_line),
                        new_line_no: None,
                    });
                }
                ChangeTag::Insert => {
                    // Line was added - show it in green
                    current_new_line += 1;
                    full_file_view.push(DiffLine {
                        line_type: LineType::Addition,
                        content: change.value().trim_end().to_string(),
                        old_line_no: None,
                        new_line_no: Some(current_new_line),
                    });
                }
            }
        }

        // If we have a patch, also parse it to get hunks (for navigation)
        if !patch.is_empty() {
            if let Ok(parsed) = Self::parse_unified_diff(patch) {
                hunks = parsed.hunks;
            }
        }

        Ok(DiffContent {
            hunks,
            full_file_view,
        })
    }

    pub async fn enrich_file_changes(
        files: &mut [FileChange],
        client: &crate::github::GitHubClient,
        owner: &str,
        repo: &str,
        base_ref: &str,
        head_ref: &str,
    ) -> Result<()> {
        for file in files.iter_mut() {
            // Get file content from both refs
            let old_content = if file.status != FileStatus::Added {
                client
                    .get_file_content(owner, repo, &file.filename, base_ref)
                    .await
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let new_content = if file.status != FileStatus::Deleted {
                client
                    .get_file_content(owner, repo, &file.filename, head_ref)
                    .await
                    .unwrap_or_default()
            } else {
                String::new()
            };

            // Generate full file diff view
            let diff_content = if let Some(ref patch) = file.patch {
                Self::create_full_file_diff(&old_content, &new_content, patch)?
            } else {
                Self::create_full_file_diff(&old_content, &new_content, "")?
            };

            file.raw_content = Some(new_content.clone());
            file.diff_content = Some(diff_content);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_file_diff_creation() {
        let old_content = "line 1\nline 2\nline 3\nline 4\nline 5";
        let new_content = "line 1\nline 2 modified\nline 3\nnew line\nline 5";

        let result = DiffParser::create_full_file_diff(old_content, new_content, "").unwrap();

        // Check that we have a full file view
        assert!(!result.full_file_view.is_empty());

        // Check that we have all lines represented
        let total_lines = result.full_file_view.len();
        assert!(total_lines > 0);

        // Check that we have different line types
        let has_context = result
            .full_file_view
            .iter()
            .any(|l| matches!(l.line_type, LineType::Context));
        let has_addition = result
            .full_file_view
            .iter()
            .any(|l| matches!(l.line_type, LineType::Addition));
        let has_deletion = result
            .full_file_view
            .iter()
            .any(|l| matches!(l.line_type, LineType::Deletion));

        assert!(has_context, "Should have context lines");
        assert!(has_addition, "Should have addition lines");
        assert!(has_deletion, "Should have deletion lines");

        // Check line numbering
        for line in &result.full_file_view {
            match line.line_type {
                LineType::Context => {
                    assert!(
                        line.old_line_no.is_some() && line.new_line_no.is_some(),
                        "Context lines should have both line numbers"
                    );
                }
                LineType::Addition => {
                    assert!(
                        line.old_line_no.is_none() && line.new_line_no.is_some(),
                        "Addition lines should only have new line number"
                    );
                }
                LineType::Deletion => {
                    assert!(
                        line.old_line_no.is_some() && line.new_line_no.is_none(),
                        "Deletion lines should only have old line number"
                    );
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_full_file_diff_with_no_changes() {
        let content = "line 1\nline 2\nline 3";

        let result = DiffParser::create_full_file_diff(content, content, "").unwrap();

        // All lines should be context
        assert!(!result.full_file_view.is_empty());
        assert!(result
            .full_file_view
            .iter()
            .all(|l| matches!(l.line_type, LineType::Context)));

        // Should have 3 lines
        assert_eq!(result.full_file_view.len(), 3);
    }
}
