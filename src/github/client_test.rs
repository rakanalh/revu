#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_parse_pr_url() {
        // Test full URL parsing
        let result = GitHubClient::parse_pr_url("https://github.com/rust-lang/rust/pull/12345");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.owner, "rust-lang");
        assert_eq!(parsed.repo, "rust");
        assert_eq!(parsed.number, 12345);

        // Test PR number parsing
        std::env::set_var("GITHUB_OWNER", "test-owner");
        std::env::set_var("GITHUB_REPO", "test-repo");
        let result = GitHubClient::parse_pr_url("789");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.owner, "test-owner");
        assert_eq!(parsed.repo, "test-repo");
        assert_eq!(parsed.number, 789);
    }

    #[test]
    fn test_invalid_pr_url() {
        let result = GitHubClient::parse_pr_url("not-a-valid-url");
        assert!(result.is_err());
    }
}
