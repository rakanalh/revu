use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// Represents authentication credentials from .authinfo/.netrc
#[derive(Debug, Clone)]
struct AuthInfo {
    machine: String,
    password: String,
}

/// Attempts to find GitHub token from multiple sources with priority ordering
pub fn get_github_token(cli_token: Option<String>) -> Result<Option<String>> {
    // 1. First priority: Command-line argument
    if let Some(token) = cli_token {
        return Ok(Some(token));
    }

    // 2. Second priority: ~/.authinfo or ~/.netrc file
    match read_authinfo_token() {
        Ok(Some(token)) => return Ok(Some(token)),
        Ok(None) => {
            // File exists but no matching entry found
            if std::env::var("REVU_DEBUG").is_ok() {
                eprintln!("Debug: authinfo file found but no entry for machine api.github.com with login ending in ^revu");
            }
        }
        Err(e) => {
            // Error reading file
            if std::env::var("REVU_DEBUG").is_ok() {
                eprintln!("Debug: Error reading authinfo: {}", e);
            }
        }
    }

    // 3. Third priority: GITHUB_TOKEN environment variable
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        return Ok(Some(token));
    }

    Ok(None)
}

/// Reads GitHub token from ~/.authinfo or ~/.netrc file
/// Looks for entries matching: machine api.github.com login USERNAME password TOKEN
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

        // Parse all entries and find the one we want
        let entries = parse_all_authinfo(&contents)?;
        for auth in entries {
            // Just look for api.github.com, regardless of login suffix
            if auth.machine == "api.github.com" {
                return Ok(Some(auth.password));
            }
        }

        if std::env::var("REVU_DEBUG").is_ok() {
            eprintln!("Debug: Found {} entries in {}, but none match api.github.com",
                     parse_all_authinfo(&contents)?.len(), path.display());
        }
    }

    Ok(None)
}

/// Parses all entries from .authinfo/.netrc file format
/// Format: machine HOSTNAME login USERNAME password PASSWORD
fn parse_all_authinfo(contents: &str) -> Result<Vec<AuthInfo>> {
    let mut entries = Vec::new();
    let mut machine: Option<String> = None;
    let mut has_login = false;
    let mut password: Option<String> = None;
    let mut current_field: Option<&str> = None;

    for token in contents.split_whitespace() {
        match token {
            "machine" => {
                // If we have a complete entry, save it
                if let (Some(m), Some(p)) = (&machine, &password) {
                    if has_login {
                        entries.push(AuthInfo {
                            machine: m.clone(),
                            password: p.clone(),
                        });
                    }
                }
                // Start new entry
                machine = None;
                has_login = false;
                password = None;
                current_field = Some("machine");
            }
            "login" => {
                has_login = true;
                current_field = Some("login");
            }
            "password" => current_field = Some("password"),
            _ => {
                match current_field {
                    Some("machine") => machine = Some(token.to_string()),
                    Some("login") => {}, // Skip the login value, we don't need it
                    Some("password") => password = Some(token.to_string()),
                    _ => {}
                }
            }
        }
    }

    // Don't forget the last entry
    if let (Some(m), Some(p)) = (machine, password) {
        if has_login {
            entries.push(AuthInfo {
                machine: m,
                password: p,
            });
        }
    }

    Ok(entries)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_authinfo_basic() {
        let content = "machine api.github.com login myuser password ghp_token123";
        let result = parse_all_authinfo(content).unwrap();
        assert_eq!(result.len(), 1);
        let auth = &result[0];
        assert_eq!(auth.machine, "api.github.com");
        assert_eq!(auth.password, "ghp_token123");
    }

    #[test]
    fn test_parse_authinfo_multiple_entries() {
        let content = r#"
            machine example.com login user1 password pass1
            machine api.github.com login myuser password ghp_token123
            machine other.com login user2 password pass2
        "#;
        let result = parse_all_authinfo(content).unwrap();
        assert_eq!(result.len(), 3);
        // Find the GitHub entry
        let github_auth = result.iter().find(|a| a.machine == "api.github.com").unwrap();
        assert_eq!(github_auth.password, "ghp_token123");
    }

    #[test]
    fn test_parse_authinfo_multiline() {
        let content = r#"
machine api.github.com
login myuser
password ghp_token123
"#;
        let result = parse_all_authinfo(content).unwrap();
        assert_eq!(result.len(), 1);
        let auth = &result[0];
        assert_eq!(auth.machine, "api.github.com");
        assert_eq!(auth.password, "ghp_token123");
    }

    #[test]
    fn test_parse_authinfo_no_login() {
        // Entry without login field should be skipped
        let content = "machine api.github.com password ghp_token123";
        let result = parse_all_authinfo(content).unwrap();
        assert_eq!(result.len(), 0);
    }
}
