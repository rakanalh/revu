use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// Represents authentication credentials from .authinfo/.netrc
#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub machine: String,
    pub login: String,
    pub password: String,
}

/// Attempts to find GitHub token from multiple sources with priority ordering
pub fn get_github_token(cli_token: Option<String>) -> Result<Option<String>> {
    // 1. First priority: Command-line argument
    if let Some(token) = cli_token {
        return Ok(Some(token));
    }

    // 2. Second priority: ~/.authinfo or ~/.netrc file
    if let Ok(Some(token)) = read_authinfo_token() {
        return Ok(Some(token));
    }

    // 3. Third priority: GITHUB_TOKEN environment variable
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        return Ok(Some(token));
    }

    Ok(None)
}

/// Reads GitHub token from ~/.authinfo or ~/.netrc file
/// Looks for entries matching: machine api.github.com login USERNAME^revu password TOKEN
fn read_authinfo_token() -> Result<Option<String>> {
    // Try ~/.authinfo first, then ~/.netrc
    let home = std::env::var("HOME").context("HOME environment variable not set")?;
    let paths = vec![
        PathBuf::from(&home).join(".authinfo"),
        PathBuf::from(&home).join(".netrc"),
    ];

    for path in paths {
        if !path.exists() {
            continue;
        }

        // Check file permissions (should be 600 or 400)
        #[cfg(unix)]
        {
            let metadata = fs::metadata(&path)?;
            let permissions = metadata.permissions();
            let mode = permissions.mode() & 0o777;

            if mode != 0o600 && mode != 0o400 {
                eprintln!(
                    "Warning: {} has permissions {:o} (should be 600 or 400)",
                    path.display(),
                    mode
                );
                eprintln!("Fix with: chmod 600 {}", path.display());
                // Continue anyway but warn the user
            }
        }

        // Read and parse the file
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        if let Some(auth) = parse_authinfo(&contents)? {
            if auth.machine == "api.github.com" && auth.login.ends_with("^revu") {
                return Ok(Some(auth.password));
            }
        }
    }

    Ok(None)
}

/// Parses .authinfo/.netrc file format
/// Format: machine HOSTNAME login USERNAME password PASSWORD
fn parse_authinfo(contents: &str) -> Result<Option<AuthInfo>> {
    let mut machine: Option<String> = None;
    let mut login: Option<String> = None;
    let mut password: Option<String> = None;
    let mut current_field: Option<&str> = None;

    for token in contents.split_whitespace() {
        match token {
            "machine" => current_field = Some("machine"),
            "login" => current_field = Some("login"),
            "password" => current_field = Some("password"),
            _ => {
                match current_field {
                    Some("machine") => {
                        // If we already have a complete entry, check if it's what we want
                        if machine.is_some() && login.is_some() && password.is_some() {
                            if let (Some(m), Some(l), Some(p)) = (&machine, &login, &password) {
                                if m == "api.github.com" && l.ends_with("^revu") {
                                    return Ok(Some(AuthInfo {
                                        machine: m.clone(),
                                        login: l.clone(),
                                        password: p.clone(),
                                    }));
                                }
                            }
                        }
                        // Start new entry
                        machine = Some(token.to_string());
                        login = None;
                        password = None;
                    }
                    Some("login") => login = Some(token.to_string()),
                    Some("password") => password = Some(token.to_string()),
                    _ => {}
                }
                current_field = None;
            }
        }
    }

    // Check the last entry if we have one
    if let (Some(m), Some(l), Some(p)) = (machine, login, password) {
        if m == "api.github.com" && l.ends_with("^revu") {
            return Ok(Some(AuthInfo {
                machine: m,
                login: l,
                password: p,
            }));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_authinfo_basic() {
        let content = "machine api.github.com login myuser^revu password ghp_token123";
        let result = parse_authinfo(content).unwrap();
        assert!(result.is_some());
        let auth = result.unwrap();
        assert_eq!(auth.machine, "api.github.com");
        assert_eq!(auth.login, "myuser^revu");
        assert_eq!(auth.password, "ghp_token123");
    }

    #[test]
    fn test_parse_authinfo_multiple_entries() {
        let content = r#"
            machine example.com login user1 password pass1
            machine api.github.com login myuser^revu password ghp_token123
            machine other.com login user2 password pass2
        "#;
        let result = parse_authinfo(content).unwrap();
        assert!(result.is_some());
        let auth = result.unwrap();
        assert_eq!(auth.machine, "api.github.com");
        assert_eq!(auth.login, "myuser^revu");
        assert_eq!(auth.password, "ghp_token123");
    }

    #[test]
    fn test_parse_authinfo_no_revu_suffix() {
        let content = "machine api.github.com login myuser password ghp_token123";
        let result = parse_authinfo(content).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_authinfo_wrong_machine() {
        let content = "machine github.com login myuser^revu password ghp_token123";
        let result = parse_authinfo(content).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_authinfo_multiline() {
        let content = r#"
machine api.github.com
login myuser^revu
password ghp_token123
"#;
        let result = parse_authinfo(content).unwrap();
        assert!(result.is_some());
        let auth = result.unwrap();
        assert_eq!(auth.machine, "api.github.com");
        assert_eq!(auth.login, "myuser^revu");
        assert_eq!(auth.password, "ghp_token123");
    }
}
